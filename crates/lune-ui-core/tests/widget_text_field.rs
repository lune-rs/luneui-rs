//! The text field (C4 retrofit, `widget.text-field` M4 part): rest identity with the Painter API,
//! motion conformance, focus and set-value behaviour, probes, the node.

use lune_ui_core::access::{AccessAction, AccessValue, Action, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::field::{ERROR, FOCUS, FieldState, HOVER, field};

const WIN: (f32, f32) = (400.0, 300.0);
const RECT: Rect = Rect::new(20.0, 20.0, 140.0, 32.0);
const CENTRE: (f32, f32) = (90.0, 36.0);

/// How the field is reached before the settled frame.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Reach {
    /// Nothing: unhovered and unfocused.
    Rest,
    /// The pointer is over it.
    Hover,
    /// A click: focused, and the pointer is over it.
    Click,
    /// Tab: focused, the pointer elsewhere.
    Tab,
}

fn id() -> WidgetId {
    WidgetId::ROOT.with("f")
}

fn old_scene(text: &str, reach: Reach) -> Scene {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut t = FakeText;
    let hover = matches!(reach, Reach::Hover | Reach::Click).then_some(1_u8);
    let focused = matches!(reach, Reach::Click | Reach::Tab);
    let mut p = Painter::<u8>::new(1.0, win, &mut t, hover, DARK);
    field(&mut p, RECT, text, "Search", focused, 1);
    p.scene
}

fn frame(h: &mut UiHarness, text: &mut String) -> (Scene, Response, Repaint) {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| {
        resp = Some(ui.text_field("f", RECT, text, "Search", FieldState::default()));
    });
    (scene, resp.expect("frame ran"), out.repaint)
}

fn settled(text: &str, reach: Reach) -> Scene {
    let mut h = UiHarness::new(WIN);
    let mut text = text.to_owned();
    frame(&mut h, &mut text);
    match reach {
        Reach::Rest => {}
        Reach::Hover => {
            h.input.pointer_to(CENTRE.0, CENTRE.1);
        }
        Reach::Click => {
            h.input
                .pointer_to(CENTRE.0, CENTRE.1)
                .click(PointerButton::Primary);
        }
        Reach::Tab => {
            h.input.key_down(Key::Named(NamedKey::Tab), None);
        }
    }
    h.at_ms(10);
    frame(&mut h, &mut text);
    h.at_ms(1000);
    let (scene, _, repaint) = frame(&mut h, &mut text);
    assert_eq!(repaint, Repaint::Idle, "{reach:?}");
    scene
}

#[test]
fn at_rest_the_ui_field_draws_exactly_what_the_painter_field_does() {
    for (text, reach) in [
        ("", Reach::Rest),
        ("", Reach::Hover),
        ("lune", Reach::Click),
        ("lune", Reach::Tab),
        ("Untitled", Reach::Click),
    ] {
        assert_eq!(
            settled(text, reach),
            old_scene(text, reach),
            "text={text:?} {reach:?}"
        );
    }
}

#[test]
fn at_rest_no_shape_is_transparent() {
    let scene = settled("", Reach::Rest);
    let g = &scene.layers[0].geometry;
    assert!(g.under.iter().chain(&g.over).all(|s| s.color[3] > 0.0));
    // The fill and the border; no caret while unfocused.
    assert_eq!(g.under.len(), 2);
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    let surface_input = DARK.surface_input.map(f32::from);
    let surface_hover = DARK.surface_hover.map(f32::from);
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&HOVER, &theme.motion, tier, surface_input, surface_hover)
            .unwrap_or_else(|e| panic!("hover {tier:?}: {e}"));
        conformance(&FOCUS, &theme.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("focus {tier:?}: {e}"));
        conformance(&ERROR, &theme.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("error {tier:?}: {e}"));
    }
    assert!(theme.motion.resolve(&HOVER, MotionTier::Reduced).is_some());
    assert!(theme.motion.resolve(&FOCUS, MotionTier::Reduced).is_some());
}

#[test]
fn a_click_focuses_and_tab_moves_focus_away() {
    let mut h = UiHarness::new(WIN);
    let mut a = String::new();
    let mut b = String::new();
    let run = |h: &mut UiHarness, a: &mut String, b: &mut String| {
        let mut out = (None, None);
        h.frame(|ui| {
            out.0 = Some(ui.text_field("f", RECT, a, "Search", FieldState::default()));
            out.1 = Some(ui.text_field(
                "g",
                Rect::new(20.0, 80.0, 140.0, 32.0),
                b,
                "Other",
                FieldState::default(),
            ));
        });
        (out.0.expect("f"), out.1.expect("g"))
    };
    run(&mut h, &mut a, &mut b);
    h.input
        .pointer_to(CENTRE.0, CENTRE.1)
        .click(PointerButton::Primary);
    let (f, _) = run(&mut h, &mut a, &mut b);
    assert!(f.focused);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let (f, g) = run(&mut h, &mut a, &mut b);
    assert!(!f.focused && g.focused, "Tab moves focus to the next field");
}

