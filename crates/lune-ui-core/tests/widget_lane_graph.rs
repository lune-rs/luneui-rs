//! The lane graph row (`data.lane-graph` acceptance, D170): the dot, the lane lines and the edges
//! at the geometry the entry states, the canonical edge points and their flattening, the three
//! passes in draw order, the probes at scale 2, input that is huge or empty, and that the row is
//! decorative (no node, no region).

use lune_ui_core::color::{Rgba, premul};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::{Scene, StrokeVertex};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::lane_graph::{
    CORNER_STEPS, LaneEdge, LaneLine, LaneRow, ROW_H, edge_points, paint, width,
};

const EDGE: Rgba = [200, 30, 30, 255];
const LINE: Rgba = [30, 200, 30, 255];
const DOT: Rgba = [30, 30, 200, 255];

fn scene(scale: f32, row: &LaneRow<'_>) -> Scene {
    let mut t = FakeText;
    let mut p: Painter<'_, u8> =
        Painter::new(scale, Rect::new(0.0, 0.0, 400.0, 300.0), &mut t, None, DARK);
    paint(&mut p, (10.0, 20.0), row);
    p.scene
}

fn strokes(s: &Scene) -> &[StrokeVertex] {
    &s.layers[0].geometry.strokes
}

fn of(s: &Scene, c: Rgba) -> Vec<StrokeVertex> {
    strokes(s)
        .iter()
        .copied()
        .filter(|v| v.color == premul(c))
        .collect()
}

/// The bounding box (min x, min y, max x, max y) of the vertices.
fn bbox(v: &[StrokeVertex]) -> [f32; 4] {
    let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for v in v {
        b = [
            b[0].min(v.pos[0]),
            b[1].min(v.pos[1]),
            b[2].max(v.pos[0]),
            b[3].max(v.pos[1]),
        ];
    }
    b
}

fn base_row<'a>(lanes: &'a [LaneLine], edges: &'a [LaneEdge], dot_lane: u16) -> LaneRow<'a> {
    LaneRow {
        dot_lane,
        dot_color: DOT,
        lanes,
        edges,
    }
}

#[test]
fn the_dot_is_in_lane_2_at_x_70_on_the_centre_line() {
    // Origin (10, 20): lane 2 is at 10 + 20 + 2 * 25 = 80, the centre line at 20 + 22.5.
    let s = scene(1.0, &base_row(&[], &[], 2));
    let b = bbox(&of(&s, DOT));
    // The tessellated ring's box is a polygon's, with the antialiasing fringe: centred within
    // half a px, and 10 px across plus the fringe. The GPU probe in lune-ui-wgpu reads the pixels.
    assert!(((b[0] + b[2]) / 2.0 - 80.0).abs() < 0.5, "{b:?}");
    assert!(((b[1] + b[3]) / 2.0 - 42.5).abs() < 0.5, "{b:?}");
    assert!(
        b[2] - b[0] >= 10.0 && b[2] - b[0] < 12.5,
        "diameter 10: {b:?}"
    );
}

#[test]
fn a_line_continuing_above_starts_at_the_row_top_and_one_not_at_the_centre() {
    let up = [LaneLine {
        lane: 0,
        color: LINE,
        above: true,
        below: false,
    }];
    let b = bbox(&of(&scene(1.0, &base_row(&up, &[], 1)), LINE));
    assert!((b[1] - 20.0).abs() < 0.01, "from the top: {b:?}");
    assert!((b[3] - 42.5).abs() < 0.01, "to the centre: {b:?}");
    let down = [LaneLine {
        lane: 0,
        color: LINE,
        above: false,
        below: true,
    }];
    let b = bbox(&of(&scene(1.0, &base_row(&down, &[], 1)), LINE));
    assert!((b[1] - 42.5).abs() < 0.01, "from the centre: {b:?}");
    assert!((b[3] - 65.0).abs() < 0.01, "to the bottom: {b:?}");
    let neither = [LaneLine {
        lane: 0,
        color: LINE,
        above: false,
        below: false,
    }];
    assert!(of(&scene(1.0, &base_row(&neither, &[], 1)), LINE).is_empty());
}

#[test]
fn edge_points_start_at_the_child_and_end_at_the_parent() {
    for (dl, dr) in [
        (1, 1),
        (3, 1),
        (3, 3),
        (-3, 3),
        (-1, 5),
        (2, 40),
        (-2, 0),
        (5, -2),
    ] {
        let p = edge_points(dl, dr);
        assert_eq!(p[0], [0.0, 0.0], "{dl},{dr}");
        assert_eq!(
            *p.last().expect("a point"),
            [dl as f32 * 25.0, dr as f32 * 45.0],
            "{dl},{dr}"
        );
        assert!(p.iter().flatten().all(|v| v.is_finite()));
    }
}

