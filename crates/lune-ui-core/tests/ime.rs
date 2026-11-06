//! Composition in a text field (`text.ime`): the event order, offsets, drawing, `ime_area`, keys,
//! focus and the node. The last acceptance line, real typing with an input method, is unverified
//! here (no hands-on run).

use lune_ui_core::access::{Node, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, ImeEvent, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Dismissal, OverlayOptions, Sense, WidgetId};
use lune_ui_core::widgets::field::{FieldOptions, FieldResponse, Variant};
use lune_ui_core::widgets::menu::{CLOSE, OPEN};

const WIN: (f32, f32) = (400.0, 300.0);
const RECT: Rect = Rect::new(20.0, 20.0, 140.0, 32.0);
const OTHER: Rect = Rect::new(20.0, 80.0, 140.0, 32.0);
const TX: f32 = 30.0;

fn id() -> WidgetId {
    WidgetId::ROOT.with("f")
}

fn glyph() -> f32 {
    DARK.font_base * 0.5
}

fn run(h: &mut UiHarness, text: &mut String) -> (Scene, FieldResponse, FrameOutput) {
    with(h, text, &FieldOptions::default())
}

fn with(
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

fn focused(text: &mut String) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    run(&mut h, text);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    run(&mut h, text);
    h
}

fn preedit(h: &mut UiHarness, text: &str, cursor: Option<(usize, usize)>) {
    h.input.ime(ImeEvent::Preedit {
        text: text.to_owned(),
        cursor,
    });
}

/// The thin rects drawn over the text, by height.
fn underlines(scene: &Scene, height: f32) -> Vec<Shape> {
    let over = &scene.layers[0].geometry.over;
    over.iter()
        .filter(|s| s.rect[3] == height && s.color == premul(DARK.fg_primary))
        .copied()
        .collect()
}

fn carets(scene: &Scene) -> Vec<Shape> {
    let over = &scene.layers[0].geometry.over;
    over.iter().filter(|s| s.rect[2] == 1.5).copied().collect()
}

fn spec_key(text: &str) -> lune_ui_core::text::TextKey {
    let style = Style::ui(DARK.font_base, DARK.fg_primary);
    FakeText.prepare(&Spec::plain(text, style), 1.0).0
}

#[test]
fn enabled_preedit_commit_disabled_leaves_the_text_and_the_caret_after_it() {
    let mut text = String::new();
    let mut h = focused(&mut text);
    h.input.ime(ImeEvent::Enabled);
    run(&mut h, &mut text);
    preedit(&mut h, "か", Some((0, 3)));
    let (_, r, _) = run(&mut h, &mut text);
    assert_eq!(text, "");
    assert!(!r.response.changed);
    h.input.ime(ImeEvent::Commit("か".to_owned()));
    let (scene, r, out) = run(&mut h, &mut text);
    assert_eq!(text, "か");
    assert!(r.response.changed);
    let n = out.tree.get(id()).expect("node");
    assert_eq!(n.text.as_ref().map(|t| t.selection), Some((1, 1)));
    assert!(underlines(&scene, 1.0).is_empty(), "no preedit afterwards");
    h.input.ime(ImeEvent::Disabled);
    let (_, _, _) = run(&mut h, &mut text);
    assert_eq!(text, "か");
    // Focus leaves with the field: no widget reports an area.
    let (_, out) = h.frame(|_| {});
    assert_eq!(out.focused, None);
    assert_eq!(out.ime_area, None);
}

#[test]
fn the_preedit_shows_inline_and_the_text_after_it_shifts() {
    let mut text = String::from("lune");
    let mut h = focused(&mut text);
    h.input.key_down(Key::Named(NamedKey::ArrowLeft), None);
    h.input.key_down(Key::Named(NamedKey::ArrowLeft), None);
    preedit(&mut h, "かな", Some((6, 6)));
    let (scene, _, _) = run(&mut h, &mut text);
    assert_eq!(text, "lune");
    assert_eq!(scene.layers[0].texts[0].key, spec_key("luかなne"));
    // Exactly the preedit's range: from column 2 for two chars.
    let u = underlines(&scene, 1.0);
    assert_eq!(u.len(), 1);
    assert_eq!(u[0].rect[0], TX + 2.0 * glyph());
    assert_eq!(u[0].rect[2], 2.0 * glyph());
    // The underline sits at the line's bottom, inside the line box.
    let line = Style::ui(DARK.font_base, DARK.fg_primary).line;
    assert_eq!(u[0].rect[1], RECT.y + (RECT.h - line) / 2.0 + line - 1.0);
    // The collapsed cursor is the caret, after the preedit.
    let c = carets(&scene);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].rect[0], TX + 4.0 * glyph() + 1.0);
}

