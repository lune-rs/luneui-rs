//! The toggle (C4 retrofit, `widget.toggle` acceptance): rest identity with the Painter API,
//! motion conformance, the knob's slide sampled at 60 Hz, activation, probes, the node.

use lune_ui_core::access::{AccessAction, Action, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::toggle::{FOCUS, HOVER, HOVER_SIZE, MOVE, SELECTION, toggle};

const WIN: (f32, f32) = (400.0, 300.0);
/// Track at (20, 22), 40 by 20; centre (40, 32).
const RECT: Rect = Rect::new(20.0, 20.0, 40.0, 24.0);
const CENTRE: (f32, f32) = (40.0, 32.0);

fn id() -> WidgetId {
    WidgetId::ROOT.with("t")
}

fn old_scene(on: bool, hovered: bool) -> Scene {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    toggle(&mut p, RECT, on, hovered, 1);
    p.scene
}

fn frame(h: &mut UiHarness, on: &mut bool) -> (Scene, Response, Repaint) {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| resp = Some(ui.toggle("t", RECT, on, "Switch")));
    (scene, resp.expect("frame ran"), out.repaint)
}

fn settled_scene(on: bool, hovered: bool) -> Scene {
    let mut h = UiHarness::new(WIN);
    let mut on = on;
    frame(&mut h, &mut on);
    if hovered {
        h.input.pointer_to(CENTRE.0, CENTRE.1);
        h.at_ms(10);
        frame(&mut h, &mut on);
    }
    h.at_ms(1000);
    let (scene, _, repaint) = frame(&mut h, &mut on);
    assert_eq!(repaint, Repaint::Idle);
    scene
}

#[test]
fn at_rest_the_ui_toggle_draws_exactly_what_the_painter_toggle_does() {
    for on in [false, true] {
        for hovered in [false, true] {
            assert_eq!(
                settled_scene(on, hovered),
                old_scene(on, hovered),
                "on={on} hovered={hovered}"
            );
        }
    }
}

#[test]
fn at_rest_no_shape_is_transparent() {
    for on in [false, true] {
        let scene = settled_scene(on, false);
        let g = &scene.layers[0].geometry;
        assert!(g.under.iter().chain(&g.over).all(|s| s.color[3] > 0.0));
        // Off: ring and knob; on: fill and knob.
        assert_eq!(g.under.len(), 2, "on={on}");
    }
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [
            (MOVE, "move"),
            (SELECTION, "selection"),
            (HOVER, "hover"),
            (HOVER_SIZE, "hover size"),
            (FOCUS, "focus"),
        ] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
        let (a, b) = (
            DARK.fg_primary.map(f32::from),
            DARK.on_accent.map(f32::from),
        );
        conformance(&SELECTION, &theme.motion, tier, a, b)
            .unwrap_or_else(|e| panic!("selection colour {tier:?}: {e}"));
    }
    assert!(theme.motion.resolve(&MOVE, MotionTier::Reduced).is_none());
    assert!(
        theme
            .motion
            .resolve(&HOVER_SIZE, MotionTier::Reduced)
            .is_none()
    );
    assert!(theme.motion.resolve(&HOVER, MotionTier::Reduced).is_some());
    assert!(
        theme
            .motion
            .resolve(&SELECTION, MotionTier::Reduced)
            .is_some()
    );
}

/// The knob: the 12 px (hovered 14) wide fill.
fn knob(scene: &Scene) -> Shape {
    *scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|s| s.rect[2] == 12.0 || s.rect[2] == 14.0)
        .expect("a knob")
}

/// A harness with the switch focused by Tab and settled at app time 500 ms.
fn focused(tier: MotionTier) -> (UiHarness, bool) {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let mut on = false;
    frame(&mut h, &mut on);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    let (_, r, _) = frame(&mut h, &mut on);
    assert!(r.focused && r.focus_visible);
    h.at_ms(500);
    let (_, _, repaint) = frame(&mut h, &mut on);
    assert_eq!(repaint, Repaint::Idle);
    (h, on)
}

/// Space at 500 ms, then the knob's x on a 60 Hz grid.
fn slide(tier: MotionTier) -> (Vec<(u64, f32)>, Repaint) {
    let (mut h, mut on) = focused(tier);
    h.input.key_down(Key::Named(NamedKey::Space), None);
    h.at_ms(500);
    let (scene, r, _) = frame(&mut h, &mut on);
    assert!(on && r.changed);
    let mut xs = vec![(0, knob(&scene).rect[0])];
    let mut repaint = Repaint::Now;
    for k in 1..=15_u64 {
        let dt = (k * 1000 + 30) / 60;
        h.at_ms(500 + dt);
        let (scene, _, rp) = frame(&mut h, &mut on);
        xs.push((dt, knob(&scene).rect[0]));
        repaint = rp;
    }
    (xs, repaint)
}

