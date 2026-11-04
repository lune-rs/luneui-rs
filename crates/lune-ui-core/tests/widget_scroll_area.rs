//! The scroll area (`layout.scroll-area` acceptance): the reference app's known answers for the pure
//! functions, the wheel, trackpad, keys, preservation, keep-alive, reveals, follow-tail, the node,
//! the million-row offset and the pixel probes.

use lune_ui_core::access::{AccessAction, Action, Node, Role, Value};
use lune_ui_core::input::{FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, Sense, Ui, WidgetId};
use lune_ui_core::widgets::scroll_area::{
    Bars, Extent, INTO_VIEW, LINE, Reveal, ScrollOptions, ScrollResponse, Target, WHEEL,
    centre_target, clamp_offset, restore_offset, reveal_offset, should_restore,
};

const WIN: (f32, f32) = (400.0, 600.0);
const AREA: Rect = Rect::new(20.0, 20.0, 300.0, 400.0);
/// Child A (near the top) and child B (900 px down), relative to the area's origin.
const A: Rect = Rect::new(30.0, 30.0, 100.0, 40.0);
const B: Rect = Rect::new(30.0, 920.0, 100.0, 40.0);
const FRAME_MS: f64 = 1000.0 / 60.0;

fn area_id() -> WidgetId {
    WidgetId::ROOT.with("a")
}

fn a_id() -> WidgetId {
    WidgetId::ROOT.with("child-a")
}

fn b_id() -> WidgetId {
    WidgetId::ROOT.with("child-b")
}

struct Kids {
    a: Response,
}

struct Run {
    out: FrameOutput,
    r: ScrollResponse<Kids>,
}

fn opts(identity: Option<u64>) -> ScrollOptions {
    ScrollOptions {
        identity,
        ..ScrollOptions::default()
    }
}

/// One frame of the area with two focusable children over a content of `ext`.
fn run(h: &mut UiHarness, o: &ScrollOptions, ext: (f64, f64)) -> Run {
    run_with(h, o, ext, |_| {})
}

fn run_with(
    h: &mut UiHarness,
    o: &ScrollOptions,
    ext: (f64, f64),
    before: impl FnOnce(&mut Ui<'_>),
) -> Run {
    let mut got = None;
    let (_, out) = h.frame(|ui| {
        before(ui);
        got = Some(ui.scroll_area("a", AREA, o, |ui, _view| {
            let a = child(ui, a_id(), A, "a");
            child(ui, b_id(), B, "b");
            (Kids { a }, Extent::new(ext.0, ext.1))
        }));
    });
    Run {
        out,
        r: got.expect("frame ran"),
    }
}

fn child(ui: &mut Ui<'_>, id: WidgetId, local: Rect, name: &str) -> Response {
    let rect = Rect::new(AREA.x + local.x, AREA.y + local.y, local.w, local.h);
    let r = ui.interact(id, rect, Sense::CLICK_FOCUS);
    ui.node(Node::new(id, Role::Button, name, rect));
    r
}

fn at(h: &mut UiHarness, ms: f64) {
    h.at_ms(ms.round() as u64);
}

/// Pointer over the area, frames drawn so its hit regions exist.
fn hovered(ext: (f64, f64), o: &ScrollOptions) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    run(&mut h, o, ext);
    h.input.pointer_to(200.0, 200.0);
    at(&mut h, 10.0);
    run(&mut h, o, ext);
    h
}

fn tall() -> (f64, f64) {
    (300.0, 1200.0)
}

/// Frames at 60 Hz from `from` ms for `span` ms; the vertical offsets.
fn offsets(
    h: &mut UiHarness,
    o: &ScrollOptions,
    ext: (f64, f64),
    from: f64,
    span: f64,
) -> Vec<f64> {
    let mut xs = Vec::new();
    let mut k = 1.0;
    while k * FRAME_MS <= span {
        at(h, from + k * FRAME_MS);
        xs.push(run(h, o, ext).r.offset.1);
        k += 1.0;
    }
    xs
}

// Pure functions: the reference app's tests, by name.

#[test]
fn should_restore_same_document_restores() {
    assert!(should_restore(true, Some(7), Some(7)));
}

#[test]
fn should_restore_different_document_does_not() {
    assert!(!should_restore(true, Some(8), Some(7)));
}

#[test]
fn should_restore_first_bind_does_not() {
    assert!(!should_restore(false, Some(7), Some(7)));
}

#[test]
fn should_restore_null_identity_does_not() {
    assert!(!should_restore(true, None, None));
    assert!(!should_restore(true, None, Some(7)));
    assert!(!should_restore(true, Some(7), None));
}

