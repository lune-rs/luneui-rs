//! The vector primitives: dash, arc, sector, path-fill and the rounded clip. Each is checked for
//! what it records on which layer, under which clip and transform, and for bad input.

use lune_ui_core::paint::{LayerId, Painter, Rect, StrokeStyle, Xform};
use lune_ui_core::scene::{FrameParams, Scene, StrokeVertex};
use lune_ui_core::testing::{DrawCall, FakeText, RecordingRenderer};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK as THEME;

type P<'a> = Painter<'a, u8>;

const WHITE: [u8; 4] = [255, 255, 255, 255];

fn painter(text: &mut FakeText, scale: f32) -> P<'_> {
    Painter::new(scale, Rect::new(0.0, 0.0, 400.0, 300.0), text, None, THEME)
}

fn xf(scale: f32, alpha: f32) -> Xform {
    Xform {
        origin: (0.0, 0.0),
        scale,
        dx: 0.0,
        dy: 0.0,
        alpha,
    }
}

/// The x extent of each run of 6 stroke vertices (one segment), left edge to right edge.
fn runs_x(v: &[StrokeVertex]) -> Vec<(f32, f32)> {
    v.chunks(6)
        .map(|c| {
            let xs = c.iter().map(|v| v.pos[0]);
            (
                xs.clone().fold(f32::MAX, f32::min),
                xs.fold(f32::MIN, f32::max),
            )
        })
        .collect()
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

// ── dash ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn dash_one_two_at_scale_two_is_two_on_four_off() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.dash(&[[0.0, 5.0], [12.0, 5.0]], false, 1.0, 1.0, 2.0, WHITE);
    let v = &p.scene.layers[0].geometry.strokes;
    let runs = runs_x(v);
    assert_eq!(runs.len(), 4);
    for (k, (a, b)) in runs.iter().enumerate() {
        assert!(approx(*a, 6.0 * k as f32), "run {k}: {runs:?}");
        assert!(approx(*b, 6.0 * k as f32 + 2.0), "run {k}: {runs:?}");
    }
    // 1 logical px wide at scale 2: half width 1 device px.
    assert!(v.iter().all(|v| approx(v.style[1], 1.0)));
}

#[test]
fn dash_is_one_pixel_wide_at_scale_one() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    p.dash(&[[0.0, 5.0], [6.0, 5.0]], false, 1.0, 1.0, 2.0, WHITE);
    let v = &p.scene.layers[0].geometry.strokes;
    assert!(v.iter().all(|v| approx(v.style[1], 0.5)));
    assert_eq!(runs_x(v).len(), 2);
}

#[test]
fn dash_respects_layer_clip_scale_and_alpha() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.on(LayerId::Popup);
    p.set_clip(Some(Rect::new(1.0, 2.0, 30.0, 40.0)));
    p.push_xform(xf(2.0, 0.5));
    p.dash(&[[0.0, 5.0], [12.0, 5.0]], false, 1.0, 1.0, 2.0, WHITE);
    assert!(p.scene.layers[0].geometry.strokes.is_empty());
    let v = &p.scene.layers[2].geometry.strokes;
    // Xform scale 2 at scale 2: a run is 1 * 2 * 2 = 4 device px long, a period 12.
    let runs = runs_x(v);
    assert!(approx(runs[0].1 - runs[0].0, 4.0), "{runs:?}");
    assert!(approx(runs[1].0 - runs[0].0, 12.0), "{runs:?}");
    // The clip goes through the transform, in physical px.
    assert!(v.iter().all(|v| v.clip == [4.0, 8.0, 120.0, 160.0]));
    // Alpha 0.5 on an opaque white: premultiplied alpha 0.5.
    assert!(v.iter().all(|v| approx(v.color[3], 0.5)));
}

#[test]
fn dash_with_no_gap_is_solid_and_a_fine_pattern_is_solid() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    p.dash(&[[0.0, 0.0], [10.0, 0.0]], false, 1.0, 2.0, 0.0, WHITE);
    assert_eq!(p.scene.layers[0].geometry.strokes.len(), 6);
    let mut p = painter(&mut t, 1.0);
    p.dash(&[[0.0, 0.0], [1e6, 0.0]], false, 1.0, 1.0, 1.0, WHITE);
    assert_eq!(p.scene.layers[0].geometry.strokes.len(), 6);
}

