//! The form field (`widget.form-field` acceptance): the stacked and the inline layout, the label
//! and the hint, the error ring (one, even around a text field) and the error text, a label press
//! focusing the control without being a tab stop, and the `Group` node with its `Invalid` state.

use lune_ui_core::access::{Role, SemanticTree, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape, TextItem};
use lune_ui_core::testing::{ScriptedInput, UiHarness};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::field::FieldState;
use lune_ui_core::widgets::form_field::FormField;

const WIN: (f32, f32) = (600.0, 300.0);
const RECT: Rect = Rect::new(20.0, 30.0, 300.0, 0.0);

struct Drawn {
    scene: Scene,
    tree: SemanticTree,
    height: f32,
    control: Response,
    rect: Rect,
}

fn draw(h: &mut UiHarness, field: &FormField<'_>, text: &mut String) -> Drawn {
    let (mut height, mut control, mut rect) = (0.0, None, Rect::new(0.0, 0.0, 0.0, 0.0));
    let (scene, out) = h.frame(|ui| {
        height = ui.form_field("ff", RECT, field, |ui, r| {
            rect = r;
            let resp = ui.text_field("f", r, text, "Name", FieldState::default());
            let id = resp.id;
            control = Some(resp);
            id
        });
    });
    Drawn {
        scene,
        tree: out.tree,
        height,
        control: control.expect("the control ran"),
        rect,
    }
}

fn field(label: &'static str) -> FormField<'static> {
    FormField {
        label,
        hint: Some("Shown to others"),
        error: None,
        inline: false,
        control_size: (0.0, 28.0),
    }
}

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers.iter().flat_map(|l| l.texts.clone()).collect()
}

fn shapes(scene: &Scene) -> Vec<Shape> {
    let g = &scene.layers[0].geometry;
    g.under.iter().chain(&g.over).copied().collect()
}

fn label_key(text: &str) -> u64 {
    Spec::plain(
        text,
        Style {
            line: 20.0,
            ..Style::ui(13.0, DARK.fg_secondary)
        },
    )
    .cache_key(1.0)
}

fn note_key(text: &str, color: [u8; 4]) -> u64 {
    Spec::plain(
        text,
        Style {
            line: 16.0,
            ..Style::ui(11.0, color)
        },
    )
    .wrapped(300.0)
    .cache_key(1.0)
}

fn ctl_id() -> WidgetId {
    WidgetId::ROOT.with("ff").with("f")
}

#[test]
fn stacked_the_label_is_20_high_then_4_px_then_the_control_then_4_px_then_the_hint() {
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &field("Name"), &mut String::new());
    assert_eq!(
        d.rect,
        Rect::new(20.0, 54.0, 300.0, 28.0),
        "the control's rect"
    );
    // 20 + 4 + 28 + 4 + 16.
    assert_eq!(d.height, 72.0);
    let t = texts(&d.scene);
    assert_eq!(
        t[0].key.0,
        label_key("Name"),
        "13 on a 20 px line, secondary"
    );
    assert_eq!((t[0].x, t[0].y), (20.0, 30.0));
    let hint = t
        .iter()
        .find(|i| i.key.0 == note_key("Shown to others", DARK.fg_tertiary));
    let hint = hint.expect("11 on a 16 px line, tertiary");
    assert_eq!((hint.x, hint.y), (20.0, 30.0 + 20.0 + 4.0 + 28.0 + 4.0));
}

#[test]
fn without_a_hint_the_field_ends_at_the_control() {
    let mut h = UiHarness::new(WIN);
    let f = FormField {
        hint: None,
        ..field("Name")
    };
    assert_eq!(draw(&mut h, &f, &mut String::new()).height, 52.0);
}

#[test]
fn inline_the_label_is_at_the_left_centred_on_the_control_with_the_hint_under_it() {
    let mut h = UiHarness::new(WIN);
    let f = FormField {
        inline: true,
        control_size: (120.0, 28.0),
        ..field("Name")
    };
    let d = draw(&mut h, &f, &mut String::new());
    // "Name" is 4 chars of 13 px at half the size: 26 px, then 8.
    assert_eq!(d.rect, Rect::new(20.0 + 26.0 + 8.0, 30.0, 120.0, 28.0));
    let label = &texts(&d.scene)[0];
    assert_eq!(
        (label.x, label.y),
        (20.0, 30.0 + 4.0),
        "centred on the 28 px control"
    );
    assert_eq!(d.height, 28.0 + 4.0 + 16.0);
    let hint = texts(&d.scene)
        .into_iter()
        .find(|i| i.y == 30.0 + 28.0 + 4.0)
        .expect("the hint under the control");
    assert_eq!(hint.x, 54.0);
}