#[test]
fn the_same_lane_edge_is_two_points() {
    assert_eq!(edge_points(0, 4), vec![[0.0, 0.0], [0.0, 180.0]]);
    assert_eq!(edge_points(0, 0), vec![[0.0, 0.0], [0.0, 0.0]]);
}

#[test]
fn a_branch_out_goes_across_first_then_down_and_a_merge_goes_down_first() {
    // Three lanes right, three rows down: diagonal 45 (the row), bend (75, 45), corner radius 10.
    let out = edge_points(3, 3);
    let (ab, bc) = (87.46_f32, 90.0_f32);
    let p_in = [75.0 - 10.0 * 75.0 / ab, 45.0 - 10.0 * 45.0 / ab];
    assert!(
        (out[1][0] - p_in[0]).abs() < 1e-2 && (out[1][1] - p_in[1]).abs() < 1e-2,
        "{:?}",
        out[1]
    );
    let p_out = [75.0, 45.0 + 10.0 * 90.0 / bc];
    assert!((out[1 + CORNER_STEPS][0] - p_out[0]).abs() < 1e-3);
    assert!((out[1 + CORNER_STEPS][1] - p_out[1]).abs() < 1e-3);
    assert_eq!(out.len(), CORNER_STEPS + 3);

    // The merge: straight down to (0, 90), then across.
    let merge = edge_points(-3, 3);
    assert!(
        merge[1][0].abs() < 1e-3 && (merge[1][1] - 80.0).abs() < 1e-3,
        "{:?}",
        merge[1]
    );
    let last = merge[merge.len() - 2];
    assert!(last[0] < 0.0 && last[1] > 90.0, "{last:?}");
}

#[test]
fn an_edge_one_row_tall_uses_the_0_6_rule_and_a_taller_one_the_other() {
    // Height 45, lane gap 75: diagonal min(75, 0.6 * 45) = 27, so the bend is (75, 27); the first
    // leg is 79.7 long and the second 18, so the radius is 9.
    let p = edge_points(3, 1);
    let ab = (75.0_f32.hypot(27.0)).max(0.0);
    let want = [75.0 - 9.0 * 75.0 / ab, 27.0 - 9.0 * 27.0 / ab];
    assert!(
        (p[1][0] - want[0]).abs() < 1e-2 && (p[1][1] - want[1]).abs() < 1e-2,
        "{:?}",
        p[1]
    );
    // Height 135 (above a row): diagonal min(45, 75, 135) = 45, not 0.6 of anything.
    let p = edge_points(3, 3);
    let ab = 75.0_f32.hypot(45.0);
    assert!((p[1][1] - (45.0 - 10.0 * 45.0 / ab)).abs() < 1e-2);
}

/// The distance from `p` to the polyline `pts`.
fn dist(p: [f64; 2], pts: &[[f32; 2]]) -> f64 {
    pts.windows(2)
        .map(|w| {
            let (a, b) = (
                [f64::from(w[0][0]), f64::from(w[0][1])],
                [f64::from(w[1][0]), f64::from(w[1][1])],
            );
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len2 = dx * dx + dy * dy;
            let t = if len2 == 0.0 {
                0.0
            } else {
                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
            };
            (p[0] - (a[0] + t * dx)).hypot(p[1] - (a[1] + t * dy))
        })
        .fold(f64::MAX, f64::min)
}

#[test]
fn the_flattened_corner_stays_within_a_quarter_device_pixel_at_scale_2() {
    for (dl, dr) in [
        (3, 3),
        (-3, 3),
        (3, 1),
        (1, 1),
        (-1, 1),
        (2, 2),
        (-5, 8),
        (8, 2),
    ] {
        let pts = edge_points(dl, dr);
        // The true curve: the same rule with the quadratic sampled finely.
        let (x2, y2) = (f64::from(dl) * 25.0, f64::from(dr) * 45.0);
        let gap = x2.abs();
        let diag = if y2 <= 45.0 {
            gap.min(y2 * 0.6)
        } else {
            45.0_f64.min(gap).min(y2)
        };
        let bend = if x2 > 0.0 {
            [x2, diag]
        } else {
            [0.0, y2 - diag]
        };
        let (ab, bc) = ([bend[0], bend[1]], [x2 - bend[0], y2 - bend[1]]);
        let (al, bl) = (ab[0].hypot(ab[1]), bc[0].hypot(bc[1]));
        let r = 10.0_f64.min(al / 2.0).min(bl / 2.0);
        let p_in = [bend[0] - r * ab[0] / al, bend[1] - r * ab[1] / al];
        let p_out = [bend[0] + r * bc[0] / bl, bend[1] + r * bc[1] / bl];
        for k in 0..=1000 {
            let t = f64::from(k) / 1000.0;
            let u = 1.0 - t;
            let q = [
                u * u * p_in[0] + 2.0 * u * t * bend[0] + t * t * p_out[0],
                u * u * p_in[1] + 2.0 * u * t * bend[1] + t * t * p_out[1],
            ];
            let d = dist(q, &pts) * 2.0;
            assert!(d < 0.25, "({dl},{dr}) t {t}: {d} device px");
        }
    }
}