#[test]
fn clamp_restore_offset_known_answers() {
    // content unchanged keeps the offset
    assert_eq!(clamp_offset(300.0, 1000.0, 400.0), 300.0);
    // content shrank: the new maximum
    assert_eq!(clamp_offset(300.0, 500.0, 400.0), 100.0);
    // content now fits: the top
    assert_eq!(clamp_offset(300.0, 200.0, 400.0), 0.0);
    // content grew: unchanged
    assert_eq!(clamp_offset(300.0, 5000.0, 400.0), 300.0);
    // the spec's acceptance: 700 with extent 1000 and viewport 400 gives 600
    assert_eq!(clamp_offset(700.0, 1000.0, 400.0), 600.0);
    // a non-positive saved offset is 0
    assert_eq!(clamp_offset(0.0, 1000.0, 400.0), 0.0);
    assert_eq!(clamp_offset(-50.0, 1000.0, 400.0), 0.0);
    assert_eq!(clamp_offset(-5.0, 1000.0, 400.0), 0.0);
}

#[test]
fn clamp_restore_offset_non_finite_input_is_zero() {
    let (nan, inf) = (f64::NAN, f64::INFINITY);
    for (saved, extent, viewport) in [
        (nan, 1000.0, 400.0),
        (inf, 1000.0, 400.0),
        (300.0, nan, 400.0),
        (300.0, inf, 400.0),
        (300.0, 1000.0, nan),
        (300.0, 1000.0, inf),
    ] {
        assert_eq!(clamp_offset(saved, extent, viewport), 0.0);
    }
}

#[test]
fn restore_offset_skips_a_non_positive_saved_offset_and_a_change_under_half_a_pixel() {
    assert_eq!(restore_offset(0.0, 1000.0, 400.0, 100.0), None);
    assert_eq!(restore_offset(-3.0, 1000.0, 400.0, 100.0), None);
    assert_eq!(restore_offset(300.0, 1000.0, 400.0, 299.6), None);
    assert_eq!(restore_offset(300.0, 1000.0, 400.0, 300.0), None);
    assert_eq!(restore_offset(300.0, 1000.0, 400.0, 299.0), Some(300.0));
    // clamped against the new content
    assert_eq!(restore_offset(300.0, 500.0, 400.0, 0.0), Some(100.0));
    assert_eq!(restore_offset(f64::NAN, 500.0, 400.0, 0.0), None);
}

#[test]
fn centring_row_100_of_45_in_a_400_viewport_is_4322_5_before_the_clamp() {
    assert_eq!(100.0 * 45.0 - (400.0 - 45.0) / 2.0, 4322.5);
    assert_eq!(centre_target(100, 45.0, 400.0, 100_000.0), 4322.5);
    assert_eq!(
        reveal_offset(4500.0, 45.0, 400.0, 0.0, Reveal::Center),
        4322.5
    );
    // the clamp: near the top and past the end
    assert_eq!(centre_target(0, 45.0, 400.0, 100_000.0), 0.0);
    assert_eq!(centre_target(100, 45.0, 400.0, 4600.0), 4200.0);
}

#[test]
fn nearest_moves_the_least() {
    // visible: no move; below: the end lines up; above: the start lines up
    assert_eq!(reveal_offset(100.0, 40.0, 400.0, 0.0, Reveal::Nearest), 0.0);
    assert_eq!(
        reveal_offset(900.0, 40.0, 400.0, 0.0, Reveal::Nearest),
        540.0
    );
    assert_eq!(
        reveal_offset(50.0, 40.0, 400.0, 300.0, Reveal::Nearest),
        50.0
    );
    // taller than the viewport: align the start unless it already covers it
    assert_eq!(
        reveal_offset(100.0, 900.0, 400.0, 0.0, Reveal::Nearest),
        100.0
    );
    assert_eq!(
        reveal_offset(0.0, 900.0, 400.0, 100.0, Reveal::Nearest),
        100.0
    );
}

// The wheel.

#[test]
fn a_wheel_line_steps_48_px_and_settles_exactly() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_lines(0.0, -1.0);
    let first = run(&mut h, &o, tall());
    assert_eq!(first.r.offset.1, 0.0);
    assert_eq!(first.out.repaint, Repaint::Now);
    let xs = offsets(&mut h, &o, tall(), 500.0, 400.0);
    assert!(xs.windows(2).all(|w| w[1] >= w[0]), "monotonic: {xs:?}");
    assert!(xs.iter().any(|v| *v > 0.0 && *v < LINE), "tweens: {xs:?}");
    // settled within 180 ms plus one frame, exactly on the step
    let k = ((180.0 + FRAME_MS) / FRAME_MS).ceil() as usize;
    assert!(xs[k - 1..].iter().all(|v| *v == LINE), "settled: {xs:?}");
    let last = run(&mut h, &o, tall());
    assert_eq!(last.out.repaint, Repaint::Idle);
}

#[test]
fn the_wheel_and_into_view_rows_conform_in_every_tier() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&WHEEL, &DARK.motion, tier, 0.0_f32, 48.0)
            .unwrap_or_else(|e| panic!("wheel {tier:?}: {e}"));
        conformance(&INTO_VIEW, &DARK.motion, tier, 0.0_f32, 740.0)
            .unwrap_or_else(|e| panic!("into view {tier:?}: {e}"));
    }
}

