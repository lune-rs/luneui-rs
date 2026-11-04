//! The combo (C4 retrofit, `widget.combo` acceptance): rest identity with the Painter API, motion
//! conformance, opening by pointer, Enter, Space, Down and accessibility, probes, the node.

use lune_ui_core::access::{AccessAction, Action, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::combo::{BORDER, HOVER, combo};

const WIN: (f32, f32) = (400.0, 300.0);
const RECT: Rect = Rect::new(20.0, 20.0, 210.0, 32.0);
const CENTRE: (f32, f32) = (125.0, 36.0);

fn id() -> WidgetId {
    WidgetId::ROOT.with("c")
}

/// What the Painter-API `combo` draws.
fn old_scene(hovered: bool, open: bool) -> Scene {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    combo(&mut p, RECT, "RznSans", hovered, open, 1);
    p.scene
}

fn frame(h: &mut UiHarness, open: &mut bool) -> (Scene, Response, Repaint) {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| resp = Some(ui.combo("c", RECT, "RznSans", open, "Font")));
    (scene, resp.expect("frame ran"), out.repaint)
}

fn settled_scene(hovered: bool, open: bool) -> Scene {
    let mut h = UiHarness::new(WIN);
    let mut open = open;
    frame(&mut h, &mut open);
    if hovered {
        h.input.pointer_to(CENTRE.0, CENTRE.1);
        h.at_ms(10);
        frame(&mut h, &mut open);
    }
    h.at_ms(1000);
    let (scene, _, repaint) = frame(&mut h, &mut open);
    assert_eq!(repaint, Repaint::Idle);
    scene
}

#[test]
fn at_rest_the_ui_combo_draws_exactly_what_the_painter_combo_does() {
    // Closed (the `controls` combo), hovered, and open (the `dropdown` scenes' combo).
    for (hovered, open) in [(false, false), (true, false), (false, true), (true, true)] {
        assert_eq!(
            settled_scene(hovered, open),
            old_scene(hovered, open),
            "hovered={hovered} open={open}"
        );
    }
}

#[test]
fn at_rest_no_shape_is_transparent_and_no_transform_is_pushed() {
    let scene = settled_scene(false, false);
    let g = &scene.layers[0].geometry;
    let shapes: Vec<_> = g.under.iter().chain(&g.over).collect();
    assert_eq!(shapes.len(), 2, "fill and ring");
    assert!(shapes.iter().all(|s| s.color[3] > 0.0));
    assert_eq!(shapes[0].rect, [20.0, 20.0, 210.0, 32.0]);
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, a, b) in [
            (HOVER, DARK.surface_input, DARK.surface_hover),
            (BORDER, DARK.border, DARK.accent),
        ] {
            conformance(
                &row,
                &theme.motion,
                tier,
                a.map(f32::from),
                b.map(f32::from),
            )
            .unwrap_or_else(|e| panic!("{tier:?}: {e}"));
        }
    }
    // Both rows keep their tween under Reduced.
    assert!(theme.motion.resolve(&HOVER, MotionTier::Reduced).is_some());
    assert!(theme.motion.resolve(&BORDER, MotionTier::Reduced).is_some());
}

/// The fill's premultiplied alpha at the grid times after the pointer enters at 500 ms.
fn hover_samples(tier: MotionTier) -> Vec<(u64, f32)> {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let mut open = false;
    frame(&mut h, &mut open);
    h.at_ms(500);
    frame(&mut h, &mut open);
    h.input.pointer_to(CENTRE.0, CENTRE.1);
    let mut out = Vec::new();
    for k in 0..=10_u64 {
        let dt = (k * 1000 + 30) / 60;
        h.at_ms(500 + dt);
        let (scene, _, _) = frame(&mut h, &mut open);
        out.push((dt, scene.layers[0].geometry.under[0].color[3]));
    }
    out
}

#[test]
fn the_hover_fill_is_monotonic_and_settles_within_120_ms_and_a_frame() {
    let xs = hover_samples(MotionTier::Full);
    let end = premul(DARK.surface_hover)[3];
    assert!(xs.windows(2).all(|w| w[1].1 >= w[0].1), "monotonic: {xs:?}");
    assert!(
        xs.iter()
            .any(|&(_, a)| a > premul(DARK.surface_input)[3] && a < end),
        "{xs:?}"
    );
    for &(dt, a) in &xs {
        if dt >= 120 + 17 {
            assert_eq!(a, end, "settled at {dt} ms");
        }
    }
}

#[test]
fn reduced_keeps_the_tween_and_off_snaps() {
    let xs = hover_samples(MotionTier::Reduced);
    let end = premul(DARK.surface_hover)[3];
    assert!(xs.iter().any(|&(_, a)| a < end), "Reduced tweens: {xs:?}");
    let xs = hover_samples(MotionTier::Off);
    assert!(xs.iter().all(|&(_, a)| a == end), "Off snaps: {xs:?}");
}

/// Focus the combo with Tab.
fn focused() -> (UiHarness, bool) {
    let mut h = UiHarness::new(WIN);
    let mut open = false;
    frame(&mut h, &mut open);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    let (_, r, _) = frame(&mut h, &mut open);
    assert!(r.focused && r.focus_visible);
    (h, open)
}

