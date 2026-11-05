//! One row of a lane graph (`data.lane-graph`, D170): edges, lane lines and a node dot, stateless.
//!
//! A row is 45 px high; a lane is 25 px wide from a 20 px left offset, so a lane's x is
//! `20 + lane * 25`, and the centre line is at 22.5. The app owns the lane layout and decides which
//! row holds which edges; rows in an edge are relative to the row drawn. Three passes, all on the
//! stroke list so that record order is draw order: the edges, the lane lines, then the dot, which
//! is the top of the row's strokes. Nothing clips an edge to its row (the owner's list clips) and
//! nothing animates (a graph is never animated). The ends are butt (there is no cap mode): an
//! edge ends under the dot and a lane line meets the next row's at the row's edge.

use std::f32::consts::TAU;

use crate::color::Rgba;
use crate::paint::{Painter, Rect, StrokeStyle};
use crate::ui::Ui;

/// The row's height.
pub const ROW_H: f32 = 45.0;
/// A lane's width.
pub const LANE_W: f32 = 25.0;
/// The offset of lane 0 from the row's left, and the margin past the last lane.
pub const LEFT: f32 = 20.0;
/// The dot's radius.
pub const DOT_R: f32 = 5.0;
/// The stroke width of an edge and a lane line.
pub const EDGE_W: f32 = 2.5;
/// A corner's largest radius.
pub const CORNER_R: f32 = 10.0;
/// The segments a quadratic corner is flattened to.
pub const CORNER_STEPS: usize = 8;
/// The points the dot's circle is flattened to.
const DOT_SEGMENTS: usize = 32;

/// A lane line through the row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneLine {
    pub lane: u16,
    pub color: Rgba,
    /// Runs from the row's top; else it starts at the centre line.
    pub above: bool,
    /// Runs to the row's bottom; else it ends at the centre line.
    pub below: bool,
}

/// An edge from this row's node (the child, at `child_row`) to its parent. Rows are relative to
/// the row drawn: 0 is this row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneEdge {
    pub child_lane: u16,
    pub child_row: i32,
    pub parent_lane: u16,
    pub parent_row: i32,
    pub color: Rgba,
}

/// What one row draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneRow<'a> {
    pub dot_lane: u16,
    pub dot_color: Rgba,
    pub lanes: &'a [LaneLine],
    pub edges: &'a [LaneEdge],
}

/// The width of a row whose lanes go up to `max_lane`: the offset, the lanes and the margin.
pub fn width(max_lane: u16) -> f32 {
    LEFT + (f32::from(max_lane) + 1.0) * LANE_W + LEFT
}

fn lane_x(lane: u16) -> f32 {
    LEFT + f32::from(lane) * LANE_W
}

