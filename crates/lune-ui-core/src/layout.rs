//! Pure layout helpers: a parent rect and child sizes in, child rects out. [`stack`] (with
//! [`row`] and [`column`](fn@column)) lays children along one axis; [`grid`] resolves column and row tracks.
//! They draw nothing and keep nothing, so a frame's layout is a function of its inputs.
//!
//! A child's main-axis size is fixed, hug (the size the caller measured) or fill with a weight.
//! The leftover after fixed sizes and gaps is split among fill children by weight from prefix
//! sums, so the rects tile the parent to the last pixel however many there are. A gap goes only
//! between children. Fixed and hug children that exceed the parent run past it; clipping and
//! scrolling are the parent's business.
//!
//! Totality: NaN, infinite, negative and zero sizes, gaps, weights and insets count as 0 (a
//! non-finite parent position counts as 0 too); an empty list gives an empty result; every
//! returned rect is finite.

use crate::paint::Rect;

/// The axis a stack runs along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Left to right.
    Horizontal,
    /// Top to bottom.
    Vertical,
}

/// A child's size along the stack's axis, in logical px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    Fixed(f32),
    /// The size the caller measured for the child.
    Hug(f32),
    /// A share of the leftover, by weight.
    Fill(f32),
}

/// Where a child sits across the stack's axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    /// The full cross size of the parent; the child's own `cross` is ignored.
    Stretch,
}

/// Per-side distances that shrink a parent rect before layout.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Insets {
    /// No inset on any side.
    pub const ZERO: Insets = Insets::all(0.0);

    /// Insets of `left`, `top`, `right` and `bottom`.
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// The same distance on every side.
    pub const fn all(v: f32) -> Self {
        Self::new(v, v, v, v)
    }
}

/// A grid column or row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Track {
    Fixed(f32),
    /// The size the caller measured for the track's content.
    Auto(f32),
    /// A share of the leftover, by weight.
    Star(f32),
}

/// One child of a stack: its size along the axis and across it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Child {
    pub main: Size,
    /// The size across the axis; ignored under [`Align::Stretch`].
    pub cross: f32,
}

/// A size, gap, weight or inset: finite and positive, else 0.
fn clean(v: f32) -> f64 {
    if v.is_finite() && v > 0.0 {
        f64::from(v)
    } else {
        0.0
    }
}

/// A position: finite, else 0.
fn coord(v: f32) -> f64 {
    if v.is_finite() { f64::from(v) } else { 0.0 }
}

/// Back to `f32` without ever overflowing to infinity.
fn out(v: f64) -> f32 {
    v.clamp(-f64::from(f32::MAX), f64::from(f32::MAX)) as f32
}

/// `parent` shrunk by `insets`, as `(x, y, w, h)`.
fn inner(parent: Rect, insets: Insets) -> (f64, f64, f64, f64) {
    let (l, t, r, b) = (
        clean(insets.left),
        clean(insets.top),
        clean(insets.right),
        clean(insets.bottom),
    );
    (
        coord(parent.x) + l,
        coord(parent.y) + t,
        (clean(parent.w) - l - r).max(0.0),
        (clean(parent.h) - t - b).max(0.0),
    )
}

/// A size along one axis: fixed, or a weight of the leftover.
#[derive(Debug, Clone, Copy)]
enum Slot {
    Fixed(f64),
    Fill(f64),
}

/// The leftover's share up to the weight `before` of `total`.
fn share(leftover: f64, before: f64, total: f64) -> f64 {
    if total > 0.0 {
        leftover * before / total
    } else {
        0.0
    }
}

/// The start and length of each slot along one axis from `origin`, with `avail` room and `gap`
/// between neighbours. Every position comes from prefix sums, never from a running position.
fn split(slots: &[Slot], origin: f64, avail: f64, gap: f64) -> Vec<(f64, f64)> {
    let fixed: f64 = slots
        .iter()
        .map(|s| if let Slot::Fixed(v) = s { *v } else { 0.0 })
        .sum();
    let weight: f64 = slots
        .iter()
        .map(|s| if let Slot::Fill(w) = s { *w } else { 0.0 })
        .sum();
    let gaps = gap * slots.len().saturating_sub(1) as f64;
    let leftover = (avail - fixed - gaps).max(0.0);
    let (mut fixed_before, mut weight_before) = (0.0, 0.0);
    slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let lead = share(leftover, weight_before, weight);
            let start = origin + fixed_before + gap * i as f64 + lead;
            let len = match *slot {
                Slot::Fixed(v) => {
                    fixed_before += v;
                    v
                }
                Slot::Fill(w) => {
                    weight_before += w;
                    share(leftover, weight_before, weight) - lead
                }
            };
            (start, len)
        })
        .collect()
}

