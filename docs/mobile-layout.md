# Phone and tablet layouts

> **Last updated:** 2026-11-20 · **Change:** major (first version: the four layouts, their size
> classes and touch rules; slice 1 landed: the size class and the phone-portrait layout)

FigCraft's editor on touch screens: phones and tablets, each held upright or sideways. The route
is the existing WASM web build in mobile browsers (and eframe on Android later); there is no
webview shell and no JavaScript UI. The same layouts serve a desktop window resized to the same
sizes.

Figma's own iOS and Android apps only view files, comment and mirror prototypes; they do not edit.
There is no incumbent editing layout on a phone to match, so these layouts are FigCraft's own
design. Nothing here is taken from Figma's screens, and no Figma screens or assets are used.

## Size classes

Every frame the window's size in points (after the interface size, `pref.uiScale`) gives one of
four classes (`crates/ui-egui/src/size_class.rs`). `ui.inspect` reports it as `layout.sizeClass`.

| Class | Rule | Reference size (pt) | Typical devices |
|---|---|---|---|
| phone portrait | shorter side < 600, taller than wide | 390 × 844 | phones held upright |
| phone landscape | shorter side < 600, wider than tall | 844 × 390 | phones sideways |
| tablet portrait | shorter side ≥ 600, taller than wide | 820 × 1180 | tablets upright, foldables open |
| tablet landscape | shorter side ≥ 600, wider than tall | 1180 × 820 | tablets sideways, every desktop window |

**Hysteresis.** Each boundary has a band either side, so a window resized to about a boundary, or a
rotation whose intermediate sizes pass near one, changes layout once instead of on every frame:

- **Phone or tablet**, by the shorter side: a tablet becomes a phone below **600 pt**; a phone
  becomes a tablet again only at **640 pt**. Between the two the class it had stays.
- **Portrait or landscape**: portrait turns landscape once the window is **1.1 ×** wider than tall,
  and landscape turns portrait once it is 1.1 × taller than wide. A near-square window keeps its
  orientation.
- A size with no area, or not finite (a minimized window), keeps the class.
- A new window starts as tablet landscape (the desktop), which a phone-sized first frame leaves at
  once.

