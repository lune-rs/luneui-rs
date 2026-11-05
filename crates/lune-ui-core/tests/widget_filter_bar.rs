//! The filter bar (`widget.filter-bar` acceptance): typing, the clear button and where it leaves
//! focus, a press on the surface, the disabled bar, the bare chrome, probes at scale 2 and the
//! nodes.

use lune_ui_core::access::{AccessAction, Action, Role, SemanticTree, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::text::Spec;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::field::FieldResponse;
use lune_ui_core::widgets::filter_bar::{CLEAR, CLEAR_ICON, FilterOptions, ICON, RADIUS};
use lune_ui_core::widgets::label;

const WIN: (f32, f32) = (600.0, 300.0);
const RECT: Rect = Rect::new(100.0, 100.0, 300.0, 30.0);
const P: PointerButton = PointerButton::Primary;
const OPTS: FilterOptions<'static> = FilterOptions {
    placeholder: "Filter",
    label: None,
    clear_unfocusable: false,
};
/// Where the search icon's centre is, on the surface and off the field.
const ON_ICON: (f32, f32) = (115.0, 115.0);
/// A point in the text.
const ON_TEXT: (f32, f32) = (150.0, 115.0);
/// The clear button's centre.
const ON_CLEAR: (f32, f32) = (383.0, 115.0);

fn id() -> WidgetId {
    WidgetId::ROOT.with("f")
}

fn clear_id() -> WidgetId {
    id().with("clear")
}

struct Seen {
    scene: Scene,
    resp: FieldResponse,
    tree: SemanticTree,
    focused: Option<WidgetId>,
    repaint: Repaint,
}

fn frame(h: &mut UiHarness, text: &mut String) -> Seen {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| resp = Some(ui.filter_bar("f", RECT, text, &OPTS)));
    Seen {
        scene,
        resp: resp.expect("frame ran"),
        tree: out.tree,
        focused: out.focused,
        repaint: out.repaint,
    }
}

fn click(h: &mut UiHarness, at: (f32, f32)) {
    h.input.pointer_to(at.0, at.1);
    h.input.click(P);
}

/// A harness with the bar drawn once.
fn start(text: &mut String) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, text);
    h
}

/// A click on the text; two frames, as the press lands at the frame's begin.
fn focus(h: &mut UiHarness, text: &mut String) {
    click(h, ON_TEXT);
    frame(h, text);
    frame(h, text);
}

fn type_text(h: &mut UiHarness, s: &str) {
    h.input.key_down(Key::Character(s.into()), Some(s));
}

#[test]
fn typing_sets_the_text_and_marks_it_changed() {
    let mut text = String::new();
    let mut h = start(&mut text);
    focus(&mut h, &mut text);
    type_text(&mut h, "a");
    let s = frame(&mut h, &mut text);
    assert_eq!(text, "a");
    assert!(s.resp.response.changed);
    assert_eq!(s.resp.response.id, id(), "the response names the field");
    assert!(s.tree.get(clear_id()).is_some(), "the button comes with it");
}

#[test]
fn the_clear_button_shows_only_while_there_is_text() {
    let mut text = String::new();
    let mut h = start(&mut text);
    let s = frame(&mut h, &mut text);
    assert!(s.tree.get(clear_id()).is_none());
    let mut text = "abc".to_owned();
    let s = frame(&mut h, &mut text);
    let node = s.tree.get(clear_id()).expect("a clear button");
    assert_eq!(node.role, Role::Button);
    assert_eq!(node.name, "Clear");
    assert_eq!((node.bounds.w, node.bounds.h), (CLEAR, CLEAR));
}

