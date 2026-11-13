//! The desktop in one coordinate space: points and rects in desktop units, monitors, popup placement
//! in that space, and window shapes. Pure maths: nothing here reads the OS (the platform crate
//! does), so an app passes the values in and frames stay deterministic.
//!
//! **Desktop units** are the OS's own global space, as whole numbers. On Windows and X11 that is
//! physical px with the origin at the primary display's top-left; on macOS it is points from the
//! primary display's top-left, y down. Monitors of different scales share the one space, which is
//! why neither physical nor logical px everywhere would do (D309). A window's logical px become
//! desktop units by the scale of the monitor it is on ([`Monitor::units_per_logical`]).

use std::fmt;

use crate::paint::Rect;
use crate::widgets::popover::{PlaceAlign, PlaceRequest, Side, place};

/// A point in desktop units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DesktopPoint {
    /// Distance right of the desktop's origin.
    pub x: i32,
    /// Distance below the desktop's origin.
    pub y: i32,
}

impl DesktopPoint {
    /// The point at `x`, `y`.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// A rect in desktop units: its top-left corner and size. A rect with `w <= 0` or `h <= 0` is
/// empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DesktopRect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl DesktopRect {
    /// The rect at `x`, `y` of size `w` by `h`.
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// The right edge, `x + w`, saturating.
    pub const fn right(&self) -> i32 {
        self.x.saturating_add(self.w)
    }

    /// The bottom edge, `y + h`, saturating.
    pub const fn bottom(&self) -> i32 {
        self.y.saturating_add(self.h)
    }

    /// The top-left corner.
    pub const fn origin(&self) -> DesktopPoint {
        DesktopPoint::new(self.x, self.y)
    }

    /// Whether the rect has no area: `w <= 0` or `h <= 0`.
    pub const fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// Whether `p` is inside: the left and top edges count, the right and bottom do not. An empty
    /// rect contains nothing.
    pub const fn contains(&self, p: DesktopPoint) -> bool {
        !self.is_empty()
            && p.x >= self.x
            && p.x < self.right()
            && p.y >= self.y
            && p.y < self.bottom()
    }

    /// Whether the two rects share area (a shared edge is not enough). False when either is empty.
    pub const fn intersects(&self, o: DesktopRect) -> bool {
        !self.is_empty()
            && !o.is_empty()
            && self.x < o.right()
            && o.x < self.right()
            && self.y < o.bottom()
            && o.y < self.bottom()
    }

    /// The same size with its top-left corner at `p`.
    pub const fn moved_to(&self, p: DesktopPoint) -> Self {
        Self::new(p.x, p.y, self.w, self.h)
    }

    /// The same size, moved by `dx` and `dy`, saturating.
    pub const fn translated(&self, dx: i32, dy: i32) -> Self {
        Self::new(
            self.x.saturating_add(dx),
            self.y.saturating_add(dy),
            self.w,
            self.h,
        )
    }

    /// The rect as logical-looking `f32` values, for the maths that works in `f32`.
    fn as_rect(&self) -> Rect {
        Rect::new(self.x as f32, self.y as f32, self.w as f32, self.h as f32)
    }
}

/// One monitor, read by the platform crate and passed in by the app.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Monitor {
    /// The whole monitor.
    pub bounds: DesktopRect,
    /// The part the OS leaves to windows: `bounds` less the taskbar, menu bar and the like.
    pub work: DesktopRect,
    /// Desktop units per logical px on this monitor (1.5 for 150 % scaling on Windows, 1.0 on
    /// macOS where the units are points).
    pub units_per_logical: f32,
    /// Whether this is the primary monitor.
    pub primary: bool,
}

/// `logical` px in desktop units on a monitor of `units_per_logical`: rounded, halves away from
/// zero (the way winit rounds a logical size), saturating at the `i32` range. A non-finite input
/// gives 0.
pub fn to_units(logical: f32, units_per_logical: f32) -> i32 {
    if !logical.is_finite() || !units_per_logical.is_finite() {
        return 0;
    }
    // `as` saturates, so a huge product stays in range.
    (logical * units_per_logical).round() as i32
}

