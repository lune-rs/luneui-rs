//! The one routine that draws a text selection (R18, D17): every text view hands it the selected
//! part of each visual row it shows, and it fills them as joined shapes with rounded outer and
//! inner corners, `Theme::selection_pad` past each row's ends.
//!
//! Rows that touch (one's bottom is the next one's top, within half a pixel) and share at least a
//! pixel of width join into one shape; rows that do not are shapes of their own. The shape is cut
//! into one band per row, each a small polygon triangulated alone, and one outline loop takes the
//! antialiasing fringe ([`Painter::fill_joined`]), so no seam shows between rows and the cost is
//! linear in the rows. A caller builds `rows` from the rows in view only (D179, D195) and says with
//! `open` whether the selection runs on past the first or the last of them: that side is flat.

use std::f32::consts::{FRAC_PI_2, PI};

use crate::color::Rgba;
use crate::paint::{Painter, Rect};

/// Segments per quarter arc: at radius 3 and scale 2 a chord is off the circle by about 0.1 px.
const ARC_STEPS: usize = 4;
/// Rows whose gap (or overlap) is at most this, vertically, touch.
const TOUCH: f32 = 0.5;
/// Touching rows that share less width than this, before the padding, are separate shapes.
const MIN_SHARED: f32 = 1.0;
/// A step between rows narrower than this is drawn square, without arcs.
const MIN_STEP: f32 = 1.0;
/// An arc of a smaller radius is its corner point.
const MIN_RADIUS: f32 = 0.01;

/// One joined selection shape: the bands it is cut into, top to bottom, and its outline.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Joined {
    /// One simple polygon per row, in logical px, tiling the shape.
    pub parts: Vec<Vec<[f32; 2]>>,
    /// The shape's one outer loop, in logical px: down the right side, up the left.
    pub outline: Vec<[f32; 2]>,
}

/// Fills the selection `rows` with `color`, below the layer's text. `rows` run top to bottom, each
/// the selected part of one visual row in the current drawing space; `open` says the selection
/// continues past the first row (`.0`) or the last (`.1`), so that side has no rounded corners.
/// The radius and the padding are the painter theme's `selection_radius` and `selection_pad`.
/// Rows that are empty or not finite are left out, and so are rows wholly above or below the
/// painter's clip: a side whose next row was left out that way is open.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rows: &[Rect],
    open: (bool, bool),
    color: Rgba,
) {
    let (radius, pad) = (p.theme.selection_radius, p.theme.selection_pad);
    let (rows, open) = in_view(rows, p.clip_rect(), open);
    for shape in shapes(rows, open, radius, pad) {
        p.fill_joined(&shape.parts, &shape.outline, color);
    }
}

/// The joined shapes [`paint`] fills for `rows` and `open`, with corners of `radius` and rows
/// reaching `pad` past each end. A corner's radius is at most half its step, half its row's
/// height and, for a corner of the row itself, half the row's width, so a narrow row stays a
/// valid shape. A radius or padding that is negative or not finite counts as 0.
pub fn shapes(rows: &[Rect], open: (bool, bool), radius: f32, pad: f32) -> Vec<Joined> {
    let sane = |v: f32| if v.is_finite() && v > 0.0 { v } else { 0.0 };
    let (radius, pad) = (sane(radius), sane(pad));
    let rows: Vec<Rect> = rows
        .iter()
        .copied()
        .filter(|r| {
            [r.x, r.y, r.w, r.h, r.x - pad, r.right() + pad, r.bottom()]
                .iter()
                .all(|v| v.is_finite())
                && r.w > 0.0
                && r.h > 0.0
        })
        .collect();
    let mut groups: Vec<Vec<Rect>> = Vec::new();
    for r in rows {
        let joins = groups
            .last()
            .and_then(|g| g.last())
            .is_some_and(|prev| joins(prev, &r));
        match groups.last_mut() {
            Some(g) if joins => g.push(r),
            _ => groups.push(vec![r]),
        }
    }
    let last = groups.len().saturating_sub(1);
    groups
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let open = (open.0 && i == 0, open.1 && i == last);
            joined(&bands(g, pad), open, radius)
        })
        .collect()
}

