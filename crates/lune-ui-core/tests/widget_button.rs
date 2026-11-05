//! The button (C4 retrofit, `widget.button` acceptance): rest identity with the Painter API,
//! motion conformance, activation by pointer, Enter, Space and accessibility, probes, the node.

use lune_ui_core::access::{Action, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::shortcuts::{Binding, Chord, CommandId, Scope};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, Ui, WidgetId};
use lune_ui_core::widgets::button::{
    ButtonKind, ButtonLook, ButtonOptions, ButtonState, FOCUS, HOVER, ICON_GAP, ICON_SIZE,
    IconButton, PRESS, SMALL_HEIGHT, SMALL_RADIUS, button, button_width, button_width_with,
};
use lune_ui_core::widgets::radio_group::RadioOption;

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

// ---- Variants (M6 step 3): ghost, small, icon button, labelled icon ----

const ICON_RECT: Rect = Rect::new(20.0, 20.0, 28.0, 28.0);

fn icon_button<'a>() -> IconButton<'a> {
    IconButton {
        glyph: icon::CLOSE,
        glyph_size: 16.0,
        color: None,
        radius: 4.0,
        name: "Close",
    }
}

fn ghost() -> ButtonOptions<'static> {
    ButtonOptions {
        kind: ButtonKind::Ghost,
        ..ButtonOptions::default()
    }
}