#[test]
fn a_click_on_clear_empties_the_text_hides_it_and_leaves_focus_on_the_field() {
    let mut text = "abc".to_owned();
    let mut h = start(&mut text);
    focus(&mut h, &mut text);
    assert_eq!(frame(&mut h, &mut text).focused, Some(id()));
    click(&mut h, ON_CLEAR);
    let s = frame(&mut h, &mut text);
    assert_eq!(text, "");
    assert!(s.resp.response.changed);
    assert_ne!(s.repaint, Repaint::Idle, "the frame after shows it");

    let s = frame(&mut h, &mut text);
    assert!(s.tree.get(clear_id()).is_none(), "it hides");
    assert_eq!(s.focused, Some(id()));
    assert!(!s.resp.response.changed, "one change");
    let field = s.tree.get(id()).expect("the field");
    assert_eq!(field.value, Some(Value::Text(String::new())));

    type_text(&mut h, "x");
    frame(&mut h, &mut text);
    assert_eq!(text, "x", "typing still reaches the field");
}

#[test]
fn clear_from_an_unfocused_bar_focuses_the_field() {
    let mut text = "abc".to_owned();
    let mut h = start(&mut text);
    click(&mut h, ON_CLEAR);
    frame(&mut h, &mut text);
    assert_eq!(text, "");
    assert_eq!(frame(&mut h, &mut text).focused, Some(id()));
}

#[test]
fn the_tab_order_is_the_field_then_clear_and_enter_clears() {
    let mut text = "abc".to_owned();
    let mut h = start(&mut text);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    assert_eq!(frame(&mut h, &mut text).focused, Some(id()));
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    assert_eq!(frame(&mut h, &mut text).focused, Some(clear_id()));
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let s = frame(&mut h, &mut text);
    assert_eq!(text, "");
    assert!(s.resp.response.changed);
    assert_eq!(frame(&mut h, &mut text).focused, Some(id()));
}

#[test]
fn the_default_action_on_clear_clears() {
    let mut text = "abc".to_owned();
    let mut h = start(&mut text);
    h.input.access(AccessAction {
        target: clear_id(),
        action: Action::Default,
        value: None,
    });
    frame(&mut h, &mut text);
    assert_eq!(text, "");
}

#[test]
fn a_clear_settles_to_idle() {
    let mut text = "abc".to_owned();
    let mut h = start(&mut text);
    click(&mut h, ON_CLEAR);
    frame(&mut h, &mut text);
    h.at_ms(1000);
    frame(&mut h, &mut text);
    h.at_ms(2000);
    frame(&mut h, &mut text);
    assert_eq!(frame(&mut h, &mut text).repaint, Repaint::Idle);
}

#[test]
fn a_press_on_the_surface_outside_the_field_focuses_the_field() {
    let mut text = String::new();
    let mut h = start(&mut text);
    h.input.pointer_to(ON_ICON.0, ON_ICON.1);
    h.input.press(P);
    frame(&mut h, &mut text);
    assert_eq!(frame(&mut h, &mut text).focused, Some(id()), "on the press");
    h.input.release(P);
    frame(&mut h, &mut text);
    type_text(&mut h, "a");
    frame(&mut h, &mut text);
    assert_eq!(text, "a");
}

#[test]
fn a_disabled_bar_takes_no_text_no_clear_and_no_focus() {
    let mut text = "abc".to_owned();
    let mut h = UiHarness::new(WIN);
    let disabled = |h: &mut UiHarness, text: &mut String| {
        let (_, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| ui.filter_bar("f", RECT, text, &OPTS));
        });
        out
    };
    disabled(&mut h, &mut text);
    click(&mut h, ON_CLEAR);
    disabled(&mut h, &mut text);
    assert_eq!(text, "abc");
    click(&mut h, ON_ICON);
    disabled(&mut h, &mut text);
    click(&mut h, ON_TEXT);
    disabled(&mut h, &mut text);
    type_text(&mut h, "x");
    let out = disabled(&mut h, &mut text);
    assert_eq!(text, "abc");
    assert_eq!(out.focused, None);
    let field = out.tree.get(id()).expect("the field");
    assert!(field.states.contains(State::Disabled));
    let clear = out.tree.get(clear_id()).expect("the clear button");
    assert!(clear.states.contains(State::Disabled));
}