#[test]
fn at_scale_2_the_gap_is_8_px_and_the_error_ring_2_px_in_err_at_the_control_rect() {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let f = FormField {
        error: Some("Required"),
        ..field("Name")
    };
    let d = draw(&mut h, &f, &mut String::new());
    assert_eq!((d.rect.y - (RECT.y + 20.0)) * 2.0, 8.0);
    let ring: Vec<Shape> = shapes(&d.scene)
        .into_iter()
        .filter(|s| s.color == premul(DARK.err) && s.rect == [40.0, 108.0, 600.0, 56.0])
        .collect();
    assert_eq!(ring.len(), 1);
    assert_eq!(ring[0].params[1], 2.0, "2 px");
}

#[test]
fn an_error_draws_one_ring_even_around_a_text_field_and_replaces_the_hint() {
    let mut h = UiHarness::new(WIN);
    let ok = draw(&mut h, &field("Name"), &mut String::new());
    let rect = [ok.rect.x, ok.rect.y, ok.rect.w, ok.rect.h];
    let in_err = |d: &Drawn| {
        shapes(&d.scene)
            .iter()
            .filter(|s| s.color == premul(DARK.err) && s.rect == rect)
            .count()
    };
    assert_eq!(in_err(&ok), 0);
    let f = FormField {
        error: Some("Required"),
        ..field("Name")
    };
    let bad = draw(&mut h, &f, &mut String::new());
    assert_eq!(in_err(&bad), 1, "one ring at the control rect");
    let t = texts(&bad.scene);
    assert!(t.iter().any(|i| i.key.0 == note_key("Required", DARK.err)));
    assert!(
        !t.iter()
            .any(|i| i.key.0 == note_key("Shown to others", DARK.fg_tertiary))
    );
    assert_eq!(bad.height, ok.height, "the error takes the hint's place");
}

#[test]
fn a_label_press_focuses_the_control_and_the_label_is_no_tab_stop() {
    let mut h = UiHarness::new(WIN);
    let mut text = String::new();
    let d = draw(&mut h, &field("Name"), &mut text);
    assert!(!d.control.focused);
    // The label: 26 by 20 at the field's top left.
    h.input.pointer_to(30.0, 40.0).click(PointerButton::Primary);
    h.at_ms(10);
    draw(&mut h, &field("Name"), &mut text);
    h.at_ms(20);
    let d = draw(&mut h, &field("Name"), &mut text);
    assert!(d.control.focused, "the press moved focus to the control");
    assert!(!d.control.focus_visible, "a press shows no ring");

    // Tab from nothing goes to the control: the label took no stop.
    let mut h = UiHarness::new(WIN);
    draw(&mut h, &field("Name"), &mut text);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    let d = draw(&mut h, &field("Name"), &mut text);
    assert!(d.control.focused && d.control.focus_visible);
}

#[test]
fn a_disabled_field_ignores_the_label_press() {
    let mut h = UiHarness::new(WIN);
    let mut text = String::new();
    let mut control = None;
    let mut frame = |h: &mut UiHarness| {
        h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                ui.form_field("ff", RECT, &field("Name"), |ui, r| {
                    let resp = ui.text_field("f", r, &mut text, "Name", FieldState::default());
                    let id = resp.id;
                    control = Some(resp);
                    id
                });
            });
        });
    };
    frame(&mut h);
    h.input.pointer_to(30.0, 40.0).click(PointerButton::Primary);
    h.at_ms(10);
    frame(&mut h);
    h.at_ms(20);
    frame(&mut h);
    assert!(!control.expect("ran").focused);
}

#[test]
fn the_node_is_a_group_named_by_the_label_invalid_with_an_error() {
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &field("Name"), &mut String::new());
    let group = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Group)
        .expect("a group");
    assert_eq!(group.name, "Name");
    assert_eq!(group.description.as_deref(), Some("Shown to others"));
    assert!(!group.states.contains(State::Invalid));
    let control = d.tree.get(ctl_id()).expect("the control's node");
    assert_eq!(control.parent, Some(group.id));

    let f = FormField {
        error: Some("Required"),
        ..field("Name")
    };
    let d = draw(&mut h, &f, &mut String::new());
    let group = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Group)
        .expect("a group");
    assert!(group.states.contains(State::Invalid));
    assert_eq!(group.description.as_deref(), Some("Shown to others"));
    let error = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::StaticText && n.value == Some(Value::Text("Required".into())))
        .expect("the error text");
    assert_eq!(error.parent, Some(group.id));
}