/// Whether `next`, the row after `prev`, joins it: they touch and share some width.
fn joins(prev: &Rect, next: &Rect) -> bool {
    (next.y - prev.bottom()).abs() <= TOUCH
        && next.right().min(prev.right()) - next.x.max(prev.x) >= MIN_SHARED
}

/// The run of `rows` that reaches into `clip` vertically, and `open` with a side opened where the
/// row past it was left out and joined it.
fn in_view(rows: &[Rect], clip: Option<Rect>, open: (bool, bool)) -> (&[Rect], (bool, bool)) {
    let Some(c) = clip else {
        return (rows, open);
    };
    let from = rows
        .iter()
        .position(|r| r.bottom() > c.y)
        .unwrap_or(rows.len());
    let to = rows
        .iter()
        .rposition(|r| r.y < c.bottom())
        .map_or(0, |i| i + 1)
        .max(from);
    let kept = &rows[from..to];
    let (Some(first), Some(last)) = (kept.first(), kept.last()) else {
        return (kept, open);
    };
    let above = from > 0 && joins(&rows[from - 1], first);
    let below = to < rows.len() && joins(last, &rows[to]);
    (kept, (open.0 || above, open.1 || below))
}

/// One padded row: its left and right ends, top and bottom.
#[derive(Debug, Clone, Copy)]
struct Band {
    l: f32,
    r: f32,
    t: f32,
    b: f32,
}

impl Band {
    fn w(&self) -> f32 {
        self.r - self.l
    }

    fn h(&self) -> f32 {
        self.b - self.t
    }

    /// The band mirrored about x = 0: its left end becomes the right one.
    fn mirrored(self) -> Self {
        Self {
            l: -self.r,
            r: -self.l,
            ..self
        }
    }
}

/// The rows of one group padded sideways, each shared edge moved to the middle of the two rows'
/// edges, so the bands touch exactly.
fn bands(group: &[Rect], pad: f32) -> Vec<Band> {
    let mut out: Vec<Band> = group
        .iter()
        .map(|r| Band {
            l: r.x - pad,
            r: r.right() + pad,
            t: r.y,
            b: r.bottom(),
        })
        .collect();
    for i in 1..out.len() {
        let y = (out[i - 1].b + out[i].t) / 2.0;
        out[i - 1].b = y;
        out[i].t = y;
    }
    out
}

/// The shape of one group of touching `bands`.
fn joined(bands: &[Band], open: (bool, bool), radius: f32) -> Joined {
    let (right, rs) = chain(bands, open, radius);
    let mirrored: Vec<Band> = bands.iter().map(|b| b.mirrored()).collect();
    let (mut left, ls) = chain(&mirrored, open, radius);
    for q in &mut left {
        q[0] = -q[0];
    }
    // Band i runs from split i - 1 to split i on both chains.
    let cut = |splits: &[usize], len: usize, i: usize| {
        let from = if i == 0 { 0 } else { splits[i - 1] };
        let to = splits.get(i).copied().unwrap_or(len - 1);
        from..=to
    };
    let parts = (0..bands.len())
        .map(|i| {
            let mut part: Vec<[f32; 2]> = right[cut(&rs, right.len(), i)].to_vec();
            part.extend(left[cut(&ls, left.len(), i)].iter().rev());
            part
        })
        .collect();
    let mut outline = right;
    outline.extend(left.iter().rev());
    Joined { parts, outline }
}