#[test]
fn a_closed_dash_walks_the_closing_segment() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    p.dash(&sq, true, 1.0, 5.0, 5.0, WHITE);
    // 40 long, period 10: four runs; the third lies on the bottom edge (y = 10).
    let v = &p.scene.layers[0].geometry.strokes;
    assert!(v.len() >= 4 * 6);
    assert!(
        v.iter()
            .any(|v| (v.pos[1] - 10.0).abs() < 2.0 && v.pos[0] < 8.0)
    );
}

// ── arc ─────────────────────────────────────────────────────────────────────────────────────

/// The centreline points of a recorded open or closed stroke: the midpoint of each segment
/// start's two vertices.
fn centreline(v: &[StrokeVertex]) -> Vec<[f32; 2]> {
    v.chunks(6)
        .map(|c| {
            [
                (c[0].pos[0] + c[1].pos[0]) / 2.0,
                (c[0].pos[1] + c[1].pos[1]) / 2.0,
            ]
        })
        .collect()
}

#[test]
fn an_arc_stays_within_a_quarter_pixel_of_the_circle() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    let (c, r) = ([100.0, 80.0], 40.0);
    p.arc(
        c,
        r,
        0.0,
        std::f32::consts::FRAC_PI_2,
        StrokeStyle::solid(WHITE, 2.0),
    );
    let pts = centreline(&p.scene.layers[0].geometry.strokes);
    assert!(pts.len() > 4);
    let (cx, cy, dr) = (
        f64::from(c[0]) * 2.0,
        f64::from(c[1]) * 2.0,
        f64::from(r) * 2.0,
    );
    let dist =
        |q: [f32; 2]| ((f64::from(q[0]) - cx).powi(2) + (f64::from(q[1]) - cy).powi(2)).sqrt();
    for q in &pts {
        assert!((dist(*q) - dr).abs() < 0.01, "{q:?}");
    }
    for w in pts.windows(2) {
        let mid = [(w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0];
        assert!(
            dr - dist(mid) <= 0.25 + 0.01,
            "chord error {}",
            dr - dist(mid)
        );
    }
    // Starts at angle 0 (right of the centre) and turns clockwise on screen (y down).
    assert!(approx(pts[0][0], cx as f32 + dr as f32) && approx(pts[0][1], cy as f32));
    assert!(pts[1][1] > pts[0][1]);
}

#[test]
fn a_full_sweep_is_a_closed_circle() {
    let mut t = FakeText;
    let st = StrokeStyle::solid(WHITE, 2.0);
    let mut a = painter(&mut t, 2.0);
    a.arc([50.0, 50.0], 30.0, 0.5, std::f32::consts::TAU, st);
    let mut t2 = FakeText;
    let mut b = painter(&mut t2, 2.0);
    b.arc([50.0, 50.0], 30.0, 0.5, 9.0, st);
    let va = &a.scene.layers[0].geometry.strokes;
    assert!(!va.is_empty());
    assert_eq!(*va, b.scene.layers[0].geometry.strokes);
    // Closed: as many segments as points, the last one back to the first.
    let pts = centreline(va);
    let last_seg_end = va[va.len() - 6 + 2].pos;
    let first = va[0].pos;
    assert!((last_seg_end[0] - first[0]).abs() < 3.0 && (last_seg_end[1] - first[1]).abs() < 3.0);
    assert!(pts.len() >= 3);
}

#[test]
fn an_arc_is_clipped_and_lands_on_its_layer() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.on(LayerId::Modal);
    p.push_clip(Rect::new(10.0, 10.0, 50.0, 50.0));
    p.arc([40.0, 40.0], 20.0, 0.0, 3.0, StrokeStyle::solid(WHITE, 1.0));
    assert!(
        p.scene.layers[3]
            .geometry
            .strokes
            .iter()
            .all(|v| v.clip == [20.0, 20.0, 100.0, 100.0])
    );
    assert!(p.scene.layers[3].geometry.strokes.len() > 6);
    assert!(p.scene.layers[0].geometry.strokes.is_empty());
}

// ── fill_path and sector ────────────────────────────────────────────────────────────────────

fn tri_area_sum(v: &[StrokeVertex]) -> f64 {
    v.chunks(3)
        .map(|t| {
            let (a, b, c) = (t[0].pos, t[1].pos, t[2].pos);
            (f64::from(b[0] - a[0]) * f64::from(c[1] - a[1])
                - f64::from(c[0] - a[0]) * f64::from(b[1] - a[1]))
            .abs()
                / 2.0
        })
        .sum()
}

