//! The window's size class: phone or tablet, portrait or landscape, read from the window's size in
//! points every frame. It picks the editor's layout (`docs/mobile-layout.md`).
//!
//! Each boundary has a band on either side (hysteresis): a class changes only once the size is
//! clearly past the boundary, so a window dragged to just about a boundary, or a rotation whose
//! intermediate sizes pass near one, doesn't flip the layout back and forth every frame.

/// A phone or a tablet, by the window's shorter side.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Device {
    Phone,
    Tablet,
}

/// Taller than wide, or wider than tall.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// The layout a window gets. A desktop window is a tablet in landscape: that class keeps the
/// desktop layout.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SizeClass {
    pub device: Device,
    pub orientation: Orientation,
}

/// The smallest a desktop window can be made, in points: small enough for the phone layouts, so a
/// desktop window (or a narrow snapped one) can show them too.
pub const MIN_WINDOW: [f32; 2] = [320.0, 320.0];

/// A window whose shorter side is below this many points becomes a phone…
pub const PHONE_BELOW: f32 = 600.0;
/// …and a phone becomes a tablet again only once its shorter side reaches this.
pub const TABLET_FROM: f32 = 640.0;
/// A portrait window turns to landscape once it is this much wider than tall, and back once it is
/// this much taller than wide; a near-square window keeps the orientation it had.
pub const TURN_RATIO: f32 = 1.1;

impl SizeClass {
    pub const PHONE_PORTRAIT: Self = Self { device: Device::Phone, orientation: Orientation::Portrait };
    pub const PHONE_LANDSCAPE: Self = Self { device: Device::Phone, orientation: Orientation::Landscape };
    pub const TABLET_PORTRAIT: Self = Self { device: Device::Tablet, orientation: Orientation::Portrait };
    pub const TABLET_LANDSCAPE: Self = Self { device: Device::Tablet, orientation: Orientation::Landscape };

    /// The class of a window `size` points large that had the class `before` last frame. A size
    /// that isn't finite or isn't positive (a minimized window, a test without a screen) keeps
    /// `before`.
    pub fn of(size: egui::Vec2, before: Self) -> Self {
        let (w, h) = (size.x, size.y);
        if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
            return before;
        }
        let short = w.min(h);
        let device = match before.device {
            Device::Phone if short >= TABLET_FROM => Device::Tablet,
            Device::Tablet if short < PHONE_BELOW => Device::Phone,
            same => same,
        };
        let orientation = match before.orientation {
            Orientation::Portrait if w > h * TURN_RATIO => Orientation::Landscape,
            Orientation::Landscape if h > w * TURN_RATIO => Orientation::Portrait,
            same => same,
        };
        Self { device, orientation }
    }

    /// The name `ui.inspect` reports (`layout.sizeClass`).
    pub fn name(self) -> &'static str {
        match (self.device, self.orientation) {
            (Device::Phone, Orientation::Portrait) => "phonePortrait",
            (Device::Phone, Orientation::Landscape) => "phoneLandscape",
            (Device::Tablet, Orientation::Portrait) => "tabletPortrait",
            (Device::Tablet, Orientation::Landscape) => "tabletLandscape",
        }
    }
}

impl Default for SizeClass {
    /// A desktop window's class, which a first frame of a phone-sized window leaves at once.
    fn default() -> Self {
        Self::TABLET_LANDSCAPE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::vec2;

    /// From the desktop default, as on a window's first frame.
    fn fresh(w: f32, h: f32) -> SizeClass {
        SizeClass::of(vec2(w, h), SizeClass::default())
    }

    #[test]
    fn the_four_reference_sizes_get_their_classes() {
        assert_eq!(fresh(390.0, 844.0), SizeClass::PHONE_PORTRAIT);
        assert_eq!(fresh(844.0, 390.0), SizeClass::PHONE_LANDSCAPE);
        assert_eq!(fresh(820.0, 1180.0), SizeClass::TABLET_PORTRAIT);
        assert_eq!(fresh(1180.0, 820.0), SizeClass::TABLET_LANDSCAPE);
        // The smallest and largest phones, a small tablet, a desktop window.
        assert_eq!(fresh(320.0, 568.0), SizeClass::PHONE_PORTRAIT);
        assert_eq!(fresh(440.0, 956.0), SizeClass::PHONE_PORTRAIT);
        assert_eq!(fresh(744.0, 1133.0), SizeClass::TABLET_PORTRAIT);
        assert_eq!(fresh(1440.0, 900.0), SizeClass::TABLET_LANDSCAPE);
    }

    #[test]
    fn a_rotation_swaps_the_orientation_and_keeps_the_device() {
        let mut c = fresh(390.0, 844.0);
        for (size, want) in [
            (vec2(844.0, 390.0), SizeClass::PHONE_LANDSCAPE),
            (vec2(390.0, 844.0), SizeClass::PHONE_PORTRAIT),
            (vec2(1180.0, 820.0), SizeClass::TABLET_LANDSCAPE),
            (vec2(820.0, 1180.0), SizeClass::TABLET_PORTRAIT),
        ] {
            c = SizeClass::of(size, c);
            assert_eq!(c, want, "{size:?}");
        }
    }

    /// Inside a band the class it had stays, from either side: a window resized back and forth
    /// across a boundary changes layout once, not on every frame.
    #[test]
    fn near_a_boundary_the_class_it_had_stays() {
        // The shorter side between 600 and 640: a phone stays a phone, a tablet a tablet.
        let phone = fresh(390.0, 844.0);
        assert_eq!(SizeClass::of(vec2(620.0, 900.0), phone), SizeClass::PHONE_PORTRAIT);
        let tablet = fresh(820.0, 1180.0);
        assert_eq!(SizeClass::of(vec2(620.0, 900.0), tablet), SizeClass::TABLET_PORTRAIT);
        assert_eq!(SizeClass::of(vec2(640.0, 900.0), phone).device, Device::Tablet, "past the band it changes");
        assert_eq!(SizeClass::of(vec2(599.0, 900.0), tablet).device, Device::Phone);
        // Near square: the orientation it had stays.
        let portrait = fresh(800.0, 1000.0);
        assert_eq!(SizeClass::of(vec2(900.0, 860.0), portrait).orientation, Orientation::Portrait);
        let landscape = fresh(1000.0, 800.0);
        assert_eq!(SizeClass::of(vec2(860.0, 900.0), landscape).orientation, Orientation::Landscape);
        // Jitter around a boundary never flips it.
        let mut c = phone;
        for k in 0..40 {
            let w = if k % 2 == 0 { 610.0 } else { 630.0 };
            c = SizeClass::of(vec2(w, 1000.0), c);
            assert_eq!(c, SizeClass::PHONE_PORTRAIT, "frame {k}");
        }
    }

    #[test]
    fn a_size_with_no_area_or_not_finite_keeps_the_class() {
        let phone = fresh(390.0, 844.0);
        for size in [vec2(0.0, 0.0), vec2(-5.0, 800.0), vec2(f32::NAN, 800.0), vec2(800.0, f32::INFINITY)] {
            assert_eq!(SizeClass::of(size, phone), phone, "{size:?}");
        }
    }

    #[test]
    fn each_class_has_its_own_name() {
        let names: Vec<_> = [SizeClass::PHONE_PORTRAIT, SizeClass::PHONE_LANDSCAPE, SizeClass::TABLET_PORTRAIT, SizeClass::TABLET_LANDSCAPE]
            .map(SizeClass::name)
            .into();
        assert_eq!(names, ["phonePortrait", "phoneLandscape", "tabletPortrait", "tabletLandscape"]);
    }
}