#[test]
fn the_knob_slides_monotonically_and_settles_within_180_ms_and_a_frame() {
    let (xs, repaint) = slide(MotionTier::Full);
    // The track spans 20..60: off x = 20 + 4, on x = 60 - 4 - 12.
    assert_eq!(xs[0].1, 24.0);
    assert!(xs.windows(2).all(|w| w[1].1 >= w[0].1), "monotonic: {xs:?}");
    assert!(
        xs.iter().any(|&(_, x)| x > 24.0 && x < 44.0),
        "moves through: {xs:?}"
    );
    for &(dt, x) in &xs {
        if dt >= 180 + 17 {
            assert_eq!(x, 44.0, "settled at {dt} ms");
        }
    }
    assert_eq!(xs.last().expect("samples").1, 44.0);
    assert_eq!(repaint, Repaint::Idle);
}

#[test]
fn reduced_snaps_the_knob_and_off_snaps_everything() {
    let (xs, _) = slide(MotionTier::Reduced);
    assert!(xs.iter().all(|&(_, x)| x == 44.0), "Reduced: {xs:?}");
    // Off: the first frame after the flip is already the settled scene.
    let (mut h, mut on) = focused(MotionTier::Off);
    h.input.key_down(Key::Named(NamedKey::Space), None);
    h.at_ms(500);
    let (first, _, repaint) = frame(&mut h, &mut on);
    // A consumed key asks for one catch-up frame (D81), which is already still.
    assert_eq!(repaint, Repaint::Now);
    h.at_ms(516);
    let (_, _, repaint) = frame(&mut h, &mut on);
    assert_eq!(repaint, Repaint::Idle);
    h.at_ms(2000);
    let (last, _, _) = frame(&mut h, &mut on);
    assert_eq!(first, last);
}

#[test]
fn click_and_space_flip_once_each_and_mark_changed() {
    let mut h = UiHarness::new(WIN);
    let mut on = false;
    frame(&mut h, &mut on);
    h.input
        .pointer_to(CENTRE.0, CENTRE.1)
        .click(PointerButton::Primary);
    let (_, r, _) = frame(&mut h, &mut on);
    assert!(on && r.changed);
    let (_, r, _) = frame(&mut h, &mut on);
    assert!(on && !r.changed, "one flip per click");
    // The click focused it; Space flips back, once.
    h.input.key_down(Key::Named(NamedKey::Space), None);
    let (_, r, _) = frame(&mut h, &mut on);
    assert!(!on && r.changed);
    let (_, r, _) = frame(&mut h, &mut on);
    assert!(!on && !r.changed);
    h.input.access(AccessAction {
        target: id(),
        action: Action::Default,
        value: None,
    });
    let (_, r, _) = frame(&mut h, &mut on);
    assert!(on && r.changed);
}

#[test]
fn a_disabled_toggle_reports_no_click() {
    let mut h = UiHarness::new(WIN);
    let mut on = false;
    let mut run = |h: &mut UiHarness| {
        let mut resp = None;
        h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                resp = Some(ui.toggle("t", RECT, &mut on, "Switch"));
            });
        });
        resp.expect("ran")
    };
    run(&mut h);
    h.input
        .pointer_to(CENTRE.0, CENTRE.1)
        .click(PointerButton::Primary);
    h.input.key_down(Key::Named(NamedKey::Space), None);
    let r = run(&mut h);
    assert!(!r.enabled && !r.clicked && !r.changed);
    assert!(!on);
}

#[test]
fn pixel_probes_at_scale_two() {
    let probe = |on: bool, hovered: bool| {
        let mut h = UiHarness::new(WIN);
        h.input.set_scale(2.0);
        let mut on = on;
        frame(&mut h, &mut on);
        if hovered {
            h.input.pointer_to(CENTRE.0, CENTRE.1);
            h.at_ms(10);
            frame(&mut h, &mut on);
        }
        h.at_ms(1000);
        frame(&mut h, &mut on).0
    };
    let shapes = |s: &Scene| s.layers[0].geometry.under.clone();
    // Off, unhovered: the ring is the 80 x 40 track; the knob is 24 with its centre 20 px in.
    let s = shapes(&probe(false, false));
    let (ring, k) = (s[0], s[1]);
    assert_eq!((ring.rect[2], ring.rect[3]), (80.0, 40.0));
    assert_eq!(k.rect[2], 24.0);
    assert_eq!(k.rect[0] + 12.0 - ring.rect[0], 20.0);
    // On: the fill is the track in `accent`; the knob centre is 20 px from the right end.
    let s = shapes(&probe(true, false));
    let (fill, k) = (s[0], s[1]);
    assert_eq!((fill.rect[2], fill.rect[3]), (80.0, 40.0));
    assert_eq!(fill.color, premul(DARK.accent));
    assert_eq!(fill.rect[0] + 80.0 - (k.rect[0] + 12.0), 20.0);
    // Hovered: knob 28.
    let s = shapes(&probe(false, true));
    assert_eq!(s.last().expect("knob").rect[2], 28.0);
}

#[test]
fn the_node_is_a_switch_whose_checked_state_follows_the_value() {
    let mut h = UiHarness::new(WIN);
    for on in [false, true] {
        let mut value = on;
        let (_, out) = h.frame(|ui| {
            ui.toggle("t", RECT, &mut value, "Switch");
        });
        let node = out.tree.get(id()).expect("node");
        assert_eq!((node.role, node.name.as_str()), (Role::Switch, "Switch"));
        assert_eq!(node.states.contains(State::Checked), on);
        assert!(node.states.contains(State::Focusable));
        assert!(node.actions.contains(Action::Default));
    }
}
