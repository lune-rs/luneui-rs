//! The slider (`widget.slider` acceptance, D119): every source of a value (press, drag, key,
//! action, set-value, the app) and every way the drawn knob could differ from it, the stops, the
//! degenerate inputs, motion conformance, probes at scale 2 and the node.

use lune_ui_core::access::{AccessAction, AccessValue, Action, Role, SemanticTree, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};
use lune_ui_core::widgets::slider::{
    FOCUS, HOVER_SIZE, SliderLook, SliderOptions, SliderState, VALUE, fraction, hit_rect, paint,
    track,
};

const WIN: (f32, f32) = (600.0, 300.0);
/// 400 wide, so a fraction of the track is easy to name; the track's y centre is 112.
const RECT: Rect = Rect::new(100.0, 100.0, 400.0, 24.0);
const MID_Y: f32 = 112.0;
const P: PointerButton = PointerButton::Primary;
const OPTS: SliderOptions<'static> = SliderOptions {
    min: 0.0,
    max: 100.0,
    step: 10.0,
    name: "Volume",
};

fn id() -> WidgetId {
    WidgetId::ROOT.with("s")
}

/// The window x of `f` on the track.
fn at(f: f32) -> f32 {
    RECT.x + RECT.w * f
}

struct Seen {
    scene: Scene,
    resp: Response,
    repaint: Repaint,
    tree: SemanticTree,
}

fn draw(h: &mut UiHarness, rect: Rect, value: &mut f32, opts: &SliderOptions<'_>) -> Seen {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| resp = Some(ui.slider("s", rect, value, opts)));
    Seen {
        scene,
        resp: resp.expect("frame ran"),
        repaint: out.repaint,
        tree: out.tree,
    }
}

fn frame(h: &mut UiHarness, value: &mut f32) -> Seen {
    draw(h, RECT, value, &OPTS)
}

/// A harness with the slider drawn once at `value`.
fn start(value: &mut f32) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, value);
    h
}

fn key(h: &mut UiHarness, k: NamedKey) {
    h.input.key_down(Key::Named(k), None);
    h.input.key_up(Key::Named(k));
}

/// Tabs onto the slider; two frames, as focus lands at the frame's begin.
fn focus(h: &mut UiHarness, value: &mut f32) {
    key(h, NamedKey::Tab);
    frame(h, value);
    frame(h, value);
}

fn access(h: &mut UiHarness, action: Action, value: Option<AccessValue>) {
    h.input.access(AccessAction {
        target: id(),
        action,
        value,
    });
}

fn knob_shape(scene: &Scene) -> Shape {
    let g = &scene.layers[0].geometry;
    let color = premul(DARK.fg_primary);
    *g.under
        .iter()
        .chain(&g.over)
        .find(|s| s.color == color)
        .expect("a knob")
}

/// The knob centre's x (the harness runs at scale 1).
fn knob_x(scene: &Scene) -> f32 {
    let k = knob_shape(scene);
    k.rect[0] + k.rect[2] / 2.0
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn right_and_left_change_the_value_by_one_step_and_stop_at_the_ends() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    for (k, want) in [
        (NamedKey::ArrowRight, 50.0),
        (NamedKey::ArrowUp, 60.0),
        (NamedKey::ArrowLeft, 50.0),
        (NamedKey::ArrowDown, 40.0),
    ] {
        key(&mut h, k);
        let s = frame(&mut h, &mut v);
        assert_eq!(v, want, "{k:?}");
        assert!(s.resp.changed, "{k:?}");
        assert!(s.resp.focus_visible);
    }

    let mut v = 100.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    key(&mut h, NamedKey::ArrowRight);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 100.0);
    assert!(!s.resp.changed, "nothing stored changed at the top");

    let mut v = 0.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    key(&mut h, NamedKey::ArrowLeft);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 0.0);
    assert!(!s.resp.changed, "nothing stored changed at the bottom");
}

#[test]
fn every_arrow_of_a_frame_counts_in_arrival_order() {
    let mut v = 100.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    key(&mut h, NamedKey::ArrowLeft);
    key(&mut h, NamedKey::ArrowLeft);
    key(&mut h, NamedKey::ArrowRight);
    frame(&mut h, &mut v);
    assert_eq!(v, 90.0);
}

#[test]
fn a_press_at_three_quarters_sets_the_value_at_once_with_no_drag_threshold() {
    let mut v = 0.0;
    let mut h = start(&mut v);
    h.at_ms(10).input.pointer_to(at(0.75), MID_Y);
    frame(&mut h, &mut v);
    assert_eq!(v, 0.0, "hover writes nothing");
    h.at_ms(20).input.press(P);
    let s = frame(&mut h, &mut v);
    // 75 between stops 70 and 80 rounds up.
    assert_eq!(v, 80.0);
    assert!(s.resp.changed);
    assert!(s.resp.pressed);
    assert!(
        !s.resp.dragged,
        "no drag has started: the press alone set it"
    );
    assert!(near(knob_x(&s.scene), at(0.8)), "the knob is at the value");
    assert_eq!(
        s.repaint,
        Repaint::Now,
        "the rest of the frame drew the old"
    );
    // Past the hover size's tween, so the frame has nothing left to do.
    h.at_ms(500);
    let s = frame(&mut h, &mut v);
    assert!(!s.resp.changed, "a held pointer changes nothing");
    assert_eq!(s.repaint, Repaint::Idle);
}