#[test]
fn focus_shows_the_caret_only_no_border_and_no_ring() {
    let mut text = "abc".to_owned();
    let mut h = start(&mut text);
    let rest = frame(&mut h, &mut text);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    frame(&mut h, &mut text);
    h.at_ms(1000);
    frame(&mut h, &mut text);
    let focused = frame(&mut h, &mut text);
    let g = &focused.scene.layers[0].geometry;
    let shapes = || g.under.iter().chain(&g.over);
    assert!(!shapes().any(|s| s.color == premul(DARK.accent)), "no ring");
    let caret = premul(DARK.fg_tertiary);
    assert_eq!(shapes().filter(|s| s.color == caret).count(), 1, "a caret");
    let at_rest = &rest.scene.layers[0].geometry;
    assert!(!at_rest.over.iter().any(|s| s.color == caret));
}

#[test]
fn the_node_is_a_text_input_named_by_the_label_or_the_placeholder() {
    let mut text = "abc".to_owned();
    let mut h = start(&mut text);
    let s = frame(&mut h, &mut text);
    let node = s.tree.get(id()).expect("a node");
    assert_eq!(node.role, Role::TextInput);
    assert_eq!(node.name, "Filter");
    assert_eq!(node.value, Some(Value::Text("abc".to_owned())));
    assert!(node.actions.contains(Action::SetValue));
    assert!(node.states.contains(State::Focusable));

    let named = FilterOptions {
        label: Some("Files"),
        ..OPTS
    };
    let (_, out) = h.frame(|ui| {
        ui.filter_bar("f", RECT, &mut text, &named);
    });
    assert_eq!(out.tree.get(id()).expect("a node").name, "Files");
}

#[test]
fn hover_changes_nothing_on_the_surface() {
    let mut text = String::new();
    let mut h = start(&mut text);
    let rest = frame(&mut h, &mut text);
    h.input.pointer_to(ON_ICON.0, ON_ICON.1);
    frame(&mut h, &mut text);
    h.at_ms(1000);
    frame(&mut h, &mut text);
    let hovered = frame(&mut h, &mut text);
    assert_eq!(rest.scene, hovered.scene);
}

#[test]
fn pixel_probes_at_scale_2() {
    let mut text = "abc".to_owned();
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let s = frame(&mut h, &mut text);
    let layer = &s.scene.layers[0];
    let surface = layer
        .geometry
        .under
        .iter()
        .find(|x| x.color == premul(DARK.surface_input))
        .expect("the surface");
    assert_eq!(
        surface.rect,
        [RECT.x * 2.0, RECT.y * 2.0, RECT.w * 2.0, RECT.h * 2.0]
    );
    assert_eq!(surface.radii, [RADIUS * 2.0; 4], "radius 8 px");

    let key = |spec: &Spec| spec.cache_key(2.0);
    let item = |spec: &Spec| {
        *layer
            .texts
            .iter()
            .find(|t| t.key.0 == key(spec))
            .expect("the text")
    };
    let search = icon::glyph(icon::SEARCH, ICON, DARK.fg_dim);
    let close = icon::glyph(icon::CHROME_CLOSE, CLEAR_ICON, DARK.fg_dim);
    // A text item's position stays logical. FakeText's glyph is half an em wide: 7 of the 14 px
    // box, centred.
    assert_eq!(
        item(&search).x,
        RECT.x + 8.0 + 3.5,
        "icon 28 px at the left"
    );
    let body = label(&DARK, "abc", DARK.fg_primary);
    assert_eq!(item(&body).x, RECT.x + 30.0, "text after the icon's margin");
    let glyph = item(&close).x;
    let clear = RECT.right() - 8.0 - CLEAR;
    assert_eq!(
        glyph,
        clear + (CLEAR - 6.0) / 2.0,
        "12 px glyph in the 36 px box"
    );
}
