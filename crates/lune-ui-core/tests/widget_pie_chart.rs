//! The pie chart (`data.pie-chart` acceptance, D164, D166): sweeps that total a turn, the start at
//! the top, the whole disc and the large slice, the highlight, the value transitions on every tier
//! with the total held at 2π in every frame, and the semantic node.
//!
//! A drawn slice is read back from the scene: the fan's first and last arc points give its start
//! and end angle, and a point is covered when an interior triangle holds it.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use lune_ui_core::access::Role;
use lune_ui_core::color::{Rgba, premul};
use lune_ui_core::input::Repaint;
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, StrokeVertex};
use lune_ui_core::testing::{ScriptedInput, UiHarness, disabled_violations};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::pie_chart::{DIAMETER, HIGHLIGHT, PieOptions, PieSlice, VALUE, sweeps};

const WIN: (f32, f32) = (400.0, 300.0);
const RECT: Rect = Rect::new(20.0, 30.0, 160.0, 160.0);
const A: Rgba = [200, 30, 30, 255];
const B: Rgba = [30, 200, 30, 255];
const C: Rgba = [30, 30, 200, 255];

fn slices(values: &[f64]) -> Vec<PieSlice> {
    values
        .iter()
        .enumerate()
        .map(|(i, &value)| PieSlice {
            key: i as u64 + 1,
            value,
            color: [A, B, C][i % 3],
        })
        .collect()
}

fn at_scale_2() -> UiHarness {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    h
}

fn draw_in(h: &mut UiHarness, s: &[PieSlice]) -> (Scene, Repaint) {
    let (scene, out) = h.frame(|ui| {
        ui.pie_chart(
            "p",
            RECT,
            &PieOptions {
                slices: s,
                ..PieOptions::default()
            },
        )
    });
    (scene, out.repaint)
}

fn draw(s: &[PieSlice]) -> Scene {
    draw_in(&mut UiHarness::new(WIN), s).0
}

/// The interior triangles of the layer's fills (the fringe has an outer vertex), in record order.
fn interior(scene: &Scene) -> Vec<[StrokeVertex; 3]> {
    scene.layers[0]
        .geometry
        .fills
        .chunks(3)
        .filter(|t| t.len() == 3 && t.iter().all(|v| v.aa[0] == 0.0))
        .map(|t| [t[0], t[1], t[2]])
        .collect()
}