#[test]
fn reduced_and_off_snap_the_wheel_step() {
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let o = opts(None);
        let mut h = hovered(tall(), &o);
        h.tier = tier;
        at(&mut h, 500.0);
        h.input.scroll_lines(0.0, -1.0);
        assert_eq!(run(&mut h, &o, tall()).r.offset.1, LINE, "{tier:?}");
    }
}

#[test]
fn the_wheel_clamps_at_both_ends() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_lines(0.0, 100.0);
    run(&mut h, &o, tall());
    at(&mut h, 1000.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 0.0);
    h.input.scroll_lines(0.0, -100.0);
    at(&mut h, 1100.0);
    run(&mut h, &o, tall());
    at(&mut h, 2000.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 800.0);
    // a step back from the end is not dead
    h.input.scroll_lines(0.0, 1.0);
    at(&mut h, 2100.0);
    run(&mut h, &o, tall());
    at(&mut h, 3000.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 752.0);
}

#[test]
fn repeated_steps_retarget_and_add_up() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_lines(0.0, -1.0);
    run(&mut h, &o, tall());
    at(&mut h, 530.0);
    h.input.scroll_lines(0.0, -1.0);
    run(&mut h, &o, tall());
    at(&mut h, 1500.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 2.0 * LINE);
}

#[test]
fn a_trackpad_delta_moves_the_offset_one_to_one_at_once() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -37.0);
    let r = run(&mut h, &o, tall());
    assert_eq!(r.r.offset.1, 37.0);
    assert_eq!(r.out.repaint, Repaint::Now);
    at(&mut h, 520.0);
    assert_eq!(run(&mut h, &o, tall()).out.repaint, Repaint::Idle);
}

#[test]
fn shift_turns_a_vertical_wheel_into_a_horizontal_one() {
    let o = opts(None);
    let ext = (900.0, 1200.0);
    let mut h = hovered(ext, &o);
    h.input.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    at(&mut h, 500.0);
    h.input.scroll_lines(0.0, -1.0);
    run(&mut h, &o, ext);
    at(&mut h, 1000.0);
    let r = run(&mut h, &o, ext);
    assert_eq!(r.r.offset, (LINE, 0.0));
}

#[test]
fn a_hidden_bar_scrolls_and_a_disabled_one_does_not() {
    for (bars, moves) in [
        ((Bars::Auto, Bars::Hidden), true),
        ((Bars::Auto, Bars::Disabled), false),
    ] {
        let o = ScrollOptions { bars, ..opts(None) };
        let mut h = hovered(tall(), &o);
        at(&mut h, 500.0);
        h.input.scroll_pixels(0.0, -50.0);
        let r = run(&mut h, &o, tall());
        assert_eq!(r.r.offset.1 == 50.0, moves, "{bars:?}");
    }
}

#[test]
fn the_wheel_goes_to_the_topmost_area_and_an_area_at_its_end_does_not_pass_it_up() {
    let mut h = UiHarness::new(WIN);
    let outer = Rect::new(0.0, 0.0, 400.0, 600.0);
    let inner = Rect::new(20.0, 20.0, 200.0, 200.0);
    let o = opts(None);
    let draw = |h: &mut UiHarness| {
        let mut res = (0.0, 0.0);
        h.frame(|ui| {
            let r = ui.scroll_area("outer", outer, &o, |ui, _| {
                let i = ui.scroll_area("inner", inner, &o, |_, _| ((), Extent::new(200.0, 500.0)));
                (i.offset.1, Extent::new(400.0, 2000.0))
            });
            res = (r.offset.1, r.value);
        });
        res
    };
    draw(&mut h);
    h.input.pointer_to(100.0, 100.0);
    at(&mut h, 10.0);
    draw(&mut h);
    h.input.scroll_pixels(0.0, -50.0);
    at(&mut h, 20.0);
    assert_eq!(draw(&mut h), (0.0, 50.0), "inner takes the delta");
    // at the inner end it stays put and the outer does not move either
    h.input.scroll_pixels(0.0, -1000.0);
    at(&mut h, 30.0);
    assert_eq!(draw(&mut h), (0.0, 300.0));
    h.input.scroll_pixels(0.0, -10.0);
    at(&mut h, 40.0);
    assert_eq!(draw(&mut h), (0.0, 300.0));
}

#[test]
fn a_disabled_area_ignores_the_wheel() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -50.0);
    let mut got = None;
    h.frame(|ui| {
        ui.with_enabled(false, |ui| {
            got = Some(ui.scroll_area("a", AREA, &o, |_, _| ((), Extent::new(300.0, 1200.0))));
        });
    });
    assert_eq!(got.expect("ran").offset, (0.0, 0.0));
}

// A million rows.