#[test]
fn the_passes_draw_in_order_edges_then_lane_lines_then_the_dot() {
    let lanes = [LaneLine {
        lane: 1,
        color: LINE,
        above: true,
        below: true,
    }];
    let edges = [LaneEdge {
        child_lane: 1,
        child_row: 0,
        parent_lane: 2,
        parent_row: 2,
        color: EDGE,
    }];
    let s = scene(1.0, &base_row(&lanes, &edges, 1));
    let order: Vec<Rgba> = {
        let mut seen: Vec<Rgba> = Vec::new();
        for v in strokes(&s) {
            for c in [EDGE, LINE, DOT] {
                if v.color == premul(c) && seen.last() != Some(&c) {
                    seen.push(c);
                }
            }
        }
        seen
    };
    assert_eq!(order, vec![EDGE, LINE, DOT]);
    let last_other = strokes(&s)
        .iter()
        .rposition(|v| v.color != premul(DOT))
        .expect("other strokes");
    let first_dot = strokes(&s)
        .iter()
        .position(|v| v.color == premul(DOT))
        .expect("dot");
    assert!(
        first_dot > last_other,
        "the dot is the top of the row's strokes"
    );
}

#[test]
fn the_width_is_the_offset_the_lanes_and_the_margin() {
    assert_eq!(width(0), 65.0);
    assert_eq!(width(3), 20.0 + 4.0 * 25.0 + 20.0);
    assert_eq!(width(u16::MAX), 20.0 + 65536.0 * 25.0 + 20.0);
}

#[test]
fn probes_at_scale_2_row_90_dot_20_at_40_plus_lane_times_50_and_an_edge_5_wide() {
    let lanes = [LaneLine {
        lane: 1,
        color: LINE,
        above: true,
        below: true,
    }];
    let edges = [LaneEdge {
        child_lane: 3,
        child_row: 0,
        parent_lane: 3,
        parent_row: 1,
        color: EDGE,
    }];
    let s = scene(2.0, &base_row(&lanes, &edges, 2));
    let line = bbox(&of(&s, LINE));
    assert!(
        (line[3] - line[1] - 90.0).abs() < 0.01,
        "the row is 90 px: {line:?}"
    );
    let dot = bbox(&of(&s, DOT));
    // Origin x 10 -> 20 px; lane 2 is at 20 + 40 + 100 = 160 px: 40 + 2 * 50 past the origin.
    assert!(((dot[0] + dot[2]) / 2.0 - 160.0).abs() < 1.0, "{dot:?}");
    assert!(
        dot[2] - dot[0] >= 20.0 && dot[2] - dot[0] < 24.0,
        "diameter 20 px and the fringe: {dot:?}"
    );
    let edge = bbox(&of(&s, EDGE));
    assert!(
        edge[2] - edge[0] >= 5.0 && edge[2] - edge[0] <= 7.5,
        "5 px wide and the 1 px fringe each side: {edge:?}"
    );
    assert!(((edge[0] + edge[2]) / 2.0 - (20.0 + 40.0 + 150.0)).abs() < 0.01);
}

#[test]
fn huge_empty_and_far_input_never_panics_and_never_makes_a_nan() {
    let edges = [
        LaneEdge {
            child_lane: 0,
            child_row: 0,
            parent_lane: 1,
            parent_row: 1_000_000_000,
            color: EDGE,
        },
        LaneEdge {
            child_lane: u16::MAX,
            child_row: i32::MIN,
            parent_lane: 0,
            parent_row: i32::MAX,
            color: EDGE,
        },
        LaneEdge {
            child_lane: 0,
            child_row: 0,
            parent_lane: 0,
            parent_row: 0,
            color: EDGE,
        },
        LaneEdge {
            child_lane: 2,
            child_row: 5,
            parent_lane: 1,
            parent_row: -5,
            color: EDGE,
        },
    ];
    let lanes = [LaneLine {
        lane: u16::MAX,
        color: LINE,
        above: true,
        below: true,
    }];
    let s = scene(2.0, &base_row(&lanes, &edges, u16::MAX));
    for v in strokes(&s) {
        assert!(v.pos.iter().all(|c| c.is_finite()), "{v:?}");
    }
    let empty = scene(1.0, &base_row(&[], &[], 0));
    assert!(!of(&empty, DOT).is_empty(), "an empty row is the dot alone");
    for (dl, dr) in [
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (1, 0),
        (-1, 0),
        (0, i32::MIN),
    ] {
        assert!(edge_points(dl, dr).iter().flatten().all(|v| v.is_finite()));
    }
}