#[test]
fn a_press_in_the_hit_margin_clamps_to_the_end() {
    let hit = hit_rect(RECT);
    assert_eq!(hit.x, RECT.x - 4.0);
    assert_eq!(hit.right(), RECT.right() + 4.0);
    assert_eq!(hit.h, 16.0);
    let mut v = 50.0;
    let mut h = start(&mut v);
    h.input.pointer_to(RECT.x - 3.0, MID_Y - 7.0);
    frame(&mut h, &mut v);
    h.input.press(P);
    frame(&mut h, &mut v);
    assert_eq!(v, 0.0);
}

#[test]
fn a_drag_follows_on_every_frame_with_the_knob_on_the_value_and_nothing_in_flight() {
    let mut v = 0.0;
    let mut h = start(&mut v);
    h.input.pointer_to(at(0.2), MID_Y);
    frame(&mut h, &mut v);
    h.input.press(P);
    frame(&mut h, &mut v);
    assert_eq!(v, 20.0);
    for (i, f) in [0.3, 0.5, 0.9].into_iter().enumerate() {
        h.at_ms(16 * (i as u64 + 1)).input.pointer_to(at(f), MID_Y);
        let s = frame(&mut h, &mut v);
        assert!(near(v, f * 100.0), "{v}");
        assert!(s.resp.changed);
        assert!(near(knob_x(&s.scene), at(f)), "knob at {f}");
    }
    // Past the end the value clamps, and the pointer off the track vertically still counts.
    h.input.pointer_to(590.0, 280.0);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 100.0);
    assert!(near(knob_x(&s.scene), at(1.0)));
    h.input.pointer_to(2.0, 5.0);
    frame(&mut h, &mut v);
    assert_eq!(v, 0.0);
    h.input.pointer_to(590.0, 5.0);
    frame(&mut h, &mut v);
    assert_eq!(v, 100.0);
    h.input.pointer_to(595.0, 5.0);
    let s = frame(&mut h, &mut v);
    assert!(!s.resp.changed, "`changed` only when the stored value did");
    h.input.release(P);
    frame(&mut h, &mut v);
    h.at_ms(1000);
    let s = frame(&mut h, &mut v);
    assert_eq!(s.repaint, Repaint::Idle);
}

#[test]
fn a_press_moved_less_than_the_threshold_still_follows() {
    let opts = SliderOptions { step: 0.0, ..OPTS };
    let mut v = 0.0;
    let mut h = UiHarness::new(WIN);
    draw(&mut h, RECT, &mut v, &opts);
    h.input.pointer_to(at(0.5), MID_Y);
    draw(&mut h, RECT, &mut v, &opts);
    h.input.press(P);
    draw(&mut h, RECT, &mut v, &opts);
    h.input.pointer_to(at(0.5) + 2.0, MID_Y);
    let s = draw(&mut h, RECT, &mut v, &opts);
    assert!(near(v, 50.5), "{v}");
    assert!(s.resp.changed && !s.resp.dragged);
}

#[test]
fn the_release_frame_takes_the_pointer_s_last_move() {
    let mut v = 0.0;
    let mut h = start(&mut v);
    h.input.pointer_to(at(0.2), MID_Y);
    frame(&mut h, &mut v);
    h.input.press(P);
    frame(&mut h, &mut v);
    h.input.pointer_to(at(0.5), MID_Y);
    frame(&mut h, &mut v);
    assert_eq!(v, 50.0);
    // The move and the release arrive in one frame.
    h.input.pointer_to(at(0.7), MID_Y).release(P);
    let s = frame(&mut h, &mut v);
    assert!(!s.resp.pressed);
    assert_eq!(v, 70.0);
    assert!(near(knob_x(&s.scene), at(0.7)));
}

