//! The progress bar (`data.progress` acceptance, D118): the fill is the fraction of the track at
//! 0, 0.5 and 1, a value outside 0 to 1 clamps, the tones, the head, the node, and the
//! indeterminate bar: closed in time, wrapping, moving under Full and Reduced, still under Off.

use std::time::Duration;

use lune_ui_core::access::{Role, SemanticTree, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::Repaint;
use lune_ui_core::motion::MotionTier;
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::progress::{
    self, Indeterminate, ProgressOptions, breath_opacity, indeterminate_at, travel_phase,
};
use lune_ui_core::widgets::tone::Tone;

const WIN: (f32, f32) = (400.0, 200.0);
const RECT: Rect = Rect::new(10.0, 20.0, 200.0, 4.0);

fn shapes(scene: &Scene) -> Vec<Shape> {
    scene.layers[0].geometry.under.clone()
}

fn painted(fraction: f32, tone: Tone) -> Vec<Shape> {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    progress::paint(&mut p, RECT, fraction, tone);
    shapes(&p.scene)
}

#[test]
fn the_fill_width_is_the_fraction_of_the_track_at_0_half_and_1() {
    let s = painted(0.0, Tone::Neutral);
    assert_eq!(s.len(), 1, "no fill op at 0");
    assert_eq!(s[0].rect, [10.0, 20.0, 200.0, 4.0]);
    assert_eq!(s[0].color, premul(DARK.surface_overlay));
    for (f, w) in [(0.5, 100.0), (1.0, 200.0), (0.4, 80.0)] {
        let s = painted(f, Tone::Neutral);
        assert_eq!(s.len(), 2, "{f}");
        assert_eq!(s[0].rect, [10.0, 20.0, 200.0, 4.0]);
        assert_eq!(s[1].rect, [10.0, 20.0, w, 4.0], "{f}");
        assert_eq!(
            s[1].radii, [0.0; 4],
            "square leading edge, the track clips it"
        );
    }
}

#[test]
fn a_value_outside_the_range_clamps_and_nan_is_zero() {
    assert_eq!(painted(-3.0, Tone::Neutral).len(), 1);
    assert_eq!(painted(f32::NAN, Tone::Neutral).len(), 1);
    assert_eq!(painted(7.0, Tone::Neutral)[1].rect[2], 200.0);
    assert_eq!(painted(f32::INFINITY, Tone::Neutral)[1].rect[2], 200.0);
}

#[test]
fn the_fill_colour_is_accent_for_neutral_and_the_tone_otherwise() {
    assert_eq!(painted(0.5, Tone::Neutral)[1].color, premul(DARK.accent));
    assert_eq!(painted(0.5, Tone::Warn)[1].color, premul(DARK.warn));
    assert_eq!(painted(0.5, Tone::Err)[1].color, premul(DARK.err));
}

#[test]
fn at_scale_2_the_track_is_8_px_high_with_an_8_px_radius() {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(2.0, win, &mut text, None, DARK);
    progress::paint(&mut p, RECT, 0.5, Tone::Neutral);
    let s = shapes(&p.scene);
    assert_eq!(s[0].rect[3], 8.0);
    assert_eq!(s[0].radii, [8.0; 4]);
}

fn run(
    tier: MotionTier,
    ms: u64,
    opts: &ProgressOptions<'_>,
) -> (Scene, SemanticTree, f32, Repaint) {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    h.at_ms(ms);
    let mut height = 0.0;
    let (scene, out) = h.frame(|ui| height = ui.progress("p", RECT, opts));
    (scene, out.tree, height, out.repaint)
}

fn det(v: f32) -> ProgressOptions<'static> {
    ProgressOptions {
        value: Some(v),
        ..ProgressOptions::default()
    }
}

fn indet() -> ProgressOptions<'static> {
    ProgressOptions {
        value: None,
        ..ProgressOptions::default()
    }
}

#[test]
fn the_ui_bar_matches_paint_and_never_animates_when_determinate() {
    let mut h = UiHarness::new(WIN);
    let mut scenes = Vec::new();
    for ms in [0, 16, 700, 5000] {
        h.at_ms(ms);
        let (scene, out) = h.frame(|ui| {
            ui.progress("p", RECT, &det(0.4));
        });
        assert_eq!(out.repaint, Repaint::Idle, "{ms}");
        scenes.push(scene);
    }
    assert!(scenes.windows(2).all(|w| w[0] == w[1]));
    assert_eq!(shapes(&scenes[0]), painted(0.4, Tone::Neutral));
}

#[test]
fn heights_and_the_head() {
    assert_eq!(run(MotionTier::Full, 0, &det(0.5)).2, 4.0);
    assert_eq!(run(MotionTier::Full, 0, &indet()).2, 3.0);
    let opts = ProgressOptions {
        head: Some(("Indexing", "40%")),
        ..det(0.4)
    };
    let (scene, tree, h, _) = run(MotionTier::Full, 0, &opts);
    assert_eq!(h, 20.0 + 4.0 + 4.0);
    assert_eq!(scene.layers[0].texts.len(), 2, "name and value");
    let s = shapes(&scene);
    assert_eq!(s[0].rect, [10.0, 44.0, 200.0, 4.0], "track below the head");
    assert_eq!(tree.nodes[0].name, "Indexing", "name from the head");
    let named = ProgressOptions {
        name: "Repository index",
        ..opts
    };
    assert_eq!(
        run(MotionTier::Full, 0, &named).1.nodes[0].name,
        "Repository index"
    );
}

