//! The text field (C4 retrofit, `widget.text-field` M4 part): rest identity with the Painter API,
//! motion conformance, focus and set-value behaviour, probes, the node.

use lune_ui_core::access::{AccessAction, AccessValue, Action, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::Style;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::field::{ERROR, FOCUS, FieldOptions, FieldState, HOVER, field};
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};
use lune_ui_core::widgets::text_area::AreaOptions;

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
fn set_value_over_the_one_line_cap_is_refused_whole_and_clipboard_text_the_same() {
    use lune_ui_core::text_edit::MAX_ONE_LINE_INSERT;
    // The field shapes its whole text, so a value past 1 MiB (an assistive set-value or a paste of
    // anything from outside the process) would cost seconds and gigabytes: the text stays.
    let over = "a".repeat(MAX_ONE_LINE_INSERT + 1);
    let mut h = UiHarness::new(WIN);
    let mut text = String::from("old");
    frame(&mut h, &mut text);
    h.input.access(AccessAction {
        target: id(),
        action: Action::SetValue,
        value: Some(AccessValue::Text(over.clone())),
    });
    let (_, r, _) = frame(&mut h, &mut text);
    assert_eq!(text, "old");
    assert!(!r.changed);
    // Under the cap it still replaces.
    h.input.access(AccessAction {
        target: id(),
        action: Action::SetValue,
        value: Some(AccessValue::Text("new".to_owned())),
    });
    frame(&mut h, &mut text);
    assert_eq!(text, "new");
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

// The editing behaviour (`text.text-edit`, `widget.text-field`, M5).

mod edit {
    use super::*;
    use lune_ui_core::input::{ClipboardRequest, CursorIcon, FrameOutput, Modifiers};
    use lune_ui_core::scene::Shape;
    use lune_ui_core::shortcuts::{Binding, Chord, Scope};
    use lune_ui_core::text::{Font, Spec, Style, TextSystem};
    use lune_ui_core::widgets::field::{FieldOptions, FieldResponse, MASK_DOTS, Variant};

    /// Width of one ASCII glyph of the base style under the fake.
    fn glyph() -> f32 {
        FakeText
            .measure(
                &Spec::plain("a", Style::ui(DARK.font_base, DARK.fg_primary)),
                1.0,
            )
            .0
    }

    fn run(
        h: &mut UiHarness,
        text: &mut String,
        opts: &FieldOptions<'_>,
    ) -> (Scene, FieldResponse, FrameOutput) {
        let mut resp = None;
        let (scene, out) = h.frame(|ui| {
            resp = Some(ui.text_field_with("f", RECT, text, opts));
        });
        (scene, resp.expect("ran"), out)
    }

    fn plain(h: &mut UiHarness, text: &mut String) -> (Scene, FieldResponse, FrameOutput) {
        run(h, text, &FieldOptions::default())
    }

    /// A focused field over `text`, the caret at its end.
    fn focused(text: &mut String, opts: &FieldOptions<'_>) -> UiHarness {
        let mut h = UiHarness::new(WIN);
        run(&mut h, text, opts);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        run(&mut h, text, opts);
        h
    }

    fn typed(h: &mut UiHarness, s: &str) {
        for c in s.chars() {
            let t = c.to_string();
            h.input.key_down(Key::Character(t.clone()), Some(&t));
        }
    }

    fn press(h: &mut UiHarness, key: NamedKey) {
        h.input.key_down(Key::Named(key), None);
    }

    /// `key` with the macOS primary modifier.
    fn chord(h: &mut UiHarness, key: &str) {
        let logo = Modifiers {
            logo: true,
            ..Modifiers::default()
        };
        h.input.set_modifiers(logo);
        h.input.key_down(Key::Character(key.to_owned()), None);
        h.input.set_modifiers(Modifiers::default());
    }

    fn shift(h: &mut UiHarness, held: bool) {
        h.input.set_modifiers(Modifiers {
            shift: held,
            ..Modifiers::default()
        });
    }

    fn selection(out: &FrameOutput) -> (usize, usize) {
        out.tree
            .get(id())
            .and_then(|n| n.text.as_ref())
            .expect("text")
            .selection
    }

    fn caret_shape(scene: &Scene) -> Shape {
        let over = &scene.layers[0].geometry.over;
        assert_eq!(over.len(), 1, "one caret");
        over[0]
    }

    /// The fills in the text selection's colour.
    fn selection_fills(scene: &Scene) -> Vec<Shape> {
        let fill = premul(DARK.text_selection);
        let g = &scene.layers[0].geometry;
        g.under
            .iter()
            .filter(|s| s.color == fill)
            .copied()
            .collect()
    }

    /// Text x of column `n` of the field at rest.
    fn column(n: usize) -> f32 {
        RECT.x + 10.0 + n as f32 * glyph()
    }

    #[test]
    fn a_click_focuses_and_places_the_caret() {
        let mut h = UiHarness::new(WIN);
        let mut text = String::from("lune");
        plain(&mut h, &mut text);
        h.input
            .pointer_to(column(1) + 0.5, CENTRE.1)
            .click(PointerButton::Primary);
        let (scene, r, out) = plain(&mut h, &mut text);
        assert!(r.response.focused);
        assert_eq!(selection(&out), (1, 1));
        assert_eq!(caret_shape(&scene).rect[0], column(1) + 1.0);
    }

    #[test]
    fn typing_changes_the_text_and_reports_changed() {
        let mut text = String::new();
        let mut h = focused(&mut text, &FieldOptions::default());
        typed(&mut h, "abc");
        let (_, r, out) = plain(&mut h, &mut text);
        assert_eq!(text, "abc");
        assert!(r.response.changed);
        assert_eq!(selection(&out), (3, 3));
        press(&mut h, NamedKey::ArrowLeft);
        press(&mut h, NamedKey::ArrowLeft);
        let (_, r, out) = plain(&mut h, &mut text);
        assert!(!r.response.changed);
        assert_eq!(selection(&out), (1, 1));
        shift(&mut h, true);
        press(&mut h, NamedKey::ArrowRight);
        shift(&mut h, false);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (1, 2));
        chord(&mut h, "a");
        typed(&mut h, "z");
        plain(&mut h, &mut text);
        assert_eq!(text, "z");
    }

    #[test]
    fn backspace_then_type_keeps_the_order_of_one_frame() {
        let mut text = String::from("ab");
        let mut h = focused(&mut text, &FieldOptions::default());
        press(&mut h, NamedKey::Backspace);
        typed(&mut h, "c");
        plain(&mut h, &mut text);
        assert_eq!(text, "ac");
    }

    #[test]
    fn a_field_starts_with_the_caret_after_its_text() {
        let mut text = String::from("lune");
        let mut h = focused(&mut text, &FieldOptions::default());
        let (scene, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (4, 4));
        assert_eq!(caret_shape(&scene).rect[0], column(4) + 1.0);
    }

    #[test]
    fn undo_and_redo_run_through_the_primary_chords() {
        let mut text = String::new();
        let mut h = focused(&mut text, &FieldOptions::default());
        typed(&mut h, "abc");
        plain(&mut h, &mut text);
        chord(&mut h, "z");
        plain(&mut h, &mut text);
        assert_eq!(text, "");
        h.input.set_modifiers(Modifiers {
            logo: true,
            shift: true,
            ..Modifiers::default()
        });
        h.input.key_down(Key::Character("z".to_owned()), None);
        h.input.set_modifiers(Modifiers::default());
        plain(&mut h, &mut text);
        assert_eq!(text, "abc");
    }

    #[test]
    fn enter_submits_and_escape_and_page_keys_stay_free() {
        let mut text = String::from("x");
        let mut h = focused(&mut text, &FieldOptions::default());
        press(&mut h, NamedKey::Enter);
        let (_, r, _) = plain(&mut h, &mut text);
        assert!(r.submitted && !r.response.changed);
        assert_eq!(text, "x");
        let (_, r, _) = plain(&mut h, &mut text);
        assert!(!r.submitted);
        h.shortcuts
            .add(Binding::new(
                Scope::Global,
                Chord::key(Key::Named(NamedKey::Escape)),
                "esc",
            ))
            .add(Binding::new(
                Scope::Global,
                Chord::key(Key::Named(NamedKey::PageUp)),
                "up",
            ));
        press(&mut h, NamedKey::Escape);
        press(&mut h, NamedKey::PageUp);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(out.fired.len(), 2, "a field consumes neither");
    }

    #[test]
    fn a_typed_key_fires_no_binding_and_a_declined_chord_does() {
        let mut text = String::new();
        let mut h = focused(&mut text, &FieldOptions::default());
        h.shortcuts
            .add(Binding::new(
                Scope::Global,
                Chord::key(Key::Character("j".into())),
                "j",
            ))
            .add(Binding::new(
                Scope::Global,
                Chord::key(Key::Character("c".into())).primary(),
                "c",
            ));
        typed(&mut h, "j");
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(text, "j");
        assert!(
            out.fired.is_empty(),
            "the press that typed belongs to the field"
        );
        // Copy with nothing selected is declined, so the binding has it.
        chord(&mut h, "c");
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(out.fired.len(), 1);
        assert!(out.clipboard.is_empty());
    }

    #[test]
    fn copy_cut_and_paste_go_through_the_frame_output() {
        let mut text = String::from("lune");
        let mut h = focused(&mut text, &FieldOptions::default());
        chord(&mut h, "a");
        chord(&mut h, "c");
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(out.clipboard, [ClipboardRequest::Write("lune".to_owned())]);
        chord(&mut h, "x");
        let (_, r, out) = plain(&mut h, &mut text);
        assert_eq!(out.clipboard, [ClipboardRequest::Write("lune".to_owned())]);
        assert!(r.response.changed);
        assert_eq!(text, "");
        chord(&mut h, "v");
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(out.clipboard, [ClipboardRequest::Read]);
        assert_eq!(text, "");
        h.input.clipboard_text("a\nb");
        let (_, r, _) = plain(&mut h, &mut text);
        assert!(r.response.changed);
        assert_eq!(text, "a b");
    }

    #[test]
    fn a_clipboard_answer_applies_only_while_a_paste_is_pending() {
        let mut text = String::from("x");
        let mut h = focused(&mut text, &FieldOptions::default());
        h.input.clipboard_text("nobody asked");
        plain(&mut h, &mut text);
        assert_eq!(text, "x");
        // One paste, one answer: the second answer is not this paste's.
        chord(&mut h, "v");
        plain(&mut h, &mut text);
        h.input.clipboard_text("1").clipboard_text("2");
        plain(&mut h, &mut text);
        assert_eq!(text, "x1");
        // "Unavailable" ends the wait too.
        chord(&mut h, "v");
        plain(&mut h, &mut text);
        h.input.clipboard_unavailable();
        plain(&mut h, &mut text);
        h.input.clipboard_text("late");
        plain(&mut h, &mut text);
        assert_eq!(text, "x1");
    }

    #[test]
    fn focus_loss_clears_a_pending_paste() {
        let mut a = String::new();
        let mut b = String::new();
        let other = Rect::new(20.0, 80.0, 140.0, 32.0);
        let mut h = UiHarness::new(WIN);
        let two = |h: &mut UiHarness, a: &mut String, b: &mut String| {
            h.frame(|ui| {
                ui.text_field_with("f", RECT, a, &FieldOptions::default());
                ui.text_field_with("g", other, b, &FieldOptions::default());
            });
        };
        two(&mut h, &mut a, &mut b);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        two(&mut h, &mut a, &mut b);
        chord(&mut h, "v");
        two(&mut h, &mut a, &mut b);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        two(&mut h, &mut a, &mut b);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        two(&mut h, &mut a, &mut b);
        h.input.clipboard_text("late");
        two(&mut h, &mut a, &mut b);
        assert_eq!((a.as_str(), b.as_str()), ("", ""));
    }

    #[test]
    fn a_press_a_drag_a_shift_press_and_multi_presses_select() {
        let mut text = String::from("lune term");
        let mut h = UiHarness::new(WIN);
        plain(&mut h, &mut text);
        // A drag keeps the capture past the box.
        h.input
            .pointer_to(column(1), CENTRE.1)
            .press(PointerButton::Primary);
        plain(&mut h, &mut text);
        h.input.pointer_to(column(5), RECT.bottom() + 40.0);
        let (scene, r, out) = plain(&mut h, &mut text);
        assert!(r.response.pressed);
        assert_eq!(selection(&out), (1, 5));
        let fills = selection_fills(&scene);
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].rect[0], column(1));
        assert_eq!(fills[0].rect[2], 4.0 * glyph());
        h.input.release(PointerButton::Primary);
        plain(&mut h, &mut text);
        // Shift-press extends from the anchor.
        h.at_ms(5000);
        shift(&mut h, true);
        h.input
            .pointer_to(column(7), CENTRE.1)
            .click(PointerButton::Primary);
        shift(&mut h, false);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (1, 7));
        // A second and a third press in time select the word, then the line.
        h.at_ms(10_000);
        h.input
            .pointer_to(column(2), CENTRE.1)
            .click(PointerButton::Primary);
        plain(&mut h, &mut text);
        h.input.click(PointerButton::Primary);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (0, 4));
        h.input.click(PointerButton::Primary);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (0, 9));
    }

    #[test]
    fn a_word_press_keeps_its_selection_while_it_drags() {
        let mut text = String::from("lune term");
        let mut h = UiHarness::new(WIN);
        plain(&mut h, &mut text);
        h.input
            .pointer_to(column(2), CENTRE.1)
            .click(PointerButton::Primary);
        plain(&mut h, &mut text);
        h.input.press(PointerButton::Primary);
        plain(&mut h, &mut text);
        h.input.pointer_to(column(8), CENTRE.1);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (0, 4));
    }

    #[test]
    fn the_cursor_is_text_over_the_field_only() {
        let mut text = String::from("x");
        let mut h = UiHarness::new(WIN);
        plain(&mut h, &mut text);
        h.input.pointer_to(CENTRE.0, CENTRE.1);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(out.cursor, CursorIcon::Text);
        h.input.pointer_to(300.0, 200.0);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(out.cursor, CursorIcon::Default);
    }

    #[test]
    fn the_caret_stays_in_the_clip_as_the_text_scrolls() {
        let long = "w".repeat(60);
        let mut text = long.clone();
        let mut h = focused(&mut text, &FieldOptions::default());
        let clip = RECT.inset(6.0, 0.0);
        let check = |scene: &Scene| {
            let c = caret_shape(scene).rect;
            assert!(
                c[0] >= clip.x + 2.0 && c[0] + c[2] <= clip.right() - 2.0,
                "{c:?}"
            );
        };
        let (scene, _, _) = plain(&mut h, &mut text);
        check(&scene);
        // The text moved left by exactly what the caret needs.
        let x = scene.layers[0].texts[0].x;
        assert!(x < RECT.x + 10.0);
        let end = x + 60.0 * glyph() + 1.0 + 1.5;
        assert_eq!(end, clip.right() - 2.0);
        // Scroll is persistent: a settled frame does not move it.
        let (again, _, _) = plain(&mut h, &mut text);
        assert_eq!(again.layers[0].texts[0].x, x);
        // Walking left: inside the clip at every step, back to the start at 0.
        for _ in 0..60 {
            press(&mut h, NamedKey::ArrowLeft);
            let (scene, _, _) = plain(&mut h, &mut text);
            check(&scene);
        }
        let (scene, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (0, 0));
        assert_eq!(scene.layers[0].texts[0].x, RECT.x + 10.0);
        // A shorter text gives the space back.
        press(&mut h, NamedKey::End);
        plain(&mut h, &mut text);
        chord(&mut h, "a");
        typed(&mut h, "a");
        let (scene, _, _) = plain(&mut h, &mut text);
        assert_eq!(scene.layers[0].texts[0].x, RECT.x + 10.0);
    }

    #[test]
    fn a_press_lands_on_the_scrolled_text() {
        let mut text = "abcdefghij".repeat(6);
        let mut h = focused(&mut text, &FieldOptions::default());
        let (scene, _, _) = plain(&mut h, &mut text);
        let x = scene.layers[0].texts[0].x;
        h.input
            .pointer_to(x + 50.0 * glyph() + 0.5, CENTRE.1)
            .click(PointerButton::Primary);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (50, 50));
    }

    #[test]
    fn the_password_mask_is_fixed_and_the_secret_goes_nowhere() {
        let opts = FieldOptions {
            variant: Variant::Password,
            ..FieldOptions::default()
        };
        const SECRET: &str = "hunter2-very-long-secret-of-many-chars";
        let dots = "\u{2022}".repeat(MASK_DOTS);
        let style = Style::ui(DARK.font_base, DARK.fg_primary);
        let mask = TextSystem::prepare(&mut FakeText, &Spec::plain(dots.as_str(), style), 1.0);
        for secret in ["zq", SECRET] {
            let mut text = secret.to_owned();
            let mut h = focused(&mut text, &opts);
            let mut ops = None;
            let (scene, out) = h.frame(|ui| {
                ui.painter().start_recording();
                ui.text_field_with("f", RECT, &mut text, &opts);
                ops = Some(ui.painter().take_recording());
            });
            assert_eq!(scene.layers[0].texts[0].key, mask.0, "the same 16 dots");
            assert_eq!(caret_shape(&scene).rect[0], RECT.x + 10.0 + mask.1.0 + 1.0);
            let ops = format!("{:?}", ops.expect("ran"));
            assert!(!ops.contains(secret));
            let node = out.tree.get(id()).expect("node");
            assert_eq!((&node.value, &node.text), (&None, &None));
            assert!(!format!("{:?}{:?}", node, scene).contains(secret));
        }
    }

    #[test]
    fn a_password_field_is_a_password_input_and_a_plain_one_a_text_input() {
        // The role is how assistive technology knows not to speak what is typed (S1): a node of
        // `Role::TextInput` with no value is not enough.
        for (variant, role) in [
            (Variant::Password, Role::PasswordInput),
            (Variant::Plain, Role::TextInput),
            (Variant::Mono, Role::TextInput),
            (Variant::Search, Role::TextInput),
        ] {
            let opts = FieldOptions {
                variant,
                ..FieldOptions::default()
            };
            let mut text = String::from("hunter2");
            let mut h = focused(&mut text, &opts);
            let (_, out) = h.frame(|ui| {
                ui.text_field_with("f", RECT, &mut text, &opts);
            });
            let node = out.tree.get(id()).expect("node");
            assert_eq!(node.role, role, "{variant:?}");
            if variant == Variant::Password {
                assert_eq!((&node.value, &node.text), (&None, &None));
            }
        }
    }

    #[test]
    fn a_password_copies_nothing_and_its_selection_fills_the_mask() {
        let opts = FieldOptions {
            variant: Variant::Password,
            ..FieldOptions::default()
        };
        let mut text = String::from("abc");
        let mut h = focused(&mut text, &opts);
        chord(&mut h, "a");
        chord(&mut h, "c");
        let (scene, _, out) = run(&mut h, &mut text, &opts);
        assert!(out.clipboard.is_empty());
        let fills = selection_fills(&scene);
        assert_eq!(fills.len(), 1);
        let dots = "\u{2022}".repeat(MASK_DOTS);
        let style = Style::ui(DARK.font_base, DARK.fg_primary);
        let w = FakeText.measure(&Spec::plain(dots, style), 1.0).0;
        assert_eq!((fills[0].rect[0], fills[0].rect[2]), (RECT.x + 10.0, w));
        chord(&mut h, "x");
        let (_, r, out) = run(&mut h, &mut text, &opts);
        assert!(out.clipboard.is_empty() && !r.response.changed);
        assert_eq!(text, "abc");
    }

    #[test]
    fn home_end_and_a_press_on_a_password_reach_the_real_ends() {
        let opts = FieldOptions {
            variant: Variant::Password,
            ..FieldOptions::default()
        };
        let mut text = "x".repeat(30);
        let mut h = focused(&mut text, &opts);
        press(&mut h, NamedKey::Home);
        run(&mut h, &mut text, &opts);
        typed(&mut h, "a");
        run(&mut h, &mut text, &opts);
        assert!(text.starts_with('a'));
        press(&mut h, NamedKey::End);
        run(&mut h, &mut text, &opts);
        typed(&mut h, "b");
        run(&mut h, &mut text, &opts);
        assert!(text.ends_with('b') && text.len() == 32);
        press(&mut h, NamedKey::Home);
        run(&mut h, &mut text, &opts);
        h.input
            .pointer_to(column(2), CENTRE.1)
            .click(PointerButton::Primary);
        run(&mut h, &mut text, &opts);
        typed(&mut h, "c");
        run(&mut h, &mut text, &opts);
        assert!(
            text.ends_with("bc"),
            "the mask has no positions: the press is at the end"
        );
    }

    #[test]
    fn small_is_24_high_whatever_the_rect() {
        let opts = FieldOptions {
            small: true,
            ..FieldOptions::default()
        };
        let mut h = UiHarness::new(WIN);
        h.input.set_scale(2.0);
        let mut text = String::new();
        let (scene, r, _) = run(&mut h, &mut text, &opts);
        assert_eq!(scene.layers[0].geometry.under[0].rect[3], 48.0);
        assert_eq!(r.response.rect.h, 24.0);
    }

    #[test]
    fn mono_and_date_draw_in_the_mono_face() {
        for variant in [Variant::Mono, Variant::Date] {
            let opts = FieldOptions {
                variant,
                ..FieldOptions::default()
            };
            let mut h = UiHarness::new(WIN);
            let mut text = String::from("id");
            let (scene, _, _) = run(&mut h, &mut text, &opts);
            let style = Style {
                font: Font::Mono,
                ..Style::ui(DARK.font_base, DARK.fg_primary)
            };
            let key = FakeText.prepare(&Spec::plain("id", style), 1.0).0;
            assert_eq!(scene.layers[0].texts[0].key, key, "{variant:?}");
        }
    }

    #[test]
    fn the_search_variant_insets_its_text_and_shows_the_clear_button_while_non_empty() {
        let opts = FieldOptions {
            variant: Variant::Search,
            ..FieldOptions::default()
        };
        let mut h = UiHarness::new(WIN);
        h.input.set_scale(2.0);
        let mut text = String::from("x");
        let (scene, _, out) = run(&mut h, &mut text, &opts);
        // Text 66 device px from the left edge, then the search icon and the clear icon.
        assert_eq!(scene.layers[0].texts[0].x, RECT.x + 33.0);
        assert_eq!(scene.layers[0].texts.len(), 3);
        let clear = out.tree.get(id().with("clear")).expect("clear node");
        assert_eq!(clear.role, Role::Button);
        assert_eq!(clear.name, "Clear");
        assert_eq!((clear.bounds.w, clear.bounds.h), (18.0, 18.0));
        text.clear();
        let (scene, _, out) = run(&mut h, &mut text, &opts);
        assert!(out.tree.get(id().with("clear")).is_none());
        assert_eq!(
            scene.layers[0].texts.len(),
            2,
            "placeholder and search icon"
        );
    }

    #[test]
    fn a_press_on_clear_empties_the_text_and_keeps_focus() {
        let opts = FieldOptions {
            variant: Variant::Search,
            ..FieldOptions::default()
        };
        let mut text = String::from("lune");
        let mut h = focused(&mut text, &opts);
        h.input
            .pointer_to(RECT.right() - 7.0 - 9.0, CENTRE.1)
            .press(PointerButton::Primary);
        let (_, r, out) = run(&mut h, &mut text, &opts);
        assert_eq!(text, "");
        assert!(r.response.changed && r.response.focused);
        assert_eq!(out.focused, Some(id()));
        h.input.release(PointerButton::Primary);
        let (_, r, _) = run(&mut h, &mut text, &opts);
        assert!(r.response.focused && !r.response.changed);
        // It is an edit like any other.
        chord(&mut h, "z");
        run(&mut h, &mut text, &opts);
        assert_eq!(text, "lune");
    }

    #[test]
    fn the_date_variant_keeps_its_padding_clear() {
        let opts = FieldOptions {
            variant: Variant::Date,
            ..FieldOptions::default()
        };
        let mut text = "2".repeat(50);
        let mut h = focused(&mut text, &opts);
        let (scene, _, _) = run(&mut h, &mut text, &opts);
        let clip = scene.layers[0].texts[0].clip;
        assert_eq!(clip.right(), RECT.right() - 28.0);
        let c = caret_shape(&scene).rect;
        assert!(c[0] + c[2] <= RECT.right() - 28.0 - 2.0);
    }

    #[test]
    fn read_only_selects_and_copies_but_never_edits() {
        let opts = FieldOptions {
            read_only: true,
            ..FieldOptions::default()
        };
        let mut text = String::from("lune");
        let mut h = focused(&mut text, &opts);
        typed(&mut h, "x");
        press(&mut h, NamedKey::Backspace);
        run(&mut h, &mut text, &opts);
        assert_eq!(text, "lune");
        chord(&mut h, "a");
        chord(&mut h, "c");
        let (_, _, out) = run(&mut h, &mut text, &opts);
        assert_eq!(out.clipboard, [ClipboardRequest::Write("lune".to_owned())]);
        h.input.access(AccessAction {
            target: id(),
            action: Action::SetValue,
            value: Some(AccessValue::Text("no".to_owned())),
        });
        let (_, r, out) = run(&mut h, &mut text, &opts);
        assert_eq!(text, "lune");
        assert!(!r.response.changed);
        let node = out.tree.get(id()).expect("node");
        assert!(node.states.contains(State::ReadOnly));
        assert!(!node.actions.contains(Action::SetValue));
    }

    #[test]
    fn max_chars_limits_typing_and_set_value_and_set_value_stays_on_one_line() {
        let opts = FieldOptions {
            max_chars: Some(3),
            ..FieldOptions::default()
        };
        let mut text = String::new();
        let mut h = focused(&mut text, &opts);
        typed(&mut h, "abcd");
        run(&mut h, &mut text, &opts);
        assert_eq!(text, "abc");
        h.input.access(AccessAction {
            target: id(),
            action: Action::SetValue,
            value: Some(AccessValue::Text("x\ny\nz".to_owned())),
        });
        run(&mut h, &mut text, &opts);
        assert_eq!(text, "x y");
    }

    #[test]
    fn set_value_clears_the_history_even_for_an_equal_text() {
        let mut text = String::new();
        let mut h = focused(&mut text, &FieldOptions::default());
        typed(&mut h, "abc");
        plain(&mut h, &mut text);
        h.input.access(AccessAction {
            target: id(),
            action: Action::SetValue,
            value: Some(AccessValue::Text("abc".to_owned())),
        });
        let (_, r, out) = plain(&mut h, &mut text);
        assert!(r.response.changed);
        assert_eq!(selection(&out), (3, 3));
        chord(&mut h, "z");
        plain(&mut h, &mut text);
        assert_eq!(text, "abc");
    }

    #[test]
    fn an_external_change_clamps_the_caret_and_clears_undo() {
        let mut text = String::new();
        let mut h = focused(&mut text, &FieldOptions::default());
        typed(&mut h, "abcdef");
        plain(&mut h, &mut text);
        text = String::from("ab");
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (2, 2));
        chord(&mut h, "z");
        plain(&mut h, &mut text);
        assert_eq!(text, "ab");
    }

    #[test]
    fn the_node_carries_text_selection_and_states() {
        let opts = FieldOptions {
            placeholder: "Hint",
            label: Some("Name"),
            description: Some("Pick one"),
            error: true,
            ..FieldOptions::default()
        };
        let mut text = String::from("h\u{e9}llo");
        let mut h = focused(&mut text, &opts);
        chord(&mut h, "a");
        let (_, _, out) = run(&mut h, &mut text, &opts);
        let n = out.tree.get(id()).expect("node");
        assert_eq!(n.role, Role::TextInput);
        assert_eq!(
            (n.name.as_str(), n.description.as_deref()),
            ("Name", Some("Pick one"))
        );
        assert_eq!(n.value, Some(Value::Text(text.clone())));
        let t = n.text.as_ref().expect("text");
        assert_eq!((t.text.as_str(), t.selection), ("h\u{e9}llo", (0, 5)));
        for s in [State::Invalid, State::Focusable, State::Focused] {
            assert!(n.states.contains(s), "{s:?}");
        }
        let mut h = UiHarness::new(WIN);
        let (_, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                ui.text_field_with("f", RECT, &mut text, &opts);
            });
        });
        let n = out.tree.get(id()).expect("node");
        assert!(n.states.contains(State::Disabled) && !n.states.contains(State::Focusable));
    }

    #[test]
    fn the_selection_colour_is_accent_at_0x66() {
        let [r, g, b, _] = DARK.accent;
        assert_eq!(DARK.text_selection, [r, g, b, 0x66]);
    }

    #[test]
    fn a_settled_focused_field_is_idle_and_typing_never_animates() {
        let mut text = String::new();
        let mut h = focused(&mut text, &FieldOptions::default());
        h.at_ms(1000);
        plain(&mut h, &mut text);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(out.repaint, Repaint::Idle);
        typed(&mut h, "a");
        let (scene, _, out) = plain(&mut h, &mut text);
        assert_eq!(out.repaint, Repaint::Now, "one catch-up frame");
        let c = caret_shape(&scene).rect[0];
        assert_eq!(c, column(1) + 1.0, "the caret is there at once");
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(out.repaint, Repaint::Idle);
    }

    #[test]
    fn caret_selection_and_undo_survive_600_hidden_frames_under_keep_alive() {
        let mut text = String::new();
        let mut h = UiHarness::new(WIN);
        let shown = |h: &mut UiHarness, text: &mut String| {
            h.frame(|ui| {
                ui.keep_alive("tab", |ui| {
                    ui.text_field_with("f", RECT, text, &FieldOptions::default());
                });
            })
            .1
        };
        shown(&mut h, &mut text);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        shown(&mut h, &mut text);
        typed(&mut h, "abc");
        shown(&mut h, &mut text);
        shift(&mut h, true);
        press(&mut h, NamedKey::ArrowLeft);
        shift(&mut h, false);
        shown(&mut h, &mut text);
        for _ in 0..600 {
            h.frame(|ui| ui.mark_alive("tab"));
        }
        let out = shown(&mut h, &mut text);
        let sel = out
            .tree
            .nodes
            .iter()
            .find_map(|n| n.text.as_ref())
            .expect("text")
            .selection;
        assert_eq!(sel, (2, 3));
        chord(&mut h, "z");
        shown(&mut h, &mut text);
        assert_eq!(text, "");
    }

    #[test]
    fn outside_a_scope_the_edit_state_is_gone_after_one_undrawn_frame() {
        let mut text = String::from("abc");
        let mut h = focused(&mut text, &FieldOptions::default());
        press(&mut h, NamedKey::Home);
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(selection(&out), (0, 0));
        h.frame(|_| {});
        let (_, _, out) = plain(&mut h, &mut text);
        assert_eq!(
            selection(&out),
            (3, 3),
            "a new field: the caret after the text"
        );
    }

    #[test]
    fn a_bare_field_draws_no_fill_border_or_ring_and_the_box_field_does() {
        use lune_ui_core::widgets::field::FieldChrome;
        let bare = FieldOptions {
            chrome: FieldChrome::Bare,
            placeholder: "Filter",
            ..FieldOptions::default()
        };
        let boxed = FieldOptions {
            placeholder: "Filter",
            ..FieldOptions::default()
        };
        // At rest, hovered and focused (by Tab and by a click), after the transitions settle.
        for reach in [Reach::Rest, Reach::Hover, Reach::Tab, Reach::Click] {
            for (opts, expect_box) in [(&bare, false), (&boxed, true)] {
                let mut text = String::new();
                let mut h = UiHarness::new(WIN);
                run(&mut h, &mut text, opts);
                match reach {
                    Reach::Rest => {}
                    Reach::Hover => {
                        h.input.pointer_to(CENTRE.0, CENTRE.1);
                    }
                    Reach::Tab => {
                        press(&mut h, NamedKey::Tab);
                    }
                    Reach::Click => {
                        h.input
                            .pointer_to(CENTRE.0, CENTRE.1)
                            .click(PointerButton::Primary);
                    }
                }
                h.at_ms(10);
                run(&mut h, &mut text, opts);
                h.at_ms(1000);
                let (scene, _, _) = run(&mut h, &mut text, opts);
                let under = scene.layers[0].geometry.under.len();
                assert_eq!(under > 0, expect_box, "{reach:?} box={expect_box}: {under}");
                if !expect_box {
                    let g = &scene.layers[0].geometry;
                    let focused = matches!(reach, Reach::Tab | Reach::Click);
                    assert_eq!(g.under.len(), 0, "{reach:?}: no fill, border or ring");
                    assert_eq!(g.over.len(), usize::from(focused), "{reach:?}: caret only");
                }
            }
        }
    }

    #[test]
    fn a_bare_field_has_its_caret_in_fg_tertiary_and_takes_typing_and_focus() {
        use lune_ui_core::widgets::field::FieldChrome;
        let opts = FieldOptions {
            chrome: FieldChrome::Bare,
            ..FieldOptions::default()
        };
        let mut text = String::new();
        let mut h = focused(&mut text, &opts);
        typed(&mut h, "hi");
        let (scene, r, out) = run(&mut h, &mut text, &opts);
        assert_eq!(text, "hi");
        assert!(r.response.changed && r.response.focused);
        assert_eq!(caret_shape(&scene).color, premul(DARK.fg_tertiary));
        // The node is the box field's: a focused text input holding the text.
        let node = out.tree.get(id()).expect("node");
        assert_eq!(node.role, Role::TextInput);
        assert!(node.states.contains(State::Focused));
        assert_eq!(selection(&out), (2, 2));
        // The box field's caret stays `fg_primary`.
        let mut text = String::new();
        let mut h = focused(&mut text, &FieldOptions::default());
        typed(&mut h, "hi");
        let (scene, _, _) = plain(&mut h, &mut text);
        assert_eq!(caret_shape(&scene).color, premul(DARK.fg_primary));
    }
}