/// A slider on a track wider than its viewport, scrolled both ways, so the clip cuts the track's
/// left end: `Response.rect` then starts at the viewport, and only the transformed track maps.
fn scrolled_drag() -> (f32, Response) {
    let opts = ScrollOptions {
        extent: Some((1000.0, 1000.0)),
        ..ScrollOptions::default()
    };
    let area = Rect::new(0.0, 0.0, 300.0, 200.0);
    // Content at (100, 300), scrolled by (150, 250): on screen at x = -50, y = 50, 400 wide.
    let content = Rect::new(100.0, 300.0, 400.0, 24.0);
    let mut h = UiHarness::new(WIN);
    let mut v = 0.0_f32;
    let mut resp = None;
    let mut run = |h: &mut UiHarness, ms: u64, v: &mut f32| {
        h.at_ms(ms).frame(|ui| {
            ui.scroll_area("area", area, &opts, |ui, _| {
                resp = Some(ui.slider("s", content, v, &OPTS));
                ((), Extent::new(1000.0, 1000.0))
            });
        });
    };
    h.input.pointer_to(100.0, 100.0);
    run(&mut h, 0, &mut v);
    h.input.scroll_pixels(-150.0, -250.0);
    run(&mut h, 5, &mut v);
    run(&mut h, 2000, &mut v);
    // Window x 150 is 200 px along the track: half. The clipped rect would read 150 / 354.
    h.input.pointer_to(150.0, 62.0);
    run(&mut h, 2010, &mut v);
    h.input.press(P);
    run(&mut h, 2020, &mut v);
    assert_eq!(v, 50.0, "the press");
    // 270 is 320 along: 0.8.
    h.input.pointer_to(270.0, 62.0);
    run(&mut h, 2030, &mut v);
    (v, resp.expect("frame ran"))
}

#[test]
fn a_drag_in_a_scrolled_area_maps_through_the_transform_not_the_clipped_rect() {
    let (v, r) = scrolled_drag();
    assert_eq!(v, 80.0);
    assert_eq!(
        r.rect.x, 0.0,
        "the clip cut the hit rect, so this test can tell"
    );
}

#[test]
fn a_key_step_tweens_to_exactly_the_value_monotonically_within_fast_and_a_frame() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    h.at_ms(1000);
    frame(&mut h, &mut v);
    key(&mut h, NamedKey::ArrowRight);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 50.0, "the value is there at once");
    assert_eq!(s.repaint, Repaint::Now);
    let mut xs = vec![knob_x(&s.scene)];
    assert!(near(xs[0], at(0.4)), "the knob starts where it was");
    for k in 1..=11_u64 {
        h.at_ms(1000 + (k * 1000 + 30) / 60);
        xs.push(knob_x(&frame(&mut h, &mut v).scene));
    }
    assert!(xs.windows(2).all(|w| w[1] >= w[0]), "monotonic: {xs:?}");
    assert!(xs.iter().any(|&x| x > at(0.4) + 1.0 && x < at(0.5) - 1.0));
    // 180 ms and a frame: sample 11 lands at 183 ms.
    assert_eq!(*xs.last().expect("samples"), at(0.5), "ends exactly");
    assert_eq!(frame(&mut h, &mut v).repaint, Repaint::Idle);
}

#[test]
fn a_key_step_during_the_tween_retargets_from_the_current_position() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    h.at_ms(1000);
    frame(&mut h, &mut v);
    key(&mut h, NamedKey::ArrowRight);
    frame(&mut h, &mut v);
    h.at_ms(1090);
    let mid = knob_x(&frame(&mut h, &mut v).scene);
    assert!(mid > at(0.4) + 1.0 && mid < at(0.5) - 1.0, "{mid}");
    h.at_ms(1090);
    key(&mut h, NamedKey::ArrowRight);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 60.0);
    assert!(near(knob_x(&s.scene), mid), "no jump on the retarget");
    h.at_ms(1100);
    let after = knob_x(&frame(&mut h, &mut v).scene);
    assert!(after > mid && after < at(0.6), "{after}");
    h.at_ms(1500);
    assert_eq!(knob_x(&frame(&mut h, &mut v).scene), at(0.6));
}

#[test]
fn increment_and_decrement_step_and_tween_like_the_keys_with_no_focus() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    h.at_ms(1000);
    access(&mut h, Action::Increment, None);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 50.0);
    assert!(s.resp.changed);
    assert_eq!(s.repaint, Repaint::Now);
    assert!(!s.resp.focus_visible, "an action turns no ring on");
    assert!(near(knob_x(&s.scene), at(0.4)));
    h.at_ms(1090);
    let x = knob_x(&frame(&mut h, &mut v).scene);
    assert!(x > at(0.4) && x < at(0.5), "in flight: {x}");
    h.at_ms(2000);
    access(&mut h, Action::Decrement, None);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 40.0);
    assert!(s.resp.changed);
}