/// The points of an edge between nodes `d_lanes` lanes and `d_rows` rows apart, in canonical
/// coordinates: the child at (0, 0), the parent at (`d_lanes` × 25, `d_rows` × 45). The same lane
/// is a straight line of two points. Otherwise the edge branches out (the parent's x is greater:
/// a diagonal first, then straight down) or merges in (straight down first, then the diagonal);
/// the diagonal spans `min(lane gap, 0.6 of the height)` when the edge is at most a row tall, else
/// `min(row, lane gap, height)`. The bend is rounded by a quadratic of radius
/// `min(10, half of each leg)`, flattened to 8 segments; a leg under 0.1 gets no corner.
pub fn edge_points(d_lanes: i32, d_rows: i32) -> Vec<[f32; 2]> {
    let x2 = f64::from(d_lanes) * f64::from(LANE_W);
    let y2 = f64::from(d_rows) * f64::from(ROW_H);
    let mut pts: Vec<[f64; 2]> = vec![[0.0, 0.0]];
    if d_lanes == 0 {
        pts.push([0.0, y2]);
    } else {
        let gap = x2.abs();
        let diag = if y2 <= f64::from(ROW_H) {
            gap.min(y2 * 0.6)
        } else {
            f64::from(ROW_H).min(gap).min(y2)
        };
        let bend = if x2 > 0.0 {
            [x2, diag]
        } else {
            [0.0, y2 - diag]
        };
        let (ab, bc) = ([bend[0], bend[1]], [x2 - bend[0], y2 - bend[1]]);
        let (ab_len, bc_len) = (ab[0].hypot(ab[1]), bc[0].hypot(bc[1]));
        if ab_len < 0.1 || bc_len < 0.1 {
            pts.push(bend);
        } else {
            let r = f64::from(CORNER_R).min(ab_len / 2.0).min(bc_len / 2.0);
            let p_in = [bend[0] - r * ab[0] / ab_len, bend[1] - r * ab[1] / ab_len];
            let p_out = [bend[0] + r * bc[0] / bc_len, bend[1] + r * bc[1] / bc_len];
            pts.push(p_in);
            for k in 1..=CORNER_STEPS {
                let t = k as f64 / CORNER_STEPS as f64;
                let u = 1.0 - t;
                pts.push([
                    u * u * p_in[0] + 2.0 * u * t * bend[0] + t * t * p_out[0],
                    u * u * p_in[1] + 2.0 * u * t * bend[1] + t * t * p_out[1],
                ]);
            }
        }
        pts.push([x2, y2]);
        pts.dedup();
    }
    pts.into_iter()
        .map(|p| [p[0] as f32, p[1] as f32])
        .collect()
}

/// Draws `row` with the top left of its 45 px at `origin` (logical px): the edges, the lane lines
/// and the dot, in that order.
pub fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, origin: (f32, f32), row: &LaneRow<'_>) {
    let (ox, oy) = origin;
    let centre = oy + ROW_H / 2.0;
    for e in row.edges {
        let (x, y) = (
            ox + lane_x(e.child_lane),
            centre + e.child_row as f32 * ROW_H,
        );
        let d_lanes = i32::from(e.parent_lane) - i32::from(e.child_lane);
        let d_rows = e.parent_row.saturating_sub(e.child_row);
        let pts: Vec<[f32; 2]> = edge_points(d_lanes, d_rows)
            .into_iter()
            .map(|[px, py]| [x + px, y + py])
            .collect();
        p.stroke(&pts, false, StrokeStyle::solid(e.color, EDGE_W));
    }
    for l in row.lanes {
        let x = ox + lane_x(l.lane);
        let top = if l.above { oy } else { centre };
        let bottom = if l.below { oy + ROW_H } else { centre };
        if top < bottom {
            p.stroke(
                &[[x, top], [x, bottom]],
                false,
                StrokeStyle::solid(l.color, EDGE_W),
            );
        }
    }
    // A closed stroke on the circle of radius 2, 6 wide, is a disc of radius 5 that, being a stroke,
    // is drawn above the lines. It is flattened here on the outer radius (`Painter::arc` flattens on
    // the centre radius, which leaves visible flats on a disc this size).
    let (cx, cy) = (ox + lane_x(row.dot_lane), centre);
    let pts: Vec<[f32; 2]> = (0..DOT_SEGMENTS)
        .map(|i| {
            let a = TAU * i as f32 / DOT_SEGMENTS as f32;
            [cx + (DOT_R - 3.0) * a.cos(), cy + (DOT_R - 3.0) * a.sin()]
        })
        .collect();
    p.stroke(&pts, true, StrokeStyle::solid(row.dot_color, 6.0));
}

impl Ui<'_> {
    /// Draws `row` with the top left of `rect` as its origin (the rect's size is ignored: a row is
    /// [`ROW_H`] high and [`width`] wide). Decorative: no node, no region.
    pub fn lane_graph_row(&mut self, rect: Rect, row: &LaneRow<'_>) {
        paint(self.painter(), (rect.x, rect.y), row);
    }
}
