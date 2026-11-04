//! The checkbox (C4 retrofit, `widget.checkbox` acceptance): rest identity with the Painter API,
//! motion conformance, the fill's change sampled at 60 Hz, activation, probes, the node.

use lune_ui_core::access::{AccessAction, Action, Role, State};
use lune_ui_core::color::{premul, with_alpha};
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::checkbox::{FOCUS, HOVER, SELECTION, checkbox};

const WIN: (f32, f32) = (400.0, 300.0);
/// Box at (20, 24), 16 by 16; the label starts 8 px after it.
const RECT: Rect = Rect::new(20.0, 20.0, 120.0, 24.0);
const CENTRE: (f32, f32) = (28.0, 32.0);

fn id() -> WidgetId {
    WidgetId::ROOT.with("c")
}

fn old_scene(checked: bool) -> Scene {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    checkbox(&mut p, RECT, checked, "Check", 1);
    p.scene
}

fn frame(h: &mut UiHarness, checked: &mut bool) -> (Scene, Response, Repaint) {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| resp = Some(ui.checkbox("c", RECT, checked, "Check")));
    (scene, resp.expect("frame ran"), out.repaint)
}

fn settled_scene(checked: bool, hovered: bool) -> Scene {
    let mut h = UiHarness::new(WIN);
    let mut checked = checked;
    frame(&mut h, &mut checked);
    if hovered {
        h.input.pointer_to(CENTRE.0, CENTRE.1);
        h.at_ms(10);
        frame(&mut h, &mut checked);
    }
    h.at_ms(1000);
    let (scene, _, repaint) = frame(&mut h, &mut checked);
    assert_eq!(repaint, Repaint::Idle);
    scene
}

#[test]
fn at_rest_the_ui_checkbox_draws_exactly_what_the_painter_checkbox_does() {
    for checked in [false, true] {
        assert_eq!(
            settled_scene(checked, false),
            old_scene(checked),
            "checked={checked}"
        );
    }
}

#[test]
fn at_rest_no_shape_is_transparent() {
    for checked in [false, true] {
        let scene = settled_scene(checked, false);
        let g = &scene.layers[0].geometry;
        assert!(g.under.iter().chain(&g.over).all(|s| s.color[3] > 0.0));
        // Unchecked: the outline; checked: the accent fill. Never both.
        assert_eq!(g.under.len(), 1, "checked={checked}");
    }
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    let clear_accent = with_alpha(DARK.accent, 0.0).map(f32::from);
    let accent = DARK.accent.map(f32::from);
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [(SELECTION, "selection"), (HOVER, "hover"), (FOCUS, "focus")] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
        conformance(&SELECTION, &theme.motion, tier, clear_accent, accent)
            .unwrap_or_else(|e| panic!("fill {tier:?}: {e}"));
    }
    assert!(
        theme
            .motion
            .resolve(&SELECTION, MotionTier::Reduced)
            .is_some()
    );
    assert!(theme.motion.resolve(&HOVER, MotionTier::Reduced).is_some());
    assert!(theme.motion.resolve(&FOCUS, MotionTier::Reduced).is_some());
}

/// The checked fill: the box-sized shape whose premultiplied colour is `accent` at its alpha.
fn fill_alpha(scene: &Scene) -> f32 {
    let acc = premul(DARK.accent);
    scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.rect == [20.0, 24.0, 16.0, 16.0])
        .find(|s| {
            let a = s.color[3];
            a > 0.0
                && (s.color[0] - acc[0] * a).abs() < 1e-4
                && (s.color[1] - acc[1] * a).abs() < 1e-4
                && (s.color[2] - acc[2] * a).abs() < 1e-4
        })
        .map_or(0.0, |s: &Shape| s.color[3])
}

/// Click at 500 ms, then the fill alpha on a 60 Hz grid.
fn fill_run(tier: MotionTier) -> (Vec<(u64, f32)>, Repaint, Scene) {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let mut checked = false;
    frame(&mut h, &mut checked);
    h.at_ms(500);
    h.input
        .pointer_to(CENTRE.0, CENTRE.1)
        .click(PointerButton::Primary);
    let (scene, r, _) = frame(&mut h, &mut checked);
    assert!(checked && r.changed);
    let mut xs = vec![(0, fill_alpha(&scene))];
    let mut repaint = Repaint::Now;
    let mut last = scene;
    for k in 1..=15_u64 {
        let dt = (k * 1000 + 30) / 60;
        h.at_ms(500 + dt);
        let (scene, _, rp) = frame(&mut h, &mut checked);
        xs.push((dt, fill_alpha(&scene)));
        repaint = rp;
        last = scene;
    }
    (xs, repaint, last)
}