#[test]
fn a_one_pixel_trackpad_delta_at_45_million_moves_the_offset_by_exactly_one() {
    let ext = (300.0, 100_000_000.0);
    let o = ScrollOptions {
        extent: Some(ext),
        ..opts(None)
    };
    let mut h = hovered(ext, &o);
    at(&mut h, 100.0);
    // centre a 45 px row whose raw offset is exactly 45,000,000
    let target = Target::span_y(45_000_000.0 + (400.0 - 45.0) / 2.0, 45.0);
    let r = run_with(&mut h, &o, ext, |ui| {
        ui.scroll_to("a", target, Reveal::Center)
    });
    assert_eq!(r.r.offset.1, 45_000_000.0);
    for k in 1..=3_u32 {
        at(&mut h, 100.0 + f64::from(k) * 20.0);
        h.input.scroll_pixels(0.0, -1.0);
        let r = run(&mut h, &o, ext);
        assert_eq!(r.r.offset.1, 45_000_000.0 + f64::from(k));
    }
    h.input.scroll_pixels(0.0, 1.0);
    at(&mut h, 300.0);
    assert_eq!(run(&mut h, &o, ext).r.offset.1, 45_000_002.0);
}

// Preservation.

#[test]
fn the_same_key_keeps_its_offset_through_a_short_frame_and_restores_it() {
    let o = opts(Some(1));
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -300.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 300.0);
    at(&mut h, 520.0);
    assert_eq!(run(&mut h, &o, (300.0, 100.0)).r.offset.1, 0.0);
    at(&mut h, 540.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 300.0);
}

#[test]
fn no_key_never_restores() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -300.0);
    run(&mut h, &o, tall());
    at(&mut h, 520.0);
    run(&mut h, &o, (300.0, 100.0));
    at(&mut h, 540.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 0.0);
}

#[test]
fn a_new_key_starts_at_the_top() {
    let mut h = hovered(tall(), &opts(Some(1)));
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -300.0);
    run(&mut h, &opts(Some(1)), tall());
    at(&mut h, 520.0);
    assert_eq!(run(&mut h, &opts(Some(2)), tall()).r.offset.1, 0.0);
    // and it does not come back with the old key
    at(&mut h, 540.0);
    assert_eq!(run(&mut h, &opts(Some(1)), tall()).r.offset.1, 0.0);
}

#[test]
fn a_measured_extent_that_shrinks_clamps_the_offset_and_asks_for_a_catch_up() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -800.0);
    run(&mut h, &o, tall());
    at(&mut h, 520.0);
    let r = run(&mut h, &o, (300.0, 900.0));
    assert_eq!(r.r.offset.1, 500.0);
    assert_eq!(r.out.repaint, Repaint::Now);
    at(&mut h, 540.0);
    assert_eq!(run(&mut h, &o, (300.0, 900.0)).out.repaint, Repaint::Idle);
    // an empty extent: the top, never negative
    at(&mut h, 560.0);
    assert_eq!(run(&mut h, &o, (0.0, 0.0)).r.offset, (0.0, 0.0));
}

#[test]
fn a_known_extent_clamps_before_the_content_draws() {
    let o = ScrollOptions {
        extent: Some((300.0, 900.0)),
        ..opts(None)
    };
    let mut h = hovered((300.0, 900.0), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -5000.0);
    let mut seen = None;
    h.frame(|ui| {
        ui.scroll_area("a", AREA, &o, |_, view| {
            seen = Some(view.offset);
            ((), Extent::new(300.0, 900.0))
        });
    });
    assert_eq!(seen, Some((0.0, 500.0)));
}

// Keep-alive.

#[test]
fn a_hidden_area_under_keep_alive_keeps_its_offset_for_600_undrawn_frames() {
    let o = opts(None);
    let draw = |h: &mut UiHarness, scroll: Option<f32>| -> f64 {
        let mut off = 0.0;
        if let Some(d) = scroll {
            h.input.scroll_pixels(0.0, d);
        }
        h.frame(|ui| {
            ui.keep_alive("tab", |ui| {
                off = ui
                    .scroll_area("a", AREA, &o, |_, _| ((), Extent::new(300.0, 1200.0)))
                    .offset
                    .1;
            });
        });
        off
    };
    let mut h = UiHarness::new(WIN);
    draw(&mut h, None);
    h.input.pointer_to(200.0, 200.0);
    at(&mut h, 10.0);
    draw(&mut h, None);
    at(&mut h, 20.0);
    assert_eq!(draw(&mut h, Some(-300.0)), 300.0);
    for k in 0..600_u64 {
        h.at_ms(30 + k);
        h.frame(|_| {});
    }
    at(&mut h, 700.0);
    assert_eq!(draw(&mut h, None), 300.0);
}

#[test]
fn outside_a_scope_the_offset_is_gone_after_one_undrawn_frame() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -300.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 300.0);
    at(&mut h, 520.0);
    h.frame(|_| {});
    at(&mut h, 540.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 0.0);
}

// Follow-tail.