/// The index of the monitor holding `p`: the first whose bounds contain it, else the nearest by
/// squared distance to its bounds (ties go to the lower index). `None` for no monitors.
pub fn monitor_at(monitors: &[Monitor], p: DesktopPoint) -> Option<usize> {
    if let Some(i) = monitors.iter().position(|m| m.bounds.contains(p)) {
        return Some(i);
    }
    let mut best: Option<(usize, i128)> = None;
    for (i, m) in monitors.iter().enumerate() {
        let b = m.bounds;
        let dx = (i64::from(b.x) - i64::from(p.x))
            .max(i64::from(p.x) - i64::from(b.right()))
            .max(0);
        let dy = (i64::from(b.y) - i64::from(p.y))
            .max(i64::from(p.y) - i64::from(b.bottom()))
            .max(0);
        let d = i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy);
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

/// Where a popup of `size` logical px goes next to `anchor` on `monitor`: [`place`] in desktop
/// units, inside the monitor's work area, flipped and shifted as it does (D311).
///
/// The size and `gap` are multiplied by the monitor's `units_per_logical` and the size is rounded
/// with [`to_units`]. `None` when `units_per_logical` is not finite or not above 0, when `gap` or
/// a size part is not finite, or when a size part is negative.
pub fn place_popup(
    anchor: DesktopRect,
    size: (f32, f32),
    side: Side,
    align: PlaceAlign,
    gap: f32,
    monitor: &Monitor,
) -> Option<DesktopRect> {
    let upl = monitor.units_per_logical;
    if !upl.is_finite()
        || upl <= 0.0
        || !size.0.is_finite()
        || !size.1.is_finite()
        || size.0 < 0.0
        || size.1 < 0.0
        || !gap.is_finite()
    {
        return None;
    }
    let w = to_units(size.0, upl);
    let h = to_units(size.1, upl);
    let placed = place(&PlaceRequest {
        anchor: anchor.as_rect(),
        size: (w as f32, h as f32),
        side,
        align,
        gap: gap * upl,
        margin: 0.0,
        shift: true,
        window: monitor.work.as_rect(),
    });
    Some(DesktopRect::new(
        placed.rect.x.round() as i32,
        placed.rect.y.round() as i32,
        w,
        h,
    ))
}

/// Why a [`WindowShape`] refused a polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeError {
    /// A polygon needs at least 3 points.
    TooFewPoints,
    /// A coordinate was NaN or infinite.
    NotFinite,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewPoints => f.write_str("a polygon needs at least 3 points"),
            Self::NotFinite => f.write_str("a polygon point is not a finite number"),
        }
    }
}

impl std::error::Error for ShapeError {}

/// The part of a window that exists: a union of simple polygons in the window's logical px. A
/// point of the window outside the shape is not drawn and does not take the pointer. Each polygon
/// is closed by itself and filled by the even-odd rule; the polygons together are a union, so two
/// that overlap cover both.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WindowShape {
    polygons: Vec<Vec<(f32, f32)>>,
}

impl WindowShape {
    /// A shape with no area.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a polygon of `points` (at least 3, all finite).
    pub fn with_polygon(mut self, points: &[(f32, f32)]) -> Result<Self, ShapeError> {
        if points.len() < 3 {
            return Err(ShapeError::TooFewPoints);
        }
        if points
            .iter()
            .any(|&(x, y)| !x.is_finite() || !y.is_finite())
        {
            return Err(ShapeError::NotFinite);
        }
        self.polygons.push(points.to_vec());
        Ok(self)
    }

    /// Adds the four corners of `r` as a polygon.
    pub fn with_rect(self, r: Rect) -> Result<Self, ShapeError> {
        self.with_polygon(&[
            (r.x, r.y),
            (r.right(), r.y),
            (r.right(), r.bottom()),
            (r.x, r.bottom()),
        ])
    }

    /// The polygons, in the order they were added.
    pub fn polygons(&self) -> &[Vec<(f32, f32)>] {
        &self.polygons
    }

    /// Whether the shape has no polygon.
    pub fn is_empty(&self) -> bool {
        self.polygons.is_empty()
    }

    /// The smallest rect holding every point, or `None` for an empty shape.
    pub fn bounds(&self) -> Option<Rect> {
        let mut points = self.polygons.iter().flatten();
        let &(x0, y0) = points.next()?;
        let (mut lo_x, mut lo_y, mut hi_x, mut hi_y) = (x0, y0, x0, y0);
        for &(x, y) in points {
            lo_x = lo_x.min(x);
            lo_y = lo_y.min(y);
            hi_x = hi_x.max(x);
            hi_y = hi_y.max(y);
        }
        Some(Rect::new(lo_x, lo_y, hi_x - lo_x, hi_y - lo_y))
    }

    /// Whether `p` is inside the shape: inside any polygon by the even-odd rule. A point on an
    /// edge may count either way.
    pub fn contains(&self, p: (f32, f32)) -> bool {
        self.polygons.iter().any(|poly| inside_even_odd(poly, p))
    }
}

/// The even-odd ray test: casts a ray from `p` to the right and counts the edges it crosses.
fn inside_even_odd(poly: &[(f32, f32)], p: (f32, f32)) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > p.1) != (yj > p.1) && p.0 < (xj - xi) * (p.1 - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}