/// Lays `children` along `axis` inside `parent` shrunk by `insets`, with `gap` between them.
/// `align` places each across the axis.
pub fn stack(
    axis: Axis,
    parent: Rect,
    children: &[Child],
    gap: f32,
    align: Align,
    insets: Insets,
) -> Vec<Rect> {
    let (x, y, w, h) = inner(parent, insets);
    let (main_origin, main_len, cross_origin, cross_len) = match axis {
        Axis::Horizontal => (x, w, y, h),
        Axis::Vertical => (y, h, x, w),
    };
    let slots: Vec<Slot> = children
        .iter()
        .map(|c| match c.main {
            Size::Fixed(v) | Size::Hug(v) => Slot::Fixed(clean(v)),
            Size::Fill(v) => Slot::Fill(clean(v)),
        })
        .collect();
    split(&slots, main_origin, main_len, clean(gap))
        .into_iter()
        .zip(children)
        .map(|((start, len), c)| {
            let size = match align {
                Align::Stretch => cross_len,
                _ => clean(c.cross),
            };
            let at = match align {
                Align::Start | Align::Stretch => cross_origin,
                Align::Center => cross_origin + (cross_len - size) / 2.0,
                Align::End => cross_origin + cross_len - size,
            };
            match axis {
                Axis::Horizontal => Rect::new(out(start), out(at), out(len), out(size)),
                Axis::Vertical => Rect::new(out(at), out(start), out(size), out(len)),
            }
        })
        .collect()
}

/// [`stack`] left to right.
pub fn row(parent: Rect, children: &[Child], gap: f32, align: Align, insets: Insets) -> Vec<Rect> {
    stack(Axis::Horizontal, parent, children, gap, align, insets)
}

/// [`stack`] top to bottom.
pub fn column(
    parent: Rect,
    children: &[Child],
    gap: f32,
    align: Align,
    insets: Insets,
) -> Vec<Rect> {
    stack(Axis::Vertical, parent, children, gap, align, insets)
}

/// Resolved column and row tracks; read cells with [`Grid::cell`].
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// Start and width of each column.
    cols: Vec<(f32, f32)>,
    /// Start and height of each row.
    rows: Vec<(f32, f32)>,
}

impl Grid {
    /// The rect of column `col`, row `row`; a zero rect when either is out of range.
    pub fn cell(&self, col: usize, row: usize) -> Rect {
        match (self.cols.get(col), self.rows.get(row)) {
            (Some(&(x, w)), Some(&(y, h))) => Rect::new(x, y, w, h),
            _ => Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    /// How many columns the grid has.
    pub fn columns(&self) -> usize {
        self.cols.len()
    }

    /// How many rows the grid has.
    pub fn rows(&self) -> usize {
        self.rows.len()
    }
}

fn tracks(tracks: &[Track], origin: f64, avail: f64, gap: f64) -> Vec<(f32, f32)> {
    let slots: Vec<Slot> = tracks
        .iter()
        .map(|t| match *t {
            Track::Fixed(v) | Track::Auto(v) => Slot::Fixed(clean(v)),
            Track::Star(v) => Slot::Fill(clean(v)),
        })
        .collect();
    split(&slots, origin, avail, gap)
        .into_iter()
        .map(|(s, l)| (out(s), out(l)))
        .collect()
}

/// Resolves `cols` and `rows` inside `parent` shrunk by `insets`, with `col_gap` between columns
/// and `row_gap` between rows.
pub fn grid(
    parent: Rect,
    cols: &[Track],
    rows: &[Track],
    col_gap: f32,
    row_gap: f32,
    insets: Insets,
) -> Grid {
    let (x, y, w, h) = inner(parent, insets);
    Grid {
        cols: tracks(cols, x, w, clean(col_gap)),
        rows: tracks(rows, y, h, clean(row_gap)),
    }
}