#[test]
fn follow_tail_goes_to_the_end_when_the_extent_grows() {
    let o = ScrollOptions {
        follow_tail: true,
        ..opts(None)
    };
    let mut h = hovered((300.0, 1000.0), &o);
    // the first frame of a log with content ends at the tail already
    assert_eq!(run(&mut h, &o, (300.0, 1000.0)).r.offset.1, 600.0);
    at(&mut h, 500.0);
    assert_eq!(run(&mut h, &o, (300.0, 1100.0)).r.offset.1, 700.0);
    // the user scrolls up; no growth, no move
    h.input.scroll_pixels(0.0, 300.0);
    at(&mut h, 520.0);
    assert_eq!(run(&mut h, &o, (300.0, 1100.0)).r.offset.1, 400.0);
    at(&mut h, 540.0);
    assert_eq!(run(&mut h, &o, (300.0, 1100.0)).r.offset.1, 400.0);
    // growth again
    at(&mut h, 560.0);
    assert_eq!(run(&mut h, &o, (300.0, 1150.0)).r.offset.1, 750.0);
    // a shrink to nothing goes to the top, not negative
    at(&mut h, 580.0);
    assert_eq!(run(&mut h, &o, (0.0, 0.0)).r.offset.1, 0.0);
}

#[test]
fn follow_tail_off_does_not_follow_and_can_be_toggled() {
    let off = opts(None);
    let on = ScrollOptions {
        follow_tail: true,
        ..opts(None)
    };
    let mut h = hovered((300.0, 1000.0), &off);
    assert_eq!(run(&mut h, &off, (300.0, 1000.0)).r.offset.1, 0.0);
    at(&mut h, 500.0);
    assert_eq!(run(&mut h, &off, (300.0, 1100.0)).r.offset.1, 0.0);
    at(&mut h, 520.0);
    assert_eq!(run(&mut h, &on, (300.0, 1200.0)).r.offset.1, 800.0);
    at(&mut h, 540.0);
    assert_eq!(run(&mut h, &off, (300.0, 1300.0)).r.offset.1, 800.0);
}

// Reveals.

fn access(h: &mut UiHarness, target: WidgetId, action: Action) {
    h.input.access(AccessAction {
        target,
        action,
        value: None,
    });
}

#[test]
fn scroll_into_view_on_a_child_centres_it() {
    // within two viewports: animated, then exact
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    let ext = (300.0, 3000.0);
    run(&mut h, &o, ext);
    let tall_b = (300.0, 3000.0);
    access(&mut h, b_id(), Action::ScrollIntoView);
    at(&mut h, 600.0);
    run(&mut h, &o, tall_b);
    // B at 920 .. 960, centred: 920 - (400 - 40) / 2 = 740; 740 <= 800, so it animates
    let xs = offsets(&mut h, &o, tall_b, 600.0, 400.0);
    assert!(xs.windows(2).all(|w| w[1] >= w[0]), "monotonic: {xs:?}");
    assert_eq!(*xs.last().expect("frames"), 740.0);
    assert!(xs.iter().any(|v| *v > 0.0 && *v < 740.0));
}

#[test]
fn scroll_into_view_beyond_two_viewports_snaps() {
    let o = opts(None);
    let mut h = hovered((300.0, 5000.0), &o);
    at(&mut h, 500.0);
    let far = |h: &mut UiHarness| {
        let mut got = None;
        h.frame(|ui| {
            got = Some(ui.scroll_area("a", AREA, &o, |ui, _| {
                let rect = Rect::new(AREA.x, AREA.y + 3000.0, 100.0, 40.0);
                ui.interact(a_id(), rect, Sense::CLICK_FOCUS);
                ui.node(Node::new(a_id(), Role::Button, "far", rect));
                ((), Extent::new(300.0, 5000.0))
            }));
        });
        got.expect("ran").offset.1
    };
    assert_eq!(far(&mut h), 0.0);
    access(&mut h, a_id(), Action::ScrollIntoView);
    at(&mut h, 520.0);
    // 3000 - 180 = 2820 at once (more than two viewports from 0)
    assert_eq!(far(&mut h), 2820.0);
}

#[test]
fn a_target_outside_the_content_changes_nothing() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -300.0);
    run(&mut h, &o, tall());
    for target in [
        Target::span_y(5000.0, 45.0),
        Target::span_y(-100.0, 45.0),
        Target::span_y(f64::NAN, 45.0),
        Target::span_y(10.0, f64::INFINITY),
    ] {
        at(&mut h, 600.0);
        let r = run_with(&mut h, &o, tall(), |ui| {
            ui.scroll_to("a", target, Reveal::Center)
        });
        assert_eq!(r.r.offset.1, 300.0, "{target:?}");
    }
}