Size decides the layout; it does not decide how big the controls are. A desktop window and a
tablet sideways share a class. Larger targets follow touch input instead (see [Touch rules](#touch-rules)).

## The four layouts

### Phone portrait (landed)

```
┌────────────────────────┐
│ tabs                   │  tab bar (40 pt), as today
├────────────────────────┤
│                  [▤]   │  canvas buttons, 44 pt: Layers, Design
│                  [◧]   │
│        canvas          │
│                        │
│   ( toolbar pill )     │  floating, scrolls sideways when wider than the canvas
├──────── ── ────────────┤  sheet: grab handle
│ [Layers][Design]   [×] │  header row, 44 pt targets
│ rail │ layers list     │  or the Design panel, full width
│      │                 │
└────────────────────────┘
```

- **Canvas**: the whole window under the tab bar. With a sheet open it keeps the top part and
  stays fully visible (the sheet is under it, not over it), so what you edit stays in view.
- **Toolbar**: the floating pill at the canvas's bottom centre. It stays with a sheet open,
  above the sheet, and scrolls when it is wider than the canvas (#1209).
- **Layers and Design panels**: one at a time, in a **bottom sheet** 55 % of the height under
  the tab bar (at least 240 pt, leaving the canvas at least 200 pt). The header switches between
  them and closes the sheet. Layers keeps its icon rail beside the list. The Design panel
  (with its Design and Prototype tabs) takes the sheet's full width.
- **Opening**: two 44 pt buttons in a column at the canvas's top end corner (top start corner in a
  right-to-left language). The open panel's button closes it again. Commands that open a panel
  (`view.assets`, the layers toggle) open the sheet, as they open the narrow window's panel.
- **Menus**: the main menu and the tab bar as today. **Dialogs**: as today for now; they become
  full-screen sheets in slice 4.
- **Popovers** (colour picker, font menu, variable and style pickers): as today for now; slice 4
  opens them as a second sheet over the first.
- Later in this class (slice 4): the handle drags the sheet between detents (about 35 %, 55 % and
  90 %), a swipe down closes it, and the sheet stays above the on-screen keyboard.

### Phone landscape (slice 5)

The height (about 390 pt) has no room for a sheet under the canvas.

- **Canvas**: the whole window under the tab bar.
- **Toolbar**: a vertical pill along the canvas's start edge, so the short height stays for the
  canvas.
- **Panels**: one at a time in a **side drawer** from the end edge, 320 pt or 45 % of the width
  (the smaller), with the same header as the phone sheet. The canvas narrows beside it rather than
  being covered, so the selection stays in view.
- **Canvas buttons**: as in portrait, at the top end corner of what the drawer leaves.
- Until this slice lands: the desktop layout (both panels beside a narrow canvas).

### Tablet portrait (slice 6)

About 820 pt wide: room for the canvas and one panel, not two (the desktop layout leaves a
284 pt canvas).

- **Design panel**: docked on the end side, 280 pt, collapsible to the canvas buttons.
- **Layers**: a drawer from the start edge over the canvas, rail included, closed by default;
  opening it closes nothing else, and a tap on the canvas outside it closes it.
- **Toolbar**: the floating pill at the bottom, full width of its items.
- **Dialogs and popovers**: as on the desktop, anchored where they are.
- Until this slice lands: the desktop layout; windows narrower than 760 pt keep the narrow
  window's one-panel-at-a-time layout.

### Tablet landscape (slice 7)

The desktop layout: rail, Layers panel, canvas and Design panel side by side. With touch input
(slice 3) the controls grow to touch targets and the panel edges get a wider grab area.
Every desktop window is in this class, so its slice changes nothing for a mouse.

## Touch rules

They apply in every class once the window sees touch input (a touch event this session), and to
every new control in the phone layouts from the start.

- **Hit targets at least 44 × 44 pt.** The phone layout's own controls are (the canvas buttons, the
  sheet's switches and Close; `app::phone::tests` checks them). In touch mode the toolbar cells
  (32 pt today), layer rows, the panels' fields and their icon buttons grow to 44 pt targets; a
  glyph may stay smaller inside its target.
- **Pinch zooms** around the fingers' midpoint and **two fingers pan**, on the canvas and in the
  prototype player. Both exist (`canvas/input.rs` reads egui's multi-touch zoom and translation);
  slice 8 checks them on devices.
- **One finger** does what the mouse's left button does with the current tool: with Move, a drag
  on empty canvas draws a selection box. Panning with one finger needs the Hand tool.
- **Long press is right-click**: holding still for 0.5 s (moving less than 8 pt) opens the context
  menu on the canvas, a layer row, a page or a property row, where a right-click would. egui's
  `Response::long_touched` reports it.
- **No hover-only controls.** Whatever hover reveals needs a tap path too: a layer row's
  visibility and lock buttons show on the selected row and in touch mode on every row; a tooltip is
  never the only label (a long press on a toolbar button shows its name); hover outlines on the
  canvas are a nicety, never the only way to find a layer.
- **The on-screen keyboard** never covers the field being typed in: the sheet or drawer moves
  above it (the browser's visual viewport in the web build).
- **Safe areas**: the web build lays out inside the device's safe area (notch, home indicator,
  rounded corners).

## How it is built

- `crates/ui-egui/src/size_class.rs`: `SizeClass::of(size, before)`, the classes and bands.
- `crates/ui-egui/src/app.rs` (`FigApp::frame`): reads the class each frame and picks the layout;
  `crates/ui-egui/src/app/phone.rs`: the phone sheet, its header and the canvas buttons.
- The open phone panel is `ui.mobile_panel`, shared with the narrow window's layout, so a panel
  stays open across a rotation and commands open the sheet.
- `ui.inspect` → `layout: {sizeClass, sheet}` (`sheet`: `layers`, `design` or `null`).
- The desktop window can be made as small as 320 × 320 pt (it was 800 × 500), so the phone
  layouts are reachable on a desktop and in a narrow snapped window.
- Tests: `size_class::tests` (classes, rotation, bands), `app::phone::tests` (sheet height, touch
  targets) and `mobile_layout_tests` (the four reference sizes drawn headless).

## Slices

| # | Slice | Estimate | State |
|---|---|---|---|
| 1 | Size class with hysteresis; phone portrait: bottom sheet, canvas buttons, toolbar kept | 6–8 h | landed |
| 2 | Touch input: long press as right-click (canvas, layers, pages, property rows), long-press names on toolbar buttons, a touch-mode flag | 4–6 h | |
| 3 | Touch sizing: 44 pt toolbar cells, layer rows, fields and icon buttons in touch mode; tap paths for hover-only controls | 8–12 h | |
| 4 | Phone portrait polish: sheet detents and swipe to close, popovers as sheets, dialogs full screen, keyboard insets | 10–16 h | |
| 5 | Phone landscape: side drawer, vertical toolbar | 6–10 h | |
| 6 | Tablet portrait: docked Design panel, Layers drawer | 6–8 h | |
| 7 | Tablet landscape: touch-mode panel edges and spacing over the desktop layout | 2–4 h | |
| 8 | Web build on devices: viewport and safe areas, rotation, browser gestures (page zoom, back swipe) kept off the canvas, QA in Safari on iOS and Chrome on Android | 8–12 h | |

Slices 2 to 8: about 44–68 h. Slices 2 and 3 help every class and come first; 5 and 6 are
independent of each other.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-20 | major | First version: the four layouts, size classes with hysteresis, touch rules and slices; slice 1 landed (size class, phone-portrait sheet layout). |