/// One frame of `draw` inside a parent that scrolls over 1000 px on both axes.
fn in_parent(
    h: &mut UiHarness,
    ms: u64,
    draw: impl FnOnce(&mut lune_ui_core::ui::Ui<'_>),
) -> FrameOutput {
    let opts = ScrollOptions {
        extent: Some((1000.0, 1000.0)),
        ..ScrollOptions::default()
    };
    let area = Rect::new(0.0, 0.0, 380.0, 280.0);
    h.at_ms(ms)
        .frame(|ui| {
            ui.scroll_area("o", area, &opts, |ui, _| {
                draw(ui);
                ((), Extent::default())
            });
        })
        .1
}

fn selection_of(out: &FrameOutput, key: &str) -> (usize, usize) {
    let node = out.tree.get(WidgetId::ROOT.with(key)).expect("node");
    node.text.as_ref().expect("text").selection
}

#[test]
fn a_press_and_a_drag_place_the_caret_in_a_field_inside_a_scrolled_parent() {
    let glyph = DARK.font_base * 0.5;
    let mut text = String::from("abcdefghij");
    // Content (35, 300) at offset (15, 200) is screen (20, 100); the text starts 10 px in.
    let rect = Rect::new(35.0, 300.0, 140.0, 32.0);
    let mut h = UiHarness::new(WIN);
    let draw = |ui: &mut lune_ui_core::ui::Ui<'_>, text: &mut String| {
        ui.text_field_with("f", rect, text, &FieldOptions::default());
    };
    h.input.pointer_to(370.0, 270.0);
    in_parent(&mut h, 0, |ui| draw(ui, &mut text));
    h.input.scroll_pixels(-15.0, -200.0);
    in_parent(&mut h, 5, |ui| draw(ui, &mut text));
    in_parent(&mut h, 10, |ui| draw(ui, &mut text));
    in_parent(&mut h, 20, |ui| draw(ui, &mut text));
    h.input.pointer_to(30.0 + 3.0 * glyph, 116.0);
    in_parent(&mut h, 30, |ui| draw(ui, &mut text));
    h.input.press(PointerButton::Primary);
    let out = in_parent(&mut h, 40, |ui| draw(ui, &mut text));
    assert_eq!(selection_of(&out, "f"), (3, 3));
    h.input.pointer_to(30.0 + 6.0 * glyph, 116.0);
    in_parent(&mut h, 50, |ui| draw(ui, &mut text));
    let out = in_parent(&mut h, 60, |ui| draw(ui, &mut text));
    assert_eq!(selection_of(&out, "f"), (3, 6));
}