#[test]
fn scroll_to_centres_a_row_and_leaves_the_other_axis_alone() {
    let o = opts(None);
    let ext = (900.0, 6000.0);
    let mut h = hovered(ext, &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(-120.0, 0.0);
    assert_eq!(run(&mut h, &o, ext).r.offset, (120.0, 0.0));
    at(&mut h, 600.0);
    let r = run_with(&mut h, &o, ext, |ui| {
        ui.scroll_to("a", Target::span_y(100.0 * 45.0, 45.0), Reveal::Center);
    });
    // 4322.5 is more than two viewports away: at once, x untouched
    assert_eq!(r.r.offset, (120.0, 4322.5));
}

fn tab(h: &mut UiHarness) {
    h.input.key_down(Key::Named(NamedKey::Tab), None);
}

#[test]
fn a_keyboard_focus_move_to_a_child_outside_the_viewport_reveals_it_with_the_least_movement() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 100.0);
    tab(&mut h);
    run(&mut h, &o, tall());
    at(&mut h, 120.0);
    assert!(run(&mut h, &o, tall()).r.value.a.focused);
    // A is in view: no movement. Tab to B at 920 .. 960: end 960 - 400 = 560
    tab(&mut h);
    at(&mut h, 200.0);
    run(&mut h, &o, tall());
    let xs = offsets(&mut h, &o, tall(), 200.0, 400.0);
    assert!(xs.windows(2).all(|w| w[1] >= w[0]), "monotonic: {xs:?}");
    assert_eq!(*xs.last().expect("frames"), 560.0);
    // a wheel step away is not undone by the frame that follows
    h.input.scroll_pixels(0.0, 100.0);
    at(&mut h, 800.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 460.0);
    at(&mut h, 820.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 460.0);
}

#[test]
fn a_pointer_press_on_a_partly_visible_child_does_not_scroll() {
    let o = opts(None);
    let mut h = UiHarness::new(WIN);
    let ext = (300.0, 1200.0);
    run(&mut h, &o, ext);
    h.input.pointer_to(AREA.x + 40.0, AREA.y + 50.0);
    at(&mut h, 10.0);
    run(&mut h, &o, ext);
    h.input.press(PointerButton::Primary);
    at(&mut h, 20.0);
    let r = run(&mut h, &o, ext);
    assert!(r.r.value.a.press_started);
    at(&mut h, 40.0);
    assert!(run(&mut h, &o, ext).r.value.a.focused);
    assert_eq!(run(&mut h, &o, ext).r.offset.1, 0.0);
}

#[test]
fn page_keys_home_and_end_scroll_when_the_focused_child_declines_them() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 100.0);
    tab(&mut h);
    run(&mut h, &o, tall());
    for (key, want) in [
        (NamedKey::PageDown, 400.0),
        (NamedKey::PageDown, 800.0),
        (NamedKey::PageDown, 800.0),
        (NamedKey::PageUp, 400.0),
        (NamedKey::End, 800.0),
        (NamedKey::Home, 0.0),
    ] {
        h.input.key_down(Key::Named(key), None);
        at(&mut h, 200.0);
        let r = run(&mut h, &o, tall());
        assert_eq!(r.r.offset.1, want, "{key:?}");
    }
}

#[test]
fn arrow_keys_do_not_scroll_and_a_child_that_takes_the_key_keeps_it() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 100.0);
    tab(&mut h);
    run(&mut h, &o, tall());
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    at(&mut h, 200.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 0.0);
    // the focused child consumes PageDown itself
    h.input.key_down(Key::Named(NamedKey::PageDown), None);
    at(&mut h, 300.0);
    let mut got = None;
    h.frame(|ui| {
        got = Some(ui.scroll_area("a", AREA, &o, |ui, _| {
            let a = child(ui, a_id(), A, "a");
            let took = ui.consume_key(a_id(), NamedKey::PageDown);
            ((a, took), Extent::new(300.0, 1200.0))
        }));
    });
    let r = got.expect("ran");
    assert!(r.value.1);
    assert_eq!(r.offset.1, 0.0);
}

// The node.

#[test]
fn the_node_is_a_scroll_area_with_the_offset_as_its_value_and_children_nested() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -250.0);
    let r = run(&mut h, &o, tall());
    let node = r.out.tree.get(area_id()).expect("area node");
    assert_eq!(node.role, Role::ScrollArea);
    assert_eq!(
        node.value,
        Some(Value::Number {
            value: 250.0,
            min: 0.0,
            max: 800.0,
            step: LINE
        })
    );
    assert!(node.children.contains(&a_id()) && node.children.contains(&b_id()));
    assert_eq!(r.out.tree.get(a_id()).expect("a").parent, Some(area_id()));
    assert!(node.actions.contains(Action::Increment) && node.actions.contains(Action::Decrement));
    assert!(node.states.contains(lune_ui_core::access::State::Focusable));
}

#[test]
fn increment_and_decrement_are_a_wheel_step() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    run(&mut h, &o, tall());
    access(&mut h, area_id(), Action::Increment);
    at(&mut h, 500.0);
    run(&mut h, &o, tall());
    at(&mut h, 1000.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, LINE);
    access(&mut h, area_id(), Action::Decrement);
    at(&mut h, 1100.0);
    run(&mut h, &o, tall());
    at(&mut h, 1600.0);
    assert_eq!(run(&mut h, &o, tall()).r.offset.1, 0.0);
}