#[test]
fn the_fill_rises_monotonically_and_settles_within_180_ms_and_a_frame() {
    let (xs, repaint, last) = fill_run(MotionTier::Full);
    assert!(xs.windows(2).all(|w| w[1].1 >= w[0].1), "monotonic: {xs:?}");
    assert!(
        xs.iter().any(|&(_, a)| a > 0.0 && a < 1.0),
        "moves through: {xs:?}"
    );
    for &(dt, a) in &xs {
        if dt >= 180 + 17 {
            assert_eq!(a, 1.0, "settled at {dt} ms");
        }
    }
    assert_eq!(xs.last().expect("samples").1, 1.0);
    assert_eq!(repaint, Repaint::Idle);
    // The end is exactly the settled checked look.
    assert_eq!(last, {
        let mut h = UiHarness::new(WIN);
        let mut checked = true;
        frame(&mut h, &mut checked);
        h.at_ms(1000);
        frame(&mut h, &mut checked).0
    });
}

#[test]
fn reduced_keeps_the_fill_change_and_off_snaps_everything() {
    let (xs, _, _) = fill_run(MotionTier::Reduced);
    assert!(
        xs.iter().any(|&(_, a)| a > 0.0 && a < 1.0),
        "Reduced keeps the tween: {xs:?}"
    );
    let (xs, _, _) = fill_run(MotionTier::Off);
    assert_eq!(xs.last().expect("samples").1, 1.0);
    assert!(xs[1..].iter().all(|&(_, a)| a == 1.0), "Off: {xs:?}");
}

#[test]
fn click_and_space_flip_once_each_and_mark_changed() {
    let mut h = UiHarness::new(WIN);
    let mut checked = false;
    frame(&mut h, &mut checked);
    h.input
        .pointer_to(CENTRE.0, CENTRE.1)
        .click(PointerButton::Primary);
    let (_, r, _) = frame(&mut h, &mut checked);
    assert!(checked && r.changed);
    let (_, r, _) = frame(&mut h, &mut checked);
    assert!(checked && !r.changed, "one flip per click");
    // The click focused it; Space flips back, once.
    h.input.key_down(Key::Named(NamedKey::Space), None);
    let (_, r, _) = frame(&mut h, &mut checked);
    assert!(!checked && r.changed);
    let (_, r, _) = frame(&mut h, &mut checked);
    assert!(!checked && !r.changed);
    h.input.access(AccessAction {
        target: id(),
        action: Action::Default,
        value: None,
    });
    let (_, r, _) = frame(&mut h, &mut checked);
    assert!(checked && r.changed);
}

#[test]
fn a_disabled_checkbox_reports_no_click() {
    let mut h = UiHarness::new(WIN);
    let mut checked = false;
    let mut run = |h: &mut UiHarness| {
        let mut resp = None;
        h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                resp = Some(ui.checkbox("c", RECT, &mut checked, "Check"));
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
    assert!(!checked);
}

#[test]
fn pixel_probes_at_scale_two() {
    let probe = |checked: bool| {
        let mut h = UiHarness::new(WIN);
        h.input.set_scale(2.0);
        let mut checked = checked;
        frame(&mut h, &mut checked);
        h.at_ms(1000);
        frame(&mut h, &mut checked).0
    };
    let box_px = [40.0, 48.0, 32.0, 32.0];
    // Unchecked: a 32 by 32 outline, radius 6 (3 at scale 1).
    let s = probe(false);
    let ring = s.layers[0]
        .geometry
        .under
        .iter()
        .find(|sh| sh.rect == box_px)
        .expect("outline");
    assert_eq!(ring.radii, [6.0; 4]);
    // Checked: the box is filled with `accent`, the check glyph is drawn, and the label starts
    // 16 px past the box edge (8 px at scale 1).
    let s = probe(true);
    let fill = s.layers[0]
        .geometry
        .under
        .iter()
        .find(|sh| sh.rect == box_px)
        .expect("fill");
    assert_eq!(fill.color, premul(DARK.accent));
    // Text items are in logical pixels (the scale is a separate field): the glyph is first, the
    // label last; the box's right edge is 36, so the label starts at 44.
    let label = s.layers[0].texts.last().expect("label");
    assert_eq!(label.x, 36.0 + 8.0);
}

#[test]
fn the_node_is_a_checkbox_whose_checked_state_follows_the_value() {
    let mut h = UiHarness::new(WIN);
    for checked in [false, true] {
        let mut value = checked;
        let (_, out) = h.frame(|ui| {
            ui.checkbox("c", RECT, &mut value, "Check");
        });
        let node = out.tree.get(id()).expect("node");
        assert_eq!((node.role, node.name.as_str()), (Role::Checkbox, "Check"));
        assert_eq!(node.states.contains(State::Checked), checked);
        assert!(node.states.contains(State::Focusable));
        assert!(node.actions.contains(Action::Default));
    }
}