#[test]
fn fill_path_a_square_is_two_triangles_and_a_fringe() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.fill_path(
        &[[10.0, 10.0], [30.0, 10.0], [30.0, 30.0], [10.0, 30.0]],
        WHITE,
    );
    let v = &p.scene.layers[0].geometry.fills;
    assert_eq!(v.len(), 6 + 24);
    let (inner, fringe) = v.split_at(6);
    assert!(inner.iter().all(|v| v.aa[0] == 0.0));
    assert!((tri_area_sum(inner) - 40.0 * 40.0).abs() < 1e-3);
    // aa.x rises to half + 1 at the fringe.
    let half = v[0].style[1];
    assert!(approx(half, 0.5));
    let top = fringe.iter().map(|v| v.aa[0]).fold(0.0, f32::max);
    assert!(approx(top, half + 1.0), "{top}");
    // The fringe lies one pixel (miter at the corners) outside the polygon.
    let max_x = fringe.iter().map(|v| v.pos[0]).fold(f32::MIN, f32::max);
    assert!(approx(max_x, 61.0), "{max_x}");
    assert!(p.scene.layers[0].geometry.strokes.is_empty());
}

#[test]
fn fill_path_either_winding_gives_the_same_area() {
    let sq = [
        [0.0, 0.0],
        [20.0, 0.0],
        [20.0, 10.0],
        [10.0, 10.0],
        [10.0, 30.0],
        [0.0, 30.0],
    ];
    let rev: Vec<_> = sq.iter().rev().copied().collect();
    let mut t = FakeText;
    let mut a = painter(&mut t, 1.0);
    a.fill_path(&sq, WHITE);
    let mut t2 = FakeText;
    let mut b = painter(&mut t2, 1.0);
    b.fill_path(&rev, WHITE);
    for p in [&a, &b] {
        let v = &p.scene.layers[0].geometry.fills;
        // 4 interior triangles for an L of six points.
        assert!((tri_area_sum(&v[..12]) - 400.0).abs() < 1e-3);
        assert_eq!(v.len(), 12 + 6 * 6);
    }
}

#[test]
fn a_self_crossing_polygon_draws_what_it_found_without_panicking() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    p.fill_path(&[[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0]], WHITE);
    p.fill_path(&[[0.0, 0.0]], WHITE);
    p.fill_path(&[], WHITE);
    p.fill_path(&[[0.0, 0.0], [5.0, 5.0], [10.0, 10.0]], WHITE);
    let _ = &p.scene.layers[0].geometry.fills;
}

#[test]
fn a_fill_path_draws_under_text() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    let spec = Spec::plain("hi", Style::ui(13.0, THEME.fg_primary));
    p.text(&spec, 1.0, 1.0);
    p.fill_path(&[[0.0, 0.0], [20.0, 0.0], [20.0, 20.0]], WHITE);
    p.sector([50.0, 50.0], 10.0, 0.0, 1.0, WHITE);
    let params = FrameParams {
        size: (400, 300),
        scale: 1.0,
        blur_sigma: 4.0,
        saturation: 1.0,
    };
    let mut rec = RecordingRenderer::new();
    rec.render(&p.scene, &params);
    let calls = &rec.last().expect("frame").calls;
    let fills = calls
        .iter()
        .position(|c| matches!(c, DrawCall::Fills { .. }))
        .expect("fills");
    let text = calls
        .iter()
        .position(|c| matches!(c, DrawCall::Text { .. }))
        .expect("text");
    assert!(fills < text);
    assert_eq!(
        calls
            .iter()
            .filter(|c| matches!(c, DrawCall::Fills { .. }))
            .count(),
        1
    );
}

#[test]
fn a_sector_is_one_fan_around_the_centre() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.on(LayerId::Popup);
    p.push_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
    p.sector([50.0, 40.0], 20.0, 0.0, std::f32::consts::FRAC_PI_2, WHITE);
    let v = &p.scene.layers[2].geometry.fills;
    assert!(!v.is_empty());
    assert!(v.iter().all(|v| v.clip == [0.0, 0.0, 200.0, 200.0]));
    // Boundary: centre + (segments + 1) arc points; segments = the interior triangle count.
    // v.len() = 3 s + 6 (s + 2)  =>  s = (len - 12) / 9
    assert_eq!((v.len() - 12) % 9, 0);
    let s = (v.len() - 12) / 9;
    // Every interior triangle starts at the centre.
    for t in v[..3 * s].chunks(3) {
        assert_eq!(t[0].pos, [100.0, 80.0]);
    }
    // The fan covers a quarter disc: the area of a polygon inscribed in it, close to pi r^2 / 4.
    let area = tri_area_sum(&v[..3 * s]);
    let want = std::f64::consts::PI * 40.0 * 40.0 / 4.0;
    assert!((want - area).abs() / want < 0.02, "{area} vs {want}");
}