#[test]
fn the_head_value_ends_at_the_right_edge() {
    let opts = ProgressOptions {
        head: Some(("Indexing", "40%")),
        ..det(0.4)
    };
    let (scene, ..) = run(MotionTier::Full, 0, &opts);
    let texts = &scene.layers[0].texts;
    assert_eq!(texts[0].x, 10.0);
    assert!(texts[1].x > texts[0].x && texts[1].x < 210.0);
}

#[test]
fn the_node_is_progress_valued_0_to_100_and_has_none_when_indeterminate() {
    for (v, want) in [
        (0.0, 0.0),
        (0.5, 50.0),
        (1.0, 100.0),
        (9.0, 100.0),
        (-1.0, 0.0),
    ] {
        let (_, tree, ..) = run(MotionTier::Full, 0, &det(v));
        let n = &tree.nodes[0];
        assert_eq!(n.role, Role::Progress);
        assert_eq!(
            n.value,
            Some(Value::Number {
                value: want,
                min: 0.0,
                max: 100.0,
                step: 0.0
            })
        );
    }
    let (_, tree, ..) = run(MotionTier::Full, 0, &indet());
    assert_eq!(tree.nodes[0].role, Role::Progress);
    assert_eq!(tree.nodes[0].value, None);
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn the_travel_phase_is_closed_in_time_and_wraps() {
    assert_eq!(travel_phase(ms(0)), 0.0);
    assert_eq!(travel_phase(ms(600)), 0.25);
    assert_eq!(travel_phase(ms(1200)), 0.5);
    assert_eq!(travel_phase(ms(2400)), 0.0, "wraps");
    for k in 0..300 {
        let t = ms(k * 17);
        let p = travel_phase(t);
        assert!((0.0..1.0).contains(&p), "{k}");
        assert_eq!(p, travel_phase(t + ms(2400 * 5)), "closed in time at {k}");
    }
}

/// The travelling segment (the second shape) at `t` under Full.
fn segment(t: u64) -> Shape {
    let (scene, ..) = run(MotionTier::Full, t, &indet());
    let s = shapes(&scene);
    assert_eq!(s.len(), 2);
    s[1]
}

#[test]
fn the_segment_is_30_percent_of_the_track_travelling_left_to_right_clipped_by_it() {
    let seg = 60.0;
    // Just off the left at 0, 25% of the way over (210 px of travel) at 600 ms.
    let near = |a: f32, b: f32| assert!((a - b).abs() < 1e-3, "{a} vs {b}");
    let r = segment(0).rect;
    near(r[0], 10.0 - seg);
    near(r[2], seg);
    assert_eq!((r[1], r[3]), (20.0, 3.0));
    near(segment(600).rect[0], 10.0 - seg + 260.0 * 0.25);
    near(segment(1200).rect[0], 10.0 - seg + 260.0 * 0.5);
    let xs: Vec<f32> = (0..2400).step_by(100).map(|t| segment(t).rect[0]).collect();
    assert!(xs.windows(2).all(|w| w[1] > w[0]), "left to right: {xs:?}");
    assert_eq!(segment(2400).rect, segment(0).rect, "wraps");
    // Clipped by the 3 px track.
    let s = segment(900);
    assert_eq!(s.round, [10.0, 20.0, 200.0, 3.0]);
    assert!(s.round_radius > 0.0);
}

#[test]
fn full_and_reduced_keep_asking_for_frames_and_off_is_idle() {
    for t in [0, 600, 1799, 2400, 7777] {
        assert_eq!(
            run(MotionTier::Full, t, &indet()).3,
            Repaint::Now,
            "Full {t}"
        );
        assert_eq!(
            run(MotionTier::Reduced, t, &indet()).3,
            Repaint::Now,
            "Reduced {t}"
        );
        assert_eq!(
            run(MotionTier::Off, t, &indet()).3,
            Repaint::Idle,
            "Off {t}"
        );
    }
}

#[test]
fn reduced_breathes_a_full_width_fill_between_038_and_1() {
    assert_eq!(breath_opacity(ms(0)), 0.38);
    assert_eq!(breath_opacity(ms(900)), 1.0);
    assert!((breath_opacity(ms(1800)) - 0.38).abs() < 1e-6, "periodic");
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for t in (0..3600).step_by(37) {
        let o = breath_opacity(ms(t));
        assert!((0.38 - 1e-6..=1.0 + 1e-6).contains(&o), "{t}: {o}");
        lo = lo.min(o);
        hi = hi.max(o);
    }
    assert!(lo < 0.4 && hi > 0.99, "{lo} {hi}");
    for t in [0, 450, 900, 1350] {
        let (scene, ..) = run(MotionTier::Reduced, t, &indet());
        let s = shapes(&scene);
        assert_eq!(s.len(), 2);
        assert_eq!(s[1].rect, s[0].rect, "full width at {t}");
        let want = premul(DARK.accent)[3] * breath_opacity(ms(t));
        assert!((s[1].color[3] - want).abs() < 0.01, "{t}");
    }
}

#[test]
fn off_holds_a_static_full_fill_at_038() {
    let first = run(MotionTier::Off, 0, &indet()).0;
    for t in [300, 900, 2400] {
        assert_eq!(run(MotionTier::Off, t, &indet()).0, first, "{t}");
    }
    let s = shapes(&first);
    assert_eq!(s[1].rect, s[0].rect);
    assert!((s[1].color[3] - 0.38).abs() < 0.01);
    assert_eq!(
        indeterminate_at(MotionTier::Off, ms(1234)),
        Indeterminate::Fade(0.38)
    );
    assert_eq!(
        indeterminate_at(MotionTier::Full, ms(600)),
        Indeterminate::Travel(0.25)
    );
}
