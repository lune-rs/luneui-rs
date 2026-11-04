//! The button (C4 retrofit, `widget.button` acceptance): rest identity with the Painter API,
//! motion conformance, activation by pointer, Enter, Space and accessibility, probes, the node.

use lune_ui_core::access::{Action, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::button::{ButtonKind, FOCUS, HOVER, PRESS, button};

const WIN: (f32, f32) = (400.0, 300.0);
const RECT: Rect = Rect::new(20.0, 20.0, 120.0, 32.0);
const KINDS: [ButtonKind; 2] = [ButtonKind::Primary, ButtonKind::Secondary];

fn id() -> WidgetId {
    WidgetId::ROOT.with("b")
}

/// What the Painter-API `button` draws.
fn old_scene(kind: ButtonKind, hovered: bool) -> Scene {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, hovered.then_some(1), DARK);
    button(&mut p, RECT, "Label", kind == ButtonKind::Primary, 1);
    p.scene
}

fn frame(h: &mut UiHarness, kind: ButtonKind) -> (Scene, Response, Repaint) {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| resp = Some(ui.button("b", RECT, "Label", kind)));
    (scene, resp.expect("frame ran"), out.repaint)
}

/// The Ui button's scene once settled.
fn settled_scene(kind: ButtonKind, hovered: bool) -> Scene {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, kind);
    if hovered {
        h.input.pointer_to(80.0, 36.0);
        h.at_ms(10);
        frame(&mut h, kind);
    }
    h.at_ms(1000);
    let (scene, _, repaint) = frame(&mut h, kind);
    assert_eq!(repaint, Repaint::Idle);
    scene
}

#[test]
fn at_rest_the_ui_button_draws_exactly_what_the_painter_button_does() {
    for kind in KINDS {
        for hovered in [false, true] {
            assert_eq!(
                settled_scene(kind, hovered),
                old_scene(kind, hovered),
                "{kind:?} hovered={hovered}"
            );
        }
    }
}

#[test]
fn at_rest_no_shape_is_transparent_and_no_transform_is_pushed() {
    for kind in KINDS {
        let scene = settled_scene(kind, false);
        let g = &scene.layers[0].geometry;
        let shapes: Vec<_> = g.under.iter().chain(&g.over).collect();
        assert_eq!(shapes.len(), 1, "{kind:?}: one fill, no ring");
        assert!(shapes.iter().all(|s| s.color[3] > 0.0));
        // The fill is the full rect: an untransformed fill (a pushed scale would shrink it).
        assert_eq!(shapes[0].rect, [20.0, 20.0, 120.0, 32.0]);
    }
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [(HOVER, "hover"), (PRESS, "press"), (FOCUS, "focus")] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
        let (a, b) = (
            DARK.surface_hover.map(f32::from),
            DARK.surface_active.map(f32::from),
        );
        conformance(&HOVER, &theme.motion, tier, a, b)
            .unwrap_or_else(|e| panic!("hover colour {tier:?}: {e}"));
    }
    // Reduced snaps the press scale and keeps the hover and focus tweens.
    assert!(theme.motion.resolve(&PRESS, MotionTier::Reduced).is_none());
    assert!(theme.motion.resolve(&HOVER, MotionTier::Reduced).is_some());
    assert!(theme.motion.resolve(&FOCUS, MotionTier::Reduced).is_some());
}

/// Focus the button with Tab.
fn focused(kind: ButtonKind) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, kind);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    let (_, r, _) = frame(&mut h, kind);
    assert!(r.focused && r.focus_visible);
    h
}

#[test]
fn enter_and_space_activate_once_each() {
    for key in [NamedKey::Enter, NamedKey::Space] {
        let mut h = focused(ButtonKind::Primary);
        h.input.key_down(Key::Named(key), None);
        h.at_ms(20);
        let (_, r, _) = frame(&mut h, ButtonKind::Primary);
        assert!(r.clicked, "{key:?}");
        h.at_ms(30);
        let (_, r, _) = frame(&mut h, ButtonKind::Primary);
        assert!(!r.clicked, "{key:?} fires once per press");
    }
}

