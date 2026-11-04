//! The stepper (C4 retrofit, `widget.stepper` acceptance): rest identity with the Painter API,
//! motion conformance, the button hover fill sampled at 60 Hz, steps and set-value, probes, node.

use lune_ui_core::access::{AccessAction, AccessValue, Action, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::stepper::{
    FOCUS, HOVER, StepRange, StepperResponse, minus_rect, plus_rect, settle, stepper,
};

const WIN: (f32, f32) = (400.0, 300.0);
/// The control at (20, 20), 140 by 32, as the `controls` scene draws it.
const RECT: Rect = Rect::new(20.0, 20.0, 140.0, 32.0);
const RANGE: StepRange = StepRange {
    min: 0.0,
    max: 99.0,
    step: 1.0,
};
const MINUS: (f32, f32) = (116.0, 36.0);
const PLUS: (f32, f32) = (144.0, 36.0);

fn id() -> WidgetId {
    WidgetId::ROOT.with("s")
}

/// The Painter-API scene: `minus` and `plus` are the hit values, `hover` the hovered one.
fn old_scene(hover: Option<u8>) -> Scene {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, hover, DARK);
    stepper(&mut p, RECT, 12, 1, 2);
    p.scene
}

fn frame(h: &mut UiHarness, value: u32) -> (Scene, StepperResponse, Repaint) {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| resp = Some(ui.stepper("s", RECT, value, RANGE, "Count")));
    (scene, resp.expect("frame ran"), out.repaint)
}

fn settled_scene(hover: Option<(f32, f32)>) -> Scene {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 12);
    if let Some((x, y)) = hover {
        h.input.pointer_to(x, y);
        h.at_ms(10);
        frame(&mut h, 12);
    }
    h.at_ms(1000);
    let (scene, _, repaint) = frame(&mut h, 12);
    assert_eq!(repaint, Repaint::Idle);
    scene
}

#[test]
fn at_rest_the_ui_stepper_draws_exactly_what_the_painter_stepper_does() {
    assert_eq!(settled_scene(None), old_scene(None));
    assert_eq!(settled_scene(Some(MINUS)), old_scene(Some(1)));
    assert_eq!(settled_scene(Some(PLUS)), old_scene(Some(2)));
}

#[test]
fn at_rest_no_fill_is_transparent() {
    let scene = settled_scene(None);
    let g = &scene.layers[0].geometry;
    assert!(g.under.iter().chain(&g.over).all(|s| s.color[3] > 0.0));
    // Only the input box's fill and border: no button fill at rest.
    assert_eq!(g.under.len(), 2);
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    let clear = lune_ui_core::color::with_alpha(DARK.surface_hover, 0.0).map(f32::from);
    let fill = DARK.surface_hover.map(f32::from);
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&FOCUS, &theme.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("focus {tier:?}: {e}"));
        conformance(&HOVER, &theme.motion, tier, clear, fill)
            .unwrap_or_else(|e| panic!("hover {tier:?}: {e}"));
    }
    assert!(theme.motion.resolve(&HOVER, MotionTier::Reduced).is_some());
    assert!(theme.motion.resolve(&FOCUS, MotionTier::Reduced).is_some());
}

/// The minus button's fill alpha: the only shape with the button's rect.
fn minus_fill(scene: &Scene) -> f32 {
    let m = minus_rect(RECT);
    let rect = [m.x, m.y, m.w, m.h];
    scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|s: &&Shape| s.rect == rect)
        .map_or(0.0, |s| s.color[3])
}

#[test]
fn the_minus_fill_rises_monotonically_and_settles_within_120_ms_and_a_frame() {
    for tier in [MotionTier::Full, MotionTier::Reduced] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        frame(&mut h, 12);
        h.at_ms(500);
        h.input.pointer_to(MINUS.0, MINUS.1);
        let (scene, _, _) = frame(&mut h, 12);
        // The fill is premultiplied: `surface_hover` at its alpha, so settled is its alpha / 255.
        let settled = premul(DARK.surface_hover)[3];
        let mut xs = vec![(0_u64, minus_fill(&scene))];
        for k in 1..=10_u64 {
            let dt = (k * 1000 + 30) / 60;
            h.at_ms(500 + dt);
            let (scene, _, _) = frame(&mut h, 12);
            xs.push((dt, minus_fill(&scene)));
        }
        assert!(
            xs.windows(2).all(|w| w[1].1 >= w[0].1),
            "{tier:?} monotonic: {xs:?}"
        );
        // Reduced keeps the fill tween (the row's Reduced rule is Keep).
        assert!(
            xs.iter().any(|&(_, a)| a > 0.0 && a < 1.0),
            "{tier:?} tween: {xs:?}"
        );
        for &(dt, a) in &xs {
            if dt >= 120 + 17 {
                assert_eq!(a, settled, "{tier:?} settled at {dt} ms");
            }
        }
        // Exact at the end: the settled hover fill.
        assert_eq!(xs.last().expect("samples").1, settled);
    }
}