#[test]
fn a_selected_clause_is_filled_and_thicker_and_hides_the_caret() {
    let mut text = String::from("lune");
    let mut h = focused(&mut text);
    preedit(&mut h, "かな", Some((0, 3)));
    let (scene, r, _) = run(&mut h, &mut text);
    let u = underlines(&scene, 2.0);
    assert_eq!(u.len(), 1);
    assert_eq!(u[0].rect[0], TX + 4.0 * glyph());
    assert_eq!(u[0].rect[2], glyph());
    let fill = premul(DARK.text_selection);
    let under = &scene.layers[0].geometry.under;
    let fills: Vec<_> = under.iter().filter(|s| s.color == fill).collect();
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].rect[2], glyph());
    assert!(carets(&scene).is_empty());
    // The candidate window goes by the clause.
    let area = r.ime_area.expect("area");
    assert_eq!((area.x, area.w), (TX + 4.0 * glyph(), glyph()));
}

#[test]
fn no_cursor_means_no_caret_and_no_clause() {
    let mut text = String::from("lune");
    let mut h = focused(&mut text);
    preedit(&mut h, "かな", None);
    let (scene, r, _) = run(&mut h, &mut text);
    assert!(carets(&scene).is_empty());
    assert!(underlines(&scene, 2.0).is_empty());
    assert_eq!(underlines(&scene, 1.0).len(), 1);
    assert!(r.ime_area.is_some());
}

#[test]
fn an_empty_preedit_clears_and_draws_no_underline() {
    let mut text = String::from("lune");
    let mut h = focused(&mut text);
    preedit(&mut h, "か", Some((0, 3)));
    run(&mut h, &mut text);
    preedit(&mut h, "", None);
    let (scene, _, _) = run(&mut h, &mut text);
    assert!(underlines(&scene, 1.0).is_empty());
    assert_eq!(scene.layers[0].texts[0].key, spec_key("lune"));
}

#[test]
fn a_preedit_cursor_in_bytes_is_a_char_offset_in_the_widget() {
    // In "héllo" byte 3 lies after "hé" (h is one byte, é two): char 2, so the caret sits two
    // columns into the preedit, not three.
    let mut text = String::new();
    let mut h = focused(&mut text);
    preedit(&mut h, "h\u{e9}llo", Some((3, 3)));
    let (scene, _, _) = run(&mut h, &mut text);
    let c = carets(&scene);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].rect[0], TX + 2.0 * glyph() + 1.0);
}

#[test]
fn a_wild_cursor_is_clamped() {
    let mut text = String::new();
    let mut h = focused(&mut text);
    // Inside a character, past the end, reversed.
    for cursor in [(1, 2), (2, 300), (usize::MAX, 0)] {
        preedit(&mut h, "h\u{e9}llo", Some(cursor));
        run(&mut h, &mut text);
    }
    assert_eq!(text, "");
}

#[test]
fn the_area_lies_in_the_field_and_moves_with_the_caret() {
    let mut text = String::new();
    let mut h = focused(&mut text);
    let (_, r, out) = run(&mut h, &mut text);
    let first = r.ime_area.expect("focused");
    assert_eq!(out.ime_area, Some(first));
    h.input.key_down(Key::Character("a".to_owned()), Some("a"));
    let (_, r, _) = run(&mut h, &mut text);
    let next = r.ime_area.expect("focused");
    assert_eq!(next.x, first.x + glyph());
    for a in [first, next] {
        assert!(
            a.x >= RECT.x
                && a.right() <= RECT.right()
                && a.y >= RECT.y
                && a.bottom() <= RECT.bottom(),
            "{a:?}"
        );
    }
}