#[test]
fn a_full_sector_is_a_disc() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    p.sector([50.0, 50.0], 30.0, 1.0, 100.0, WHITE);
    let v = &p.scene.layers[0].geometry.fills;
    let s = v.len() / 9;
    assert_eq!(v.len(), 9 * s);
    let area = tri_area_sum(&v[..3 * s]);
    let want = std::f64::consts::PI * 900.0;
    assert!((want - area).abs() / want < 0.02, "{area} vs {want}");
}

#[test]
fn a_negative_sweep_turns_the_other_way() {
    let mut t = FakeText;
    let mut a = painter(&mut t, 1.0);
    a.sector([50.0, 50.0], 20.0, 0.0, -1.0, WHITE);
    let v = &a.scene.layers[0].geometry.fills;
    assert!(!v.is_empty());
    // The first arc point is at angle 0, the second is above it (y smaller).
    let first_arc = v[1].pos;
    assert!(approx(first_arc[0], 70.0) && approx(first_arc[1], 50.0));
    assert!(v[2].pos[1] < 50.0);
}

// ── input that is not finite ────────────────────────────────────────────────────────────────

#[test]
fn bad_input_draws_nothing_and_does_not_panic() {
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    let st = StrokeStyle::solid(WHITE, 1.0);
    p.dash(&[[0.0, 0.0], [nan, 5.0]], false, 1.0, 1.0, 2.0, WHITE);
    p.dash(&[[0.0, 0.0], [5.0, 5.0]], false, nan, 1.0, 2.0, WHITE);
    p.dash(&[[0.0, 0.0], [5.0, 5.0]], false, 1.0, 0.0, 2.0, WHITE);
    p.dash(&[[0.0, 0.0], [5.0, 5.0]], false, 1.0, 1.0, inf, WHITE);
    p.dash(&[[0.0, 0.0], [5.0, 5.0]], false, 1.0, 1.0, -1.0, WHITE);
    p.arc([nan, 0.0], 10.0, 0.0, 1.0, st);
    p.arc([0.0, 0.0], inf, 0.0, 1.0, st);
    p.arc([0.0, 0.0], 10.0, nan, 1.0, st);
    p.arc([0.0, 0.0], 10.0, 0.0, nan, st);
    p.arc([0.0, 0.0], 10.0, 0.0, 0.0, st);
    p.arc([0.0, 0.0], 0.0, 0.0, 1.0, st);
    p.arc([0.0, 0.0], -3.0, 0.0, 1.0, st);
    p.arc([0.0, 0.0], 10.0, 0.0, 1.0, StrokeStyle::solid(WHITE, nan));
    p.fill_path(&[[0.0, 0.0], [10.0, nan], [10.0, 10.0]], WHITE);
    p.fill_path(&[[0.0, 0.0], [10.0, inf], [10.0, 10.0]], WHITE);
    p.sector([nan, 0.0], 10.0, 0.0, 1.0, WHITE);
    p.sector([0.0, 0.0], 10.0, 0.0, nan, WHITE);
    p.sector([0.0, 0.0], 0.0, 0.0, 1.0, WHITE);
    p.sector([0.0, 0.0], 10.0, 0.0, 0.0, WHITE);
    let l = &p.scene.layers[0];
    assert!(
        l.geometry.strokes.is_empty(),
        "{}",
        l.geometry.strokes.len()
    );
    assert!(l.geometry.fills.is_empty());
}

#[test]
fn a_huge_finite_input_does_not_panic_or_emit_non_finite_vertices() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.sector([0.0, 0.0], 1e30, 0.0, 1.0, WHITE);
    p.fill_path(&[[0.0, 0.0], [1e30, 0.0], [1e30, 1e30]], WHITE);
    p.arc([0.0, 0.0], 1e30, 0.0, 1.0, StrokeStyle::solid(WHITE, 1.0));
    p.dash(&[[0.0, 0.0], [1e30, 0.0]], false, 1.0, 1.0, 1.0, WHITE);
    let g = &p.scene.layers[0].geometry;
    let finite = |v: &StrokeVertex| v.pos.iter().all(|c| c.is_finite());
    assert!(g.fills.iter().all(finite));
    assert!(g.strokes.iter().all(finite));
}