/// One frame of `draw` over a harness.
fn with<R>(h: &mut UiHarness, draw: impl FnOnce(&mut lune_ui_core::ui::Ui<'_>) -> R) -> (Scene, R) {
    let mut out = None;
    let (scene, _) = h.frame(|ui| out = Some(draw(ui)));
    (scene, out.expect("ran"))
}

/// A scene of `draw` after the pointer and the transitions settled.
fn settle(
    mut h: UiHarness,
    mut draw: impl FnMut(&mut lune_ui_core::ui::Ui<'_>) -> Response,
) -> (Scene, Response) {
    with(&mut h, &mut draw);
    h.at_ms(10);
    with(&mut h, &mut draw);
    h.at_ms(1000);
    let (scene, r) = with(&mut h, &mut draw);
    (scene, r)
}

fn hovered_harness() -> UiHarness {
    let mut h = UiHarness::new(WIN);
    with(&mut h, |ui| ui.button_with("b", RECT, "Label", &ghost()));
    h.input.pointer_to(80.0, 36.0);
    h
}

#[test]
fn a_ghost_has_no_fill_at_rest_and_hover_and_press_fill_it() {
    let draw = |ui: &mut lune_ui_core::ui::Ui<'_>| ui.button_with("b", RECT, "Label", &ghost());
    let (scene, r) = settle(UiHarness::new(WIN), draw);
    assert!(!r.hovered);
    assert_eq!(
        scene.layers[0].geometry.under.len(),
        0,
        "transparent at rest"
    );
    assert_eq!(scene.layers[0].texts.len(), 1, "the label");

    let (scene, r) = settle(hovered_harness(), draw);
    assert!(r.hovered);
    let under = &scene.layers[0].geometry.under;
    assert_eq!(under.len(), 1);
    assert_eq!(under[0].color, premul(DARK.surface_hover));
    assert_eq!(under[0].rect, [20.0, 20.0, 120.0, 32.0]);
    assert_eq!(under[0].radii, [6.0; 4]);

    let mut h = hovered_harness();
    h.input.press(PointerButton::Primary);
    let (scene, r) = settle(h, draw);
    assert!(r.pressed);
    let fill = scene.layers[0].geometry.under[0];
    assert_eq!(fill.color, premul(DARK.surface_active));
    // The press scale is the other kinds' (0.98 about the centre).
    let (w, hh) = (120.0 * 0.98, 32.0 * 0.98);
    assert_eq!(fill.rect, [80.0 - w / 2.0, 36.0 - hh / 2.0, w, hh]);
}

#[test]
fn the_ghost_look_is_transparent_hover_active_with_secondary_text() {
    let look = |state| ButtonLook::of(&DARK, ButtonKind::Ghost, state);
    let rest = look(ButtonState::default());
    assert_eq!(rest.fill[3], 0);
    assert_eq!(rest.label, DARK.fg_secondary);
    let hover = look(ButtonState {
        hovered: true,
        ..ButtonState::default()
    });
    assert_eq!(hover.fill, DARK.surface_hover);
    let press = look(ButtonState {
        hovered: true,
        pressed: true,
        ..ButtonState::default()
    });
    assert_eq!((press.fill, press.scale), (DARK.surface_active, 0.98));
    assert_eq!(look(ButtonState::default()).ring, 0.0);
    assert_eq!(
        look(ButtonState {
            focus_visible: true,
            ..ButtonState::default()
        })
        .ring,
        1.0
    );
}

#[test]
fn the_ghost_and_icon_rows_conform_in_every_tier() {
    let rest = ButtonLook::of(&DARK, ButtonKind::Ghost, ButtonState::default()).fill;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (to, name) in [
            (DARK.surface_hover, "hover"),
            (DARK.surface_active, "press"),
        ] {
            conformance(
                &HOVER,
                &DARK.motion,
                tier,
                rest.map(f32::from),
                to.map(f32::from),
            )
            .unwrap_or_else(|e| panic!("ghost {name} {tier:?}: {e}"));
        }
        for (row, name) in [(PRESS, "press"), (FOCUS, "focus")] {
            conformance(&row, &DARK.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("icon {name} {tier:?}: {e}"));
        }
    }
}

#[test]
fn a_small_button_is_24_high_with_radius_4_and_keeps_the_top() {
    let small = ButtonOptions {
        small: true,
        ..ButtonOptions::default()
    };
    let (scene, r) = settle(UiHarness::new(WIN), |ui| {
        ui.button_with("b", RECT, "Label", &small)
    });
    assert_eq!(r.rect.h, SMALL_HEIGHT);
    let fill = scene.layers[0].geometry.under[0];
    assert_eq!(fill.rect, [20.0, 20.0, 120.0, 24.0]);
    assert_eq!(fill.radii, [SMALL_RADIUS; 4]);
    assert_eq!(fill.color, premul(DARK.surface_hover));
    // The focus ring follows the corner radius.
    let mut h = UiHarness::new(WIN);
    with(&mut h, |ui| ui.button_with("b", RECT, "Label", &small));
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    with(&mut h, |ui| ui.button_with("b", RECT, "Label", &small));
    h.at_ms(1000);
    let (scene, _) = with(&mut h, |ui| ui.button_with("b", RECT, "Label", &small));
    let under = &scene.layers[0].geometry.under;
    assert_eq!(under.len(), 2);
    assert_eq!(under[1].radii, [SMALL_RADIUS; 4], "the ring");
}

#[test]
fn a_default_button_with_is_the_plain_button() {
    let (a, _) = settle(UiHarness::new(WIN), |ui| {
        ui.button_with("b", RECT, "Label", &ButtonOptions::default())
    });
    let (b, _) = settle(UiHarness::new(WIN), |ui| {
        ui.button("b", RECT, "Label", ButtonKind::Secondary)
    });
    assert_eq!(a, b);
}

#[test]
fn widths_add_the_padding_and_the_glyph() {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    let plain = button_width(&mut p, "Label");
    let default = button_width_with(&mut p, "Label", &ButtonOptions::default());
    assert_eq!(default, plain, "padding 16 each side");
    let small = ButtonOptions {
        small: true,
        ..ButtonOptions::default()
    };
    assert_eq!(button_width_with(&mut p, "Label", &small), plain - 8.0);
    let with_icon = ButtonOptions {
        icon: Some(icon::ADD),
        ..ButtonOptions::default()
    };
    let glyph = FakeText
        .measure(
            &Spec::plain(
                icon::ADD,
                Style::mono(ICON_SIZE, ICON_SIZE * 1.25, DARK.fg_primary),
            ),
            1.0,
        )
        .0;
    assert_eq!(
        button_width_with(&mut p, "Label", &with_icon),
        plain + glyph + ICON_GAP
    );
}

#[test]
fn a_labelled_icon_draws_the_glyph_and_the_label_and_is_named_by_the_label() {
    let opts = ButtonOptions {
        icon: Some(icon::ADD),
        ..ghost()
    };
    let mut h = UiHarness::new(WIN);
    let ((scene, _), out) = {
        let mut got = None;
        let (scene, out) = h.frame(|ui| got = Some(ui.button_with("b", RECT, "Label", &opts)));
        ((scene, got), out)
    };
    assert_eq!(scene.layers[0].texts.len(), 2, "glyph and label");
    let node = out.tree.get(id()).expect("node");
    assert_eq!((node.role, node.name.as_str()), (Role::Button, "Label"));
    // The group is centred: the glyph starts left of the plain label's start.
    let plain_x = {
        let mut h = UiHarness::new(WIN);
        let (scene, _) = h.frame(|ui| {
            ui.button_with("b", RECT, "Label", &ghost());
        });
        scene.layers[0].texts[0].x
    };
    let xs: Vec<f32> = scene.layers[0].texts.iter().map(|t| t.x).collect();
    assert!(xs[0] < plain_x && xs[1] > xs[0] + ICON_GAP);
}

#[test]
fn an_icon_button_is_a_named_ghost_that_clicks_once_by_pointer_enter_and_space() {
    let draw = |ui: &mut lune_ui_core::ui::Ui<'_>| ui.icon_button("b", ICON_RECT, &icon_button());
    // A pointer press and release: clicked in the release frame only.
    let mut h = UiHarness::new(WIN);
    with(&mut h, draw);
    h.input.pointer_to(34.0, 34.0).click(PointerButton::Primary);
    let (_, r) = with(&mut h, draw);
    assert!(r.clicked);
    h.at_ms(20);
    let (_, r) = with(&mut h, draw);
    assert!(!r.clicked, "once");
    for key in [NamedKey::Enter, NamedKey::Space] {
        let mut h = UiHarness::new(WIN);
        with(&mut h, draw);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        h.at_ms(10);
        let (_, r) = with(&mut h, draw);
        assert!(r.focused && r.focus_visible);
        h.input.key_down(Key::Named(key), None);
        h.at_ms(20);
        let (_, r) = with(&mut h, draw);
        assert!(r.clicked, "{key:?}");
        h.at_ms(30);
        let (_, r) = with(&mut h, draw);
        assert!(!r.clicked, "{key:?} once");
    }
}

#[test]
fn an_icon_button_has_the_glyph_centred_no_fill_at_rest_and_the_node() {
    let mut h = UiHarness::new(WIN);
    let (scene, out) = h.frame(|ui| {
        ui.icon_button("b", ICON_RECT, &icon_button());
    });
    let g = &scene.layers[0];
    assert_eq!(g.geometry.under.len(), 0);
    assert_eq!(g.texts.len(), 1, "the glyph");
    let node = out.tree.get(id()).expect("node");
    assert_eq!((node.role, node.name.as_str()), (Role::Button, "Close"));
    assert_eq!(node.bounds, ICON_RECT);
    assert!(node.actions.contains(Action::Default));
    // Hover fills the caller's rect at the button's radius.
    let mut h = UiHarness::new(WIN);
    with(&mut h, |ui| ui.icon_button("b", ICON_RECT, &icon_button()));
    h.input.pointer_to(34.0, 34.0);
    h.at_ms(10);
    with(&mut h, |ui| ui.icon_button("b", ICON_RECT, &icon_button()));
    h.at_ms(1000);
    let (scene, _) = with(&mut h, |ui| ui.icon_button("b", ICON_RECT, &icon_button()));
    let fill = scene.layers[0].geometry.under[0];
    assert_eq!(fill.rect, [20.0, 20.0, 28.0, 28.0]);
    assert_eq!(
        (fill.radii, fill.color),
        ([4.0; 4], premul(DARK.surface_hover))
    );
}

#[test]
fn a_disabled_icon_button_reports_nothing_and_is_marked_disabled() {
    let mut h = UiHarness::new(WIN);
    let draw = |ui: &mut lune_ui_core::ui::Ui<'_>| {
        let mut r = None;
        ui.with_enabled(false, |ui| {
            r = Some(ui.icon_button("b", ICON_RECT, &icon_button()));
        });
        r.expect("ran")
    };
    with(&mut h, draw);
    h.input.pointer_to(34.0, 34.0).click(PointerButton::Primary);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let mut r = None;
    let (_, out) = h.frame(|ui| r = Some(draw(ui)));
    let r = r.expect("ran");
    assert!(!r.enabled && !r.clicked);
    assert!(
        out.tree
            .get(id())
            .expect("node")
            .states
            .contains(State::Disabled)
    );
}