#[test]
fn the_area_is_none_without_focus_read_only_disabled_or_a_password() {
    let mut text = String::from("x");
    let mut h = UiHarness::new(WIN);
    let (_, r, out) = run(&mut h, &mut text);
    assert_eq!((r.ime_area, out.ime_area), (None, None));
    for opts in [
        FieldOptions {
            read_only: true,
            ..FieldOptions::default()
        },
        FieldOptions {
            variant: Variant::Password,
            ..FieldOptions::default()
        },
    ] {
        let mut h = UiHarness::new(WIN);
        with(&mut h, &mut text, &opts);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        let (_, r, out) = with(&mut h, &mut text, &opts);
        assert!(r.response.focused);
        assert_eq!((r.ime_area, out.ime_area), (None, None));
    }
    let mut h = focused(&mut text);
    let (_, out) = h.frame(|ui| {
        ui.with_enabled(false, |ui| {
            ui.text_field_with("f", RECT, &mut text, &FieldOptions::default());
        });
    });
    assert_eq!(out.ime_area, None);
}

#[test]
fn a_password_composes_nothing_on_screen_but_a_commit_is_text() {
    let opts = FieldOptions {
        variant: Variant::Password,
        ..FieldOptions::default()
    };
    let mut text = String::from("x");
    let mut h = UiHarness::new(WIN);
    with(&mut h, &mut text, &opts);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    with(&mut h, &mut text, &opts);
    preedit(&mut h, "secret", Some((0, 6)));
    let (scene, _, _) = with(&mut h, &mut text, &opts);
    assert!(underlines(&scene, 1.0).is_empty());
    assert!(!format!("{scene:?}").contains("secret"));
    h.input.ime(ImeEvent::Commit("y".to_owned()));
    with(&mut h, &mut text, &opts);
    assert_eq!(text, "xy");
}

#[test]
fn keys_belong_to_the_input_method_while_composing() {
    let mut text = String::from("lune");
    let mut h = focused(&mut text);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::ArrowLeft)),
        "left",
    ));
    preedit(&mut h, "か", Some((3, 3)));
    run(&mut h, &mut text);
    for key in [
        NamedKey::Enter,
        NamedKey::ArrowLeft,
        NamedKey::Backspace,
        NamedKey::Escape,
        NamedKey::Tab,
    ] {
        h.input.key_down(Key::Named(key), None);
    }
    let (_, r, out) = run(&mut h, &mut text);
    assert!(r.response.focused, "Tab stays with the composition");
    assert!(!r.submitted && !r.response.changed);
    assert_eq!(text, "lune");
    assert!(out.fired.is_empty());
    let n = out.tree.get(id()).expect("node");
    assert_eq!(n.text.as_ref().map(|t| t.selection), Some((4, 4)));
}

#[test]
fn a_scroll_area_leaves_page_keys_to_the_input_method_while_composing() {
    use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};
    let mut text = String::new();
    let mut h = UiHarness::new(WIN);
    let view = Rect::new(0.0, 0.0, 200.0, 100.0);
    let mut area = |h: &mut UiHarness| {
        let mut offset = 0.0;
        h.frame(|ui| {
            let out = ui.scroll_area("s", view, &ScrollOptions::default(), |ui, _| {
                ui.text_field_with("f", RECT, &mut text, &FieldOptions::default());
                ((), Extent::new(200.0, 1000.0))
            });
            offset = out.offset.1;
        });
        offset
    };
    area(&mut h);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    area(&mut h);
    preedit(&mut h, "か", Some((3, 3)));
    area(&mut h);
    h.input.key_down(Key::Named(NamedKey::PageDown), None);
    assert_eq!(area(&mut h), 0.0);
    // Without a composition the area has them, which also shows the key reaches it.
    h.input.ime(ImeEvent::Preedit {
        text: String::new(),
        cursor: None,
    });
    area(&mut h);
    h.input.key_down(Key::Named(NamedKey::PageDown), None);
    area(&mut h);
    assert_eq!(area(&mut h), 100.0);
}

#[test]
fn a_press_on_clear_while_composing_is_ignored() {
    let opts = FieldOptions {
        variant: Variant::Search,
        ..FieldOptions::default()
    };
    let mut text = String::from("lune");
    let mut h = UiHarness::new(WIN);
    with(&mut h, &mut text, &opts);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    with(&mut h, &mut text, &opts);
    preedit(&mut h, "か", Some((3, 3)));
    with(&mut h, &mut text, &opts);
    h.input
        .pointer_to(RECT.right() - 16.0, 36.0)
        .click(PointerButton::Primary);
    let (scene, r, _) = with(&mut h, &mut text, &opts);
    assert_eq!(text, "lune");
    assert!(!r.response.changed);
    assert_eq!(underlines(&scene, 1.0).len(), 1);
}