#[test]
fn enter_space_and_down_each_open_it_once() {
    for key in [NamedKey::Enter, NamedKey::Space, NamedKey::ArrowDown] {
        let (mut h, mut open) = focused();
        h.input.key_down(Key::Named(key), None);
        h.at_ms(20);
        let (_, r, _) = frame(&mut h, &mut open);
        assert!(open && r.changed && r.clicked, "{key:?}");
        h.at_ms(30);
        let (_, r, _) = frame(&mut h, &mut open);
        assert!(open && !r.changed, "{key:?} fires once per press");
    }
}

#[test]
fn a_click_opens_it_and_it_never_closes_itself() {
    let mut h = UiHarness::new(WIN);
    let mut open = false;
    frame(&mut h, &mut open);
    h.input
        .pointer_to(CENTRE.0, CENTRE.1)
        .click(PointerButton::Primary);
    let (_, r, _) = frame(&mut h, &mut open);
    assert!(open && r.changed);
    // Escape and a second click leave it as the app has it.
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    h.input.click(PointerButton::Primary);
    let (_, r, _) = frame(&mut h, &mut open);
    assert!(open && !r.changed);
}

#[test]
fn the_expand_and_default_actions_open_it() {
    for action in [Action::Expand, Action::Default] {
        let mut h = UiHarness::new(WIN);
        let mut open = false;
        frame(&mut h, &mut open);
        h.input.access(AccessAction {
            target: id(),
            action,
            value: None,
        });
        let (_, r, _) = frame(&mut h, &mut open);
        assert!(open && r.changed, "{action:?}");
    }
}

#[test]
fn a_disabled_combo_reports_no_open() {
    let mut h = UiHarness::new(WIN);
    let mut open = false;
    let mut run = |h: &mut UiHarness| {
        let mut resp = None;
        let (_, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                resp = Some(ui.combo("c", RECT, "RznSans", &mut open, "Font"));
            });
        });
        (resp.expect("ran"), out)
    };
    run(&mut h);
    h.input
        .pointer_to(CENTRE.0, CENTRE.1)
        .click(PointerButton::Primary);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    let (r, out) = run(&mut h);
    assert!(!r.enabled && !r.clicked && !r.changed);
    let node = out.tree.get(id()).expect("node");
    assert!(node.states.contains(State::Disabled));
    assert!(!node.states.contains(State::Focusable));
    assert!(!node.actions.contains(Action::Default));
    assert!(!open);
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let mut open = false;
    frame(&mut h, &mut open);
    h.at_ms(1000);
    let (scene, _, _) = frame(&mut h, &mut open);
    let layer = &scene.layers[0];
    let (fill, ring) = (layer.geometry.under[0], layer.geometry.under[1]);
    assert_eq!(fill.radii, [12.0; 4]);
    assert_eq!(fill.color, premul(DARK.surface_input));
    assert_eq!(ring.radii, [12.0; 4]);
    assert_eq!(ring.params[1], 2.0, "ring width in physical px");
    assert_eq!(ring.color, premul(DARK.border));
    // Text items are in logical px. The value starts 10 logical (20 physical) px from the left
    // edge; the chevron slot is centred 18 logical (36 physical) px from the right. The glyph is
    // 6 logical px wide under the fake text (1 char x 12 px x 0.5).
    let (value, chevron) = (layer.texts[0], layer.texts[1]);
    assert_eq!((value.x - RECT.x) * 2.0, 20.0);
    assert_eq!((RECT.right() - (chevron.x + 3.0)) * 2.0, 36.0);
}

#[test]
fn open_and_focus_visible_draw_the_accent_border() {
    let ring_colour = |s: &Scene| s.layers[0].geometry.under[1].color;
    assert_eq!(
        ring_colour(&settled_scene(false, true)),
        premul(DARK.accent)
    );
    let (mut h, mut open) = focused();
    h.at_ms(1000);
    let (scene, _, repaint) = frame(&mut h, &mut open);
    assert_eq!(repaint, Repaint::Idle);
    assert_eq!(ring_colour(&scene), premul(DARK.accent));
}

#[test]
fn the_node_is_a_combo_box_whose_expanded_state_follows_open() {
    let mut h = UiHarness::new(WIN);
    for open in [false, true] {
        let mut value = open;
        let (_, out) = h.frame(|ui| {
            ui.combo("c", RECT, "RznSans", &mut value, "Font");
        });
        let node = out.tree.get(id()).expect("node");
        assert_eq!((node.role, node.name.as_str()), (Role::ComboBox, "Font"));
        assert_eq!(node.value, Some(Value::Text("RznSans".into())));
        assert_eq!(node.states.contains(State::Expanded), open);
        assert!(node.states.contains(State::Focusable));
        assert!(node.actions.contains(Action::Default));
        assert!(node.actions.contains(Action::Expand));
        // It never closes itself, so it advertises no collapse (review F5).
        assert!(!node.actions.contains(Action::Collapse));
    }
}