/// A widget drawn in `RECT`, for the disabled key rule.
type Draw = fn(&mut Ui<'_>);

fn a_button(ui: &mut Ui<'_>) {
    ui.button("w", RECT, "Label", ButtonKind::Secondary);
}

fn an_icon_button(ui: &mut Ui<'_>) {
    let glyph = IconButton {
        glyph: icon::ADD,
        glyph_size: ICON_SIZE,
        color: None,
        radius: 6.0,
        name: "Add",
    };
    ui.icon_button("w", RECT, &glyph);
}

fn a_toolbar_label(ui: &mut Ui<'_>) {
    ui.toolbar_label("w", RECT, "", "Label", "Label");
}

fn a_checkbox(ui: &mut Ui<'_>) {
    ui.checkbox("w", RECT, &mut false, "Label");
}

fn a_toggle(ui: &mut Ui<'_>) {
    ui.toggle("w", RECT, &mut false, "Label");
}

fn a_radio_group(ui: &mut Ui<'_>) {
    let option = |key, label| RadioOption {
        key,
        label,
        enabled: true,
    };
    ui.radio_group(
        "w",
        RECT,
        &[option(0, "a"), option(1, "b")],
        &mut 0,
        "Label",
    );
}

#[test]
fn a_chord_on_enter_or_space_is_a_shortcut_and_clicks_nothing() {
    for k in [NamedKey::Enter, NamedKey::Space] {
        let mut h = UiHarness::new(WIN);
        h.shortcuts.add(Binding::new(
            Scope::Global,
            Chord::key(Key::Named(k)).ctrl(),
            "chord",
        ));
        let at = Rect::new(10.0, 10.0, 100.0, 30.0);
        let mut clicked = false;
        let mut run = |h: &mut UiHarness| {
            h.frame(|ui| clicked = ui.button("b", at, "B", ButtonKind::Secondary).clicked)
                .1
        };
        run(&mut h);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        assert!(run(&mut h).focused.is_some());
        h.input.set_modifiers(Modifiers {
            control: true,
            ..Modifiers::default()
        });
        h.input.key_down(Key::Named(k), None);
        let out = run(&mut h);
        assert!(!clicked, "{k:?}");
        assert_eq!(out.fired.len(), 1, "{k:?}");
    }
}

/// Focuses the widget, disables it, presses `keys` in one frame and returns what fired.
fn keys_on_disabled(draw: Draw, keys: &[NamedKey]) -> Vec<CommandId> {
    let mut h = UiHarness::new(WIN);
    for k in keys {
        h.shortcuts.add(Binding::new(
            Scope::Global,
            Chord::key(Key::Named(*k)),
            "go",
        ));
    }
    h.frame(draw);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let (_, out) = h.frame(draw);
    assert!(out.focused.is_some(), "tab reached it");
    let disabled = |ui: &mut Ui<'_>| ui.with_enabled(false, draw);
    h.frame(disabled);
    h.frame(disabled);
    for k in keys {
        h.input.key_down(Key::Named(*k), None);
    }
    let (_, out) = h.frame(disabled);
    out.fired
}

#[test]
fn a_disabled_widget_keeps_the_keys_it_handles_from_the_shortcuts() {
    use NamedKey::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp, End, Enter, Home, Space};
    let cases: [(&str, Draw, &[NamedKey]); 6] = [
        ("button", a_button, &[Enter, Space]),
        ("icon button", an_icon_button, &[Enter, Space]),
        ("toolbar label", a_toolbar_label, &[Enter, Space]),
        ("checkbox", a_checkbox, &[Space]),
        ("toggle", a_toggle, &[Space]),
        (
            "radio group",
            a_radio_group,
            &[Space, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Home, End],
        ),
    ];
    for (name, draw, keys) in cases {
        for k in keys {
            assert!(keys_on_disabled(draw, &[*k]).is_empty(), "{name}: {k:?}");
            let twice = keys_on_disabled(draw, &[*k, *k]);
            assert!(twice.is_empty(), "{name}: {k:?} twice in a frame");
        }
        assert!(
            keys_on_disabled(draw, keys).is_empty(),
            "{name}: all at once"
        );
    }
}

#[test]
fn an_enabled_button_takes_one_activation_key_and_leaves_the_other() {
    let mut h = UiHarness::new(WIN);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::Space)),
        "go",
    ));
    let mut clicked = false;
    h.frame(a_button);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.frame(a_button);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    h.input.key_down(Key::Named(NamedKey::Space), None);
    let (_, out) = h.frame(|ui| {
        clicked = ui.button("w", RECT, "Label", ButtonKind::Secondary).clicked;
    });
    assert!(clicked);
    assert_eq!(out.fired.len(), 1, "as before: Enter won, Space went on");
}