#[test]
fn settle_clamps_snaps_and_never_panics() {
    let r = StepRange {
        min: 0.0,
        max: 10.0,
        step: 3.0,
    };
    assert_eq!(settle(7.4, r), Some(6.0));
    assert_eq!(settle(-4.0, r), Some(0.0));
    // The clamp lands on 10, which snaps to 9 (3 x 3); the snap stays within the range.
    assert_eq!(settle(f64::INFINITY, r), Some(9.0));
    assert_eq!(settle(f64::NEG_INFINITY, r), Some(0.0));
    assert_eq!(settle(f64::NAN, r), None);
    let bad = StepRange {
        min: 0.0,
        max: 10.0,
        step: f64::NAN,
    };
    assert_eq!(settle(4.2, bad), Some(4.2));
}

#[test]
fn clicks_and_arrow_keys_report_one_step_each() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 12);
    h.input
        .pointer_to(PLUS.0, PLUS.1)
        .click(PointerButton::Primary);
    let (_, r, _) = frame(&mut h, 12);
    assert_eq!(r.step, Some(1));
    let (_, r, _) = frame(&mut h, 12);
    assert_eq!(r.step, None, "one step per click");
    h.input
        .pointer_to(MINUS.0, MINUS.1)
        .click(PointerButton::Primary);
    let (_, r, _) = frame(&mut h, 12);
    assert_eq!(r.step, Some(-1));
    // Tab focuses the stepper; Up and Down then step once each.
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    frame(&mut h, 12);
    h.input.key_down(Key::Named(NamedKey::ArrowUp), None);
    let (_, r, _) = frame(&mut h, 12);
    assert_eq!(r.step, Some(1));
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    let (_, r, _) = frame(&mut h, 12);
    assert_eq!(r.step, Some(-1));
}

#[test]
fn the_access_increment_and_decrement_step_and_set_value_is_settled() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 12);
    for (action, expect) in [(Action::Increment, 1), (Action::Decrement, -1)] {
        h.input.access(AccessAction {
            target: id(),
            action,
            value: None,
        });
        let (_, r, _) = frame(&mut h, 12);
        assert_eq!(r.step, Some(expect));
    }
    h.input.access(AccessAction {
        target: id(),
        action: Action::SetValue,
        value: Some(AccessValue::Number(99.4)),
    });
    let (_, r, _) = frame(&mut h, 12);
    assert_eq!(r.set, Some(99.0));
    h.input.access(AccessAction {
        target: id(),
        action: Action::SetValue,
        value: Some(AccessValue::Number(f64::NAN)),
    });
    let (_, r, _) = frame(&mut h, 12);
    assert_eq!(r.set, None);
}

#[test]
fn a_disabled_stepper_reports_nothing() {
    let mut h = UiHarness::new(WIN);
    let run = |h: &mut UiHarness| {
        let mut resp = None;
        h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                resp = Some(ui.stepper("s", RECT, 12, RANGE, "Count"));
            });
        });
        resp.expect("ran")
    };
    run(&mut h);
    h.input
        .pointer_to(PLUS.0, PLUS.1)
        .click(PointerButton::Primary);
    let r = run(&mut h);
    assert!(!r.response.enabled);
    assert_eq!((r.step, r.set), (None, None));
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    frame(&mut h, 12);
    h.input.pointer_to(MINUS.0, MINUS.1);
    h.at_ms(10);
    frame(&mut h, 12);
    h.at_ms(1000);
    let (scene, _, _) = frame(&mut h, 12);
    let g = &scene.layers[0].geometry;
    // The input box: 280 by 64 physical, radius 12, border `border`.
    let boxed = g
        .under
        .iter()
        .find(|s| s.rect == [40.0, 40.0, 280.0, 64.0] && s.color == premul(DARK.border))
        .expect("the input box border");
    assert_eq!(boxed.radii, [12.0; 4]);
    // The hovered minus box: 48 by 48 physical, filled with the hover fill.
    let minus = g
        .under
        .iter()
        .find(|s| s.rect == [208.0, 48.0, 48.0, 48.0])
        .expect("the minus fill");
    assert_eq!(minus.color, premul(DARK.surface_hover));
    // The value text starts 10 logical px in (20 physical): texts are logical, the scale is apart.
    let value = scene.layers[0].texts.first().expect("value text");
    assert_eq!(value.x - RECT.x, 10.0);
}

#[test]
fn the_node_is_a_spin_button_whose_value_follows_the_app() {
    let mut h = UiHarness::new(WIN);
    let (_, out) = h.frame(|ui| {
        ui.stepper("s", RECT, 12, RANGE, "Count");
    });
    let node = out.tree.get(id()).expect("node");
    assert_eq!((node.role, node.name.as_str()), (Role::SpinButton, "Count"));
    assert_eq!(
        node.value,
        Some(Value::Number {
            value: 12.0,
            min: 0.0,
            max: 99.0,
            step: 1.0,
        })
    );
    assert!(node.states.contains(State::Focusable));
    assert!(node.actions.contains(Action::Increment));
    assert!(node.actions.contains(Action::Decrement));
    assert!(node.actions.contains(Action::SetValue));
    assert_eq!(plus_rect(RECT).x, RECT.right() - 28.0);
}