#[test]
fn a_disabled_field_ignores_the_click() {
    let mut h = UiHarness::new(WIN);
    let mut text = String::new();
    let run = |h: &mut UiHarness, text: &mut String| {
        let mut resp = None;
        h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                resp = Some(ui.text_field("f", RECT, text, "Search", FieldState::default()));
            });
        });
        resp.expect("ran")
    };
    run(&mut h, &mut text);
    h.input
        .pointer_to(CENTRE.0, CENTRE.1)
        .click(PointerButton::Primary);
    let r = run(&mut h, &mut text);
    assert!(!r.enabled && !r.focused && !r.changed);
}

#[test]
fn set_value_replaces_the_text_and_marks_changed() {
    let mut h = UiHarness::new(WIN);
    let mut text = String::from("old");
    frame(&mut h, &mut text);
    h.input.access(AccessAction {
        target: id(),
        action: Action::SetValue,
        value: Some(AccessValue::Text("new text".to_owned())),
    });
    let (_, r, _) = frame(&mut h, &mut text);
    assert_eq!(text, "new text");
    assert!(r.changed);
}

#[test]
fn pixel_probes_at_scale_two() {
    let probe = |reach: Reach, error: bool| {
        let mut h = UiHarness::new(WIN);
        h.input.set_scale(2.0);
        let mut text = String::new();
        let st = FieldState { error };
        let run = |h: &mut UiHarness, text: &mut String| {
            h.frame(|ui| {
                ui.text_field("f", RECT, text, "Search", st);
            })
            .0
        };
        run(&mut h, &mut text);
        match reach {
            Reach::Hover => {
                h.input.pointer_to(CENTRE.0, CENTRE.1);
            }
            Reach::Tab => {
                h.input.key_down(Key::Named(NamedKey::Tab), None);
            }
            _ => {}
        }
        h.at_ms(10);
        run(&mut h, &mut text);
        h.at_ms(1000);
        run(&mut h, &mut text)
    };
    let boxed = [40.0, 40.0, 280.0, 64.0];
    let shapes = |s: &Scene| s.layers[0].geometry.under.clone();
    // Rest: the fill is `surface_input`; the border is 1 logical px `border`, radius 12.
    let s = shapes(&probe(Reach::Rest, false));
    let fill = s.iter().find(|x| x.rect == boxed).expect("fill");
    assert_eq!(fill.color, premul(DARK.surface_input));
    assert_eq!(fill.radii, [12.0; 4]);
    let ring = s.iter().rfind(|x| x.rect == boxed).expect("ring");
    assert_eq!(ring.color, premul(DARK.border));
    // Hovered: the fill is `surface_hover`.
    let s = shapes(&probe(Reach::Hover, false));
    let fill = s.iter().find(|x| x.rect == boxed).expect("fill");
    assert_eq!(fill.color, premul(DARK.surface_hover));
    // Focused: the ring is `accent`.
    let s = shapes(&probe(Reach::Tab, false));
    let ring = s.iter().rfind(|x| x.rect == boxed).expect("ring");
    assert_eq!(ring.color, premul(DARK.accent));
    // Error: the ring is `err`.
    let s = shapes(&probe(Reach::Rest, true));
    let ring = s.iter().rfind(|x| x.rect == boxed).expect("ring");
    assert_eq!(ring.color, premul(DARK.err));
}

#[test]
fn the_node_is_a_text_input_named_by_the_placeholder() {
    let mut h = UiHarness::new(WIN);
    for error in [false, true] {
        let mut text = String::from("lune");
        let (_, out) = h.frame(|ui| {
            ui.text_field("f", RECT, &mut text, "Search", FieldState { error });
        });
        let node = out.tree.get(id()).expect("node");
        assert_eq!((node.role, node.name.as_str()), (Role::TextInput, "Search"));
        assert_eq!(node.value, Some(Value::Text("lune".to_owned())));
        assert_eq!(node.states.contains(State::Invalid), error);
        assert!(node.states.contains(State::Focusable));
        assert!(node.actions.contains(Action::Focus));
        assert!(node.actions.contains(Action::SetValue));
    }
}