#[test]
fn a_press_while_composing_is_ignored() {
    let mut text = String::from("lune");
    let mut h = focused(&mut text);
    preedit(&mut h, "か", Some((3, 3)));
    run(&mut h, &mut text);
    h.input.pointer_to(TX, 36.0).click(PointerButton::Primary);
    let (scene, _, out) = run(&mut h, &mut text);
    let n = out.tree.get(id()).expect("node");
    assert_eq!(n.text.as_ref().map(|t| t.selection), Some((4, 4)));
    assert_eq!(underlines(&scene, 1.0).len(), 1, "it composes on");
}

#[test]
fn the_preedit_is_gone_after_focus_loss_and_nothing_is_committed() {
    let (mut a, mut b) = (String::from("lune"), String::new());
    let mut h = UiHarness::new(WIN);
    let two = |h: &mut UiHarness, a: &mut String, b: &mut String| {
        h.frame(|ui| {
            ui.text_field_with("f", RECT, a, &FieldOptions::default());
            ui.text_field_with("g", OTHER, b, &FieldOptions::default());
        })
        .0
    };
    two(&mut h, &mut a, &mut b);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    two(&mut h, &mut a, &mut b);
    preedit(&mut h, "か", Some((3, 3)));
    let scene = two(&mut h, &mut a, &mut b);
    assert_eq!(underlines(&scene, 1.0).len(), 1);
    h.input.pointer_to(90.0, 96.0).click(PointerButton::Primary);
    let scene = two(&mut h, &mut a, &mut b);
    assert!(underlines(&scene, 1.0).is_empty());
    assert_eq!(a, "lune");
    // Coming back: still nothing.
    h.input.pointer_to(90.0, 36.0).click(PointerButton::Primary);
    let scene = two(&mut h, &mut a, &mut b);
    assert!(underlines(&scene, 1.0).is_empty());
}

#[test]
fn a_composition_does_not_survive_a_hidden_tab() {
    let mut text = String::from("lune");
    let mut h = UiHarness::new(WIN);
    let shown = |h: &mut UiHarness, text: &mut String| {
        h.frame(|ui| {
            ui.keep_alive("tab", |ui| {
                ui.text_field_with("f", RECT, text, &FieldOptions::default());
            });
        })
        .0
    };
    shown(&mut h, &mut text);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    shown(&mut h, &mut text);
    preedit(&mut h, "か", Some((3, 3)));
    let scene = shown(&mut h, &mut text);
    assert_eq!(underlines(&scene, 1.0).len(), 1);
    h.frame(|ui| ui.mark_alive("tab"));
    let scene = shown(&mut h, &mut text);
    assert!(underlines(&scene, 1.0).is_empty());
}

#[test]
fn the_node_excludes_the_preedit_until_the_commit() {
    let mut text = String::from("ab");
    let mut h = focused(&mut text);
    preedit(&mut h, "か", Some((3, 3)));
    let (_, _, out) = run(&mut h, &mut text);
    let t = out
        .tree
        .get(id())
        .and_then(|n| n.text.clone())
        .expect("text");
    assert_eq!((t.text.as_str(), t.selection), ("ab", (2, 2)));
    h.input.ime(ImeEvent::Commit("か".to_owned()));
    let (_, _, out) = run(&mut h, &mut text);
    let t = out
        .tree
        .get(id())
        .and_then(|n| n.text.clone())
        .expect("text");
    assert_eq!((t.text.as_str(), t.selection), ("abか", (3, 3)));
}

#[test]
fn the_preedit_scrolls_with_the_text() {
    let mut text = "w".repeat(40);
    let mut h = focused(&mut text);
    let (scene, _, _) = run(&mut h, &mut text);
    let x = scene.layers[0].texts[0].x;
    preedit(&mut h, "かなかな", Some((12, 12)));
    let (scene, r, _) = run(&mut h, &mut text);
    assert_eq!(scene.layers[0].texts[0].x, x - 4.0 * glyph());
    let u = underlines(&scene, 1.0);
    assert_eq!(u[0].rect[0], scene.layers[0].texts[0].x + 40.0 * glyph());
    let clip = RECT.inset(6.0, 0.0);
    let c = carets(&scene)[0].rect;
    assert!(c[0] + c[2] <= clip.right() - 2.0);
    let a = r.ime_area.expect("area");
    assert!(a.right() <= RECT.right());
}