// Pixel probes at scale 2.

fn under(scene: &Scene) -> &[lune_ui_core::scene::Shape] {
    &scene.layers[0].geometry.under
}

fn probe_run(h: &mut UiHarness, o: &ScrollOptions) -> Scene {
    let (s, _) = h.frame(|ui| {
        ui.scroll_area("a", AREA, o, |ui, _| {
            ui.painter().fill(
                Rect::new(AREA.x, AREA.y + 100.0, 50.0, 50.0),
                0.0,
                [255, 0, 0, 255],
            );
            // a fill far below the viewport
            ui.painter().fill(
                Rect::new(AREA.x, AREA.y + 2000.0, 50.0, 50.0),
                0.0,
                [0, 255, 0, 255],
            );
            ((), Extent::new(300.0, 3000.0))
        });
    });
    s
}

#[test]
fn the_clip_edge_is_exact_and_content_at_offset_40_is_shifted_by_80_device_px() {
    let o = opts(None);
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let before = probe_run(&mut h, &o);
    h.input.pointer_to(200.0, 200.0);
    at(&mut h, 10.0);
    probe_run(&mut h, &o);
    h.input.scroll_pixels(0.0, -40.0);
    at(&mut h, 20.0);
    probe_run(&mut h, &o);
    at(&mut h, 30.0);
    let after = probe_run(&mut h, &o);
    let red = |s: &Scene| {
        *under(s)
            .iter()
            .find(|sh| sh.color[0] == 1.0 && sh.color[1] == 0.0)
            .expect("red fill")
    };
    let (r0, r1) = (red(&before), red(&after));
    assert_eq!(r0.rect[1] - r1.rect[1], 80.0);
    assert_eq!(r0.rect[0], r1.rect[0]);
    // the clip is the viewport in device px
    let want = [40.0, 40.0, 600.0, 800.0];
    assert_eq!(r0.clip, want);
    assert_eq!(r1.clip, want);
}

#[test]
fn a_hit_at_the_viewport_edge_is_clipped_to_it_with_a_non_zero_offset() {
    let o = opts(None);
    let mut h = hovered(tall(), &o);
    at(&mut h, 500.0);
    h.input.scroll_pixels(0.0, -10.0);
    let r = run(&mut h, &o, tall());
    // B's rect is far below; A at 30..70 of the area is above the clip top after 40 px
    h.input.scroll_pixels(0.0, -60.0);
    at(&mut h, 520.0);
    let r2 = run(&mut h, &o, tall());
    assert_eq!(r.r.offset.1, 10.0);
    assert_eq!(r2.r.offset.1, 70.0);
    // A spans 30..70 in content space; with offset 70 it is fully above the viewport
    assert!(r2.r.value.a.rect.w * r2.r.value.a.rect.h == 0.0);

    // with offset 10 A is at 20..60 on screen, inside the viewport
    assert_eq!(
        r.r.value.a.rect,
        Rect::new(A.x + AREA.x, AREA.y + 20.0, 100.0, 40.0)
    );
    // clipped at the bottom edge: put a child straddling it
    at(&mut h, 540.0);
    let mut got = None;
    h.frame(|ui| {
        ui.scroll_area("a", AREA, &o, |ui, _| {
            got = Some(ui.interact(
                a_id(),
                Rect::new(AREA.x, AREA.y + 450.0, 100.0, 40.0),
                Sense::CLICK,
            ));
            ((), Extent::new(300.0, 1200.0))
        });
    });
    // content y 450..490 at offset 70 is 380..420 in the area: 20 px of it show
    let rect = got.expect("ran").rect;
    assert_eq!(rect.y, AREA.y + 380.0);
    assert_eq!(rect.h, 20.0);
}

#[test]
fn the_area_draws_nothing_itself_and_registers_one_region() {
    let o = ScrollOptions {
        bars: (Bars::Hidden, Bars::Hidden),
        ..opts(None)
    };
    let mut h = UiHarness::new(WIN);
    let (scene, _) = h.frame(|ui| {
        ui.scroll_area("a", AREA, &o, |_, _| ((), Extent::new(300.0, 1200.0)));
    });
    let g = &scene.layers[0].geometry;
    assert!(g.under.is_empty() && g.over.is_empty());
}