#[test]
fn a_press_picks_the_line_in_a_text_area_inside_a_scrolled_parent() {
    let glyph = DARK.font_base * 0.5;
    let line = Style::ui(DARK.font_base, DARK.fg_primary).line;
    let mut text = String::from("aaaa\nbbbb\ncccc");
    // Content (35, 300) at offset (15, 200) is screen (20, 100); the text starts (10, 7) in.
    let rect = Rect::new(35.0, 300.0, 200.0, 100.0);
    let mut h = UiHarness::new(WIN);
    let draw = |ui: &mut lune_ui_core::ui::Ui<'_>, text: &mut String| {
        ui.text_area("a", rect, text, &AreaOptions::default());
    };
    h.input.pointer_to(370.0, 270.0);
    in_parent(&mut h, 0, |ui| draw(ui, &mut text));
    h.input.scroll_pixels(-15.0, -200.0);
    in_parent(&mut h, 5, |ui| draw(ui, &mut text));
    in_parent(&mut h, 10, |ui| draw(ui, &mut text));
    in_parent(&mut h, 20, |ui| draw(ui, &mut text));
    h.input.pointer_to(30.0 + 2.0 * glyph, 107.0 + 1.5 * line);
    in_parent(&mut h, 30, |ui| draw(ui, &mut text));
    h.input.press(PointerButton::Primary);
    let out = in_parent(&mut h, 40, |ui| draw(ui, &mut text));
    assert_eq!(selection_of(&out, "a"), (7, 7));
}