#[test]
fn a_pointer_click_and_the_default_action_activate() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, ButtonKind::Secondary);
    h.input.pointer_to(80.0, 36.0).click(PointerButton::Primary);
    let (_, r, _) = frame(&mut h, ButtonKind::Secondary);
    assert!(r.clicked);
    h.input.access(lune_ui_core::access::AccessAction {
        target: id(),
        action: Action::Default,
        value: None,
    });
    let (_, r, _) = frame(&mut h, ButtonKind::Secondary);
    assert!(r.clicked);
}

#[test]
fn a_disabled_button_reports_no_activation_and_is_marked_disabled() {
    let mut h = UiHarness::new(WIN);
    let run = |h: &mut UiHarness| {
        let mut resp = None;
        let (_, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                resp = Some(ui.button("b", RECT, "Label", ButtonKind::Primary));
            });
        });
        (resp.expect("ran"), out)
    };
    run(&mut h);
    h.input.pointer_to(80.0, 36.0).click(PointerButton::Primary);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (r, out) = run(&mut h);
    assert!(!r.enabled && !r.clicked && !r.hovered && !r.focused);
    let node = out.tree.get(id()).expect("node");
    assert!(node.states.contains(State::Disabled));
    assert!(!node.states.contains(State::Focusable));
    assert!(!node.actions.contains(Action::Default));
}

#[test]
fn pixel_probes_at_scale_two() {
    for (scale, radius) in [(1.0, 6.0), (2.0, 12.0)] {
        for (kind, fill) in [
            (ButtonKind::Primary, DARK.accent),
            (ButtonKind::Secondary, DARK.surface_hover),
        ] {
            let mut h = UiHarness::new(WIN);
            h.input.set_scale(scale);
            frame(&mut h, kind);
            h.at_ms(1000);
            let (scene, _, _) = frame(&mut h, kind);
            let shape = scene.layers[0].geometry.under[0];
            assert_eq!(shape.radii, [radius; 4], "{kind:?} scale {scale}");
            assert_eq!(shape.color, premul(fill), "{kind:?} fill");
            assert_eq!(
                shape.rect,
                [20.0 * scale, 20.0 * scale, 120.0 * scale, 32.0 * scale]
            );
        }
    }
}

#[test]
fn pressed_scales_about_the_centre_and_the_ring_follows_focus_visible() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, ButtonKind::Secondary);
    h.input.pointer_to(80.0, 36.0).press(PointerButton::Primary);
    h.at_ms(10);
    frame(&mut h, ButtonKind::Secondary);
    h.at_ms(1000);
    let (scene, r, repaint) = frame(&mut h, ButtonKind::Secondary);
    assert!(r.pressed && repaint == Repaint::Idle);
    let fill = scene.layers[0].geometry.under[0];
    let (w, hh) = (120.0 * 0.98, 32.0 * 0.98);
    assert_eq!(fill.rect, [80.0 - w / 2.0, 36.0 - hh / 2.0, w, hh]);
    assert_eq!(fill.color, premul(DARK.surface_active));
    // Focus-visible adds the ring: a second under shape.
    let mut h = focused(ButtonKind::Secondary);
    h.at_ms(1000);
    let (scene, _, _) = frame(&mut h, ButtonKind::Secondary);
    assert_eq!(scene.layers[0].geometry.under.len(), 2);
}

#[test]
fn the_node_names_the_button_and_follows_focus() {
    let mut h = focused(ButtonKind::Primary);
    let (_, out) = h.frame(|ui| {
        ui.button("b", RECT, "Label", ButtonKind::Primary);
    });
    let node = out.tree.get(id()).expect("node");
    assert_eq!((node.role, node.name.as_str()), (Role::Button, "Label"));
    assert_eq!(node.bounds, RECT);
    assert!(node.states.contains(State::Focusable) && node.states.contains(State::Focused));
    assert!(node.actions.contains(Action::Default));
    assert_eq!(out.focused, Some(id()));
}
