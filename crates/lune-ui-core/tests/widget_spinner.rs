//! The spinner (`widget.spinner` acceptance, D117): 360 degrees per 1200 ms from elapsed time
//! alone, wrap at 360, a stop holds the angle and a restart continues from it, a static glyph at
//! angle 0 under Reduced and Off, a frame at the busy cadence's next tick only while spinning, a
//! transient phase.

use std::time::Duration;

use lune_ui_core::access::{Role, State};
use lune_ui_core::icon;
use lune_ui_core::input::Repaint;
use lune_ui_core::motion::{Cadence, MotionTier};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::{ImageItem, ImageSource, Scene};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::TextSystem;
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::spinner::{self, SpinnerOptions};

const WIN: (f32, f32) = (200.0, 100.0);
const CENTRE: (f32, f32) = (40.0, 30.0);

fn opts(busy: bool) -> SpinnerOptions<'static> {
    SpinnerOptions {
        busy,
        name: "Indexing",
        ..SpinnerOptions::default()
    }
}

fn image(scene: &Scene) -> ImageItem {
    let all: Vec<_> = scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.images.clone())
        .collect();
    assert_eq!(all.len(), 1, "one turned glyph");
    all[0]
}

/// The glyph's turn in degrees, 0 to 360.
fn degrees(scene: &Scene) -> f32 {
    image(scene).rotation.to_degrees().rem_euclid(360.0)
}

/// One frame at `ms`: the angle and the repaint.
fn at(h: &mut UiHarness, ms: u64, busy: bool) -> (f32, Repaint) {
    h.at_ms(ms);
    let (scene, out) = h.frame(|ui| ui.spinner("s", CENTRE, &opts(busy)));
    (degrees(&scene), out.repaint)
}

/// The repaint a frame with only a busy loop in flight reports at `ms`: the next tick of the
/// default busy cadence (D298).
fn busy_tick(ms: u64) -> Repaint {
    Repaint::At(
        Cadence::BUSY
            .next_tick(Duration::from_millis(ms))
            .expect("a finite tick"),
    )
}

fn circular(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

#[test]
fn the_angle_is_360_degrees_per_1200_ms_on_a_60_hz_grid_and_wraps() {
    let mut h = UiHarness::new(WIN);
    let mut last = -1.0;
    let mut wrapped = false;
    for k in 0..=150_u64 {
        let ms = (k * 1000 + 30) / 60;
        let (a, repaint) = at(&mut h, ms, true);
        let want = (ms % 1200) as f32 * 0.3;
        assert!(circular(a, want) <= 6.0, "{ms} ms: {a} vs {want}");
        assert!((0.0..360.0).contains(&a), "{a}");
        assert_eq!(repaint, busy_tick(ms));
        if a < last {
            wrapped = true;
            assert!(last > 340.0 && a < 20.0, "wraps at 360: {last} -> {a}");
        } else {
            assert!(a > last, "advances every frame");
        }
        last = a;
    }
    assert!(wrapped);
}

#[test]
fn it_starts_on_the_first_frame_so_300_ms_is_a_quarter_turn() {
    let mut h = UiHarness::new(WIN);
    assert_eq!(at(&mut h, 0, true).0, 0.0);
    let (a, repaint) = at(&mut h, 300, true);
    assert!(circular(a, 90.0) < 0.01, "{a}");
    assert_eq!(repaint, busy_tick(300));
}

#[test]
fn a_late_frame_does_not_slow_it() {
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, true);
    let (a, _) = at(&mut h, 900, true);
    assert!(circular(a, 270.0) < 0.01, "{a}");
    let (a, _) = at(&mut h, 4000, true);
    assert!(circular(a, 4000.0 % 1200.0 * 0.3) < 0.01, "{a}");
}

#[test]
fn a_stop_holds_the_angle_and_a_restart_continues_from_it() {
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, true);
    at(&mut h, 300, true);
    // Stopped at 400 ms: 120 degrees, held, and the frame stops asking for more.
    let (a, repaint) = at(&mut h, 400, false);
    assert!(circular(a, 120.0) < 0.01, "{a}");
    assert_eq!(repaint, Repaint::Idle);
    for ms in [500, 2000, 99_000] {
        let (a, repaint) = at(&mut h, ms, false);
        assert!(circular(a, 120.0) < 0.01, "held at {ms}: {a}");
        assert_eq!(repaint, Repaint::Idle, "{ms}");
    }
    // Restarted at 100 s: the first frame is the held angle, then it turns on from there.
    let (a, repaint) = at(&mut h, 100_000, true);
    assert!(circular(a, 120.0) < 0.01, "{a}");
    assert_eq!(repaint, busy_tick(100_000));
    let (a, _) = at(&mut h, 100_300, true);
    assert!(circular(a, 210.0) < 0.01, "{a}");
}

