//! The desktop as the OS reports it, in the core's desktop units ([`lune_ui_core::desktop`]): the
//! monitors and their work areas, the pointer, a window's frame, moving windows, and which window
//! is under a point. These functions read live OS state, which is an adapter's job: the core stays
//! pure, and the app calls them and passes the values in (D313).
//!
//! Desktop units are the OS's own global space: physical px on Windows and X11 (as winit reports
//! them), points from the top-left of the primary display, y down, on macOS. A window's logical px
//! become units by [`units_per_logical`]. The macOS functions must run on the main thread and
//! answer `None` or an empty list elsewhere.
//!
//! A window can also be given a shape ([`set_window_shape`]): the part of it that is drawn and
//! takes the pointer, so the desktop shows, and takes clicks, between the parts (D315).

use lune_ui_core::desktop::{DesktopPoint, DesktopRect, Monitor, WindowShape};
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
/// outside the shape. This is live OS state, like [`pointer()`].
pub fn window_at(window: &Window, at: DesktopPoint) -> Option<bool> {
    os::window_at(window, at)
}

/// How a platform gives a window its shape, as [`set_window_shape`] reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeMode {
    /// The OS clips the window to the shape, both its drawing and the pointer (Windows: a window
    /// region). Nothing more is needed until the window's size or scale changes.
    Region,
    /// The OS has no shape for a window: the app keeps a [`ShapeFollower`] up to date with the
    /// pointer, which lets clicks outside the shape through to what is below (macOS). The app
    /// draws nothing outside the shape itself.
    Pointer,
    /// The platform cannot shape a window: the whole frame takes the pointer.
    Unsupported,
}

/// Gives `window` a shape, or takes it away with `None`: the part of the window that is drawn and
/// takes the pointer. The shape is in the frame's logical px, from the top-left of the window's
/// content; `scale` is the frame's physical px per shape unit (`RawInput::scale`), so a window
/// drawn at a zoom keeps its shape in step with what it draws. A non-finite or non-positive
/// `scale` is an `Err` and changes nothing.
///
/// The OS does not rescale a shape: the app sets it again after the window is resized or its
/// scale factor changes. On Windows the shape becomes the window region ([`ShapeMode::Region`]);
/// on macOS it is followed with the pointer ([`ShapeMode::Pointer`]; `None` makes the whole
/// window take clicks again); elsewhere nothing happens ([`ShapeMode::Unsupported`]).
pub fn set_window_shape(
    window: &Window,
    shape: Option<&WindowShape>,
    scale: f32,
) -> Result<ShapeMode, String> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(format!(
            "a window shape needs a positive scale, not {scale}"
        ));
    }
    os::set_window_shape(window, shape, scale)
}

/// What [`ShapeFollower::update`] asks of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    /// Nothing until the window's next pointer event.
    Idle,
    /// The window lets the pointer through, so it gets no pointer events: call
    /// [`ShapeFollower::update`] again soon (on a short timer) with the OS pointer, so the window
    /// takes clicks again when the pointer comes back into its shape or leaves its frame.
    Poll,
}

/// Follows the pointer for a shaped window on a platform whose mode is [`ShapeMode::Pointer`]:
/// over the window's frame but outside its shape the window lets clicks through to what is
/// below, and everywhere else it takes them. Where the mode is not `Pointer` it does nothing and
/// always answers [`Follow::Idle`]. One follower per window.
#[derive(Debug, Default)]
pub struct ShapeFollower {
    /// Whether the window lets the pointer through now.
    passing: bool,
}

impl ShapeFollower {
    /// Updates the window for the pointer at `pointer` (desktop units, from [`pointer()`]), with the
    /// window's `shape` and the frame's `scale` as given to [`set_window_shape`]. It calls the
    /// window only when what it should do changes. No pointer or no frame make the window take
    /// clicks.
    pub fn update(
        &mut self,
        window: &Window,
        shape: Option<&WindowShape>,
        scale: f32,
        pointer: Option<DesktopPoint>,
    ) -> Follow {
        if os::SHAPE_MODE != ShapeMode::Pointer {
            return Follow::Idle;
        }
        let per = scale / window.scale_factor() as f32 * units_per_logical(window);
        let hit = match (window_frame(window), pointer) {
            (Some(frame), Some(p)) => pointer_hit(frame, per, shape, p),
            _ => PointerHit::Outside,
        };
        let pass = hit == PointerHit::Through;
        // A refusal leaves the state as it was, so the next update tries again.
        if pass != self.passing && window.set_cursor_hittest(!pass).is_ok() {
            self.passing = pass;
        }
        if self.passing {
            Follow::Poll
        } else {
            Follow::Idle
        }
    }
}