/// Pushes the arc of `radius` about `c` from angle `from` to `to` (radians, y down: 0 points
/// right, π/2 down), its ends exactly `start` and `end`; a radius too small is the `corner`.
fn quarter(
    out: &mut Vec<[f32; 2]>,
    c: [f32; 2],
    radius: f32,
    (from, to): (f32, f32),
    (start, end): ([f32; 2], [f32; 2]),
    corner: [f32; 2],
) {
    if radius < MIN_RADIUS {
        out.push(corner);
        return;
    }
    out.push(start);
    for k in 1..ARC_STEPS {
        let a = from + (to - from) * k as f32 / ARC_STEPS as f32;
        out.push([c[0] + radius * a.cos(), c[1] + radius * a.sin()]);
    }
    out.push(end);
}

/// The right side of the shape of `bands`, walking down, and the index of each split point: the
/// point on the line between band i and band i + 1 where band i's polygon ends and band i + 1's
/// begins (the innermost point on that line, so neither band's polygon crosses itself).
fn chain(bands: &[Band], open: (bool, bool), radius: f32) -> (Vec<[f32; 2]>, Vec<usize>) {
    let mut pts: Vec<[f32; 2]> = Vec::with_capacity(bands.len() * 2 * (ARC_STEPS + 1) + 2);
    let mut splits = Vec::with_capacity(bands.len().saturating_sub(1));
    let own = |b: &Band| radius.min(b.w() / 2.0).min(b.h() / 2.0).max(0.0);
    let Some(first) = bands.first() else {
        return (pts, splits);
    };
    if open.0 {
        pts.push([first.r, first.t]);
    } else {
        let q = own(first);
        quarter(
            &mut pts,
            [first.r - q, first.t + q],
            q,
            (-FRAC_PI_2, 0.0),
            ([first.r - q, first.t], [first.r, first.t + q]),
            [first.r, first.t],
        );
    }
    for w in bands.windows(2) {
        let (up, down) = (w[0], w[1]);
        let (a, b, y) = (up.r, down.r, up.b);
        let step = (b - a).abs();
        if step < MIN_STEP {
            pts.push([a, y]);
            if step > 0.0 {
                pts.push([b, y]);
            }
            splits.push(if b < a {
                pts.len() - 1
            } else {
                pts.len() - 1 - usize::from(step > 0.0)
            });
        } else if b < a {
            // The upper row sticks out: its own corner, then a fillet below the line.
            let r1 = own(&up).min(step / 2.0);
            let r2 = radius.min(step / 2.0).min(down.h() / 2.0);
            quarter(
                &mut pts,
                [a - r1, y - r1],
                r1,
                (0.0, FRAC_PI_2),
                ([a, y - r1], [a - r1, y]),
                [a, y],
            );
            splits.push(pts.len());
            quarter(
                &mut pts,
                [b + r2, y + r2],
                r2,
                (-FRAC_PI_2, -PI),
                ([b + r2, y], [b, y + r2]),
                [b, y],
            );
        } else {
            // The lower row sticks out: a fillet above the line, then its own corner.
            let r1 = radius.min(step / 2.0).min(up.h() / 2.0);
            let r2 = own(&down).min(step / 2.0);
            quarter(
                &mut pts,
                [a + r1, y - r1],
                r1,
                (PI, FRAC_PI_2),
                ([a, y - r1], [a + r1, y]),
                [a, y],
            );
            splits.push(pts.len() - 1);
            quarter(
                &mut pts,
                [b - r2, y + r2],
                r2,
                (-FRAC_PI_2, 0.0),
                ([b - r2, y], [b, y + r2]),
                [b, y],
            );
        }
    }
    let end = bands[bands.len() - 1];
    if open.1 {
        pts.push([end.r, end.b]);
    } else {
        let q = own(&end);
        quarter(
            &mut pts,
            [end.r - q, end.b - q],
            q,
            (0.0, FRAC_PI_2),
            ([end.r, end.b - q], [end.r - q, end.b]),
            [end.r, end.b],
        );
    }
    (pts, splits)
}
