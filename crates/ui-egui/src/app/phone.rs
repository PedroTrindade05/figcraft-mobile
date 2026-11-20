//! The phone-portrait layout (`docs/mobile-layout.md`): the canvas takes the whole window under
//! the tab bar, the toolbar floats at its bottom, and the Layers or Design panel opens in a sheet
//! from the bottom edge, under the canvas, opened from two buttons at the canvas's top corner.

use super::{FigApp, LeftTab, MobilePanel, a11y, i18n, icons, layers, right, right_tab_label, theme};

/// The smallest a touch target is, in points (Apple's and Google's guidance agree on about this).
pub const TOUCH_TARGET: f32 = 44.0;

/// The share of the space under the tab bar an open sheet takes; the canvas keeps the rest.
const SHEET_SHARE: f32 = 0.55;

/// The least the canvas keeps above an open sheet, and the least a sheet is tall.
const CANVAS_KEEPS: f32 = 200.0;
const SHEET_MIN: f32 = 240.0;

/// The grab handle's strip at the top of the sheet, above its header row.
const HANDLE_STRIP: f32 = 10.0;

/// How tall the sheet is in `room` points (the window under the tab bar): its share, at least
/// [`SHEET_MIN`] while the canvas keeps [`CANVAS_KEEPS`], never more than the room.
pub fn sheet_height(room: f32) -> f32 {
    if !room.is_finite() || room <= 0.0 {
        return 0.0;
    }
    (room * SHEET_SHARE).max(SHEET_MIN).min(room - CANVAS_KEEPS).max(0.0).min(room)
}

/// The sheet header's controls: the Layers and Design switches, and Close.
#[derive(Clone, Copy, Debug)]
pub struct SheetHeader {
    pub layers: egui::Rect,
    pub design: egui::Rect,
    pub close: egui::Rect,
}

/// Where the header's controls go in a sheet whose top edge is `sheet` (its full rect): the
/// switches from the start edge, Close at the end (both mirrored in a right-to-left window).
pub fn sheet_header(sheet: egui::Rect, mirrored: bool) -> SheetHeader {
    let top = sheet.min.y + HANDLE_STRIP;
    let switch = egui::vec2(112.0, TOUCH_TARGET);
    let (start, end) = (sheet.min.x + 8.0, sheet.max.x - 8.0);
    let at = |x: f32, w: f32| egui::Rect::from_min_size(egui::pos2(x, top), egui::vec2(w, TOUCH_TARGET));
    let (layers, design, close) = if mirrored {
        (at(end - switch.x, switch.x), at(end - 2.0 * switch.x - 4.0, switch.x), at(start, TOUCH_TARGET))
    } else {
        (at(start, switch.x), at(start + switch.x + 4.0, switch.x), at(end - TOUCH_TARGET, TOUCH_TARGET))
    };
    SheetHeader { layers, design, close }
}

/// The height of the sheet's handle strip and header row together.
pub const SHEET_HEADER: f32 = HANDLE_STRIP + TOUCH_TARGET + 6.0;

/// Where the two canvas buttons go: a column at the canvas's top end corner (top left in a
/// mirrored window), Layers first.
pub fn canvas_buttons(canvas: egui::Rect, mirrored: bool) -> [egui::Rect; 2] {
    let x = if mirrored { canvas.min.x + 8.0 } else { canvas.max.x - 8.0 - TOUCH_TARGET };
    let at = |k: f32| egui::Rect::from_min_size(egui::pos2(x, canvas.min.y + 8.0 + k * (TOUCH_TARGET + 8.0)), egui::vec2(TOUCH_TARGET, TOUCH_TARGET));
    [at(0.0), at(1.0)]
}