/// Where a pointer is for a shaped window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerHit {
    /// Over the frame and inside the shape (or the window has no shape): the window takes it.
    Inside,
    /// Over the frame but outside the shape: the window lets it through.
    Through,
    /// Not over the frame.
    Outside,
}

/// Where `p` (desktop units) is for a window whose outer frame is `frame` and whose shape is in
/// units of which `per` desktop units make one. A bad `per` counts as no shape.
pub(crate) fn pointer_hit(
    frame: DesktopRect,
    per: f32,
    shape: Option<&WindowShape>,
    p: DesktopPoint,
) -> PointerHit {
    if !frame.contains(p) {
        return PointerHit::Outside;
    }
    let Some(shape) = shape else {
        return PointerHit::Inside;
    };
    if !per.is_finite() || per <= 0.0 {
        return PointerHit::Inside;
    }
    let local = (
        (i64::from(p.x) - i64::from(frame.x)) as f32 / per,
        (i64::from(p.y) - i64::from(frame.y)) as f32 / per,
    );
    if shape.contains(local) {
        PointerHit::Inside
    } else {
        PointerHit::Through
    }
}

/// The shape's polygons in whole px: each point times `scale`, rounded and saturating, plus
/// `offset` (where the content's top-left is in the outer frame), saturating.
#[allow(
    dead_code,
    reason = "used only by the Windows half; unit-tested here on every host"
)]
pub(crate) fn region_points(
    shape: &WindowShape,
    scale: f32,
    offset: (i32, i32),
) -> Vec<Vec<(i32, i32)>> {
    // `as` saturates and turns NaN into 0.
    let px = |v: f32| (f64::from(v) * f64::from(scale)).round() as i32;
    shape
        .polygons()
        .iter()
        .map(|poly| {
            poly.iter()
                .map(|&(x, y)| {
                    (
                        px(x).saturating_add(offset.0),
                        px(y).saturating_add(offset.1),
                    )
                })
                .collect()
        })
        .collect()
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
    use lune_ui_core::paint::Rect;

    #[test]
    fn region_points_scale_round_and_offset() {
        let shape = WindowShape::new()
            .with_polygon(&[(10.0, 20.0), (1.0e12, 0.0), (0.4, -0.6)])
            .expect("a valid polygon");
        let points = region_points(&shape, 1.5, (0, 1));
        assert_eq!(points.len(), 1);
        assert_eq!(points[0][0], (15, 31));
        assert_eq!(points[0][1], (i32::MAX, 1));
        assert_eq!(points[0][2], (1, 0));
        assert_eq!(region_points(&shape, 1.5, (0, 0))[0][1], (i32::MAX, 0));
    }

    #[test]
    fn the_pointer_is_inside_through_or_outside_the_shape() {
        let frame = DesktopRect::new(100, 100, 200, 100);
        let shape = WindowShape::new()
            .with_rect(Rect::new(0.0, 0.0, 100.0, 100.0))
            .expect("a rect is a valid polygon");
        let hit = |x, y| pointer_hit(frame, 1.0, Some(&shape), DesktopPoint::new(x, y));
        assert_eq!(hit(150, 150), PointerHit::Inside);
        assert_eq!(hit(250, 150), PointerHit::Through);
        assert_eq!(hit(400, 400), PointerHit::Outside);
        // No shape: the whole frame takes the pointer.
        assert_eq!(
            pointer_hit(frame, 1.0, None, DesktopPoint::new(250, 150)),
            PointerHit::Inside
        );
        // Two desktop units per shape unit: (250, 150) is (75, 25) in the shape.
        assert_eq!(
            pointer_hit(frame, 2.0, Some(&shape), DesktopPoint::new(250, 150)),
            PointerHit::Inside
        );
    }

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