#[test]
fn a_settled_composition_is_idle() {
    let mut text = String::new();
    let mut h = focused(&mut text);
    h.at_ms(1000);
    preedit(&mut h, "か", Some((3, 3)));
    run(&mut h, &mut text);
    run(&mut h, &mut text);
    let (_, _, out) = run(&mut h, &mut text);
    assert_eq!(out.repaint, Repaint::Idle);
}

/// One frame of a keep-alive page holding the field, shown or not, beside a button, with an
/// overlay that Escape dismisses when `overlay` is set. Returns the focus and the dismissal.
fn page(
    h: &mut UiHarness,
    text: &mut String,
    shown: bool,
    overlay: bool,
) -> (FrameOutput, Option<Dismissal>) {
    let button = WidgetId::ROOT.with("b");
    let mut dismissed = None;
    let (_, out) = h.frame(|ui| {
        ui.keep_alive("page", |ui| {
            if shown {
                ui.text_field_with("f", RECT, text, &FieldOptions::default());
            }
        });
        ui.interact(button, OTHER, Sense::CLICK_FOCUS);
        let mut node = Node::new(button, Role::Button, "b", OTHER);
        node.states.insert(State::Focusable);
        ui.node(node);
        if overlay {
            let opts = OverlayOptions {
                layer: LayerId::Popup,
                modal: false,
                dismiss_on_escape: true,
                dismiss_on_outside_press: false,
                dismiss_on_focus_loss: false,
                anchor: None,
                initial_focus: None,
                open: OPEN,
                close: CLOSE,
            };
            dismissed = ui.overlay("o", &opts, |_, _, _| {}, |_, _| ()).0.dismissed;
        }
    });
    (out, dismissed)
}

/// A field focused and composing in a keep-alive page.
fn composing_page(text: &mut String) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    page(&mut h, text, true, false);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    page(&mut h, text, true, false);
    preedit(&mut h, "か", Some((3, 3)));
    let (out, _) = page(&mut h, text, true, false);
    assert_ne!(out.focused, Some(WidgetId::ROOT.with("b")));
    assert!(out.focused.is_some());
    h
}

#[test]
fn tab_moves_on_from_a_composition_in_a_hidden_page() {
    let mut text = String::from("lune");
    let mut h = composing_page(&mut text);
    page(&mut h, &mut text, false, false);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let (out, _) = page(&mut h, &mut text, false, false);
    assert_eq!(out.focused, Some(WidgetId::ROOT.with("b")));
}

#[test]
fn escape_dismisses_past_a_composition_in_a_hidden_page() {
    let mut text = String::from("lune");
    let mut h = composing_page(&mut text);
    page(&mut h, &mut text, false, true);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (_, dismissed) = page(&mut h, &mut text, false, true);
    assert_eq!(dismissed, Some(Dismissal::Escape));
}

#[test]
fn a_shortcut_fires_past_a_composition_in_a_hidden_page() {
    let mut text = String::from("lune");
    for preempt in [false, true] {
        let mut h = composing_page(&mut text);
        let chord = Chord::key(Key::Named(NamedKey::F2));
        let binding = Binding::new(Scope::Global, chord, "go");
        h.shortcuts
            .add(if preempt { binding.preempt() } else { binding });
        page(&mut h, &mut text, false, false);
        h.input.key_down(Key::Named(NamedKey::F2), None);
        let (out, _) = page(&mut h, &mut text, false, false);
        assert_eq!(out.fired.len(), 1, "preempt {preempt}");
    }
}

#[test]
fn a_press_in_the_frame_the_page_hides_bubbles_past_the_composition() {
    let mut text = String::from("lune");
    let mut h = composing_page(&mut text);
    let chord = Chord::key(Key::Named(NamedKey::F2));
    h.shortcuts.add(Binding::new(Scope::Global, chord, "go"));
    h.input.key_down(Key::Named(NamedKey::F2), None);
    let (out, _) = page(&mut h, &mut text, false, false);
    assert_eq!(out.fired.len(), 1);
}

#[test]
fn a_page_shown_again_has_no_composition_and_leaves_tab_to_the_frame() {
    let mut text = String::from("lune");
    let mut h = composing_page(&mut text);
    page(&mut h, &mut text, false, false);
    page(&mut h, &mut text, true, false);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let (out, _) = page(&mut h, &mut text, true, false);
    assert_eq!(out.focused, Some(WidgetId::ROOT.with("b")));
    assert_eq!(text, "lune");
}