#[test]
fn bad_input_under_a_transform_draws_nothing() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.push_xform(Xform {
        origin: (0.0, 0.0),
        scale: f32::NAN,
        dx: 0.0,
        dy: 0.0,
        alpha: 1.0,
    });
    p.fill_path(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]], WHITE);
    p.sector([5.0, 5.0], 4.0, 0.0, 1.0, WHITE);
    p.arc([5.0, 5.0], 4.0, 0.0, 1.0, StrokeStyle::solid(WHITE, 1.0));
    assert!(p.scene.layers[0].geometry.fills.is_empty());
}

// ── the rounded clip ────────────────────────────────────────────────────────────────────────

#[test]
fn a_rounded_clip_travels_on_shapes_strokes_and_fills() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    let r = Rect::new(10.0, 20.0, 100.0, 60.0);
    p.push_clip_rounded(r, 8.0);
    p.fill(Rect::new(0.0, 0.0, 200.0, 200.0), 0.0, WHITE);
    p.ring(Rect::new(0.0, 0.0, 200.0, 200.0), 0.0, 1.0, WHITE, true);
    p.stroke(
        &[[0.0, 0.0], [50.0, 50.0]],
        false,
        StrokeStyle::solid(WHITE, 1.0),
    );
    p.fill_path(&[[0.0, 0.0], [50.0, 0.0], [50.0, 50.0]], WHITE);
    let g = &p.scene.layers[0].geometry;
    let want = ([20.0, 40.0, 200.0, 120.0], 16.0);
    assert_eq!((g.under[0].round, g.under[0].round_radius), want);
    assert_eq!((g.over[0].round, g.over[0].round_radius), want);
    assert!(g.strokes.iter().all(|v| (v.round, v.round_radius) == want));
    assert!(g.fills.iter().all(|v| (v.round, v.round_radius) == want));
    // The plain clip is the rect too (the rounded rect's bounding box).
    assert_eq!(g.under[0].clip, [20.0, 40.0, 200.0, 120.0]);
}

#[test]
fn radius_zero_is_a_plain_clip_and_pop_restores() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    let r = Rect::new(10.0, 20.0, 100.0, 60.0);
    p.push_clip_rounded(r, 0.0);
    p.fill(Rect::new(0.0, 0.0, 50.0, 50.0), 0.0, WHITE);
    p.pop_clip();
    p.push_clip(r);
    p.fill(Rect::new(0.0, 0.0, 50.0, 50.0), 0.0, WHITE);
    p.push_clip_rounded(r, 8.0);
    p.pop_clip();
    p.fill(Rect::new(0.0, 0.0, 50.0, 50.0), 0.0, WHITE);
    p.pop_clip();
    p.fill(Rect::new(0.0, 0.0, 50.0, 50.0), 0.0, WHITE);
    let u = &p.scene.layers[0].geometry.under;
    assert_eq!(u[0], u[1]);
    assert_eq!(u[1], u[2]);
    assert_eq!(u[0].round_radius, 0.0);
    assert_eq!(u[0].round, [0.0; 4]);
    assert_eq!(u[3].clip, [0.0; 4]);
    for bad in [-4.0, f32::NAN, f32::INFINITY] {
        let mut t = FakeText;
        let mut p = painter(&mut t, 2.0);
        p.push_clip_rounded(r, bad);
        p.fill(Rect::new(0.0, 0.0, 50.0, 50.0), 0.0, WHITE);
        let s = p.scene.layers[0].geometry.under[0];
        // Infinity is not finite: a plain clip, like the others.
        assert_eq!((s.round, s.round_radius), ([0.0; 4], 0.0), "{bad}");
    }
}

#[test]
fn the_radius_is_limited_to_half_the_short_side_and_scales_with_the_transform() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    p.push_xform(xf(2.0, 1.0));
    p.push_clip_rounded(Rect::new(0.0, 0.0, 100.0, 20.0), 500.0);
    p.fill(Rect::new(0.0, 0.0, 10.0, 10.0), 0.0, WHITE);
    let s = p.scene.layers[0].geometry.under[0];
    // 10 logical radius, x2 transform, x2 scale.
    assert_eq!(s.round_radius, 40.0);
    assert_eq!(s.round, [0.0, 0.0, 400.0, 80.0]);
}