#[test]
fn set_value_settles_on_a_stop_and_snaps() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    h.at_ms(1000);
    access(&mut h, Action::SetValue, Some(AccessValue::Number(73.0)));
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 70.0);
    assert!(s.resp.changed);
    assert!(near(knob_x(&s.scene), at(0.7)), "a jump, not a step");
    h.at_ms(1010);
    assert_eq!(frame(&mut h, &mut v).repaint, Repaint::Idle);

    for (ask, want) in [
        (f64::INFINITY, 100.0),
        (f64::NEG_INFINITY, 0.0),
        (1e300, 100.0),
    ] {
        access(&mut h, Action::SetValue, Some(AccessValue::Number(ask)));
        frame(&mut h, &mut v);
        assert_eq!(v, want, "{ask}");
    }
    v = 30.0;
    frame(&mut h, &mut v);
    access(
        &mut h,
        Action::SetValue,
        Some(AccessValue::Number(f64::NAN)),
    );
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 30.0);
    assert!(!s.resp.changed);
    access(
        &mut h,
        Action::SetValue,
        Some(AccessValue::Text("70".to_string())),
    );
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 30.0);
    assert!(!s.resp.changed);
}

#[test]
fn an_app_driven_change_snaps_even_during_a_tween() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    h.at_ms(1000);
    v = 80.0;
    let s = frame(&mut h, &mut v);
    assert_eq!(knob_x(&s.scene), at(0.8));
    assert!(!s.resp.changed, "the widget did not change it");
    assert_eq!(s.repaint, Repaint::Idle);

    focus(&mut h, &mut v);
    h.at_ms(2000);
    key(&mut h, NamedKey::ArrowLeft);
    frame(&mut h, &mut v);
    h.at_ms(2090);
    v = 10.0;
    let s = frame(&mut h, &mut v);
    assert_eq!(knob_x(&s.scene), at(0.1), "the tween is gone");
    assert_eq!(s.repaint, Repaint::Idle);
}

#[test]
fn a_changed_range_snaps_the_knob_like_a_changed_value() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    h.at_ms(1000);
    let opts = SliderOptions { max: 80.0, ..OPTS };
    let s = draw(&mut h, RECT, &mut v, &opts);
    assert_eq!(knob_x(&s.scene), at(0.5));
    assert_eq!(s.repaint, Repaint::Idle);
    assert_eq!(v, 40.0);
}

#[test]
fn an_app_change_and_a_key_step_in_one_frame_tween_from_the_app_s_value() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    h.at_ms(1000);
    v = 80.0;
    key(&mut h, NamedKey::ArrowLeft);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 70.0);
    assert!(near(knob_x(&s.scene), at(0.8)));
    assert_eq!(s.repaint, Repaint::Now);
}

#[test]
fn a_press_during_a_tween_ends_it_even_when_the_value_does_not_change() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    h.input.pointer_to(at(0.5), MID_Y);
    frame(&mut h, &mut v);
    h.at_ms(1000);
    access(&mut h, Action::Increment, None);
    frame(&mut h, &mut v);
    assert_eq!(v, 50.0);
    h.at_ms(1090).input.press(P);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 50.0, "the press lands on the value");
    assert!(!s.resp.changed);
    assert_eq!(knob_x(&s.scene), at(0.5), "the knob equals the value");
    h.at_ms(1100);
    assert_eq!(
        frame(&mut h, &mut v).repaint,
        Repaint::Idle,
        "no transition"
    );
}

#[test]
fn while_pressed_keys_actions_and_set_value_are_dropped() {
    let mut v = 50.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    h.input.pointer_to(at(0.5), MID_Y);
    frame(&mut h, &mut v);
    h.input.press(P);
    frame(&mut h, &mut v);
    h.at_ms(1000);
    key(&mut h, NamedKey::ArrowRight);
    access(&mut h, Action::Increment, None);
    access(&mut h, Action::SetValue, Some(AccessValue::Number(10.0)));
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 50.0);
    assert!(!s.resp.changed);
    assert_eq!(knob_x(&s.scene), at(0.5));
}

#[test]
fn a_key_after_a_drag_starts_from_the_dragged_value() {
    let mut v = 0.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    h.input.pointer_to(at(0.1), MID_Y);
    frame(&mut h, &mut v);
    h.input.press(P);
    frame(&mut h, &mut v);
    h.input.pointer_to(at(0.6), MID_Y);
    frame(&mut h, &mut v);
    h.input.release(P);
    frame(&mut h, &mut v);
    assert_eq!(v, 60.0);
    h.at_ms(1000);
    key(&mut h, NamedKey::ArrowRight);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 70.0);
    assert!(near(knob_x(&s.scene), at(0.6)), "and tweens from there");
}

#[test]
fn reduced_and_off_snap_a_key_step() {
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let mut v = 40.0;
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        frame(&mut h, &mut v);
        focus(&mut h, &mut v);
        h.at_ms(1000);
        key(&mut h, NamedKey::ArrowRight);
        let s = frame(&mut h, &mut v);
        assert_eq!(knob_x(&s.scene), at(0.5), "{tier:?}");
        h.at_ms(1001);
        let s = frame(&mut h, &mut v);
        assert_eq!(knob_x(&s.scene), at(0.5), "{tier:?}");
    }
}