fn inside(p: [f32; 2], t: &[StrokeVertex; 3]) -> bool {
    let s =
        |a: [f32; 2], b: [f32; 2]| (p[0] - b[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (p[1] - b[1]);
    let (d1, d2, d3) = (
        s(t[0].pos, t[1].pos),
        s(t[1].pos, t[2].pos),
        s(t[2].pos, t[0].pos),
    );
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

/// The colour (premultiplied) of the first fill covering `p` (physical px), or `None`.
fn probe(scene: &Scene, p: [f32; 2]) -> Option<[f32; 4]> {
    interior(scene)
        .iter()
        .find(|t| inside(p, t))
        .map(|t| t[0].color)
}

/// Each drawn fan other than the highlight: its colour and sweep in radians, in draw order.
fn drawn_sweeps(scene: &Scene, scale: f32) -> Vec<([f32; 4], f32)> {
    let centre = [
        (RECT.x + DIAMETER / 2.0) * scale,
        (RECT.y + DIAMETER / 2.0) * scale,
    ];
    let angle = |v: &StrokeVertex| (v.pos[1] - centre[1]).atan2(v.pos[0] - centre[0]);
    let hi = premul(HIGHLIGHT).map(|v| v);
    let mut out: Vec<([f32; 4], f32, f32)> = Vec::new();
    for t in interior(scene) {
        let color = t[0].color;
        match out.last_mut() {
            Some((c, _, end)) if *c == color => *end = angle(&t[2]),
            _ => out.push((color, angle(&t[1]), angle(&t[2]))),
        }
    }
    out.into_iter()
        .filter(|(c, _, _)| *c != hi)
        .map(|(c, start, end)| {
            let mut sweep = end - start;
            while sweep <= 0.0 {
                sweep += TAU;
            }
            (c, sweep)
        })
        .collect()
}

fn total(scene: &Scene, scale: f32) -> f32 {
    drawn_sweeps(scene, scale).iter().map(|(_, s)| s).sum()
}

fn c(color: Rgba) -> [f32; 4] {
    premul(color)
}

#[test]
fn sweeps_sum_to_a_turn_for_any_positive_total() {
    for values in [
        vec![1.0],
        vec![1.0, 2.0, 3.0],
        vec![0.001, 1000.0],
        vec![5.0; 17],
        vec![1e-6, 1e-6, 3.0],
        vec![123456.0, 0.5, 77.0, 1e9],
    ] {
        let s = sweeps(&values);
        assert_eq!(s.len(), values.len());
        let sum: f32 = s.iter().sum();
        assert!((sum - TAU).abs() < 1e-4, "{values:?}: {sum}");
    }
    let s = sweeps(&[1.0, 1.0]);
    assert_eq!(s, vec![PI, PI]);
}

#[test]
fn a_value_that_is_not_positive_or_not_finite_sweeps_nothing() {
    let s = sweeps(&[1.0, 0.0, -4.0, f32::NAN, f32::INFINITY, 1.0]);
    assert_eq!(s, vec![PI, 0.0, 0.0, 0.0, 0.0, PI]);
    for none in [
        vec![],
        vec![0.0],
        vec![-1.0, -2.0],
        vec![f32::NAN],
        vec![f32::INFINITY],
        vec![f32::MAX, f32::MAX],
    ] {
        assert!(sweeps(&none).is_empty(), "{none:?}");
    }
}

#[test]
fn empty_draws_nothing_the_highlight_included() {
    for s in [
        slices(&[]),
        slices(&[0.0, 0.0]),
        slices(&[-1.0, f64::NAN, f64::INFINITY]),
        slices(&[1e308, 1e308]),
    ] {
        let scene = draw(&s);
        assert!(scene.layers.iter().all(|l| l.geometry.fills.is_empty()));
    }
}

#[test]
fn a_zero_slice_is_skipped() {
    let scene = draw(&slices(&[1.0, 0.0, 1.0]));
    let drawn = drawn_sweeps(&scene, 1.0);
    assert_eq!(drawn.len(), 2);
    assert_eq!(drawn[0].0, c(A));
    assert_eq!(drawn[1].0, c(C));
    assert!((total(&scene, 1.0) - TAU).abs() < 1e-4);
}

#[test]
fn one_slice_of_the_whole_is_a_full_disc_probed_at_the_four_quarter_points() {
    let scene = draw_in(&mut at_scale_2(), &slices(&[7.0])).0;
    // Diameter 320 px at scale 2, centre (200, 220) in physical px.
    let (cx, cy) = (2.0 * (RECT.x + 80.0), 2.0 * (RECT.y + 80.0));
    for (dx, dy) in [(140.0, 0.0), (-140.0, 0.0), (0.0, 140.0), (0.0, -140.0)] {
        assert_eq!(probe(&scene, [cx + dx, cy + dy]), Some(c(A)), "{dx},{dy}");
    }
    assert_eq!(
        probe(&scene, [cx + 170.0, cy]),
        None,
        "outside the 160 px radius"
    );
    assert_eq!(probe(&scene, [cx, cy - 170.0]), None);
}

#[test]
fn the_first_slice_starts_at_the_top_centre_and_turns_clockwise() {
    let scene = draw_in(&mut at_scale_2(), &slices(&[1.0, 3.0])).0;
    let (cx, cy) = (2.0 * (RECT.x + 80.0), 2.0 * (RECT.y + 80.0));
    // A quarter turn: just right of the top is A, just left of it is B (the second slice ends
    // there).
    assert_eq!(probe(&scene, [cx + 8.0, cy - 120.0]), Some(c(A)));
    assert_eq!(probe(&scene, [cx - 8.0, cy - 120.0]), Some(c(B)));
    // A ends at the right.
    assert_eq!(probe(&scene, [cx + 120.0, cy - 8.0]), Some(c(A)));
    assert_eq!(probe(&scene, [cx + 120.0, cy + 8.0]), Some(c(B)));
    let drawn = drawn_sweeps(&scene, 2.0);
    assert!((drawn[0].1 - FRAC_PI_2).abs() < 1e-3);
}

#[test]
fn a_slice_above_half_fills_past_the_bottom() {
    let scene = draw_in(&mut at_scale_2(), &slices(&[7.0, 3.0])).0;
    let (cx, cy) = (2.0 * (RECT.x + 80.0), 2.0 * (RECT.y + 80.0));
    // 0.7 of a turn from the top, clockwise: through the right and the bottom to the lower left.
    assert_eq!(probe(&scene, [cx, cy + 120.0]), Some(c(A)), "the bottom");
    assert_eq!(
        probe(&scene, [cx - 80.0, cy + 80.0]),
        Some(c(A)),
        "lower left"
    );
    assert_eq!(
        probe(&scene, [cx - 100.0, cy - 30.0]),
        Some(c(B)),
        "left, above the end"
    );
}

#[test]
fn the_highlight_is_one_white_sector_over_the_top_half_after_the_slices() {
    let scene = draw_in(&mut at_scale_2(), &slices(&[1.0, 2.0])).0;
    let (cx, cy) = (2.0 * (RECT.x + 80.0), 2.0 * (RECT.y + 80.0));
    let hi = c(HIGHLIGHT);
    assert_eq!(HIGHLIGHT, [255, 255, 255, 38]);
    let tris = interior(&scene);
    let first = tris
        .iter()
        .position(|t| t[0].color == hi)
        .expect("a highlight");
    assert!(
        tris[first..].iter().all(|t| t[0].color == hi),
        "after every slice, one fan"
    );
    assert!(
        tris[first..].iter().all(|t| t[0].pos == [cx, cy]),
        "one fan about the centre"
    );
    // The fan covers the top half only.
    let covered = |p: [f32; 2]| tris[first..].iter().any(|t| inside(p, t));
    assert!(covered([cx, cy - 100.0]));
    assert!(covered([cx - 100.0, cy - 10.0]));
    assert!(covered([cx + 100.0, cy - 10.0]));
    assert!(!covered([cx, cy + 100.0]));
    assert!(!covered([cx - 100.0, cy + 10.0]));
    // Its edge points are the left and right of the disc: -pi to 0 clockwise.
    let ends = tris[first].map(|v| v.pos);
    assert!((ends[1][0] - (cx - 160.0)).abs() < 1.0 && (ends[1][1] - cy).abs() < 1.0);
    let last = tris.last().expect("last").map(|v| v.pos);
    assert!((last[2][0] - (cx + 160.0)).abs() < 1.0 && (last[2][1] - cy).abs() < 1.0);
}

#[test]
fn slices_keep_the_apps_colours_in_draw_order() {
    let scene = draw(&slices(&[1.0, 1.0, 1.0]));
    let colors: Vec<_> = drawn_sweeps(&scene, 1.0).iter().map(|(c, _)| *c).collect();
    assert_eq!(colors, vec![c(A), c(B), c(C)]);
}

fn mid_flight_check(h: &mut UiHarness, from: &[PieSlice], to: &[PieSlice]) {
    h.at_ms(1000);
    draw_in(h, from);
    for ms in (1000..=1300).step_by(16) {
        h.at_ms(ms);
        let (scene, _) = draw_in(h, to);
        let t = total(&scene, 1.0);
        assert!((t - TAU).abs() < 1e-4, "{ms} ms: {t}");
    }
}

#[test]
fn the_sweeps_total_a_turn_in_every_frame_of_a_change_and_of_a_retarget() {
    let mut h = UiHarness::new(WIN);
    mid_flight_check(&mut h, &slices(&[1.0, 2.0, 3.0]), &slices(&[3.0, 1.0, 1.0]));

    // Retarget at mid-flight, twice.
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    draw_in(&mut h, &slices(&[1.0, 2.0, 3.0]));
    draw_in(&mut h, &slices(&[6.0, 1.0, 1.0]));
    for (ms, v) in [
        (30, [1.0, 5.0, 1.0]),
        (60, [1.0, 1.0, 9.0]),
        (90, [2.0, 2.0, 2.0]),
    ] {
        h.at_ms(ms);
        let (scene, repaint) = draw_in(&mut h, &slices(&v));
        assert_eq!(repaint, Repaint::Now, "{ms}");
        assert!((total(&scene, 1.0) - TAU).abs() < 1e-4, "{ms}");
    }
    // A slice removed and one added in flight.
    h.at_ms(100);
    let (scene, _) = draw_in(&mut h, &slices(&[2.0, 2.0]));
    assert_eq!(drawn_sweeps(&scene, 1.0).len(), 2);
    assert!((total(&scene, 1.0) - TAU).abs() < 1e-4);
}

#[test]
fn a_change_animates_once_and_ends_exactly_at_each_share() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(1000);
    let (rest, repaint) = draw_in(&mut h, &slices(&[1.0, 1.0]));
    assert_eq!(repaint, Repaint::Idle);
    assert!((drawn_sweeps(&rest, 1.0)[0].1 - PI).abs() < 1e-4);
    let (start, repaint) = draw_in(&mut h, &slices(&[3.0, 1.0]));
    assert_eq!(repaint, Repaint::Now);
    assert!(
        (drawn_sweeps(&start, 1.0)[0].1 - PI).abs() < 1e-4,
        "starts where it was"
    );
    h.at_ms(1090);
    let (mid, repaint) = draw_in(&mut h, &slices(&[3.0, 1.0]));
    assert_eq!(repaint, Repaint::Now);
    let m = drawn_sweeps(&mid, 1.0)[0].1;
    assert!(m > PI + 0.05 && m < 1.5 * PI - 0.05, "{m}");
    h.at_ms(1180);
    let (end, repaint) = draw_in(&mut h, &slices(&[3.0, 1.0]));
    assert_eq!(repaint, Repaint::Idle);
    let sw = drawn_sweeps(&end, 1.0);
    assert!((sw[0].1 - 0.75 * TAU).abs() < 1e-4 && (sw[1].1 - 0.25 * TAU).abs() < 1e-4);
    h.at_ms(3000);
    let (after, repaint) = draw_in(&mut h, &slices(&[3.0, 1.0]));
    assert_eq!(repaint, Repaint::Idle);
    assert_eq!(after, end);
}

#[test]
fn a_slice_removed_leaves_at_once_and_the_rest_fill_the_turn() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    draw_in(&mut h, &slices(&[1.0, 1.0, 1.0]));
    let rest = [slices(&[1.0, 1.0, 1.0])[0], slices(&[1.0, 1.0, 1.0])[2]];
    let (scene, _) = draw_in(&mut h, &rest);
    let drawn = drawn_sweeps(&scene, 1.0);
    assert_eq!(drawn.len(), 2);
    assert!(drawn.iter().all(|(color, _)| *color != c(B)));
    assert!((total(&scene, 1.0) - TAU).abs() < 1e-4);
}

#[test]
fn from_empty_to_data_snaps_and_nothing_stays_in_flight() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    let (empty, repaint) = draw_in(&mut h, &[]);
    assert!(empty.layers[0].geometry.fills.is_empty());
    assert_eq!(repaint, Repaint::Idle);
    h.at_ms(10);
    let (scene, repaint) = draw_in(&mut h, &slices(&[1.0, 3.0]));
    assert_eq!(repaint, Repaint::Idle);
    let sw = drawn_sweeps(&scene, 1.0);
    assert!((sw[0].1 - 0.25 * TAU).abs() < 1e-4);
    // And back to nothing, then data again.
    h.at_ms(20);
    draw_in(&mut h, &[]);
    h.at_ms(30);
    let (_, repaint) = draw_in(&mut h, &slices(&[2.0, 2.0]));
    assert_eq!(repaint, Repaint::Idle);
}