#[test]
fn the_innermost_rounded_clip_holds_and_pop_returns_the_outer() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    p.push_clip_rounded(Rect::new(0.0, 0.0, 100.0, 100.0), 10.0);
    p.push_clip_rounded(Rect::new(20.0, 20.0, 40.0, 40.0), 6.0);
    p.fill(Rect::new(0.0, 0.0, 10.0, 10.0), 0.0, WHITE);
    p.pop_clip();
    p.fill(Rect::new(0.0, 0.0, 10.0, 10.0), 0.0, WHITE);
    let u = &p.scene.layers[0].geometry.under;
    assert_eq!(
        (u[0].round, u[0].round_radius),
        ([20.0, 20.0, 40.0, 40.0], 6.0)
    );
    assert_eq!(
        (u[1].round, u[1].round_radius),
        ([0.0, 0.0, 100.0, 100.0], 10.0)
    );
}

#[test]
fn text_and_customs_take_the_rect_only() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 2.0);
    let r = Rect::new(10.0, 20.0, 100.0, 60.0);
    p.push_clip_rounded(r, 8.0);
    let spec = Spec::plain("hi", Style::ui(13.0, THEME.fg_primary));
    p.text(&spec, 12.0, 22.0);
    p.custom(0, Rect::new(0.0, 0.0, 400.0, 300.0));
    let l = &p.scene.layers[0];
    assert_eq!(l.texts[0].clip, r);
    assert_eq!(l.customs[0].clip, [20, 40, 200, 120]);
}

#[test]
fn a_new_layer_drops_the_rounded_clip() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    p.push_clip_rounded(Rect::new(0.0, 0.0, 100.0, 100.0), 10.0);
    p.on(LayerId::Popup);
    p.fill(Rect::new(0.0, 0.0, 10.0, 10.0), 0.0, WHITE);
    let s = p.scene.layers[2].geometry.under[0];
    assert_eq!((s.round, s.round_radius, s.clip), ([0.0; 4], 0.0, [0.0; 4]));
}

#[test]
fn a_clip_that_is_not_finite_clips_everything() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    p.push_clip_rounded(Rect::new(f32::NAN, 0.0, 10.0, 10.0), 4.0);
    p.fill(Rect::new(0.0, 0.0, 10.0, 10.0), 0.0, WHITE);
    let c = p.scene.layers[0].geometry.under[0].clip;
    // The shader treats a zero width as no clip, so the clip is a sliver of zero height.
    assert!(c[2] > 0.0 && c[3] == 0.0, "{c:?}");
}

#[test]
fn a_clip_without_area_clips_everything() {
    let mut t = FakeText;
    let mut p = painter(&mut t, 1.0);
    p.push_clip(Rect::new(10.0, 10.0, 20.0, 20.0));
    p.push_clip(Rect::new(100.0, 100.0, 20.0, 20.0));
    p.fill(Rect::new(0.0, 0.0, 200.0, 200.0), 0.0, WHITE);
    let c = p.scene.layers[0].geometry.under[0].clip;
    assert!(c[2] > 0.0 && c[3] == 0.0, "{c:?}");
}

// ── recording and replay ────────────────────────────────────────────────────────────────────

fn draw_all(p: &mut P<'_>) {
    p.push_clip_rounded(Rect::new(5.0, 5.0, 150.0, 120.0), 6.0);
    p.dash(&[[0.0, 5.0], [50.0, 5.0]], false, 1.0, 2.0, 3.0, WHITE);
    p.arc([40.0, 40.0], 20.0, 0.2, 2.0, StrokeStyle::solid(WHITE, 2.0));
    p.fill_path(&[[10.0, 10.0], [60.0, 10.0], [30.0, 50.0]], WHITE);
    p.sector([80.0, 60.0], 25.0, 0.0, 2.5, WHITE);
    p.pop_clip();
}

#[test]
fn every_primitive_is_recorded_and_replays_to_the_same_scene() {
    let mut t = FakeText;
    let mut a = painter(&mut t, 2.0);
    a.start_recording();
    draw_all(&mut a);
    let ops = a.take_recording();
    assert_eq!(ops.len(), 6);
    let mut t2 = FakeText;
    let mut b = painter(&mut t2, 2.0);
    b.replay(&ops);
    let (sa, sb): (&Scene, &Scene) = (&a.scene, &b.scene);
    assert_eq!(sa, sb);
    assert!(!sa.layers[0].geometry.fills.is_empty());
}