#[test]
fn disabled_changes_nothing() {
    let mut v = 40.0;
    let mut h = UiHarness::new(WIN);
    let run = |h: &mut UiHarness, v: &mut f32| {
        let mut resp = None;
        let (scene, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                resp = Some(ui.slider("s", RECT, v, &OPTS));
            });
        });
        (scene, resp.expect("frame ran"), out.tree)
    };
    run(&mut h, &mut v);
    h.input.pointer_to(at(0.8), MID_Y);
    run(&mut h, &mut v);
    h.input.press(P);
    run(&mut h, &mut v);
    h.input.pointer_to(at(0.9), MID_Y);
    run(&mut h, &mut v);
    h.input.release(P);
    run(&mut h, &mut v);
    key(&mut h, NamedKey::Tab);
    run(&mut h, &mut v);
    key(&mut h, NamedKey::ArrowRight);
    access(&mut h, Action::Increment, None);
    access(&mut h, Action::SetValue, Some(AccessValue::Number(90.0)));
    let (scene, r, tree) = run(&mut h, &mut v);
    assert_eq!(v, 40.0);
    assert!(!r.changed && !r.hovered && !r.pressed);
    let n = tree.get(id()).expect("a node");
    assert!(n.states.contains(State::Disabled));
    assert!(!n.states.contains(State::Focusable));
    for a in [Action::Increment, Action::Decrement, Action::SetValue] {
        assert!(!n.actions.contains(a), "{a:?}");
    }
    // Opacity 0.4 through `with_enabled`: the knob's alpha is not the enabled knob's.
    let g = &scene.layers[0].geometry;
    assert!(g.under.iter().chain(&g.over).all(|s| s.color[3] < 0.5));
}

#[test]
fn an_off_step_value_survives_idle_frames_and_hover_until_the_user_acts() {
    let mut v = 33.3;
    let mut h = start(&mut v);
    for ms in [10, 20] {
        h.at_ms(ms);
        let s = frame(&mut h, &mut v);
        assert_eq!(v, 33.3);
        assert!(!s.resp.changed);
    }
    h.at_ms(30).input.pointer_to(at(0.333), MID_Y);
    frame(&mut h, &mut v);
    h.at_ms(500);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 33.3);
    assert!(s.resp.hovered && !s.resp.changed);
    assert!(near(knob_x(&s.scene), at(0.333)), "drawn where it is");

    // A value outside the range stays in hand too, and is drawn at the end.
    let mut far = 150.0;
    let mut h = start(&mut far);
    let s = frame(&mut h, &mut far);
    assert_eq!(far, 150.0);
    assert_eq!(knob_x(&s.scene), at(1.0));
}

#[test]
fn a_key_from_an_off_step_value_lands_on_the_stop_beside_it() {
    for (held, k, want) in [
        (33.3, NamedKey::ArrowRight, 40.0),
        (33.3, NamedKey::ArrowLeft, 30.0),
        (150.0, NamedKey::ArrowRight, 100.0),
        (150.0, NamedKey::ArrowLeft, 90.0),
        (-5.0, NamedKey::ArrowRight, 10.0),
        (-5.0, NamedKey::ArrowLeft, 0.0),
    ] {
        let mut v = held;
        let mut h = start(&mut v);
        focus(&mut h, &mut v);
        key(&mut h, k);
        let s = frame(&mut h, &mut v);
        assert_eq!(v, want, "{held} {k:?}");
        assert!(s.resp.changed);
    }
}

#[test]
fn max_stays_reachable_when_the_step_does_not_divide_the_range() {
    let opts = SliderOptions {
        min: 0.0,
        max: 10.0,
        step: 3.0,
        name: "Uneven",
    };
    let run = |h: &mut UiHarness, v: &mut f32| draw(h, RECT, v, &opts);
    let mut v = 0.0;
    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut v);
    key(&mut h, NamedKey::Tab);
    run(&mut h, &mut v);
    run(&mut h, &mut v);
    let mut up = vec![];
    for _ in 0..5 {
        key(&mut h, NamedKey::ArrowRight);
        run(&mut h, &mut v);
        up.push(v);
    }
    assert_eq!(up, [3.0, 6.0, 9.0, 10.0, 10.0]);
    let mut down = vec![];
    for _ in 0..5 {
        key(&mut h, NamedKey::ArrowLeft);
        run(&mut h, &mut v);
        down.push(v);
    }
    assert_eq!(down, [9.0, 6.0, 3.0, 0.0, 0.0]);

    // By pointer: the nearest stop, with `max` one of them (9.8 is nearer 10 than 9).
    for (f, want) in [
        (0.98, 10.0),
        (0.94, 9.0),
        (1.0, 10.0),
        (0.14, 0.0),
        (0.16, 3.0),
    ] {
        let mut v = 5.0;
        let mut h = UiHarness::new(WIN);
        run(&mut h, &mut v);
        h.input.pointer_to(at(f), MID_Y);
        run(&mut h, &mut v);
        h.input.press(P);
        run(&mut h, &mut v);
        assert_eq!(v, want, "{f}");
    }

    let mut v = 5.0;
    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut v);
    access(&mut h, Action::SetValue, Some(AccessValue::Number(9.7)));
    run(&mut h, &mut v);
    assert_eq!(v, 10.0);
}