#[test]
fn live_snaps_and_holds() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    let run = |h: &mut UiHarness, s: &[PieSlice], live: bool| {
        let (scene, out) = h.frame(|ui| {
            ui.pie_chart(
                "p",
                RECT,
                &PieOptions {
                    slices: s,
                    live,
                    ..PieOptions::default()
                },
            )
        });
        (scene, out.repaint)
    };
    run(&mut h, &slices(&[1.0, 1.0]), true);
    h.at_ms(10);
    let (scene, repaint) = run(&mut h, &slices(&[3.0, 1.0]), true);
    assert_eq!(repaint, Repaint::Idle);
    assert!((drawn_sweeps(&scene, 1.0)[0].1 - 0.75 * TAU).abs() < 1e-4);

    // Mid-flight, then live: the target at once.
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    run(&mut h, &slices(&[1.0, 1.0]), false);
    run(&mut h, &slices(&[3.0, 1.0]), false);
    h.at_ms(60);
    assert_eq!(run(&mut h, &slices(&[3.0, 1.0]), false).1, Repaint::Now);
    h.at_ms(70);
    let (scene, repaint) = run(&mut h, &slices(&[3.0, 1.0]), true);
    assert_eq!(repaint, Repaint::Idle);
    assert!((drawn_sweeps(&scene, 1.0)[0].1 - 0.75 * TAU).abs() < 1e-4);
}