/// Class 1 of D177: a text field, area or filter bar that holds the key focus when its scope turns
/// disabled takes the keys it takes enabled and changes nothing.
mod disabled_keys {
    use super::*;
    use lune_ui_core::input::Modifiers;
    use lune_ui_core::shortcuts::{Binding, Chord, Scope};
    use lune_ui_core::ui::Ui;
    use lune_ui_core::widgets::filter_bar::FilterOptions;

    type Draw = fn(&mut Ui<'_>, &mut String);

    /// A key to press: its name, the key, the text it types, and whether the primary modifier is
    /// held.
    type Probe = (&'static str, Key, Option<&'static str>, bool);

    const AREA: Rect = Rect::new(20.0, 20.0, 200.0, 100.0);

    fn field(ui: &mut Ui<'_>, text: &mut String) {
        ui.text_field_with("f", RECT, text, &FieldOptions::default());
    }

    fn area(ui: &mut Ui<'_>, text: &mut String) {
        ui.text_area("f", AREA, text, &AreaOptions::default());
    }

    fn bar(ui: &mut Ui<'_>, text: &mut String) {
        ui.filter_bar("f", RECT, text, &FilterOptions::default());
    }

    fn named(name: &'static str, key: NamedKey, typed: Option<&'static str>) -> Probe {
        (name, Key::Named(key), typed, false)
    }

    /// Command plus a letter.
    fn primary(name: &'static str, letter: &'static str) -> Probe {
        (name, Key::Character(letter.to_owned()), None, true)
    }

    /// The "Does not hold" keys of a one-line box.
    fn line_keys() -> Vec<Probe> {
        vec![
            named("Enter", NamedKey::Enter, None),
            named("Space", NamedKey::Space, Some(" ")),
            named("Backspace", NamedKey::Backspace, None),
            named("Delete", NamedKey::Delete, None),
            named("Left", NamedKey::ArrowLeft, None),
            named("Right", NamedKey::ArrowRight, None),
            named("Home", NamedKey::Home, None),
            named("End", NamedKey::End, None),
            ("a typed j", Key::Character("j".into()), Some("j"), false),
            primary("Command+A", "a"),
            primary("Command+C", "c"),
            primary("Command+X", "x"),
            primary("Command+V", "v"),
            primary("Command+Z", "z"),
        ]
    }

    fn area_keys() -> Vec<Probe> {
        let mut keys = line_keys();
        keys.extend([
            named("Up", NamedKey::ArrowUp, None),
            named("Down", NamedKey::ArrowDown, None),
            named("PageUp", NamedKey::PageUp, None),
            named("PageDown", NamedKey::PageDown, None),
        ]);
        keys
    }

    fn frame(h: &mut UiHarness, draw: Draw, text: &mut String, enabled: bool) -> FrameOutput {
        h.frame(|ui| {
            if enabled {
                draw(ui, text);
            } else {
                ui.with_enabled(false, |ui| draw(ui, text));
            }
        })
        .1
    }

    fn press(h: &mut UiHarness, (_, key, typed, primary): &Probe) {
        if *primary {
            h.input.set_modifiers(Modifiers {
                logo: true,
                ..Modifiers::default()
            });
        }
        h.input.key_down(key.clone(), *typed);
        h.input.set_modifiers(Modifiers::default());
    }

    fn selection_in(out: &FrameOutput) -> (usize, usize) {
        out.tree
            .get(id())
            .and_then(|n| n.text.as_ref())
            .expect("the field's text")
            .selection
    }

    fn binding(probe: &Probe) -> Binding {
        let (name, key, _, primary) = probe;
        let chord = Chord::key(key.clone());
        Binding::new(
            Scope::Global,
            if *primary { chord.primary() } else { chord },
            *name,
        )
    }

    /// A box focused with all its text selected, bound on `probe`'s key and on F5.
    fn start(draw: Draw, probe: &Probe, text: &mut String) -> UiHarness {
        let mut h = UiHarness::new(WIN);
        frame(&mut h, draw, text, true);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        frame(&mut h, draw, text, true);
        press(&mut h, &primary("select all", "a"));
        frame(&mut h, draw, text, true);
        h.shortcuts.add(binding(probe)).add(Binding::new(
            Scope::Global,
            Chord::key(Key::Named(NamedKey::F5)),
            "f5",
        ));
        h
    }

    fn hold(draw: Draw, probes: &[Probe]) {
        for probe in probes {
            let name = probe.0;
            // The same key, enabled: the box takes it, so the probe proves something.
            let mut text = String::from("one two\nthree");
            let mut h = start(draw, probe, &mut text);
            press(&mut h, probe);
            let out = frame(&mut h, draw, &mut text, true);
            assert!(out.fired.is_empty(), "{name}: enabled, it fired");

            let mut text = String::from("one two\nthree");
            let mut h = start(draw, probe, &mut text);
            let before = selection_in(&frame(&mut h, draw, &mut text, false));
            assert_eq!(before, (0, 13), "{name}: the selection survives disabling");
            press(&mut h, probe);
            let out = frame(&mut h, draw, &mut text, false);
            assert!(out.fired.is_empty(), "{name}: disabled, it fired a binding");
            assert_eq!(text, "one two\nthree", "{name}: the text changed");
            assert_eq!(selection_in(&out), before, "{name}: the selection moved");
            assert!(out.clipboard.is_empty(), "{name}: the clipboard was used");
            assert_eq!(out.focused, Some(id()), "{name}: focus left");
            h.input.key_down(Key::Named(NamedKey::F5), None);
            let out = frame(&mut h, draw, &mut text, false);
            assert_eq!(out.fired.len(), 1, "{name}: F5 no longer fires");
        }
    }

    #[test]
    fn a_text_field_focused_then_disabled_takes_its_keys_and_changes_nothing() {
        hold(field, &line_keys());
    }

    #[test]
    fn a_text_area_focused_then_disabled_takes_its_keys_and_changes_nothing() {
        hold(area, &area_keys());
    }

    #[test]
    fn a_filter_bar_focused_then_disabled_takes_its_keys_and_changes_nothing() {
        hold(bar, &line_keys());
    }

    /// A key the engine declines still fires its binding, enabled and disabled.
    #[test]
    fn a_key_the_engine_declines_still_fires_its_binding() {
        for draw in [field as Draw, area, bar] {
            for enabled in [true, false] {
                // Command+C with nothing selected.
                let mut text = String::from("abc");
                let mut h = UiHarness::new(WIN);
                frame(&mut h, draw, &mut text, true);
                h.input.key_down(Key::Named(NamedKey::Tab), None);
                frame(&mut h, draw, &mut text, true);
                let copy = primary("Command+C", "c");
                h.shortcuts.add(binding(&copy));
                frame(&mut h, draw, &mut text, enabled);
                press(&mut h, &copy);
                let out = frame(&mut h, draw, &mut text, enabled);
                assert_eq!(out.fired.len(), 1, "enabled={enabled}");
                assert!(out.clipboard.is_empty());
            }
        }
    }

    /// A clipboard answer that arrives in a disabled frame inserts nothing, and the text stays
    /// what it was after the scope is enabled again.
    #[test]
    fn a_clipboard_answer_in_a_disabled_frame_does_not_insert() {
        for draw in [field as Draw, area, bar] {
            let mut text = String::from("abc");
            let paste = primary("Command+V", "v");
            let mut h = start(draw, &paste, &mut text);
            press(&mut h, &paste);
            frame(&mut h, draw, &mut text, true);
            h.input.clipboard_text("zzz");
            let out = frame(&mut h, draw, &mut text, false);
            assert_eq!(text, "abc");
            assert!(out.fired.is_empty());
            frame(&mut h, draw, &mut text, true);
            assert_eq!(text, "abc", "the answer is not kept for later");
        }
    }

    /// A read-only box declines edits even disabled, so their bindings fire.
    #[test]
    fn a_read_only_box_disabled_still_declines_an_edit() {
        let mut text = String::from("abc");
        let opts = FieldOptions {
            read_only: true,
            ..FieldOptions::default()
        };
        let mut h = UiHarness::new(WIN);
        let go = |h: &mut UiHarness, text: &mut String, enabled: bool| {
            h.frame(|ui| {
                ui.with_enabled(enabled, |ui| {
                    ui.text_field_with("f", RECT, text, &opts);
                });
            })
            .1
        };
        go(&mut h, &mut text, true);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        go(&mut h, &mut text, true);
        let back = named("Backspace", NamedKey::Backspace, None);
        h.shortcuts.add(binding(&back));
        go(&mut h, &mut text, false);
        press(&mut h, &back);
        let out = go(&mut h, &mut text, false);
        assert_eq!(out.fired.len(), 1);
        assert_eq!(text, "abc");
    }
}