#[test]
fn a_step_wider_than_the_range_leaves_min_and_max() {
    let opts = SliderOptions {
        min: 2.0,
        max: 3.0,
        step: 50.0,
        name: "Wide",
    };
    let mut v = 2.0;
    let mut h = UiHarness::new(WIN);
    draw(&mut h, RECT, &mut v, &opts);
    access(&mut h, Action::Increment, None);
    draw(&mut h, RECT, &mut v, &opts);
    assert_eq!(v, 3.0);
    access(&mut h, Action::Decrement, None);
    draw(&mut h, RECT, &mut v, &opts);
    assert_eq!(v, 2.0);
}

#[test]
fn every_pointer_result_is_in_range_and_on_a_step_from_min() {
    for (min, max, step) in [
        (0.0_f32, 1.0_f32, 0.1_f32),
        (-1.0, 1.0, 0.3),
        (10.0, 11.0, 0.25),
        (-7.5, 2.5, 3.3),
        (0.0, 1.2, 0.4),
    ] {
        let opts = SliderOptions {
            min,
            max,
            step,
            name: "Sweep",
        };
        let mut v = min;
        let mut h = UiHarness::new(WIN);
        draw(&mut h, RECT, &mut v, &opts);
        h.input.pointer_to(at(0.0), MID_Y);
        draw(&mut h, RECT, &mut v, &opts);
        h.input.press(P);
        draw(&mut h, RECT, &mut v, &opts);
        for i in 0..=400 {
            h.input.pointer_to(RECT.x + i as f32, MID_Y);
            draw(&mut h, RECT, &mut v, &opts);
            assert!((min..=max).contains(&v), "{v} in {min}..{max}");
            let k = (f64::from(v) - f64::from(min)) / f64::from(step);
            let on_step = (k - k.round()).abs() < 1e-4;
            assert!(on_step || v == max, "{v} off the {step} grid from {min}");
        }
        assert_eq!(v, max);
    }
}

#[test]
fn a_step_dividing_the_range_only_up_to_rounding_adds_no_stop_below_max() {
    // In f32, 1.2 / 0.4 is not exactly 3: four key steps would otherwise be needed.
    let opts = SliderOptions {
        min: 0.0,
        max: 1.2,
        step: 0.4,
        name: "Round",
    };
    let mut v = 0.0;
    let mut h = UiHarness::new(WIN);
    draw(&mut h, RECT, &mut v, &opts);
    for _ in 0..3 {
        access(&mut h, Action::Increment, None);
        draw(&mut h, RECT, &mut v, &opts);
    }
    assert_eq!(v, 1.2);
    let mut seen = vec![];
    for _ in 0..3 {
        access(&mut h, Action::Decrement, None);
        draw(&mut h, RECT, &mut v, &opts);
        seen.push(v);
    }
    assert_eq!(seen.last(), Some(&0.0), "{seen:?}");
}

#[test]
fn no_step_is_continuous_with_a_key_step_of_a_hundredth_of_the_range() {
    for step in [0.0, -3.0] {
        let opts = SliderOptions {
            min: 0.0,
            max: 50.0,
            step,
            name: "Free",
        };
        let mut v = 20.0;
        let mut h = UiHarness::new(WIN);
        draw(&mut h, RECT, &mut v, &opts);
        access(&mut h, Action::Increment, None);
        draw(&mut h, RECT, &mut v, &opts);
        assert!(near(v, 20.5), "{v}");
        access(&mut h, Action::Decrement, None);
        draw(&mut h, RECT, &mut v, &opts);
        assert!(near(v, 20.0), "{v}");
        h.input.pointer_to(at(0.123), MID_Y);
        draw(&mut h, RECT, &mut v, &opts);
        h.input.press(P);
        draw(&mut h, RECT, &mut v, &opts);
        assert!(near(v, 6.15), "unsnapped: {v}");
    }
}

#[test]
fn the_widest_f32_range_does_not_overflow() {
    let opts = SliderOptions {
        min: -f32::MAX,
        max: f32::MAX,
        step: 0.0,
        name: "Wide",
    };
    let mut v = 0.0;
    let mut h = UiHarness::new(WIN);
    let s = draw(&mut h, RECT, &mut v, &opts);
    assert_eq!(knob_x(&s.scene), at(0.5));
    h.input.pointer_to(at(0.75), MID_Y);
    draw(&mut h, RECT, &mut v, &opts);
    h.input.press(P);
    draw(&mut h, RECT, &mut v, &opts);
    assert!(v.is_finite() && v > 0.0, "{v}");
}