/// The area over 1000 px of content, with `after` run once it has been called and `inside` from
/// its content. Returns the offset it ended on and what the frame reported.
fn requesting(
    h: &mut UiHarness,
    ms: u64,
    inside: impl FnOnce(&mut Ui<'_>),
    after: impl FnOnce(&mut Ui<'_>),
) -> (f64, Repaint) {
    let o = ScrollOptions {
        extent: Some((300.0, 1000.0)),
        ..ScrollOptions::default()
    };
    let mut offset = 0.0;
    let (_, out) = h.at_ms(ms).frame(|ui| {
        let r = ui.scroll_area("a", AREA, &o, |ui, _| {
            inside(ui);
            ((), Extent::default())
        });
        offset = r.offset.1;
        after(ui);
    });
    (offset, out.repaint)
}

fn ask(ui: &mut Ui<'_>) {
    ui.scroll_to("a", Target::span_y(800.0, 10.0), Reveal::Nearest);
}

/// Frames with `ask` in the given slot until one reports `Idle`; the offset it ended on.
fn until_idle(h: &mut UiHarness, ms: &mut u64, late: bool) -> f64 {
    for _ in 0..200 {
        *ms += 17;
        let (offset, repaint) = if late {
            requesting(h, *ms, |_| (), ask)
        } else {
            requesting(h, *ms, |_| (), |_| ())
        };
        if repaint == Repaint::Idle {
            return offset;
        }
    }
    panic!("never settled");
}

#[test]
fn scroll_to_after_the_area_on_a_quiet_frame_asks_for_the_frame_that_applies_it() {
    let mut h = UiHarness::new(WIN);
    requesting(&mut h, 0, |_| (), |_| ());
    assert_eq!(requesting(&mut h, 100, |_| (), |_| ()).1, Repaint::Idle);
    let (offset, repaint) = requesting(&mut h, 200, |_| (), ask);
    assert_eq!((offset, repaint), (0.0, Repaint::Now));
    let mut ms = 200;
    assert_eq!(until_idle(&mut h, &mut ms, false), 410.0);
}

#[test]
fn scroll_to_every_frame_after_the_area_settles_to_idle() {
    let mut h = UiHarness::new(WIN);
    requesting(&mut h, 0, |_| (), |_| ());
    let mut ms = 0;
    assert_eq!(until_idle(&mut h, &mut ms, true), 410.0);
    for _ in 0..3 {
        ms += 17;
        assert_eq!(requesting(&mut h, ms, |_| (), ask).1, Repaint::Idle);
    }
}

#[test]
fn scroll_to_after_the_area_that_changes_nothing_stays_idle() {
    let mut h = UiHarness::new(WIN);
    requesting(&mut h, 0, |_| (), |_| ());
    requesting(&mut h, 100, |_| (), |_| ());
    let showing = |ui: &mut Ui<'_>| ui.scroll_to("a", Target::span_y(10.0, 10.0), Reveal::Nearest);
    assert_eq!(requesting(&mut h, 200, |_| (), showing).1, Repaint::Idle);
    let outside =
        |ui: &mut Ui<'_>| ui.scroll_to("a", Target::span_y(5000.0, 10.0), Reveal::Nearest);
    assert_eq!(requesting(&mut h, 300, |_| (), outside).1, Repaint::Idle);
}

#[test]
fn scroll_to_from_the_areas_own_content_is_served_in_that_frame() {
    let mut h = UiHarness::new(WIN);
    requesting(&mut h, 0, |_| (), |_| ());
    requesting(&mut h, 100, |_| (), |_| ());
    assert_eq!(requesting(&mut h, 200, ask, |_| ()).1, Repaint::Now);
    let mut ms = 200;
    assert_eq!(until_idle(&mut h, &mut ms, false), 410.0);
}

#[test]
fn wheel_x_turns_a_vertical_gesture_sideways() {
    let wide = ScrollOptions {
        bars: (Bars::Hidden, Bars::Disabled),
        wheel_x: true,
        ..ScrollOptions::default()
    };
    let ext = (2000.0, 100.0);
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    h.input.pointer_to(100.0, 100.0);
    run(&mut h, &wide, ext);
    run(&mut h, &wide, ext);
    h.input.scroll_pixels(0.0, -30.0);
    assert_eq!(run(&mut h, &wide, ext).r.offset, (30.0, 0.0));
    // The larger component wins, in either direction.
    h.input.scroll_pixels(-20.0, 5.0);
    assert_eq!(run(&mut h, &wide, ext).r.offset, (50.0, 0.0));
    h.input.scroll_lines(0.0, -1.0);
    h.at_ms(10);
    run(&mut h, &wide, ext);
    h.at_ms(1000);
    assert_eq!(run(&mut h, &wide, ext).r.offset, (50.0 + LINE, 0.0));
}

#[test]
fn without_wheel_x_a_vertical_gesture_stays_vertical() {
    let o = ScrollOptions {
        bars: (Bars::Hidden, Bars::Hidden),
        ..ScrollOptions::default()
    };
    assert!(!ScrollOptions::default().wheel_x);
    let ext = (2000.0, 2000.0);
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    h.input.pointer_to(100.0, 100.0);
    run(&mut h, &o, ext);
    run(&mut h, &o, ext);
    h.input.scroll_pixels(0.0, -30.0);
    assert_eq!(run(&mut h, &o, ext).r.offset, (0.0, 30.0));
}