#[test]
fn a_spinner_that_was_never_busy_is_static_at_0() {
    let mut h = UiHarness::new(WIN);
    for ms in [0, 16, 600] {
        let (a, repaint) = at(&mut h, ms, false);
        assert_eq!(a, 0.0);
        assert_eq!(repaint, Repaint::Idle);
    }
}

#[test]
fn reduced_and_off_draw_angle_0_and_report_idle() {
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        let first = at(&mut h, 0, true);
        assert_eq!(first.0, 0.0, "{tier:?}");
        for ms in [16, 300, 1199, 1200, 5000] {
            let (a, repaint) = at(&mut h, ms, true);
            assert_eq!(a, 0.0, "{tier:?} {ms}");
            assert_eq!(repaint, Repaint::Idle, "{tier:?} {ms}");
        }
    }
}

#[test]
fn a_tier_change_drops_back_to_the_held_angle_not_a_jump() {
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, true);
    at(&mut h, 300, true);
    h.tier = MotionTier::Reduced;
    assert_eq!(at(&mut h, 400, true).0, 0.0);
    h.tier = MotionTier::Full;
    // Held 120 at the switch to Reduced; Full again continues from there.
    let (a, _) = at(&mut h, 500, true);
    assert!(circular(a, 120.0) < 0.01, "{a}");
}

#[test]
fn a_spinner_not_drawn_for_a_frame_starts_over() {
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, true);
    at(&mut h, 300, true);
    h.at_ms(400);
    h.frame(|_| {});
    let (a, _) = at(&mut h, 500, true);
    assert_eq!(a, 0.0, "a new run starts at 0");
    let (a, _) = at(&mut h, 800, true);
    assert!(circular(a, 90.0) < 0.01, "{a}");
    // A held angle goes too.
    at(&mut h, 900, false);
    h.at_ms(1000);
    h.frame(|_| {});
    assert_eq!(at(&mut h, 1100, false).0, 0.0);
}

#[test]
fn the_glyph_is_the_sync_icon_centred_on_the_given_point() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    let (scene, _) = h.frame(|ui| {
        ui.spinner(
            "s",
            CENTRE,
            &SpinnerOptions {
                size: 20.0,
                ..opts(true)
            },
        )
    });
    let img = image(&scene);
    let c = (img.dst.x + img.dst.w / 2.0, img.dst.y + img.dst.h / 2.0);
    assert_eq!(c, CENTRE);
    let want = FakeText
        .prepare(&icon::glyph(icon::SYNC, 20.0, DARK.fg_secondary), 1.0)
        .0;
    assert!(matches!(img.source, ImageSource::Text(k, _) if k == want));
    // A colour of the caller's.
    let c = [10, 20, 30, 255];
    let (scene, _) = h.frame(|ui| {
        ui.spinner(
            "s",
            CENTRE,
            &SpinnerOptions {
                color: Some(c),
                ..opts(true)
            },
        )
    });
    let want = FakeText.prepare(&icon::glyph(icon::SYNC, 14.0, c), 1.0).0;
    assert!(matches!(image(&scene).source, ImageSource::Text(k, _) if k == want));
}

#[test]
fn paint_turns_by_the_degrees_given() {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    spinner::paint(&mut p, CENTRE, 14.0, DARK.fg_secondary, 90.0);
    let img = image(&p.scene);
    assert!((img.rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
}

#[test]
fn the_node_is_progress_named_busy_with_the_flag_and_has_no_value() {
    let mut h = UiHarness::new(WIN);
    for busy in [true, false] {
        let (_, out) = h.frame(|ui| ui.spinner("s", CENTRE, &opts(busy)));
        let n = &out.tree.nodes[0];
        assert_eq!(n.role, Role::Progress);
        assert_eq!(n.name, "Indexing");
        assert_eq!(n.value, None);
        assert_eq!(n.states.contains(State::Busy), busy);
        assert!(!n.states.contains(State::Focusable));
        assert_eq!(n.bounds, Rect::new(33.0, 23.0, 14.0, 14.0));
    }
    // Busy stays flagged under Reduced, where the glyph is still.
    h.tier = MotionTier::Reduced;
    let (_, out) = h.frame(|ui| ui.spinner("s", CENTRE, &opts(true)));
    assert!(out.tree.nodes[0].states.contains(State::Busy));
}