#[test]
fn degenerate_input_changes_nothing_and_draws_the_knob_at_the_start() {
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    let cases: [(&str, Rect, f32, f32, f32, f32); 12] = [
        ("min == max", RECT, 5.0, 5.0, 5.0, 1.0),
        ("min > max", RECT, 5.0, 9.0, 1.0, 1.0),
        ("NaN value", RECT, nan, 0.0, 100.0, 10.0),
        ("infinite value", RECT, inf, 0.0, 100.0, 10.0),
        ("NaN min", RECT, 5.0, nan, 100.0, 10.0),
        ("NaN max", RECT, 5.0, 0.0, nan, 10.0),
        ("NaN step", RECT, 5.0, 0.0, 100.0, nan),
        ("infinite min", RECT, 5.0, -inf, 100.0, 10.0),
        ("infinite max", RECT, 5.0, 0.0, inf, 10.0),
        ("infinite step", RECT, 5.0, 0.0, 100.0, inf),
        ("zero width", Rect { w: 0.0, ..RECT }, 5.0, 0.0, 100.0, 10.0),
        (
            "negative width",
            Rect { w: -9.0, ..RECT },
            5.0,
            0.0,
            100.0,
            10.0,
        ),
    ];
    for (what, rect, held, min, max, step) in cases {
        let opts = SliderOptions {
            min,
            max,
            step,
            name: "Broken",
        };
        assert_eq!(fraction(rect, held, &opts), 0.0, "{what}");
        let same = |v: f32| v == held || (v.is_nan() && held.is_nan());
        let mut v = held;
        let mut h = UiHarness::new(WIN);
        let s = draw(&mut h, rect, &mut v, &opts);
        assert_eq!(knob_x(&s.scene), rect.x, "{what}");
        h.input.pointer_to(rect.x, MID_Y);
        draw(&mut h, rect, &mut v, &opts);
        h.input.press(P);
        let s = draw(&mut h, rect, &mut v, &opts);
        assert!(same(v) && !s.resp.changed, "press: {what}");
        h.input.pointer_to(rect.x + 2.0, MID_Y);
        let s = draw(&mut h, rect, &mut v, &opts);
        assert!(same(v) && !s.resp.changed, "drag: {what}");
        h.input.release(P);
        draw(&mut h, rect, &mut v, &opts);
        access(&mut h, Action::Increment, None);
        let s = draw(&mut h, rect, &mut v, &opts);
        assert!(same(v) && !s.resp.changed, "step: {what}");
        access(&mut h, Action::SetValue, Some(AccessValue::Number(3.0)));
        let s = draw(&mut h, rect, &mut v, &opts);
        assert!(same(v) && !s.resp.changed, "set: {what}");
        assert_eq!(knob_x(&s.scene), rect.x, "{what}");
        let n = s.tree.get(id()).expect("a node");
        match n.value {
            Some(Value::Number {
                value,
                min,
                max,
                step,
            }) => assert!(
                [value, min, max, step].iter().all(|x| x.is_finite()),
                "{what}"
            ),
            _ => panic!("no number: {what}"),
        }
    }
}

#[test]
fn hover_grows_the_knob_and_a_held_knob_keeps_the_size_off_the_track() {
    let mut v = 50.0;
    let mut h = start(&mut v);
    assert_eq!(knob_shape(&frame(&mut h, &mut v).scene).rect[2], 12.0);
    h.at_ms(10).input.pointer_to(at(0.5), MID_Y);
    frame(&mut h, &mut v);
    h.at_ms(500);
    assert_eq!(knob_shape(&frame(&mut h, &mut v).scene).rect[2], 14.0);
    h.at_ms(510).input.press(P);
    frame(&mut h, &mut v);
    h.input.pointer_to(at(0.5), 280.0);
    h.at_ms(600);
    let s = frame(&mut h, &mut v);
    assert!(!s.resp.hovered, "the pointer is off the hit area");
    assert_eq!(knob_shape(&s.scene).rect[2], 14.0);
}