impl FigApp {
    /// The sheet with the panel `side`, along the bottom of `ui` (the window under the tab bar).
    pub(super) fn phone_sheet(&mut self, ui: &mut egui::Ui, side: MobilePanel) {
        let t = theme::Tokens::get(ui.ctx());
        let room = ui.available_rect_before_wrap().height();
        egui::Panel::bottom("phone_sheet")
            .resizable(false)
            .exact_size(sheet_height(room))
            .frame(egui::Frame::NONE.fill(t.panel).stroke(egui::Stroke::new(1.0, t.border)))
            .show(ui, |ui| {
                let sheet = ui.max_rect();
                self.sheet_header_ui(ui, sheet, side);
                ui.painter().hline(sheet.x_range(), sheet.min.y + SHEET_HEADER, egui::Stroke::new(1.0, t.border));
                // The panels lay out in their own rect (they read `max_rect`), under the header and,
                // for Layers, beside the rail.
                let body = egui::Rect::from_min_max(egui::pos2(sheet.min.x, sheet.min.y + SHEET_HEADER), sheet.max);
                let child = |ui: &mut egui::Ui, rect: egui::Rect, salt: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
                    ui.scope_builder(egui::UiBuilder::new().id_salt(salt).max_rect(rect), |ui| {
                        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
                        add(ui);
                    });
                };
                match side {
                    MobilePanel::Left => {
                        // The icon rail and the panel side by side, as on a wide window.
                        let start = !self.ui.mirrored;
                        let (rail, list) = if start {
                            (body.with_max_x(body.min.x + layers::RAIL_WIDTH), body.with_min_x(body.min.x + layers::RAIL_WIDTH))
                        } else {
                            (body.with_min_x(body.max.x - layers::RAIL_WIDTH), body.with_max_x(body.max.x - layers::RAIL_WIDTH))
                        };
                        let edge = if start { rail.max.x } else { rail.min.x };
                        ui.painter().vline(edge, rail.y_range(), egui::Stroke::new(1.0, t.border));
                        child(ui, rail, "sheet_rail", &mut |ui| {
                            a11y::region(&mut self.ui.nav, ui, a11y::Region::Layers, egui::accesskit::Role::Navigation, i18n::tr("Sidebar"));
                            layers::rail(self, ui);
                        });
                        child(ui, list, "sheet_layers", &mut |ui| {
                            let label = match self.ui.left_tab {
                                _ if self.ui.versions.open => i18n::tr("Version history"),
                                LeftTab::File => i18n::tr("Layers"),
                                LeftTab::Assets => i18n::tr("Assets"),
                            };
                            a11y::region(&mut self.ui.nav, ui, a11y::Region::Layers, egui::accesskit::Role::Pane, label);
                            layers::left_panel(self, ui);
                        });
                    }
                    MobilePanel::Right => child(ui, body, "sheet_design", &mut |ui| {
                        let label = right_tab_label(self.ui.right_tab);
                        a11y::region(&mut self.ui.nav, ui, a11y::Region::Properties, egui::accesskit::Role::Pane, label);
                        right::right_panel(self, ui);
                    }),
                }
            });
    }

    /// The sheet's grab handle and header row: switches between the two panels, and Close.
    fn sheet_header_ui(&mut self, ui: &mut egui::Ui, sheet: egui::Rect, side: MobilePanel) {
        use a11y::Tip as _;
        let t = theme::Tokens::get(ui.ctx());
        let painter = ui.painter().clone();
        let handle = egui::Rect::from_center_size(egui::pos2(sheet.center().x, sheet.min.y + HANDLE_STRIP / 2.0 + 1.0), egui::vec2(36.0, 4.0));
        painter.rect_filled(handle, 2.0, t.border);
        let header = sheet_header(sheet, self.ui.mirrored);
        for (rect, panel, icon, name) in [
            (header.layers, MobilePanel::Left, icons::Icon::Layers, i18n::tr("Layers")),
            (header.design, MobilePanel::Right, icons::Icon::Panels, i18n::tr("Design")),
        ] {
            let on = side == panel;
            let resp = ui.interact(rect, ui.scope_id().with(("sheet_switch", name)), egui::Sense::click());
            a11y::toggle(&resp, name, on);
            painter.rect_filled(rect.shrink2(egui::vec2(0.0, 4.0)), 8.0, if on { t.accent } else { egui::Color32::TRANSPARENT });
            let ink = if on { t.accent_on } else { t.text };
            let glyph = egui::Rect::from_center_size(egui::pos2(rect.min.x + 22.0, rect.center().y), egui::vec2(16.0, 16.0));
            icons::paint(&painter, glyph, icon, ink);
            painter.text(egui::pos2(glyph.max.x + 8.0, rect.center().y), egui::Align2::LEFT_CENTER, name, theme::semibold(12.0), ink);
            if resp.clicked() && !on {
                if panel == MobilePanel::Left {
                    self.ui.open_left_panel();
                } else {
                    self.ui.mobile_panel = Some(MobilePanel::Right);
                }
            }
        }
        let resp = ui.interact(header.close, ui.scope_id().with("sheet_close"), egui::Sense::click()).tip(i18n::tr("Close"));
        icons::paint(&painter, header.close.shrink(14.0), icons::Icon::Close, t.icon);
        if resp.clicked() {
            self.ui.mobile_panel = None;
        }
    }

    /// The two buttons at the canvas's top corner that open the Layers or Design sheet; the one
    /// whose sheet is open closes it again.
    pub(super) fn phone_buttons(&mut self, ctx: &egui::Context, canvas: egui::Rect) {
        use a11y::Tip as _;
        let t = theme::Tokens::get(ctx);
        let rects = canvas_buttons(canvas, self.ui.mirrored);
        egui::Area::new(egui::Id::unique("phone_canvas_buttons")).order(egui::Order::Foreground).fixed_pos(rects[0].min).show(ctx, |ui| {
            for (rect, side, icon, name) in [
                (rects[0], MobilePanel::Left, icons::Icon::Layers, i18n::tr("Layers")),
                (rects[1], MobilePanel::Right, icons::Icon::Panels, i18n::tr("Design")),
            ] {
                let open = self.ui.mobile_panel == Some(side);
                let resp = ui.interact(rect, ui.scope_id().with(("phone_button", name)), egui::Sense::click()).tip(name);
                a11y::toggle(&resp, name, open);
                ui.painter().rect(rect, 10.0, if open { t.accent } else { t.panel }, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                icons::paint(ui.painter(), rect.shrink(12.0), icon, if open { t.accent_on } else { t.icon });
                if resp.clicked() {
                    match (side, open) {
                        (_, true) => self.ui.mobile_panel = None,
                        (MobilePanel::Left, false) => self.ui.open_left_panel(),
                        (MobilePanel::Right, false) => self.ui.mobile_panel = Some(MobilePanel::Right),
                    }
                }
            }
            // The area is as large as the column, so a press between the buttons isn't the canvas's.
            ui.allocate_rect(rects[0].union(rects[1]), egui::Sense::hover());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Rect, pos2, vec2};

    #[test]
    fn the_sheet_leaves_the_canvas_room_and_is_never_negative() {
        // A 390 x 844 phone under a 40 pt tab bar.
        let h = sheet_height(804.0);
        assert!((440.0..=450.0).contains(&h), "{h}");
        assert!(804.0 - h >= CANVAS_KEEPS);
        // A short window: the canvas keeps its least, the sheet takes the rest.
        assert_eq!(sheet_height(400.0), 200.0);
        for room in [0.0, -10.0, 100.0, f32::NAN, f32::INFINITY] {
            let h = sheet_height(room);
            assert!(h.is_finite() && h >= 0.0 && (room.is_nan() || !room.is_finite() || h <= room.max(0.0)), "{room}: {h}");
        }
    }

    #[test]
    fn every_phone_control_is_a_touch_target() {
        let sheet = Rect::from_min_size(pos2(0.0, 400.0), vec2(390.0, 444.0));
        for mirrored in [false, true] {
            let h = sheet_header(sheet, mirrored);
            for r in [h.layers, h.design, h.close] {
                assert!(r.width() >= TOUCH_TARGET && r.height() >= TOUCH_TARGET, "{r:?}");
                assert!(sheet.contains_rect(r), "{r:?} in the sheet");
            }
            assert!(!h.layers.intersects(h.design) && !h.design.intersects(h.close), "{h:?}");
            assert!(h.close.max.y <= sheet.min.y + SHEET_HEADER, "the header row holds them");
            let canvas = Rect::from_min_size(pos2(0.0, 40.0), vec2(390.0, 360.0));
            let [a, b] = canvas_buttons(canvas, mirrored);
            assert!(a.width() >= TOUCH_TARGET && a.height() >= TOUCH_TARGET && !a.intersects(b) && canvas.contains_rect(a.union(b)));
        }
    }
}
