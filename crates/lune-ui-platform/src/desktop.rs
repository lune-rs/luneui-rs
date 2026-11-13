//! The desktop as the OS reports it, in the core's desktop units ([`lune_ui_core::desktop`]): the
//! monitors and their work areas, the pointer, a window's frame, moving windows, and which window
//! is under a point. These functions read live OS state, which is an adapter's job: the core stays
//! pure, and the app calls them and passes the values in (D313).
//!
//! Desktop units are the OS's own global space: physical px on Windows and X11 (as winit reports
//! them), points from the top-left of the primary display, y down, on macOS. A window's logical px
//! become units by [`units_per_logical`]. The macOS functions must run on the main thread and
//! answer `None` or an empty list elsewhere.

use lune_ui_core::desktop::{DesktopPoint, DesktopRect, Monitor};
use winit::window::Window;

use crate::platform::desktop as os;

/// The monitors the OS has, with their bounds and work areas in desktop units. `window` is any
/// window of the app: the OS asks it for its display list. Empty when the list cannot be read.
pub fn monitors(window: &Window) -> Vec<Monitor> {
    os::monitors(window)
}

/// The pointer's position in desktop units, read from the OS now, wherever it is (also outside
/// every window of the app). `None` when the OS cannot say. This is live OS state, so a frame that
/// must stay deterministic gets it from the app.
pub fn pointer() -> Option<DesktopPoint> {
    os::pointer()
}

/// The window's outer frame in desktop units. `None` when the OS cannot say, or the window has
/// no position (a minimised window on some systems).
pub fn window_frame(window: &Window) -> Option<DesktopRect> {
    os::window_frame(window)
}

/// Moves `window` so that its outer top-left corner is at `at`, in desktop units. Its size and
/// its place in the stacking order do not change.
pub fn set_window_origin(window: &Window, at: DesktopPoint) {
    os::set_window_origin(window, at);
}

/// Moves several windows, each to its outer top-left corner in desktop units, as one batch where
/// the OS can (Windows: one `DeferWindowPos` batch, so a docked set moves in one step). Sizes and
/// stacking order do not change. An empty list is `Ok`. On `Err` no promise is made about which
/// windows moved.
pub fn move_windows(moves: &[(&Window, DesktopPoint)]) -> Result<(), String> {
    os::move_windows(moves)
}

/// Desktop units per logical px for `window`: its scale factor on Windows and X11, 1.0 on macOS
/// where the units are points.
pub fn units_per_logical(window: &Window) -> f32 {
    os::units_per_logical(window)
}

/// Whether `window` is the top-level window the OS would hit at `at` (desktop units): `Some(true)`
/// when it is, `Some(false)` when another window or the bare desktop is, `None` when the OS cannot
/// say (or the platform has no way to ask). A window with a shape does not answer for the part
/// outside the shape. This is live OS state, like [`pointer`].
pub fn window_at(window: &Window, at: DesktopPoint) -> Option<bool> {
    os::window_at(window, at)
}

/// A whole number of units for `v`: rounded, halves away from zero, saturating.
#[allow(
    dead_code,
    reason = "used only by the macOS half; unit-tested here on every host"
)]
pub(crate) fn round_units(v: f64) -> i32 {
    // `as` saturates and turns NaN into 0.
    v.round() as i32
}

/// A rect of y-up screen points (an origin at its bottom-left, as macOS reports it) in desktop
/// units: y down from the top of the primary display, which is `primary_height` points high.
#[allow(
    dead_code,
    reason = "used only by the macOS half; unit-tested here on every host"
)]
pub(crate) fn flip_rect(x: f64, y: f64, w: f64, h: f64, primary_height: f64) -> DesktopRect {
    DesktopRect::new(
        round_units(x),
        round_units(primary_height - (y + h)),
        round_units(w),
        round_units(h),
    )
}

/// A y-up screen coordinate as a desktop y (down from the top of a primary display that is
/// `primary_height` points high), rounded.
#[allow(
    dead_code,
    reason = "used only by the macOS half; unit-tested here on every host"
)]
pub(crate) fn flip_y(y: f64, primary_height: f64) -> i32 {
    round_units(primary_height - y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rect_flips_from_the_bottom_left_to_the_top_left() {
        assert_eq!(
            flip_rect(0.0, 900.0, 100.0, 50.0, 1000.0),
            DesktopRect::new(0, 50, 100, 50)
        );
        // A screen to the left and below the primary one has a negative x and a y past its height.
        assert_eq!(
            flip_rect(-1920.0, -1080.0, 1920.0, 1080.0, 1000.0),
            DesktopRect::new(-1920, 1000, 1920, 1080)
        );
    }

    #[test]
    fn a_point_flips_to_the_top() {
        assert_eq!(flip_y(0.0, 1000.0), 1000);
        assert_eq!(flip_y(1000.0, 1000.0), 0);
        assert_eq!(flip_y(250.5, 1000.0), 750);
    }

    #[test]
    fn halves_round_away_from_zero() {
        assert_eq!(round_units(2.5), 3);
        assert_eq!(round_units(-2.5), -3);
        assert_eq!(round_units(2.4), 2);
        assert_eq!(round_units(f64::NAN), 0);
        assert_eq!(round_units(1.0e20), i32::MAX);
    }
}