#[test]
fn the_row_is_decorative_no_node_no_region() {
    let mut h = UiHarness::new((400.0, 300.0));
    let (_, out) = h.frame(|ui| {
        ui.lane_graph_row(Rect::new(10.0, 20.0, 0.0, 0.0), &base_row(&[], &[], 1));
    });
    assert!(out.tree.nodes.is_empty());
    assert_eq!(ROW_H, 45.0);
}

// ---- many edges at one dot (R17) ----

const SECOND: Rgba = [220, 150, 20, 255];
const THIRD: Rgba = [20, 180, 220, 255];

/// An octopus: one dot on lane 0, an edge to each of the lanes 0, 1 and 2 (rows 1, 2, 3), each in
/// its own colour.
fn octopus() -> [LaneEdge; 3] {
    let at = |lane: u16, rows: i32, color: Rgba| LaneEdge {
        child_lane: 0,
        child_row: 0,
        parent_lane: lane,
        parent_row: rows,
        color,
    };
    [at(0, 1, EDGE), at(1, 2, SECOND), at(2, 3, THIRD)]
}

#[test]
fn a_row_with_three_edges_records_one_stroke_per_edge_and_draws_the_dot_after_them() {
    let edges = octopus();
    let all = scene(1.0, &base_row(&[], &edges, 0));
    // Each edge is the same strokes it is alone: nothing joins, drops or doubles one.
    for e in &edges {
        let alone = scene(1.0, &base_row(&[], std::slice::from_ref(e), 0));
        assert_eq!(of(&all, e.color), of(&alone, e.color), "{:?}", e.color);
        assert!(!of(&all, e.color).is_empty());
    }
    // Their colours never mix: every vertex is an edge's or the dot's.
    let known = [EDGE, SECOND, THIRD, DOT].map(premul);
    assert!(strokes(&all).iter().all(|v| known.contains(&v.color)));
    let last_edge = strokes(&all)
        .iter()
        .rposition(|v| v.color != premul(DOT))
        .expect("edges");
    let first_dot = strokes(&all)
        .iter()
        .position(|v| v.color == premul(DOT))
        .expect("dot");
    assert!(first_dot > last_edge, "the dot is drawn after every edge");
    // The dot is where all three start, and a disc: its box is 10 px plus the fringe.
    let dot = bbox(&of(&all, DOT));
    assert!(((dot[0] + dot[2]) / 2.0 - 30.0).abs() < 0.5, "{dot:?}");
    assert!(((dot[1] + dot[3]) / 2.0 - 42.5).abs() < 0.5, "{dot:?}");
    // The three edges leave that centre: each one's first vertices lie near it.
    for e in &edges {
        let b = bbox(&of(&all, e.color));
        assert!(b[0] <= 30.0 + 0.1 && b[1] <= 42.5 + 0.1, "{b:?}");
    }
}

#[test]
fn edges_that_cross_lanes_take_the_clip_the_owner_set() {
    let edges = octopus();
    let row = Rect::new(10.0, 20.0, 140.0, ROW_H);
    let mut t = FakeText;
    let mut p: Painter<'_, u8> =
        Painter::new(2.0, Rect::new(0.0, 0.0, 400.0, 300.0), &mut t, None, DARK);
    p.push_clip(row);
    paint(&mut p, (10.0, 20.0), &base_row(&[], &edges, 0));
    p.pop_clip();
    let s = p.scene;
    let clip = strokes(&s)[0].clip;
    assert_eq!(
        clip,
        [20.0, 40.0, 280.0, 90.0],
        "the row's rect in device px: x, y, w, h"
    );
    assert!(
        strokes(&s).iter().all(|v| v.clip == clip),
        "the dot and every edge, whichever lane it crosses to, carry the row's clip"
    );
    // An edge reaches past the row; only the clip stops it.
    let b = bbox(&of(&s, THIRD));
    assert!(
        b[3] > clip[1] + clip[3],
        "the third edge runs below the row: {b:?}"
    );
}
