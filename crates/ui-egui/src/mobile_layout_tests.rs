//! The layout each window size class gets (`docs/mobile-layout.md`), drawn headless at phone and
//! tablet sizes: a phone held upright opens its panels in a sheet from the bottom edge, under the
//! canvas, and the other three classes keep the desktop layout until their own slices land.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect, Vec2, pos2, vec2};
use figcraft_engine::Session;
use serde_json::{Value, json};

use crate::app::phone;
use crate::layers_drag_tests::Ui;
use crate::live_tests::{flat, texts};

const PHONE: Vec2 = vec2(390.0, 844.0);

/// A file with one rectangle, in a window `size` points large; and the rectangle's layer name.
fn at(size: Vec2) -> (Ui, String) {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 300, "height": 200})).unwrap();
    let id = figcraft_engine::params::id_of(&r["id"]).unwrap();
    let name = s.doc().unwrap().node(id).unwrap().name.clone();
    s.execute("select.set", &json!({"ids": []})).unwrap();
    let mut u = Ui::new(s);
    resize(&mut u, size);
    (u, name)
}

fn resize(u: &mut Ui, size: Vec2) {
    u.screen = size;
    for _ in 0..4 {
        u.frame(vec![]);
    }
}

/// A tap (press and release without moving) at `p`, and the frames that follow.
fn tap(u: &mut Ui, p: Pos2) {
    for _ in 0..3 {
        u.frame(vec![Event::PointerMoved(p)]);
    }
    for pressed in [true, false] {
        u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }]);
    }
    for _ in 0..4 {
        u.frame(vec![]);
    }
}

fn layout(u: &Ui) -> Value {
    u.app.inspect()["layout"].clone()
}

fn drawn(u: &mut Ui) -> Vec<String> {
    texts(&flat(&u.frame_output(vec![]).shapes)).into_iter().map(|(t, ..)| t).collect()
}

/// The sheet under the canvas, in a phone window.
fn sheet(u: &Ui) -> Rect {
    Rect::from_min_max(pos2(0.0, u.app.canvas.rect.max.y), pos2(u.screen.x, u.screen.y))
}

#[test]
fn a_phone_held_upright_opens_its_panels_in_a_sheet_under_the_canvas() {
    let (mut u, layer) = at(PHONE);
    assert_eq!(layout(&u), json!({"sizeClass": "phonePortrait", "sheet": null}));
    let canvas = u.app.canvas.rect;
    assert!(canvas.width() >= PHONE.x - 1.0 && canvas.max.y >= PHONE.y - 1.0, "the canvas has the window under the tab bar: {canvas:?}");
    assert!(u.app.ui.right_panel_rect.is_none(), "no panel beside it");
    assert!(!drawn(&mut u).contains(&layer), "the layers list isn't drawn");

    // The Layers button at the canvas's top corner opens the sheet with the layers list.
    let [layers_button, design_button] = phone::canvas_buttons(canvas, false);
    tap(&mut u, layers_button.center());
    assert_eq!(layout(&u)["sheet"], "layers");
    let canvas = u.app.canvas.rect;
    assert!(canvas.width() >= PHONE.x - 1.0, "the sheet is under the canvas, not beside it: {canvas:?}");
    assert!(canvas.height() >= 200.0 && canvas.max.y < PHONE.y * 0.6, "the canvas keeps the top: {canvas:?}");
    assert!(drawn(&mut u).contains(&layer), "the sheet lists the layer");
    let inset = u.app.session.ds().unwrap().view.bottom_inset;
    assert!(inset > 0.0, "the toolbar stays on the canvas with the sheet open: {inset}");

    // Its header switches to the Design panel, which takes the sheet's width.
    let header = phone::sheet_header(sheet(&u), false);
    tap(&mut u, header.design.center());
    assert_eq!(layout(&u)["sheet"], "design");
    let panel = u.app.ui.right_panel_rect.unwrap();
    let below_header = u.app.canvas.rect.max.y + phone::SHEET_HEADER - 1.0;
    assert!(panel.min.y >= below_header && panel.max.y <= PHONE.y + 1.0, "the Design panel is under the sheet's header: {panel:?}");
    assert!(panel.min.x >= -1.0 && panel.max.x <= PHONE.x + 1.0 && panel.width() >= PHONE.x - 3.0, "and fills its width: {panel:?}");

    // Close gives the canvas the window back; the canvas button opens and closes the sheet too.
    tap(&mut u, header.close.center());
    assert_eq!(layout(&u)["sheet"], Value::Null);
    assert!(u.app.canvas.rect.max.y >= PHONE.y - 1.0);
    tap(&mut u, design_button.center());
    assert_eq!(layout(&u)["sheet"], "design");
    tap(&mut u, design_button.center());
    assert_eq!(layout(&u)["sheet"], Value::Null, "the open panel's button closes it again");
}