#[test]
fn off_and_reduced_snap() {
    for tier in [MotionTier::Off, MotionTier::Reduced] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        h.at_ms(0);
        draw_in(&mut h, &slices(&[1.0, 1.0]));
        h.at_ms(10);
        let (scene, repaint) = draw_in(&mut h, &slices(&[3.0, 1.0]));
        assert_eq!(repaint, Repaint::Idle, "{tier:?}");
        assert!(
            (drawn_sweeps(&scene, 1.0)[0].1 - 0.75 * TAU).abs() < 1e-4,
            "{tier:?}"
        );
    }
}

#[test]
fn the_value_row_conforms_under_every_tier() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&VALUE, &DARK.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("value {tier:?}: {e}"));
    }
}

#[test]
fn idle_at_rest() {
    let mut h = UiHarness::new(WIN);
    for ms in [0, 16, 5000] {
        h.at_ms(ms);
        assert_eq!(
            draw_in(&mut h, &slices(&[1.0, 2.0])).1,
            Repaint::Idle,
            "{ms}"
        );
    }
}

#[test]
fn the_chart_is_an_image_named_by_the_app_with_no_children() {
    let mut h = UiHarness::new(WIN);
    let (_, out) = h.frame(|ui| {
        ui.pie_chart(
            "p",
            RECT,
            &PieOptions {
                slices: &slices(&[1.0, 2.0]),
                label: "Storage",
                description: "Objects 1 GB, packs 2 GB",
                live: false,
            },
        )
    });
    let n = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Image)
        .expect("image");
    assert_eq!(n.name, "Storage");
    assert_eq!(n.description.as_deref(), Some("Objects 1 GB, packs 2 GB"));
    assert!(n.children.is_empty());
    assert_eq!((n.bounds.w, n.bounds.h), (160.0, 160.0));
    assert_eq!(out.tree.nodes.len(), 1);
}

#[test]
fn an_empty_label_falls_to_the_toolkit_string_through_the_table() {
    let mut h = UiHarness::new(WIN);
    let (_, out) = h.frame(|ui| ui.pie_chart("p", RECT, &PieOptions::default()));
    assert_eq!(out.tree.nodes[0].name, "Pie chart");
    assert_eq!(out.tree.nodes[0].description, None);
    let mut h = UiHarness::new(WIN);
    h.strings
        .insert("luneui.pie-chart.name", "Carta pai")
        .expect("a toolkit key");
    let (_, out) = h.frame(|ui| ui.pie_chart("p", RECT, &PieOptions::default()));
    assert_eq!(out.tree.nodes[0].name, "Carta pai");
}

#[test]
fn a_disabled_scope_marks_the_node() {
    let mut h = UiHarness::new(WIN);
    let (_, out) =
        h.frame(|ui| ui.with_enabled(false, |ui| ui.pie_chart("p", RECT, &PieOptions::default())));
    assert_eq!(
        disabled_violations(&out.tree, WidgetId::ROOT.with("p")),
        vec![]
    );
}