#[test]
fn one_tab_stop_with_a_ring_around_the_knob() {
    let mut v = 50.0;
    let mut h = start(&mut v);
    focus(&mut h, &mut v);
    h.at_ms(500);
    let s = frame(&mut h, &mut v);
    assert!(s.resp.focused && s.resp.focus_visible);
    // The ring's stroke is the only shape in `fg_secondary`: outset 3 from the 12 px knob.
    let g = &s.scene.layers[0].geometry;
    let ring = g
        .under
        .iter()
        .chain(&g.over)
        .find(|s| s.color == premul(DARK.fg_secondary))
        .expect("a ring");
    assert_eq!(ring.rect[2], 18.0);
    assert_eq!(ring.rect[0] + ring.rect[2] / 2.0, at(0.5));
    // A press focuses too, with no ring.
    let mut v = 50.0;
    let mut h = start(&mut v);
    h.input.pointer_to(at(0.5), MID_Y);
    frame(&mut h, &mut v);
    h.input.press(P);
    frame(&mut h, &mut v);
    let s = frame(&mut h, &mut v);
    assert!(s.resp.focused && !s.resp.focus_visible);
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&VALUE, &theme.motion, tier, 0.2_f32, 0.3)
            .unwrap_or_else(|e| panic!("value {tier:?}: {e}"));
        conformance(&HOVER_SIZE, &theme.motion, tier, 12.0_f32, 14.0)
            .unwrap_or_else(|e| panic!("size {tier:?}: {e}"));
        conformance(&FOCUS, &theme.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("focus {tier:?}: {e}"));
    }
    assert!(theme.motion.resolve(&VALUE, MotionTier::Reduced).is_none());
    assert!(theme.motion.resolve(&VALUE, MotionTier::Off).is_none());
    assert!(
        theme
            .motion
            .resolve(&HOVER_SIZE, MotionTier::Reduced)
            .is_none()
    );
    assert!(theme.motion.resolve(&FOCUS, MotionTier::Reduced).is_some());
}

/// `state` painted through the public look at scale 2.
fn probe(state: SliderState) -> Scene {
    let mut text = FakeText;
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut p = Painter::<u8>::new(2.0, win, &mut text, None, DARK);
    let look = SliderLook::of(&DARK, state);
    paint(&mut p, RECT, &look);
    p.scene
}

#[test]
fn pixel_probes_at_scale_2() {
    let state = SliderState {
        fraction: 0.25,
        ..SliderState::default()
    };
    let scene = probe(state);
    let t = track(RECT);
    let g = &scene.layers[0].geometry;
    let shape = |c| {
        *g.under
            .iter()
            .chain(&g.over)
            .find(|s| s.color == premul(c))
            .expect("a shape")
    };
    let bar = shape(DARK.surface_overlay);
    assert_eq!(bar.rect, [t.x * 2.0, t.y * 2.0, t.w * 2.0, 8.0]);
    assert_eq!(bar.radii, [8.0; 4]);
    let fill = shape(DARK.accent);
    assert_eq!(fill.rect, [t.x * 2.0, t.y * 2.0, t.w * 2.0 * 0.25, 8.0]);
    let knob = knob_shape(&scene);
    assert_eq!((knob.rect[2], knob.rect[3]), (24.0, 24.0));
    assert_eq!(knob.radii, [12.0; 4]);
    assert_eq!(knob.rect[0] + 12.0, at(0.25) * 2.0, "centre at the value");
    assert_eq!(knob.rect[1] + 12.0, MID_Y * 2.0, "centred on the track");

    let hovered = probe(SliderState {
        hovered: true,
        ..state
    });
    let knob = knob_shape(&hovered);
    assert_eq!((knob.rect[2], knob.rect[3]), (28.0, 28.0));
    assert_eq!(knob.rect[0] + 14.0, at(0.25) * 2.0);
}

#[test]
fn no_fill_at_the_start_and_a_full_fill_at_the_end() {
    let colored = |s: &Scene| {
        let g = &s.layers[0].geometry;
        g.under
            .iter()
            .chain(&g.over)
            .filter(|s| s.color == premul(DARK.accent))
            .count()
    };
    assert_eq!(colored(&probe(SliderState::default())), 0);
    let end = probe(SliderState {
        fraction: 1.0,
        ..SliderState::default()
    });
    assert_eq!(colored(&end), 1);
}

#[test]
fn the_node_is_a_slider_whose_value_follows_and_whose_steps_arrive_as_actions() {
    let mut v = 40.0;
    let mut h = start(&mut v);
    let s = frame(&mut h, &mut v);
    let n = s.tree.get(id()).expect("a node");
    assert_eq!(n.role, Role::Slider);
    assert_eq!(n.name, "Volume");
    assert_eq!(
        n.value,
        Some(Value::Number {
            value: 40.0,
            min: 0.0,
            max: 100.0,
            step: 10.0,
        })
    );
    assert!(n.states.contains(State::Focusable));
    for a in [Action::Increment, Action::Decrement, Action::SetValue] {
        assert!(n.actions.contains(a), "{a:?}");
    }
    assert_eq!(n.bounds, track(RECT));

    access(&mut h, Action::Increment, None);
    let s = frame(&mut h, &mut v);
    let n = s.tree.get(id()).expect("a node");
    assert!(matches!(n.value, Some(Value::Number { value, .. }) if value == 50.0));
    access(&mut h, Action::Decrement, None);
    access(&mut h, Action::Decrement, None);
    let s = frame(&mut h, &mut v);
    assert_eq!(v, 30.0);
    let n = s.tree.get(id()).expect("a node");
    assert!(matches!(n.value, Some(Value::Number { value, .. }) if value == 30.0));
}