/// Phone landscape, tablet portrait and tablet landscape keep the desktop layout for now: both
/// panels beside the canvas, and no sheet even with a phone panel asked for.
#[test]
fn the_other_classes_keep_the_desktop_layout() {
    for (size, class) in [(vec2(844.0, 390.0), "phoneLandscape"), (vec2(820.0, 1180.0), "tabletPortrait"), (vec2(1180.0, 820.0), "tabletLandscape")] {
        let (mut u, layer) = at(size);
        u.app.ui.mobile_panel = Some(crate::MobilePanel::Left);
        resize(&mut u, size);
        assert_eq!(layout(&u), json!({"sizeClass": class, "sheet": null}), "{size:?}");
        let canvas = u.app.canvas.rect;
        let panel = u.app.ui.right_panel_rect.unwrap_or_else(|| panic!("{class}: the Design panel shows"));
        assert!(panel.min.x >= canvas.max.x - 1.0, "{class}: beside the canvas: {panel:?} {canvas:?}");
        assert!(canvas.max.y >= size.y - 1.0 && canvas.width() < size.x - 300.0, "{class}: {canvas:?}");
        assert!(drawn(&mut u).contains(&layer), "{class}: the layers panel shows too");
    }
}

/// Turning a phone sideways leaves the sheet for the desktop layout, and upright again the panel
/// that was open comes back in the sheet.
#[test]
fn a_rotation_moves_the_open_panel_out_of_the_sheet_and_back() {
    let (mut u, _) = at(PHONE);
    let layers_button = phone::canvas_buttons(u.app.canvas.rect, false)[0];
    tap(&mut u, layers_button.center());
    assert_eq!(layout(&u)["sheet"], "layers");
    resize(&mut u, vec2(PHONE.y, PHONE.x));
    assert_eq!(layout(&u), json!({"sizeClass": "phoneLandscape", "sheet": null}));
    assert!(u.app.ui.right_panel_rect.is_some_and(|p| p.min.x >= u.app.canvas.rect.max.x - 1.0), "sideways: the panels are beside the canvas");
    resize(&mut u, PHONE);
    assert_eq!(layout(&u), json!({"sizeClass": "phonePortrait", "sheet": "layers"}));
}

/// A window resized across the phone/tablet boundary changes layout once it is clearly past it,
/// not while it is within the band either side.
#[test]
fn the_layout_changes_only_past_the_band_around_a_boundary() {
    let (mut u, _) = at(PHONE);
    resize(&mut u, vec2(620.0, 900.0));
    assert_eq!(layout(&u)["sizeClass"], "phonePortrait", "620 wide: still a phone");
    assert!(u.app.canvas.rect.width() >= 619.0);
    resize(&mut u, vec2(700.0, 900.0));
    assert_eq!(layout(&u)["sizeClass"], "tabletPortrait", "700 wide: a tablet");
    resize(&mut u, vec2(620.0, 900.0));
    assert_eq!(layout(&u)["sizeClass"], "tabletPortrait", "back to 620: still a tablet");
}

/// Minimize has no sidebars to take from a phone: its canvas buttons stay and open the sheet.
#[test]
fn a_minimized_phone_still_opens_its_sheet() {
    let (mut u, layer) = at(PHONE);
    u.app.run("view.minimize", json!({"on": true}));
    resize(&mut u, PHONE);
    let layers_button = phone::canvas_buttons(u.app.canvas.rect, false)[0];
    tap(&mut u, layers_button.center());
    assert_eq!(layout(&u)["sheet"], "layers");
    assert!(u.app.canvas.rect.width() >= PHONE.x - 1.0 && drawn(&mut u).contains(&layer));
}

/// The Layers panel lays out beside the rail, inside the sheet: the button at the end of its
/// header (Minimize, by the file name) is on the screen and answers a tap. Measuring the whole
/// sheet put it 56 pt past the window's edge.
#[test]
fn the_layers_sheet_keeps_its_header_buttons_on_screen() {
    let (mut u, _) = at(PHONE);
    let layers_button = phone::canvas_buttons(u.app.canvas.rect, false)[0];
    tap(&mut u, layers_button.center());
    assert_eq!(layout(&u)["sheet"], "layers");
    let header_top = u.app.canvas.rect.max.y + phone::SHEET_HEADER;
    // The panel's right edge is the window's, less the sheet's 1 pt border; Minimize is 24 pt in.
    tap(&mut u, pos2(PHONE.x - 1.0 - 24.0, header_top + 1.0 + 24.0));
    assert!(u.app.session.ds().unwrap().view.ui_minimized, "the tap reached Minimize");
}
