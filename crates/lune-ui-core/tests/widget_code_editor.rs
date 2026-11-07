//! The code editor view (`text.code-editor-view`, D181 to D185): read-only and editable behaviour on
//! the view, composition on one line of many, Tab, spans and bands, synced panes, anchors, wrapped
//! heights, frame cost at scale, keys, the nested wheel and the node.

use std::cell::Cell;
use std::time::Duration;

use lune_ui_core::access::{AccessAction, AccessValue, Action, Role, State, Value};
use lune_ui_core::code_doc::CodeDoc;
use lune_ui_core::color::{Rgba, premul};
use lune_ui_core::input::{
    ClipboardRequest, FrameOutput, ImeEvent, Key, Keymap, Modifiers, NamedKey, PointerButton,
    Repaint,
};
use lune_ui_core::motion::MotionTier;
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::shortcuts::{Binding, Chord, Scope, Shortcuts};
use lune_ui_core::strings::{DisplayFormat, StringTable};
use lune_ui_core::testing::{
    FakeText, ScriptedInput, UiHarness, disabled_violations, fill_covers, fill_shapes,
};
use lune_ui_core::text::{CaretPos, LineSpan, Spec, Style, TextKey, TextSystem, Unit};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{FrameConfig, Ui, UiStore, WidgetId};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::code_editor::{ANCHOR_INSET, CodeOptions, CodeResponse, LINE, LineBand};
use lune_ui_core::widgets::scroll_area::{Bars, Extent, LINE as WHEEL_LINE, ScrollOptions};
use lune_ui_core::widgets::text_lines::{HighlightSpan, Highlighter};

const WIN: (f32, f32) = (500.0, 400.0);
/// Ten lines in view.
const RECT: Rect = Rect::new(20.0, 20.0, 400.0, 200.0);
const P: PointerButton = PointerButton::Primary;
/// With no gutter, the text's left edge.
const TX: f32 = 22.0;
const GLYPH: f32 = 6.5;
const META: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: true,
};
const SHIFT: Modifiers = Modifiers {
    shift: true,
    control: false,
    alt: false,
    logo: false,
};

fn id() -> WidgetId {
    WidgetId::ROOT.with("c")
}

fn plain() -> CodeOptions<'static> {
    CodeOptions {
        gutter: false,
        label: "Code",
        ..CodeOptions::default()
    }
}

fn read_only() -> CodeOptions<'static> {
    CodeOptions {
        read_only: true,
        ..plain()
    }
}

fn wrapped() -> CodeOptions<'static> {
    CodeOptions {
        wrap: true,
        ..plain()
    }
}

fn numbered(n: usize) -> CodeDoc {
    let lines: Vec<String> = (0..n).map(|i| format!("line {i}")).collect();
    CodeDoc::new(lines.join("\n"))
}

struct Run {
    scene: Scene,
    r: CodeResponse,
    out: FrameOutput,
}

fn run(h: &mut UiHarness, doc: &mut CodeDoc, opts: &CodeOptions<'_>) -> Run {
    let mut r = None;
    let (scene, out) = h.frame(|ui| r = Some(ui.code_editor("c", RECT, doc, opts)));
    Run {
        scene,
        r: r.expect("ran"),
        out,
    }
}

fn focused(doc: &mut CodeDoc, opts: &CodeOptions<'_>) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    run(&mut h, doc, opts);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    run(&mut h, doc, opts);
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

fn with_mods(h: &mut UiHarness, m: Modifiers, key: Key) {
    h.input.set_modifiers(m);
    h.input.key_down(key, None);
    h.input.set_modifiers(Modifiers::default());
}

fn chord(h: &mut UiHarness, c: &str) {
    with_mods(h, META, Key::Character(c.to_owned()));
}

fn preedit(h: &mut UiHarness, text: &str, cursor: Option<(usize, usize)>) {
    h.input.ime(ImeEvent::Preedit {
        text: text.to_owned(),
        cursor,
    });
}

fn caret(scene: &Scene) -> Option<[f32; 4]> {
    let over = &scene.layers[0].geometry.over;
    over.iter().find(|s| s.rect[2] == 1.5).map(|s| s.rect)
}

fn underlines(scene: &Scene, height: f32) -> Vec<Shape> {
    let over = &scene.layers[0].geometry.over;
    over.iter()
        .filter(|s| s.rect[3] == height && s.color == premul(DARK.fg_primary))
        .copied()
        .collect()
}

/// The fills of `color`.
/// The flat fills of `color` on the base layer: a band.
fn fills(scene: &Scene, color: Rgba) -> Vec<[f32; 4]> {
    let want = premul(color);
    let under = &scene.layers[0].geometry.under;
    under
        .iter()
        .filter(|s| s.color == want)
        .map(|s| s.rect)
        .collect()
}

/// The bounds of each joined selection shape of `color` on the base layer (scale 1), and no flat
/// fill of it anywhere.
fn selected(scene: &Scene, color: Rgba) -> Vec<[f32; 4]> {
    let g = &scene.layers[0].geometry;
    let want = premul(color);
    assert!(!g.under.iter().chain(&g.over).any(|s| s.color == want));
    fill_shapes(g, color)
        .iter()
        .map(|r| [r.x, r.y, r.w, r.h])
        .collect()
}

/// Whether the selection of `color` covers `(x, y)` (scale 1).
fn covers(scene: &Scene, color: Rgba, x: f32, y: f32) -> bool {
    fill_covers(&scene.layers[0].geometry, color, [x, y])
}

fn base() -> Style {
    Style::mono(DARK.font_base, LINE, DARK.fg_primary)
}

fn key_of(text: &str, style: Style) -> TextKey {
    FakeText.prepare(&Spec::plain(text, style), 1.0).0
}

/// The y of the first drawn text that shows `text`.
fn shown_at(scene: &Scene, text: &str) -> Option<f32> {
    let key = key_of(text, base());
    let texts = &scene.layers[0].texts;
    texts.iter().find(|t| t.key == key).map(|t| t.y)
}

fn offset(run: &Run) -> f64 {
    run.r.offset.1
}

fn node_text(out: &FrameOutput) -> (String, (usize, usize)) {
    let t = out.tree.get(id()).and_then(|n| n.text.clone());
    let t = t.expect("text");
    (t.text, t.selection)
}

/// Runs frames until the area stops moving.
fn settle(h: &mut UiHarness, doc: &mut CodeDoc, opts: &CodeOptions<'_>, from_ms: u64) -> Run {
    h.at_ms(from_ms + 1_000);
    run(h, doc, opts);
    run(h, doc, opts)
}

// Read-only.

#[test]
fn a_read_only_view_draws_no_caret_and_ignores_edits() {
    let mut doc = numbered(40);
    let mut h = focused(&mut doc, &read_only());
    typed(&mut h, "x");
    press(&mut h, NamedKey::Enter);
    press(&mut h, NamedKey::Backspace);
    let run = run(&mut h, &mut doc, &read_only());
    assert_eq!(doc.line(0), "line 0");
    assert!(!run.r.changed);
    assert_eq!(caret(&run.scene), None);
    assert_eq!(run.r.ime_area, None);
}

#[test]
fn a_press_focuses_a_read_only_view_and_a_drag_selects_and_copies() {
    let mut doc = numbered(40);
    let opts = read_only();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut doc, &opts);
    h.input.pointer_to(TX, 30.0).press(P);
    let r = run(&mut h, &mut doc, &opts);
    assert_eq!(r.out.focused, Some(id()));
    assert_eq!(caret(&r.scene), None);
    h.input.pointer_to(TX + 2.0 * GLYPH, 50.0);
    let r = run(&mut h, &mut doc, &opts);
    h.input.release(P);
    assert_eq!(node_text(&r.out).1, (0, 9));
    run(&mut h, &mut doc, &opts);
    chord(&mut h, "c");
    let r = run(&mut h, &mut doc, &opts);
    assert_eq!(
        r.out.clipboard,
        vec![ClipboardRequest::Write("line 0\nli".to_owned())]
    );
}

#[test]
fn select_all_on_a_read_only_view_does_not_jump_to_the_end() {
    let mut doc = numbered(200);
    let opts = read_only();
    let mut h = focused(&mut doc, &opts);
    chord(&mut h, "a");
    let r = run(&mut h, &mut doc, &opts);
    let r2 = run(&mut h, &mut doc, &opts);
    assert_eq!(offset(&r), 0.0);
    assert_eq!(offset(&r2), 0.0);
    assert_eq!(r2.out.repaint, Repaint::Idle);
    let (text, sel) = node_text(&r2.out);
    assert_eq!(
        sel,
        (0, text.chars().count()),
        "clipped to the lines in view"
    );
    chord(&mut h, "c");
    let r = run(&mut h, &mut doc, &opts);
    assert_eq!(
        r.out.clipboard,
        vec![ClipboardRequest::Write(doc.text().to_owned())]
    );
}

#[test]
fn page_keys_home_and_end_scroll_a_read_only_view_through_the_area() {
    let mut doc = numbered(100);
    let opts = read_only();
    let mut h = focused(&mut doc, &opts);
    press(&mut h, NamedKey::PageDown);
    run(&mut h, &mut doc, &opts);
    assert_eq!(offset(&run(&mut h, &mut doc, &opts)), 200.0);
    press(&mut h, NamedKey::End);
    run(&mut h, &mut doc, &opts);
    assert_eq!(offset(&run(&mut h, &mut doc, &opts)), 1800.0);
    press(&mut h, NamedKey::PageUp);
    run(&mut h, &mut doc, &opts);
    assert_eq!(offset(&run(&mut h, &mut doc, &opts)), 1600.0);
    press(&mut h, NamedKey::Home);
    run(&mut h, &mut doc, &opts);
    assert_eq!(offset(&run(&mut h, &mut doc, &opts)), 0.0);
}

#[test]
fn arrows_on_a_read_only_view_fire_a_binding_and_on_an_editable_one_move_the_caret() {
    let binding = |key| Binding::new(Scope::Global, Chord::key(Key::Named(key)), "go");
    for key in [NamedKey::ArrowDown, NamedKey::ArrowLeft] {
        let mut doc = numbered(40);
        let opts = read_only();
        let mut h = focused(&mut doc, &opts);
        h.shortcuts.add(binding(key));
        press(&mut h, key);
        let r = run(&mut h, &mut doc, &opts);
        assert_eq!(r.out.fired.len(), 1, "{key:?}");
        assert_eq!(offset(&r), 0.0);

        let mut doc = numbered(40);
        let mut h = focused(&mut doc, &plain());
        h.shortcuts.add(binding(key));
        press(&mut h, key);
        let r = run(&mut h, &mut doc, &plain());
        assert!(r.out.fired.is_empty(), "{key:?}");
    }
}

#[test]
fn the_ring_shows_on_a_read_only_view_with_keyboard_focus_only() {
    let ring = |scene: &Scene| {
        let under = &scene.layers[0].geometry.under;
        under.iter().any(|s| s.params[0] == 1.0)
    };
    let mut doc = numbered(40);
    let opts = read_only();
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, &mut doc, &opts);
    assert!(!ring(&r.scene), "at rest");
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    run(&mut h, &mut doc, &opts);
    let r = run(&mut h, &mut doc, &opts);
    assert!(ring(&r.scene), "keyboard focus");

    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut doc, &opts);
    h.input.pointer_to(TX, 30.0).click(P);
    run(&mut h, &mut doc, &opts);
    let r = run(&mut h, &mut doc, &opts);
    assert_eq!(r.out.focused, Some(id()));
    assert!(!ring(&r.scene), "a press shows none");

    let mut doc = numbered(40);
    let r = {
        let mut h = focused(&mut doc, &plain());
        run(&mut h, &mut doc, &plain())
    };
    assert!(!ring(&r.scene), "the caret is the editable view's mark");
}

// Editable: the `text.text-edit` Behaviour lines, on the view.

#[test]
fn typing_caret_keys_and_selection_follow_the_engine() {
    let mut doc = CodeDoc::default();
    let mut h = focused(&mut doc, &plain());
    typed(&mut h, "abc");
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "abc");
    assert!(r.r.changed);
    assert_eq!(node_text(&r.out).1, (3, 3));
    press(&mut h, NamedKey::ArrowLeft);
    press(&mut h, NamedKey::ArrowLeft);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(node_text(&r.out).1, (1, 1));
    with_mods(&mut h, SHIFT, Key::Named(NamedKey::ArrowRight));
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(node_text(&r.out).1, (1, 2));
    chord(&mut h, "a");
    typed(&mut h, "z");
    run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "z");
    press(&mut h, NamedKey::Home);
    press(&mut h, NamedKey::Backspace);
    run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "z");
    press(&mut h, NamedKey::End);
    press(&mut h, NamedKey::Delete);
    run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "z");
}

#[test]
fn backspace_removes_a_whole_grapheme() {
    let mut doc = CodeDoc::new("e\u{301}");
    let mut h = focused(&mut doc, &plain());
    with_mods(&mut h, META, Key::Named(NamedKey::End));
    press(&mut h, NamedKey::Backspace);
    run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "");
}

#[test]
fn undo_and_redo_run_on_the_view_and_an_external_change_clears_them() {
    let mut doc = CodeDoc::default();
    let mut h = focused(&mut doc, &plain());
    typed(&mut h, "abc");
    run(&mut h, &mut doc, &plain());
    chord(&mut h, "z");
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "");
    assert!(r.r.changed);
    with_mods(&mut h, META.with_shift(), Key::Character("z".to_owned()));
    run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "abc");
    doc.set_text("x");
    run(&mut h, &mut doc, &plain());
    chord(&mut h, "z");
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "x", "the history went");
    assert_eq!(node_text(&r.out).1, (1, 1), "the caret clamped");
}

#[test]
fn enter_inserts_a_line_and_the_lines_follow() {
    let mut doc = CodeDoc::new("ab");
    let mut h = focused(&mut doc, &plain());
    press(&mut h, NamedKey::End);
    press(&mut h, NamedKey::Enter);
    typed(&mut h, "c");
    press(&mut h, NamedKey::ArrowUp);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "ab\nc");
    assert_eq!(node_text(&r.out).1, (1, 1), "up at the preferred x");
    assert_eq!(shown_at(&r.scene, "c"), Some(RECT.y + LINE));
}

#[test]
fn paste_of_three_lines_lands_and_undoes_in_one_step() {
    let mut doc = CodeDoc::new("ab");
    let mut h = focused(&mut doc, &plain());
    press(&mut h, NamedKey::ArrowRight);
    chord(&mut h, "v");
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(r.out.clipboard, vec![ClipboardRequest::Read]);
    h.input.clipboard_text("x\ny\r\nz");
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "ax\ny\nzb");
    assert_eq!(doc.line_count(), 3);
    assert!(r.r.changed);
    chord(&mut h, "z");
    run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "ab");
    assert_eq!(doc.line_count(), 1);
}

#[test]
fn cut_writes_the_selection_and_removes_it_across_lines() {
    let mut doc = numbered(3);
    let mut h = focused(&mut doc, &plain());
    with_mods(&mut h, SHIFT, Key::Named(NamedKey::ArrowDown));
    chord(&mut h, "x");
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(
        r.out.clipboard,
        vec![ClipboardRequest::Write("line 0\n".to_owned())]
    );
    assert_eq!(doc.text(), "line 1\nline 2");
}

#[test]
fn caret_motion_reveals_at_once_in_y_and_x() {
    let mut doc = numbered(100);
    let mut h = focused(&mut doc, &plain());
    with_mods(&mut h, META, Key::Named(NamedKey::End));
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(offset(&r), 1800.0, "the frame that moved it");
    assert_eq!(caret(&r.scene).map(|c| c.round_y()), Some(200.0));

    let long = "x".repeat(300);
    let mut doc = CodeDoc::new(long);
    let mut h = focused(&mut doc, &plain());
    press(&mut h, NamedKey::End);
    let r = run(&mut h, &mut doc, &plain());
    let c = caret(&r.scene).expect("a caret");
    assert!(r.r.offset.0 > 0.0);
    assert!(c[0] < RECT.right() && c[0] >= RECT.x, "{c:?}");
    press(&mut h, NamedKey::Home);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(r.r.offset.0, 0.0);
}

trait Bottom {
    fn round_y(&self) -> f32;
}

impl Bottom for [f32; 4] {
    /// The rect's bottom, from the view's top.
    fn round_y(&self) -> f32 {
        self[1] + self[3] - RECT.y
    }
}

trait WithShift {
    fn with_shift(self) -> Modifiers;
}

impl WithShift for Modifiers {
    fn with_shift(self) -> Modifiers {
        Modifiers {
            shift: true,
            ..self
        }
    }
}

#[test]
fn page_keys_move_the_caret_by_a_page_and_reveal() {
    let mut doc = numbered(100);
    let mut h = focused(&mut doc, &plain());
    press(&mut h, NamedKey::PageDown);
    press(&mut h, NamedKey::PageDown);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(offset(&r), 220.0);
    typed(&mut h, "X");
    run(&mut h, &mut doc, &plain());
    assert_eq!(doc.line(20), "Xline 20");
    press(&mut h, NamedKey::PageUp);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(offset(&r), 200.0);
}

#[test]
fn a_press_places_the_caret_and_a_gutter_press_lands_on_the_line_start() {
    let mut doc = numbered(40);
    let opts = CodeOptions {
        gutter: true,
        ..plain()
    };
    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut doc, &opts);
    h.input.pointer_to(RECT.x + 3.0, 50.0).click(P);
    run(&mut h, &mut doc, &opts);
    typed(&mut h, "G");
    run(&mut h, &mut doc, &opts);
    assert_eq!(doc.line(1), "Gline 1");
}

#[test]
fn double_and_triple_presses_select_the_word_and_the_line() {
    let mut doc = CodeDoc::new("one two\nthree");
    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut doc, &plain());
    h.input.pointer_to(TX + GLYPH * 5.0, 30.0).click(P);
    run(&mut h, &mut doc, &plain());
    h.input.click(P);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(node_text(&r.out).1, (4, 7));
    h.input.click(P);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(node_text(&r.out).1, (0, 7));
}

#[test]
fn a_selection_across_lines_fills_each_with_a_sliver_at_the_ends_it_runs_past() {
    let mut doc = CodeDoc::new("abc\n\nde\nf");
    let mut h = focused(&mut doc, &plain());
    for _ in 0..2 {
        with_mods(&mut h, SHIFT, Key::Named(NamedKey::ArrowDown));
    }
    with_mods(&mut h, SHIFT, Key::Named(NamedKey::ArrowRight));
    let r = run(&mut h, &mut doc, &plain());
    let f = selected(&r.scene, DARK.text_selection);
    // One joined shape: line 0's text and sliver, line 1's sliver alone, line 2's one char.
    let pad = DARK.selection_pad;
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(
        f[0],
        [TX - pad, RECT.y, 3.0 * GLYPH + 4.0 + 2.0 * pad, 3.0 * LINE]
    );
    let sel = DARK.text_selection;
    let mid = |line: f32| RECT.y + (line + 0.5) * LINE;
    // Line 0 reaches past its text by the sliver; line 1 is the sliver and its padding only.
    assert!(covers(&r.scene, sel, TX + 3.0 * GLYPH + 3.0, mid(0.0)));
    assert!(covers(&r.scene, sel, TX + 3.0, mid(1.0)));
    assert!(!covers(&r.scene, sel, TX + 4.0 + pad + 1.0, mid(1.0)));
    // Line 2: one char, no sliver.
    assert!(covers(&r.scene, sel, TX + GLYPH - 0.5, mid(2.0)));
    assert!(!covers(&r.scene, sel, TX + GLYPH + pad + 1.0, mid(2.0)));
}

#[test]
fn the_wheel_scrolls_by_the_step_and_clamps_and_the_caret_stays() {
    let mut doc = numbered(100);
    let mut h = focused(&mut doc, &plain());
    h.input.pointer_to(100.0, 100.0).scroll_lines(0.0, -3.0);
    run(&mut h, &mut doc, &plain());
    let r = settle(&mut h, &mut doc, &plain(), 0);
    assert_eq!(offset(&r), 3.0 * WHEEL_LINE);
    h.input.scroll_lines(0.0, -1000.0);
    run(&mut h, &mut doc, &plain());
    let r = settle(&mut h, &mut doc, &plain(), 2_000);
    assert_eq!(offset(&r), 1800.0);
    assert_eq!(caret(&r.scene), None, "off screen, and nothing pulled back");
}

#[test]
fn a_set_value_replaces_the_document_when_editable_and_only_then() {
    let action = AccessAction {
        target: id(),
        action: Action::SetValue,
        value: Some(AccessValue::Text("a\r\nb".to_owned())),
    };
    let mut doc = numbered(5);
    let mut h = focused(&mut doc, &plain());
    h.input.access(action.clone());
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "a\nb");
    assert!(r.r.changed);
    assert_eq!(node_text(&r.out).1, (3, 3));

    let mut doc = numbered(5);
    let mut h = focused(&mut doc, &read_only());
    h.input.access(action);
    let r = run(&mut h, &mut doc, &read_only());
    assert_eq!(doc.line(0), "line 0");
    assert!(!r.r.changed);
}

// Composition, from `tests/ime.rs`.

#[test]
fn a_preedit_draws_in_place_underlined_and_a_commit_inserts_it() {
    let mut doc = CodeDoc::new("one\nlune\nthree");
    let mut h = focused(&mut doc, &plain());
    press(&mut h, NamedKey::ArrowDown);
    press(&mut h, NamedKey::ArrowRight);
    press(&mut h, NamedKey::ArrowRight);
    run(&mut h, &mut doc, &plain());
    preedit(&mut h, "かな", Some((6, 6)));
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(doc.line(1), "lune");
    assert!(!r.r.changed);
    assert_eq!(shown_at(&r.scene, "luかなne"), Some(RECT.y + LINE));
    let u = underlines(&r.scene, 1.0);
    assert_eq!(u.len(), 1);
    assert_eq!(u[0].rect[0], TX + 2.0 * GLYPH);
    assert_eq!(u[0].rect[2], 2.0 * GLYPH);
    assert_eq!(u[0].rect[1], RECT.y + 2.0 * LINE - 1.0);
    let c = caret(&r.scene).expect("the caret in the preedit");
    assert_eq!(c[0], TX + 4.0 * GLYPH + 1.0);
    let n = node_text(&r.out);
    assert!(n.0.contains("lune"), "the node excludes the preedit");

    h.input.ime(ImeEvent::Commit("かな".to_owned()));
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(doc.line(1), "luかなne");
    assert!(r.r.changed);
    assert!(underlines(&r.scene, 1.0).is_empty());
}

#[test]
fn a_selection_on_one_line_is_replaced_in_the_drawing_and_comes_back_on_cancel() {
    let mut doc = CodeDoc::new("lune");
    let mut h = focused(&mut doc, &plain());
    press(&mut h, NamedKey::ArrowRight);
    with_mods(&mut h, SHIFT, Key::Named(NamedKey::ArrowRight));
    with_mods(&mut h, SHIFT, Key::Named(NamedKey::ArrowRight));
    run(&mut h, &mut doc, &plain());
    preedit(&mut h, "か", Some((3, 3)));
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(shown_at(&r.scene, "lかe"), Some(RECT.y));
    assert!(selected(&r.scene, DARK.text_selection).is_empty());
    h.input.ime(ImeEvent::Disabled);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "lune");
    assert_eq!(node_text(&r.out).1, (1, 3));
    assert_eq!(shown_at(&r.scene, "lune"), Some(RECT.y));
}

#[test]
fn across_lines_the_first_line_is_cut_and_later_lines_stay_until_the_commit() {
    let mut doc = CodeDoc::new("abcd\nefgh\nij");
    let mut h = focused(&mut doc, &plain());
    press(&mut h, NamedKey::ArrowRight);
    with_mods(&mut h, SHIFT, Key::Named(NamedKey::ArrowDown));
    run(&mut h, &mut doc, &plain());
    preedit(&mut h, "か", Some((3, 3)));
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(shown_at(&r.scene, "aか"), Some(RECT.y));
    assert_eq!(shown_at(&r.scene, "efgh"), Some(RECT.y + LINE));
    h.input.ime(ImeEvent::Commit("か".to_owned()));
    run(&mut h, &mut doc, &plain());
    assert_eq!(doc.text(), "aかfgh\nij");
}

#[test]
fn a_clause_is_filled_double_underlined_and_the_ime_area_follows_it() {
    let mut doc = numbered(3);
    let mut h = focused(&mut doc, &plain());
    press(&mut h, NamedKey::ArrowDown);
    run(&mut h, &mut doc, &plain());
    preedit(&mut h, "かなた", Some((3, 6)));
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(underlines(&r.scene, 2.0).len(), 1);
    assert_eq!(caret(&r.scene), None);
    let area = r.r.ime_area.expect("an area");
    assert_eq!(area.x, TX + GLYPH);
    assert_eq!(area.y, RECT.y + LINE);
    assert_eq!(r.out.ime_area, Some(area));
}

#[test]
fn the_ime_area_follows_the_caret_on_screen_after_a_scroll() {
    let mut doc = numbered(100);
    let mut h = focused(&mut doc, &plain());
    with_mods(&mut h, META, Key::Named(NamedKey::End));
    let r = run(&mut h, &mut doc, &plain());
    let area = r.r.ime_area.expect("an area");
    assert_eq!(area.y, RECT.bottom() - LINE);
}

#[test]
fn a_press_while_composing_is_ignored() {
    let mut doc = numbered(3);
    let mut h = focused(&mut doc, &plain());
    preedit(&mut h, "か", Some((3, 3)));
    run(&mut h, &mut doc, &plain());
    h.input.pointer_to(TX + 3.0 * GLYPH, 50.0).click(P);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(underlines(&r.scene, 1.0).len(), 1);
    assert_eq!(node_text(&r.out).1, (0, 0));
}

#[test]
fn keys_belong_to_the_input_method_while_composing() {
    let mut doc = numbered(3);
    let opts = CodeOptions {
        tab_inserts: true,
        ..plain()
    };
    let mut h = focused(&mut doc, &opts);
    run(&mut h, &mut doc, &opts);
    preedit(&mut h, "か", Some((3, 3)));
    run(&mut h, &mut doc, &opts);
    for key in [
        NamedKey::Enter,
        NamedKey::ArrowDown,
        NamedKey::Backspace,
        NamedKey::Tab,
    ] {
        press(&mut h, key);
        run(&mut h, &mut doc, &opts);
    }
    assert_eq!(doc.line(0), "line 0");
    assert_eq!(doc.line_count(), 3);
    let r = run(&mut h, &mut doc, &opts);
    assert_eq!(r.out.focused, Some(id()));
    assert_eq!(underlines(&r.scene, 1.0).len(), 1);
}

#[test]
fn a_read_only_view_takes_no_composition() {
    let mut doc = numbered(3);
    let mut h = focused(&mut doc, &read_only());
    preedit(&mut h, "か", Some((3, 3)));
    let r = run(&mut h, &mut doc, &read_only());
    assert!(underlines(&r.scene, 1.0).is_empty());
    assert_eq!(r.r.ime_area, None);
    assert_eq!(r.out.ime_area, None);
    assert_eq!(shown_at(&r.scene, "line 0"), Some(RECT.y));
}

#[test]
fn focus_loss_cancels_the_composition() {
    let mut doc = numbered(3);
    let mut h = focused(&mut doc, &plain());
    preedit(&mut h, "か", Some((3, 3)));
    run(&mut h, &mut doc, &plain());
    let opts = plain();
    two_stops(&mut h, &mut doc, &opts, true);
    h.input.pointer_to(10.0, 320.0).click(P);
    two_stops(&mut h, &mut doc, &opts, true);
    two_stops(&mut h, &mut doc, &opts, true);
    let r = run(&mut h, &mut doc, &opts);
    assert_ne!(r.out.focused, Some(id()));
    assert!(underlines(&r.scene, 1.0).is_empty());
    assert_eq!(doc.line(0), "line 0");
}

// Tab.

fn tabbed() -> CodeOptions<'static> {
    CodeOptions {
        tab_inserts: true,
        ..plain()
    }
}

/// The view and a stop after it.
fn two_stops(
    h: &mut UiHarness,
    doc: &mut CodeDoc,
    opts: &CodeOptions<'_>,
    view: bool,
) -> FrameOutput {
    h.frame(|ui| {
        if view {
            ui.code_editor("c", RECT, doc, opts);
        }
        ui.button(
            "next",
            Rect::new(0.0, 300.0, 50.0, 50.0),
            "Next",
            ButtonKind::Secondary,
        );
    })
    .1
}

#[test]
fn a_plain_tab_inserts_a_tab_and_shift_tab_leaves() {
    let mut doc = CodeDoc::new("ab");
    let opts = tabbed();
    let mut h = UiHarness::new(WIN);
    two_stops(&mut h, &mut doc, &opts, true);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    two_stops(&mut h, &mut doc, &opts, true);
    two_stops(&mut h, &mut doc, &opts, true);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let out = two_stops(&mut h, &mut doc, &opts, true);
    assert_eq!(doc.text(), "\tab");
    assert_eq!(out.focused, Some(id()));
    with_mods(&mut h, SHIFT, Key::Named(NamedKey::Tab));
    let out = two_stops(&mut h, &mut doc, &opts, true);
    assert_eq!(doc.text(), "\tab");
    assert_ne!(out.focused, Some(id()), "Shift+Tab left");
}

#[test]
fn off_by_default_tab_moves_the_focus() {
    let mut doc = CodeDoc::new("ab");
    let opts = plain();
    let mut h = UiHarness::new(WIN);
    two_stops(&mut h, &mut doc, &opts, true);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    two_stops(&mut h, &mut doc, &opts, true);
    two_stops(&mut h, &mut doc, &opts, true);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let out = two_stops(&mut h, &mut doc, &opts, true);
    assert_eq!(doc.text(), "ab");
    assert_eq!(out.focused, Some(WidgetId::ROOT.with("next")));
}

#[test]
fn a_read_only_view_never_keeps_tab() {
    let mut doc = CodeDoc::new("ab");
    let opts = CodeOptions {
        read_only: true,
        ..tabbed()
    };
    let mut h = UiHarness::new(WIN);
    two_stops(&mut h, &mut doc, &opts, true);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    two_stops(&mut h, &mut doc, &opts, true);
    two_stops(&mut h, &mut doc, &opts, true);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let out = two_stops(&mut h, &mut doc, &opts, true);
    assert_eq!(doc.text(), "ab");
    assert_eq!(out.focused, Some(WidgetId::ROOT.with("next")));
}

#[test]
fn a_view_not_drawn_last_frame_frees_tab() {
    let mut doc = CodeDoc::new("ab");
    let opts = tabbed();
    let mut h = UiHarness::new(WIN);
    two_stops(&mut h, &mut doc, &opts, true);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    two_stops(&mut h, &mut doc, &opts, true);
    two_stops(&mut h, &mut doc, &opts, true);
    // The frame without the view: the focus is on a widget gone, and the view said nothing.
    two_stops(&mut h, &mut doc, &opts, false);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let out = two_stops(&mut h, &mut doc, &opts, false);
    assert_eq!(doc.text(), "ab");
    assert_eq!(out.focused, Some(WidgetId::ROOT.with("next")));
}

#[test]
fn a_disabled_view_frees_tab_a_frame_later_and_inserts_nothing() {
    let mut doc = CodeDoc::new("ab");
    let opts = tabbed();
    let mut h = focused(&mut doc, &opts);
    run(&mut h, &mut doc, &opts);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.frame(|ui| {
        ui.with_enabled(false, |ui| ui.code_editor("c", RECT, &mut doc, &opts));
    });
    assert_eq!(doc.text(), "ab");
}

// Spans and bands.

struct Fixed(Vec<HighlightSpan>);

impl Highlighter for Fixed {
    fn spans(&self, line: usize, _text: &str) -> Vec<HighlightSpan> {
        if line == 0 {
            self.0.clone()
        } else {
            Vec::new()
        }
    }
}

const RED: Rgba = [255, 0, 0, 255];

fn colored(runs: &[(&str, bool)]) -> TextKey {
    let spec = Spec {
        runs: runs
            .iter()
            .map(|(t, red)| {
                let color = if *red { RED } else { DARK.fg_primary };
                ((*t).to_owned(), Style::mono(DARK.font_base, LINE, color))
            })
            .collect(),
        wrap: None,
    };
    FakeText.prepare(&spec, 1.0).0
}

#[test]
fn a_span_past_the_end_clamps_and_empty_or_inverted_ones_draw_nothing() {
    let span = |start, end| HighlightSpan {
        start,
        end,
        color: RED,
    };
    let cases: [(Vec<HighlightSpan>, TextKey); 3] = [
        (
            vec![span(3, 99)],
            colored(&[("012", false), ("3456789", true)]),
        ),
        (vec![span(5, 5)], colored(&[("0123456789", false)])),
        (vec![span(7, 2)], colored(&[("0123456789", false)])),
    ];
    for (spans, want) in cases {
        let mut doc = CodeDoc::new("0123456789\nb");
        let hl = Fixed(spans);
        let opts = CodeOptions {
            highlighter: &hl,
            ..plain()
        };
        let mut h = UiHarness::new(WIN);
        let r = run(&mut h, &mut doc, &opts);
        assert!(r.scene.layers[0].texts.iter().any(|t| t.key == want));
    }
}

/// Counts the lines it is asked for.
struct Counting {
    asked: Cell<usize>,
    last: Cell<usize>,
}

impl Highlighter for Counting {
    fn spans(&self, line: usize, _text: &str) -> Vec<HighlightSpan> {
        self.asked.set(self.asked.get() + 1);
        self.last.set(line);
        Vec::new()
    }
}

#[test]
fn the_highlighter_is_asked_for_the_lines_in_view_only() {
    let mut doc = numbered(10_000);
    let hl = Counting {
        asked: Cell::new(0),
        last: Cell::new(0),
    };
    let opts = CodeOptions {
        highlighter: &hl,
        ..plain()
    };
    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut doc, &opts);
    assert_eq!(hl.asked.get(), 10);
    assert_eq!(hl.last.get(), 9);
}

#[test]
fn a_band_tints_a_full_row_and_a_set_of_0_3_and_10000_on_5_lines_tints_only_line_3() {
    let mut doc = numbered(5);
    let color = [10, 20, 30, 40];
    let bands: Vec<LineBand> = [0, 3, 10_000]
        .into_iter()
        .map(|line| LineBand { line, color })
        .collect();
    let opts = CodeOptions {
        bands: &bands,
        ..plain()
    };
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, &mut doc, &opts);
    assert_eq!(
        fills(&r.scene, color),
        vec![[RECT.x, RECT.y + 2.0 * LINE, RECT.w, LINE]]
    );
    let opts = plain();
    let r = run(&mut h, &mut doc, &opts);
    assert!(fills(&r.scene, color).is_empty());
}

#[test]
fn a_band_out_of_view_draws_nothing_and_one_on_a_wrapped_line_takes_its_height() {
    let color = [10, 20, 30, 40];
    let far = [LineBand { line: 50, color }];
    let mut doc = numbered(100);
    let opts = CodeOptions {
        bands: &far,
        ..plain()
    };
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, &mut doc, &opts);
    assert!(fills(&r.scene, color).is_empty());

    let near = [LineBand { line: 1, color }];
    let mut doc = CodeDoc::new("x".repeat(150));
    let opts = CodeOptions {
        bands: &near,
        ..wrapped()
    };
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, &mut doc, &opts);
    let f = fills(&r.scene, color);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0][3], 3.0 * LINE);
}

#[test]
fn gutter_numbers_right_align_in_fg_faint_and_stay_while_text_scrolls_sideways() {
    let mut doc = numbered(12);
    doc.set_text(format!("{}\n{}", "y".repeat(200), doc.text()));
    let opts = CodeOptions {
        gutter: true,
        ..plain()
    };
    let mut h = UiHarness::new(WIN);
    let style = Style::mono(DARK.font_base, LINE, DARK.fg_faint);
    let number = |scene: &Scene, n: &str| {
        let key = key_of(n, style);
        scene.layers[0]
            .texts
            .iter()
            .find(|t| t.key == key)
            .map(|t| t.x)
    };
    let r = run(&mut h, &mut doc, &opts);
    let right = RECT.x + 4.0 + 2.0 * GLYPH;
    assert_eq!(number(&r.scene, "9"), Some(right - GLYPH));
    assert_eq!(number(&r.scene, "10"), Some(right - 2.0 * GLYPH));
    h.input.pointer_to(200.0, 100.0).scroll_pixels(-120.0, 0.0);
    let r = run(&mut h, &mut doc, &opts);
    assert_eq!(r.r.offset.0, 120.0);
    assert_eq!(number(&r.scene, "9"), Some(right - GLYPH));
}

// Synced panes.

const A: Rect = Rect::new(10.0, 10.0, 100.0, 200.0);
const B: Rect = Rect::new(120.0, 10.0, 100.0, 200.0);
const C: Rect = Rect::new(230.0, 10.0, 100.0, 200.0);
const E: Rect = Rect::new(340.0, 10.0, 100.0, 200.0);

struct Panes {
    docs: [CodeDoc; 4],
    show_c: bool,
}

fn panes(h: &mut UiHarness, p: &mut Panes) -> [f64; 4] {
    let synced = CodeOptions {
        scroll_group: Some("g"),
        ..read_only()
    };
    let mut out = [-1.0; 4];
    let [a, b, c, e] = &mut p.docs;
    let show_c = p.show_c;
    h.frame(|ui| {
        out[0] = ui.code_editor("a", A, a, &synced).offset.1;
        out[1] = ui.code_editor("b", B, b, &synced).offset.1;
        if show_c {
            out[2] = ui.code_editor("c", C, c, &synced).offset.1;
        }
        out[3] = ui.code_editor("e", E, e, &plain()).offset.1;
    });
    out
}

fn four() -> Panes {
    Panes {
        docs: std::array::from_fn(|_| numbered(100)),
        show_c: true,
    }
}

#[test]
fn panes_of_a_group_scroll_together_and_the_editable_one_stays() {
    let mut p = four();
    let mut h = UiHarness::new(WIN);
    panes(&mut h, &mut p);
    h.input.pointer_to(50.0, 100.0).scroll_pixels(0.0, -120.0);
    let o = panes(&mut h, &mut p);
    assert_eq!(o, [120.0, 120.0, 120.0, 0.0]);
    // The second pane moves: the first drew before it, so a catch-up frame carries it.
    h.input.pointer_to(160.0, 100.0).scroll_pixels(0.0, -40.0);
    let o = panes(&mut h, &mut p);
    assert_eq!(o[1], 160.0);
    let o = panes(&mut h, &mut p);
    assert_eq!(o, [160.0, 160.0, 160.0, 0.0]);
    // No echo: nothing moves and nothing asks a frame.
    let o = panes(&mut h, &mut p);
    assert_eq!(o, [160.0, 160.0, 160.0, 0.0]);
    h.input.pointer_leave();
    panes(&mut h, &mut p);
    h.at_ms(5_000);
    panes(&mut h, &mut p);
    let (_, out) = {
        let synced = CodeOptions {
            scroll_group: Some("g"),
            ..read_only()
        };
        let [a, b, c, e] = &mut p.docs;
        h.frame(|ui| {
            ui.code_editor("a", A, a, &synced);
            ui.code_editor("b", B, b, &synced);
            ui.code_editor("c", C, c, &synced);
            ui.code_editor("e", E, e, &plain());
        })
    };
    assert_eq!(out.repaint, Repaint::Idle);
}

#[test]
fn a_hidden_pane_is_skipped_and_catches_up_when_drawn() {
    let mut p = four();
    let mut h = UiHarness::new(WIN);
    panes(&mut h, &mut p);
    p.show_c = false;
    panes(&mut h, &mut p);
    h.input.pointer_to(50.0, 100.0).scroll_pixels(0.0, -120.0);
    let o = panes(&mut h, &mut p);
    assert_eq!((o[0], o[1], o[2]), (120.0, 120.0, -1.0));
    p.show_c = true;
    let o = panes(&mut h, &mut p);
    assert_eq!(o, [120.0, 120.0, 120.0, 0.0]);
}

#[test]
fn a_pane_with_no_group_scrolls_alone() {
    let mut p = four();
    let mut h = UiHarness::new(WIN);
    panes(&mut h, &mut p);
    h.input.pointer_to(390.0, 100.0).scroll_pixels(0.0, -120.0);
    let o = panes(&mut h, &mut p);
    assert_eq!(o, [0.0, 0.0, 0.0, 120.0]);
}

/// Long lines, in the same pair of wrapped panes.
fn wrapped_pair(h: &mut UiHarness, a: &mut CodeDoc, b: &mut CodeDoc) -> [f64; 2] {
    let synced = CodeOptions {
        scroll_group: Some("w"),
        read_only: true,
        ..wrapped()
    };
    let mut out = [0.0; 2];
    h.frame(|ui| {
        out[0] = ui.code_editor("a", A, a, &synced).offset.1;
        out[1] = ui.code_editor("b", B, b, &synced).offset.1;
    });
    out
}

#[test]
fn wrapped_linked_panes_take_a_height_change_above_them_once() {
    // Every line wraps to several rows, so scrolling up measures lines above the first in view, and
    // both panes queue the same shift. A follower that also took it from the group would move twice.
    let text = vec!["w".repeat(60); 300].join("\n");
    let (mut a, mut b) = (CodeDoc::new(text.clone()), CodeDoc::new(text));
    let mut h = UiHarness::new(WIN);
    wrapped_pair(&mut h, &mut a, &mut b);
    wrapped_pair(&mut h, &mut a, &mut b);
    h.input.pointer_to(50.0, 100.0).scroll_pixels(0.0, -3000.0);
    for _ in 0..4 {
        wrapped_pair(&mut h, &mut a, &mut b);
    }
    for _ in 0..6 {
        h.input.scroll_pixels(0.0, 20.0);
        let [x, y] = wrapped_pair(&mut h, &mut a, &mut b);
        assert_eq!(x, y, "the first pane published, the second followed");
        let [x, y] = wrapped_pair(&mut h, &mut a, &mut b);
        assert_eq!(x, y, "the catch-up frame");
    }
}

// Anchors.

#[test]
fn line_7_has_a_rect_and_out_of_view_or_range_none() {
    let mut doc = numbered(100);
    let mut rect = None;
    let mut h = UiHarness::new(WIN);
    let opts = plain();
    let frame = |h: &mut UiHarness, doc: &mut CodeDoc, line: usize, rect: &mut Option<Rect>| {
        h.frame(|ui| {
            ui.code_editor("c", RECT, doc, &opts);
            *rect = ui.code_line_rect("c", line);
        })
    };
    frame(&mut h, &mut doc, 7, &mut rect);
    let at = rect.expect("in view");
    assert_eq!(at, Rect::new(RECT.x, RECT.y + 6.0 * LINE, RECT.w, LINE));
    assert_eq!(at.right() - ANCHOR_INSET, RECT.right() - 16.0);
    for line in [0, 11, 101] {
        frame(&mut h, &mut doc, line, &mut rect);
        assert_eq!(rect, None, "line {line}");
    }
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -200.0);
    frame(&mut h, &mut doc, 7, &mut rect);
    frame(&mut h, &mut doc, 7, &mut rect);
    assert_eq!(rect, None, "scrolled out");
}

#[test]
fn an_overlay_on_a_line_out_of_view_registers_nothing_and_hits_nothing() {
    let mut doc = numbered(100);
    let opts = plain();
    let anchor = WidgetId::ROOT.with("anchor");
    let mut h = UiHarness::new(WIN);
    let mut hit = false;
    let mut frame = |h: &mut UiHarness, doc: &mut CodeDoc| {
        h.frame(|ui| {
            ui.code_editor("c", RECT, doc, &opts);
            if let Some(r) = ui.code_line_rect("c", 7) {
                let b = Rect::new(r.right() - ANCHOR_INSET - 40.0, r.y, 40.0, r.h);
                hit |= ui
                    .interact(anchor, b, lune_ui_core::ui::Sense::CLICK)
                    .clicked;
            }
        });
    };
    frame(&mut h, &mut doc);
    h.input
        .pointer_to(RECT.right() - 30.0, RECT.y + 130.0)
        .click(P);
    frame(&mut h, &mut doc);
    frame(&mut h, &mut doc);
    assert!(hit, "in view, the overlay takes the press first");
    assert_eq!(doc.line(6), "line 6");
}

// Wrap.

fn long_lines(n: usize, chars: usize) -> CodeDoc {
    CodeDoc::new(vec!["w".repeat(chars); n].join("\n"))
}

#[test]
fn wrapped_lines_measure_where_built_and_up_and_down_cross_rows_at_the_preferred_x() {
    // 60 chars fit a row at this width: 150 chars are three rows of 50.
    let mut doc = CodeDoc::new(format!("{}\nend", "w".repeat(150)));
    let mut h = focused(&mut doc, &wrapped());
    run(&mut h, &mut doc, &wrapped());
    for _ in 0..3 {
        press(&mut h, NamedKey::ArrowRight);
    }
    press(&mut h, NamedKey::ArrowDown);
    let r = run(&mut h, &mut doc, &wrapped());
    let c = caret(&r.scene).expect("a caret");
    assert_eq!(c[1], RECT.y + LINE, "the second row of the first line");
    assert_eq!(c[0], TX + 3.0 * GLYPH + 1.0);
    press(&mut h, NamedKey::ArrowDown);
    let r = run(&mut h, &mut doc, &wrapped());
    let c = caret(&r.scene).expect("a caret");
    assert_eq!(c[1], RECT.y + 2.0 * LINE, "across the line's end");
    press(&mut h, NamedKey::ArrowUp);
    press(&mut h, NamedKey::ArrowUp);
    let r = run(&mut h, &mut doc, &wrapped());
    assert_eq!(caret(&r.scene).expect("a caret")[1], RECT.y);
}

#[test]
fn a_line_above_the_window_changing_height_shifts_the_area_so_what_shows_stays() {
    let mut doc = long_lines(300, 200);
    let opts = CodeOptions {
        read_only: true,
        ..wrapped()
    };
    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut doc, &opts);
    // Far from the top on estimates: the lines measured on the way up are 40 high, not 20.
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -4000.0);
    run(&mut h, &mut doc, &opts);
    let before = offset(&run(&mut h, &mut doc, &opts));
    h.input.scroll_pixels(0.0, 5.0);
    run(&mut h, &mut doc, &opts);
    let after = run(&mut h, &mut doc, &opts);
    // Moved by the 5 px asked, plus exactly what the lines measured above it grew by.
    let grew = offset(&after) - (before - 5.0);
    assert!(grew >= 0.0, "{grew}");
    assert_eq!(grew % 20.0, 0.0, "whole rows of growth: {grew}");
    assert!(grew > 0.0, "the instrument saw a line grow");
}

#[test]
fn a_reveal_to_a_far_wrapped_line_lands_on_it_within_two_frames() {
    let mut doc = long_lines(2_000, 200);
    let mut h = focused(&mut doc, &wrapped());
    with_mods(&mut h, META, Key::Named(NamedKey::End));
    run(&mut h, &mut doc, &wrapped());
    let r = run(&mut h, &mut doc, &wrapped());
    let c = caret(&r.scene).expect("the caret shows");
    assert!(c[1] >= RECT.y && c[1] + c[3] <= RECT.bottom(), "{c:?}");
    assert!(offset(&r) > 0.0);

    // And back up through lines that were estimates.
    with_mods(&mut h, META, Key::Named(NamedKey::Home));
    run(&mut h, &mut doc, &wrapped());
    let r = run(&mut h, &mut doc, &wrapped());
    assert_eq!(caret(&r.scene).expect("the caret shows")[1], RECT.y);
    assert_eq!(offset(&r), 0.0);
}

#[test]
fn page_down_in_wrapped_lines_moves_by_visual_rows_and_stays_on_screen() {
    let mut doc = long_lines(200, 200);
    let mut h = focused(&mut doc, &wrapped());
    press(&mut h, NamedKey::PageDown);
    run(&mut h, &mut doc, &wrapped());
    let r = run(&mut h, &mut doc, &wrapped());
    let c = caret(&r.scene).expect("the caret shows");
    assert!(c[1] >= RECT.y && c[1] + c[3] <= RECT.bottom(), "{c:?}");
    typed(&mut h, "X");
    run(&mut h, &mut doc, &wrapped());
    // Four rows a line, so ten rows down from line 0's first is line 2's third.
    assert_eq!(doc.line(2).chars().position(|c| c == 'X'), Some(100));
}

#[test]
fn page_keys_in_one_frame_cross_wrapped_lines_nothing_has_measured_yet() {
    let mut doc = long_lines(200, 200);
    let mut h = focused(&mut doc, &wrapped());
    for _ in 0..41 {
        press(&mut h, NamedKey::PageDown);
    }
    typed(&mut h, "X");
    run(&mut h, &mut doc, &wrapped());
    // Four hundred and ten rows at four a line, at x 0: line 102's third row.
    assert_eq!(doc.line(102).chars().position(|c| c == 'X'), Some(100));
}

#[test]
fn resizing_re_measures_what_is_built() {
    let mut doc = CodeDoc::new("w".repeat(100));
    let opts = wrapped();
    let mut h = UiHarness::new(WIN);
    let frame = |h: &mut UiHarness, doc: &mut CodeDoc, w: f32| {
        let mut rect = None;
        let (_, out) = h.frame(|ui| {
            ui.code_editor("c", Rect::new(20.0, 20.0, w, 200.0), doc, &opts);
            rect = ui.code_line_rect("c", 1);
        });
        (rect.expect("in view").h, out)
    };
    frame(&mut h, &mut doc, 400.0);
    assert_eq!(frame(&mut h, &mut doc, 400.0).0, 2.0 * LINE);
    frame(&mut h, &mut doc, 200.0);
    let (height, _) = frame(&mut h, &mut doc, 200.0);
    assert_eq!(height, 4.0 * LINE);
    frame(&mut h, &mut doc, 200.0);
    assert_eq!(frame(&mut h, &mut doc, 200.0).1.repaint, Repaint::Idle);
}

// Scale.

/// A text system that counts what a frame shapes.
#[derive(Default)]
struct Metered {
    inner: FakeText,
    prepares: usize,
    bytes: usize,
}

impl Metered {
    fn saw(&mut self, spec: &Spec) {
        self.prepares += 1;
        self.bytes += spec.runs.iter().map(|r| r.0.len()).sum::<usize>();
    }
}

impl TextSystem for Metered {
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
        self.saw(spec);
        self.inner.prepare(spec, scale)
    }

    fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos {
        self.saw(spec);
        self.inner.caret(spec, scale, offset)
    }

    fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize {
        self.saw(spec);
        assert!(
            pos.0 >= 0.0,
            "a press left of the text reached the text system"
        );
        self.inner.hit(spec, scale, pos)
    }

    fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan> {
        self.saw(spec);
        self.inner.lines(spec, scale)
    }

    fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
        self.inner.segments(text, unit)
    }
}

/// A window on a [`Metered`] text system.
struct Bench {
    store: UiStore,
    text: Metered,
    input: ScriptedInput,
    shortcuts: Shortcuts,
    strings: StringTable,
}

impl Bench {
    fn new() -> Bench {
        let mut input = ScriptedInput::new(WIN, 1.0);
        input.focus(true);
        Bench {
            store: UiStore::default(),
            text: Metered::default(),
            input,
            shortcuts: Shortcuts::default(),
            strings: StringTable::default(),
        }
    }

    /// One frame: what it shaped, and its repaint.
    fn frame(&mut self, f: impl FnOnce(&mut Ui<'_>)) -> ((usize, usize), Repaint) {
        self.text.prepares = 0;
        self.text.bytes = 0;
        let raw = self.input.end_frame();
        let config = FrameConfig {
            theme: DARK,
            tier: MotionTier::Full,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: Keymap::MAC,
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &raw, config, &mut self.text);
        f(&mut ui);
        let (_, out) = ui.finish();
        ((self.text.prepares, self.text.bytes), out.repaint)
    }
}

/// A settled frame, a scrolled frame, a keystroke frame, and the same wrapped.
fn costs(lines: usize, opts: &CodeOptions<'_>) -> Vec<(usize, usize)> {
    let mut doc = CodeDoc::new("let value = compute(input);\n".repeat(lines));
    let mut b = Bench::new();
    let mut out = Vec::new();
    let mut frame = |b: &mut Bench, doc: &mut CodeDoc| {
        let (cost, repaint) = b.frame(|ui| {
            ui.code_editor("c", RECT, doc, opts);
        });
        out.push(cost);
        repaint
    };
    frame(&mut b, &mut doc);
    b.input.key_down(Key::Named(NamedKey::Tab), None);
    frame(&mut b, &mut doc);
    frame(&mut b, &mut doc);
    assert_eq!(frame(&mut b, &mut doc), Repaint::Idle, "a settled frame");
    b.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -500.0);
    frame(&mut b, &mut doc);
    frame(&mut b, &mut doc);
    // The thumb fades out behind the pointer.
    b.input.pointer_leave();
    frame(&mut b, &mut doc);
    b.input.set_time(Duration::from_secs(5));
    frame(&mut b, &mut doc);
    assert_eq!(frame(&mut b, &mut doc), Repaint::Idle, "after the scroll");
    b.input.key_down(Key::Character("x".to_owned()), Some("x"));
    frame(&mut b, &mut doc);
    frame(&mut b, &mut doc);
    assert_eq!(frame(&mut b, &mut doc), Repaint::Idle, "after the key");
    out
}

#[test]
fn settled_scrolled_and_keystroke_frames_cost_the_same_at_1_000_and_1_000_000_lines() {
    for opts in [
        plain(),
        wrapped(),
        CodeOptions {
            gutter: true,
            ..plain()
        },
    ] {
        let small = costs(1_000, &opts);
        let big = costs(1_000_000, &opts);
        assert_eq!(small, big, "{opts:?}");
        // The instrument saw frames, and frames of a screenful, not of the document.
        let most = big.iter().map(|c| c.1).max().expect("frames");
        assert!(big.iter().all(|c| c.0 > 0 && c.0 < 200), "{big:?}");
        assert!(most > 100 && most < 10_000, "{most}");
    }
}

#[test]
fn a_press_in_the_gutter_reaches_the_text_system_at_the_left_edge() {
    let mut doc = numbered(40);
    let opts = CodeOptions {
        gutter: true,
        ..plain()
    };
    let mut b = Bench::new();
    b.frame(|ui| {
        ui.code_editor("c", RECT, &mut doc, &opts);
    });
    b.input.pointer_to(RECT.x + 3.0, 50.0).click(P);
    b.frame(|ui| {
        ui.code_editor("c", RECT, &mut doc, &opts);
    });
}

#[test]
fn a_million_line_view_reports_idle_at_rest_and_lands_a_far_jump() {
    let mut doc = CodeDoc::new("x\n".repeat(1_000_000));
    let mut h = focused(&mut doc, &plain());
    let r = run(&mut h, &mut doc, &plain());
    let r = if r.out.repaint == Repaint::Idle {
        r
    } else {
        run(&mut h, &mut doc, &plain())
    };
    assert_eq!(r.out.repaint, Repaint::Idle);
    with_mods(&mut h, META, Key::Named(NamedKey::End));
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(offset(&r), 1_000_001.0 * 20.0 - 200.0);
    // A press at the very end hits the right line: y stays exact where an `f32` would not.
    h.input.pointer_to(TX, RECT.y + 150.0).click(P);
    run(&mut h, &mut doc, &plain());
    typed(&mut h, "Q");
    run(&mut h, &mut doc, &plain());
    assert_eq!(doc.line(1_000_001 - 10 + 7), "Qx");
}

// Keys, the wheel in a page, the node.

#[test]
fn a_disabled_view_takes_its_keys_and_acts_on_none() {
    for opts in [plain(), read_only()] {
        let mut doc = numbered(100);
        let mut h = focused(&mut doc, &opts);
        let go = Binding::new(
            Scope::Global,
            Chord::key(Key::Named(NamedKey::PageDown)),
            "go",
        );
        h.shortcuts.add(go);
        press(&mut h, NamedKey::PageDown);
        typed(&mut h, "x");
        let mut resp = None;
        let (_, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                resp = Some(ui.code_editor("c", RECT, &mut doc, &opts));
            });
        });
        let r = resp.expect("ran");
        assert!(out.fired.is_empty(), "{opts:?}");
        assert_eq!(r.offset.1, 0.0);
        assert_eq!(doc.line(0), "line 0");
        assert!(disabled_violations(&out.tree, id()).is_empty());
    }
}

#[test]
fn a_disabled_read_only_view_leaves_the_arrows_to_a_binding() {
    let opts = read_only();
    let mut doc = numbered(100);
    let mut h = focused(&mut doc, &opts);
    let go = Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::ArrowDown)),
        "go",
    );
    h.shortcuts.add(go);
    press(&mut h, NamedKey::ArrowDown);
    let (_, out) = h.frame(|ui| {
        ui.with_enabled(false, |ui| {
            ui.code_editor("c", RECT, &mut doc, &opts);
        });
    });
    assert_eq!(out.fired.len(), 1);
}

/// The view in an outer area of 2,000 px: what the outer and the view scrolled to.
fn in_page(h: &mut UiHarness, doc: &mut CodeDoc, opts: &CodeOptions<'_>) -> (f64, f64) {
    let mut inner = 0.0;
    let scroll = ScrollOptions {
        bars: (Bars::Disabled, Bars::Auto),
        ..ScrollOptions::default()
    };
    let outer = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let (mut outer_at, mut seen) = (0.0, None);
    h.frame(|ui| {
        let r = ui.scroll_area("page", outer, &scroll, |ui, _| {
            seen = Some(ui.code_editor("c", RECT, doc, opts));
            ((), Extent::new(WIN.0 as f64, 2_000.0))
        });
        outer_at = r.offset.1;
    });
    if let Some(r) = seen {
        inner = r.offset.1;
    }
    (outer_at, inner)
}

#[test]
fn the_wheel_over_a_view_that_fits_goes_to_the_page_wrapped_and_not() {
    for opts in [plain(), wrapped()] {
        let mut doc = numbered(3);
        let mut h = UiHarness::new(WIN);
        in_page(&mut h, &mut doc, &opts);
        in_page(&mut h, &mut doc, &opts);
        h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -50.0);
        assert_eq!(in_page(&mut h, &mut doc, &opts), (50.0, 0.0), "{opts:?}");
    }
}

#[test]
fn the_wheel_over_a_view_with_lines_to_scroll_stays_in_it_wrapped_and_not() {
    for opts in [plain(), wrapped()] {
        let mut doc = numbered(100);
        let mut h = UiHarness::new(WIN);
        in_page(&mut h, &mut doc, &opts);
        in_page(&mut h, &mut doc, &opts);
        h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -50.0);
        assert_eq!(in_page(&mut h, &mut doc, &opts), (0.0, 50.0), "{opts:?}");
    }
}

#[test]
fn the_vertical_wheel_passes_a_view_that_only_scrolls_sideways() {
    let mut doc = CodeDoc::new("z".repeat(300));
    let opts = plain();
    let mut h = UiHarness::new(WIN);
    in_page(&mut h, &mut doc, &opts);
    in_page(&mut h, &mut doc, &opts);
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -50.0);
    assert_eq!(in_page(&mut h, &mut doc, &opts), (50.0, 0.0));
}

#[test]
fn the_node_carries_the_lines_in_view_and_the_selection_clipped_to_them() {
    let mut doc = numbered(100);
    let opts = plain();
    let mut h = focused(&mut doc, &opts);
    let r = run(&mut h, &mut doc, &opts);
    let n = r.out.tree.get(id()).expect("node");
    assert_eq!(n.role, Role::MultilineText);
    assert_eq!(n.name, "Code");
    assert!(!n.states.contains(State::ReadOnly));
    assert!(n.states.contains(State::Focusable));
    assert!(n.actions.contains(Action::SetValue));
    let lines: Vec<String> = (0..10).map(|i| format!("line {i}")).collect();
    let (text, sel) = node_text(&r.out);
    assert_eq!(text, lines.join("\n"));
    assert_eq!(n.value, Some(Value::Text(text.clone())));
    assert_eq!(sel, (0, 0));

    // Selected from line 2 to far below the view.
    press(&mut h, NamedKey::ArrowDown);
    press(&mut h, NamedKey::ArrowDown);
    press(&mut h, NamedKey::ArrowRight);
    run(&mut h, &mut doc, &opts);
    h.input.set_modifiers(SHIFT);
    press(&mut h, NamedKey::PageDown);
    press(&mut h, NamedKey::PageDown);
    press(&mut h, NamedKey::PageDown);
    h.input.set_modifiers(Modifiers::default());
    run(&mut h, &mut doc, &opts);
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, 1000.0);
    let r = run(&mut h, &mut doc, &opts);
    let (text, sel) = node_text(&r.out);
    assert_eq!(sel, (15, text.chars().count()));

    let r = run(&mut h, &mut doc, &read_only());
    let n = r.out.tree.get(id()).expect("node");
    assert!(n.states.contains(State::ReadOnly));
    assert!(!n.actions.contains(Action::SetValue));
}

#[test]
fn crlf_terminators_do_not_move_the_nodes_selection() {
    let mut doc = CodeDoc::new("ab\r\ncd\r\nef");
    let mut h = focused(&mut doc, &plain());
    press(&mut h, NamedKey::ArrowDown);
    press(&mut h, NamedKey::ArrowDown);
    press(&mut h, NamedKey::ArrowRight);
    let r = run(&mut h, &mut doc, &plain());
    let (text, sel) = node_text(&r.out);
    assert_eq!(text, "ab\ncd\nef");
    assert_eq!(sel, (7, 7));
}

#[test]
fn a_new_identity_starts_at_the_top_and_forgets_the_heights() {
    let mut doc = long_lines(100, 200);
    let opts = CodeOptions {
        identity: Some(1),
        ..wrapped()
    };
    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut doc, &opts);
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -200.0);
    run(&mut h, &mut doc, &opts);
    assert!(offset(&run(&mut h, &mut doc, &opts)) > 0.0);
    let next = CodeOptions {
        identity: Some(2),
        ..wrapped()
    };
    let r = run(&mut h, &mut doc, &next);
    assert_eq!(offset(&r), 0.0);
}

#[test]
fn a_view_with_no_room_does_not_panic() {
    let mut doc = numbered(3);
    let mut h = UiHarness::new(WIN);
    for rect in [
        Rect::new(0.0, 0.0, 0.0, 0.0),
        Rect::new(0.0, 0.0, f32::NAN, 5.0),
        Rect::new(0.0, 0.0, 10.0, -5.0),
    ] {
        h.frame(|ui| {
            ui.code_editor("c", rect, &mut doc, &CodeOptions::default());
        });
    }
}

#[test]
fn an_empty_document_shows_one_empty_line_and_takes_typing() {
    let mut doc = CodeDoc::default();
    let opts = CodeOptions::default();
    let mut h = focused(&mut doc, &opts);
    let r = run(&mut h, &mut doc, &opts);
    assert_eq!(caret(&r.scene).map(|c| c[3]), Some(LINE));
    typed(&mut h, "x");
    run(&mut h, &mut doc, &opts);
    assert_eq!(doc.text(), "x");
}

#[test]
fn at_scale_2_the_numbers_are_fg_faint_a_band_spans_the_view_and_the_text_is_mono_13() {
    let mut doc = numbered(5);
    let color = [10, 20, 30, 40];
    let bands = [LineBand { line: 2, color }];
    let opts = CodeOptions {
        gutter: true,
        bands: &bands,
        ..plain()
    };
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let r = run(&mut h, &mut doc, &opts);
    let band = fills(&r.scene, color);
    assert_eq!(band.len(), 1);
    assert_eq!(band[0][2], 2.0 * RECT.w, "the band spans the viewport");
    let key = |text: &str, style: Style| FakeText.prepare(&Spec::plain(text, style), 2.0).0;
    let texts = &r.scene.layers[0].texts;
    let number = key("1", Style::mono(DARK.font_base, LINE, DARK.fg_faint));
    assert!(
        texts.iter().any(|t| t.key == number),
        "a number in fg_faint"
    );
    assert_eq!(DARK.font_base, 13.0);
    let line = key("line 0", base());
    assert!(texts.iter().any(|t| t.key == line), "mono at font_base");
}

// A new identity, the link's catch-up, and what an edit does to the heights.

fn other(identity: u64) -> CodeOptions<'static> {
    CodeOptions {
        identity: Some(identity),
        ..plain()
    }
}

#[test]
fn a_new_identity_starts_at_the_top_with_the_caret_there_and_nothing_of_the_old_document() {
    let mut doc = numbered(200);
    let mut h = focused(&mut doc, &other(1));
    with_mods(&mut h, META, Key::Named(NamedKey::End));
    run(&mut h, &mut doc, &other(1));
    let r = run(&mut h, &mut doc, &other(1));
    assert!(offset(&r) > 0.0, "the instrument: it stood at the end");

    doc.set_text(numbered(300).text());
    let r = run(&mut h, &mut doc, &other(2));
    assert_eq!(offset(&r), 0.0);
    assert_eq!(node_text(&r.out).1, (0, 0), "the caret at the top");
    assert_eq!(caret(&r.scene).expect("the caret shows")[1], RECT.y);

    typed(&mut h, "X");
    let r = run(&mut h, &mut doc, &other(2));
    assert_eq!(
        doc.line(0),
        "Xline 0",
        "the first key lands on the first line"
    );
    assert_eq!(offset(&r), 0.0, "and the view stays");
}

#[test]
fn a_new_identity_drops_the_selection_and_the_undo() {
    let mut doc = numbered(20);
    let mut h = focused(&mut doc, &other(1));
    typed(&mut h, "X");
    run(&mut h, &mut doc, &other(1));
    chord(&mut h, "a");
    let r = run(&mut h, &mut doc, &other(1));
    assert_ne!(node_text(&r.out).1, (0, 0), "the instrument: all selected");

    doc.set_text("other\ndocument");
    let r = run(&mut h, &mut doc, &other(2));
    assert_eq!(node_text(&r.out).1, (0, 0));
    assert!(selected(&r.scene, DARK.text_selection).is_empty());
    chord(&mut h, "z");
    run(&mut h, &mut doc, &other(2));
    assert_eq!(
        doc.text(),
        "other\ndocument",
        "the undo stayed with the old one"
    );
}

#[test]
fn a_new_identity_ends_a_composition_and_drops_a_pending_paste() {
    let mut doc = numbered(20);
    let mut h = focused(&mut doc, &other(1));
    preedit(&mut h, "かな", Some((0, 0)));
    let r = run(&mut h, &mut doc, &other(1));
    assert!(!underlines(&r.scene, 1.0).is_empty(), "the instrument");

    let r = run(&mut h, &mut doc, &other(2));
    assert!(underlines(&r.scene, 1.0).is_empty(), "it ended");
    typed(&mut h, "a");
    run(&mut h, &mut doc, &other(2));
    assert_eq!(doc.line(0), "aline 0", "the view takes keys again");

    chord(&mut h, "v");
    let r = run(&mut h, &mut doc, &other(2));
    assert_eq!(r.out.clipboard, vec![ClipboardRequest::Read]);
    let r = run(&mut h, &mut doc, &other(3));
    assert!(r.out.clipboard.is_empty());
    h.input.clipboard_text("stale");
    run(&mut h, &mut doc, &other(3));
    assert_eq!(doc.line(0), "aline 0", "the answer was the old document's");
}

#[test]
fn a_new_identity_takes_no_shift_the_last_document_measured() {
    let mut doc = long_lines(400, 200);
    let one = CodeOptions {
        identity: Some(1),
        ..wrapped()
    };
    let mut h = UiHarness::new(WIN);
    run(&mut h, &mut doc, &one);
    run(&mut h, &mut doc, &one);
    h.input
        .pointer_to(100.0, 100.0)
        .scroll_pixels(0.0, -4_000.0);
    run(&mut h, &mut doc, &one);
    run(&mut h, &mut doc, &one);
    // Up onto lines never measured: this frame queues their growth for the area.
    h.input.scroll_pixels(0.0, 300.0);
    run(&mut h, &mut doc, &one);
    let two = CodeOptions {
        identity: Some(2),
        ..wrapped()
    };
    assert_eq!(offset(&run(&mut h, &mut doc, &two)), 0.0);
    assert_eq!(offset(&run(&mut h, &mut doc, &two)), 0.0);
}

#[test]
fn the_same_identity_keeps_the_caret() {
    let mut doc = numbered(20);
    let mut h = focused(&mut doc, &other(1));
    press(&mut h, NamedKey::ArrowDown);
    let r = run(&mut h, &mut doc, &other(1));
    let at = node_text(&r.out).1;
    assert_ne!(at, (0, 0));
    assert_eq!(node_text(&run(&mut h, &mut doc, &other(1)).out).1, at);
}

#[test]
fn a_pane_alone_on_a_new_identity_takes_its_group_to_the_top() {
    let (mut a, mut b) = (numbered(100), numbered(100));
    let synced = |identity| CodeOptions {
        scroll_group: Some("g"),
        identity: Some(identity),
        ..read_only()
    };
    let mut h = UiHarness::new(WIN);
    let mut two = |h: &mut UiHarness, second: u64| {
        let mut out = (0.0, 0.0);
        h.frame(|ui| {
            out.0 = ui.code_editor("a", A, &mut a, &synced(1)).offset.1;
            out.1 = ui.code_editor("b", B, &mut b, &synced(second)).offset.1;
        });
        out
    };
    two(&mut h, 1);
    h.input.pointer_to(50.0, 50.0).scroll_pixels(0.0, -300.0);
    assert_eq!(two(&mut h, 1), (300.0, 300.0));
    assert_eq!(two(&mut h, 2).1, 0.0);
    assert_eq!(
        two(&mut h, 2),
        (0.0, 0.0),
        "the group followed the new document"
    );
}

#[test]
fn a_pane_hidden_while_its_group_stands_still_comes_back_with_the_group() {
    // The group's record of what the pane took outlasts the pane's own state: a first draw joins
    // where the group stands, whether the group moved meanwhile or not.
    let (mut a, mut b) = (numbered(100), numbered(100));
    let synced = CodeOptions {
        scroll_group: Some("g"),
        identity: Some(1),
        ..read_only()
    };
    let mut h = UiHarness::new(WIN);
    let mut draw = |h: &mut UiHarness, show_b: bool| {
        let mut out = (0.0, -1.0);
        let (_, frame) = h.frame(|ui| {
            out.0 = ui.code_editor("a", A, &mut a, &synced).offset.1;
            if show_b {
                out.1 = ui.code_editor("b", B, &mut b, &synced).offset.1;
            }
        });
        (out, frame.repaint)
    };
    draw(&mut h, true);
    h.input.pointer_to(50.0, 50.0).scroll_pixels(0.0, -120.0);
    assert_eq!(draw(&mut h, true).0, (120.0, 120.0));
    h.input.pointer_leave();
    for _ in 0..3 {
        assert_eq!(draw(&mut h, false).0, (120.0, -1.0));
    }
    assert_eq!(
        draw(&mut h, true).0,
        (120.0, 120.0),
        "in the frame it returns"
    );
    h.at_ms(5_000);
    assert_eq!(draw(&mut h, true).1, Repaint::Idle, "and the page rests");
}

#[test]
fn a_line_wheel_on_the_second_pane_rests_with_both_panes_equal() {
    let (mut a, mut b) = (numbered(100), numbered(100));
    let synced = CodeOptions {
        scroll_group: Some("g"),
        ..read_only()
    };
    let mut h = UiHarness::new(WIN);
    let mut two = |h: &mut UiHarness| {
        let mut seen = (0.0, 0.0);
        let (_, out) = h.frame(|ui| {
            seen.0 = ui.code_editor("a", A, &mut a, &synced).offset.1;
            seen.1 = ui.code_editor("b", B, &mut b, &synced).offset.1;
        });
        (seen, out.repaint)
    };
    two(&mut h);
    h.input.pointer_to(170.0, 50.0).scroll_lines(0.0, -3.0);
    let mut ms = 0;
    for _ in 0..200 {
        ms += 16;
        h.at_ms(ms);
        let (seen, repaint) = two(&mut h);
        if repaint == Repaint::Idle {
            assert!(seen.1 > 0.0, "the wheel moved it");
            assert_eq!(seen.0, seen.1, "the panes rest equal");
            return;
        }
    }
    panic!("the page did not settle");
}

#[test]
fn a_short_pane_clamped_by_a_pull_does_not_publish_its_clamped_value() {
    let (mut long, mut short) = (numbered(100), numbered(12));
    let synced = CodeOptions {
        scroll_group: Some("g"),
        ..read_only()
    };
    let mut h = UiHarness::new(WIN);
    let mut two = |h: &mut UiHarness| {
        let mut seen = (0.0, 0.0);
        h.frame(|ui| {
            seen.0 = ui.code_editor("a", A, &mut long, &synced).offset.1;
            seen.1 = ui.code_editor("b", B, &mut short, &synced).offset.1;
        });
        seen
    };
    two(&mut h);
    h.input.pointer_to(50.0, 50.0).scroll_pixels(0.0, -500.0);
    assert_eq!(two(&mut h).0, 500.0);
    assert_eq!(
        two(&mut h),
        (500.0, 40.0),
        "the short pane stops at its end"
    );
    assert_eq!(two(&mut h), (500.0, 40.0));
    assert_eq!(two(&mut h), (500.0, 40.0), "and the long one stays");
}

#[test]
fn enter_on_the_last_line_shows_the_new_line_in_the_same_frame() {
    let mut doc = numbered(30);
    let mut h = focused(&mut doc, &plain());
    with_mods(&mut h, META, Key::Named(NamedKey::End));
    run(&mut h, &mut doc, &plain());
    let before = offset(&run(&mut h, &mut doc, &plain()));
    assert!(before > 0.0, "the instrument: the view is full");
    press(&mut h, NamedKey::Enter);
    let r = run(&mut h, &mut doc, &plain());
    assert_eq!(offset(&r), before + f64::from(LINE));
}

#[test]
fn a_set_value_draws_the_new_document_in_its_frame() {
    let action = |text: &str| AccessAction {
        target: id(),
        action: Action::SetValue,
        value: Some(AccessValue::Text(text.to_owned())),
    };
    // Shorter than where the view stands: its last lines draw, not an empty window.
    let mut doc = numbered(200);
    let mut h = focused(&mut doc, &plain());
    with_mods(&mut h, META, Key::Named(NamedKey::End));
    run(&mut h, &mut doc, &plain());
    run(&mut h, &mut doc, &plain());
    h.input.access(action("a\nb"));
    let r = run(&mut h, &mut doc, &plain());
    assert!(node_text(&r.out).0.contains('b'), "{:?}", node_text(&r.out));

    // Longer than the document was, wrapped: the new lines take their place at once.
    let text = vec!["w".repeat(150); 40].join("\n");
    let mut doc = numbered(2);
    let mut h = focused(&mut doc, &wrapped());
    h.input.access(action(&text));
    let r = run(&mut h, &mut doc, &wrapped());
    assert!(
        node_text(&r.out).0.lines().count() > 2,
        "{:?}",
        node_text(&r.out)
    );
}

#[test]
fn a_selection_ending_on_a_line_end_draws_no_sliver_there() {
    let mut doc = CodeDoc::new("abc\ndef");
    let mut h = focused(&mut doc, &plain());
    with_mods(&mut h, SHIFT, Key::Named(NamedKey::End));
    let r = run(&mut h, &mut doc, &plain());
    let f = selected(&r.scene, DARK.text_selection);
    // The text and nothing past it but the padding.
    let pad = DARK.selection_pad;
    assert_eq!(f, [[TX - pad, RECT.y, 3.0 * GLYPH + 2.0 * pad, LINE]]);
}

// A new identity and the link: the group holds nothing of the last document.

/// Two linked panes on `docs`, each with its own identity, the first drawn first; their offsets
/// and the frame's repaint.
fn pair(
    h: &mut UiHarness,
    docs: &mut [CodeDoc; 2],
    base: &CodeOptions<'_>,
    ids: [u64; 2],
) -> ([f64; 2], Repaint) {
    let [a, b] = docs;
    let opts = |id| CodeOptions {
        identity: Some(id),
        scroll_group: Some("g"),
        ..*base
    };
    let mut seen = [-1.0; 2];
    let (_, out) = h.frame(|ui| {
        seen[0] = ui.code_editor("a", A, a, &opts(ids[0])).offset.1;
        seen[1] = ui.code_editor("b", B, b, &opts(ids[1])).offset.1;
    });
    (seen, out.repaint)
}

#[test]
fn a_wheel_on_the_second_pane_then_a_new_identity_shows_no_old_offset_in_either_pane() {
    let mut docs = [numbered(100), numbered(100)];
    let mut h = UiHarness::new(WIN);
    let ro = read_only();
    pair(&mut h, &mut docs, &ro, [1, 1]);
    h.input.pointer_to(170.0, 50.0).scroll_pixels(0.0, -500.0);
    let (o, _) = pair(&mut h, &mut docs, &ro, [1, 1]);
    assert_eq!(o, [0.0, 500.0], "the first pane drew before the move");
    let (o, _) = pair(&mut h, &mut docs, &ro, [2, 2]);
    assert_eq!(o, [0.0, 0.0], "the frame of the switch");
    h.at_ms(5_000);
    let (o, repaint) = pair(&mut h, &mut docs, &ro, [2, 2]);
    assert_eq!((o, repaint), ([0.0, 0.0], Repaint::Idle));
}

#[test]
fn a_far_jump_on_the_second_pane_then_a_new_identity_tops_both_panes() {
    let mut docs = [numbered(100), numbered(100)];
    let mut h = UiHarness::new(WIN);
    let ro = read_only();
    pair(&mut h, &mut docs, &ro, [1, 1]);
    h.input.pointer_to(170.0, 50.0).click(P);
    pair(&mut h, &mut docs, &ro, [1, 1]);
    press(&mut h, NamedKey::End);
    let (o, _) = pair(&mut h, &mut docs, &ro, [1, 1]);
    assert_eq!(o, [0.0, 1800.0], "the instrument: the jump");
    let (o, _) = pair(&mut h, &mut docs, &ro, [2, 2]);
    assert_eq!(o, [0.0, 0.0], "the frame of the switch");
    h.at_ms(5_000);
    let (o, repaint) = pair(&mut h, &mut docs, &ro, [2, 2]);
    assert_eq!((o, repaint), ([0.0, 0.0], Repaint::Idle));
}

#[test]
fn a_pane_alone_on_a_new_identity_before_it_took_the_last_move_still_takes_the_group_to_the_top() {
    let mut docs = [numbered(100), numbered(100)];
    let mut h = UiHarness::new(WIN);
    let ro = read_only();
    pair(&mut h, &mut docs, &ro, [1, 1]);
    h.input.pointer_to(170.0, 50.0).scroll_pixels(0.0, -500.0);
    let (o, _) = pair(&mut h, &mut docs, &ro, [1, 1]);
    assert_eq!(o, [0.0, 500.0], "the first pane has not taken it");
    let (o, _) = pair(&mut h, &mut docs, &ro, [2, 1]);
    assert_eq!(o, [0.0, 0.0], "it pulled nothing, and the second followed");
    h.at_ms(5_000);
    let (o, repaint) = pair(&mut h, &mut docs, &ro, [2, 1]);
    assert_eq!((o, repaint), ([0.0, 0.0], Repaint::Idle));

    // The same value again reaches the pane that opened.
    h.input.scroll_pixels(0.0, -500.0);
    let (o, _) = pair(&mut h, &mut docs, &ro, [2, 1]);
    assert_eq!(o, [0.0, 500.0]);
    let (o, _) = pair(&mut h, &mut docs, &ro, [2, 1]);
    assert_eq!(o, [500.0, 500.0], "the pane follows the group again");
}

#[test]
fn a_short_pane_at_its_top_opening_a_document_alone_takes_the_group_to_the_top() {
    // It fits, so it stands at 0 with the group's value taken: no move of its own shows it left.
    let mut docs = [numbered(100), numbered(5)];
    let mut h = UiHarness::new(WIN);
    let ro = read_only();
    pair(&mut h, &mut docs, &ro, [1, 1]);
    h.input.pointer_to(50.0, 50.0).scroll_pixels(0.0, -500.0);
    let (o, _) = pair(&mut h, &mut docs, &ro, [1, 1]);
    assert_eq!(o, [500.0, 0.0]);
    docs[1] = numbered(100);
    let (o, repaint) = pair(&mut h, &mut docs, &ro, [1, 2]);
    assert_eq!(o, [500.0, 0.0], "the first pane drew before it");
    assert_eq!(
        repaint,
        Repaint::Now,
        "so it asks the frame that carries it"
    );
    let (o, _) = pair(&mut h, &mut docs, &ro, [1, 2]);
    assert_eq!(o, [0.0, 0.0], "the panes line up again");
}

#[test]
fn a_pane_with_an_identity_drawn_again_catches_up_with_its_group() {
    // A first draw is no new document: nothing of the pane's last draw is left, and it joins.
    let (mut a, mut b) = (numbered(100), numbered(100));
    let synced = CodeOptions {
        identity: Some(1),
        scroll_group: Some("g"),
        ..read_only()
    };
    let mut h = UiHarness::new(WIN);
    h.frame(|ui| {
        ui.code_editor("a", A, &mut a, &synced);
        ui.code_editor("b", B, &mut b, &synced);
    });
    h.frame(|ui| {
        ui.code_editor("a", A, &mut a, &synced);
    });
    h.input.pointer_to(50.0, 50.0).scroll_pixels(0.0, -120.0);
    h.frame(|ui| {
        ui.code_editor("a", A, &mut a, &synced);
    });
    let mut o = [-1.0; 2];
    h.frame(|ui| {
        o[0] = ui.code_editor("a", A, &mut a, &synced).offset.1;
        o[1] = ui.code_editor("b", B, &mut b, &synced).offset.1;
    });
    assert_eq!(o, [120.0, 120.0], "in the frame it came back");
}

#[test]
fn a_pane_that_joins_after_the_group_opened_a_document_finds_no_old_value() {
    // The pane that published is gone, the one left stands at 0: only the group held the value.
    let (mut long, mut short, mut late) = (numbered(100), numbered(5), numbered(100));
    let synced = |identity| CodeOptions {
        identity: Some(identity),
        scroll_group: Some("g"),
        ..read_only()
    };
    let mut h = UiHarness::new(WIN);
    h.frame(|ui| {
        ui.code_editor("a", A, &mut long, &synced(1));
        ui.code_editor("b", B, &mut short, &synced(1));
    });
    h.input.pointer_to(50.0, 50.0).scroll_pixels(0.0, -500.0);
    h.frame(|ui| {
        ui.code_editor("a", A, &mut long, &synced(1));
        ui.code_editor("b", B, &mut short, &synced(1));
    });
    h.frame(|ui| {
        ui.code_editor("b", B, &mut short, &synced(2));
    });
    let mut o = -1.0;
    h.frame(|ui| {
        ui.code_editor("b", B, &mut short, &synced(2));
        o = ui.code_editor("c", C, &mut late, &synced(1)).offset.1;
    });
    assert_eq!(o, 0.0);
}

#[test]
fn a_short_pane_with_an_identity_comes_back_with_its_group() {
    // An area keeps `wanted` for its document: asked past its end, it would stay there.
    let mut docs = [numbered(100), numbered(14)];
    let mut h = UiHarness::new(WIN);
    let ro = read_only();
    pair(&mut h, &mut docs, &ro, [1, 1]);
    h.input.pointer_to(50.0, 50.0).scroll_pixels(0.0, -500.0);
    pair(&mut h, &mut docs, &ro, [1, 1]);
    assert_eq!(pair(&mut h, &mut docs, &ro, [1, 1]).0, [500.0, 80.0]);
    h.input.scroll_pixels(0.0, 460.0);
    assert_eq!(pair(&mut h, &mut docs, &ro, [1, 1]).0, [40.0, 40.0]);
    h.input.scroll_pixels(0.0, -500.0);
    pair(&mut h, &mut docs, &ro, [1, 1]);
    assert_eq!(pair(&mut h, &mut docs, &ro, [1, 1]).0, [540.0, 80.0]);
    // The long pane opens a document alone: the short one goes to the top with the group.
    let (o, _) = pair(&mut h, &mut docs, &ro, [2, 1]);
    assert_eq!(o, [0.0, 0.0]);
}

/// A deterministic generator: the tests must not depend on the process.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % below
    }
}

/// One step of the walk on a pane: a wheel, a focus, or a key for the focused pane. Whether it
/// animates, for a pane that keeps its course through another pane's switch.
fn wander(h: &mut UiHarness, r: &mut Lcg, read_only: bool) -> bool {
    let (x, y) = ([50.0, 170.0][r.next(2)], 20.0 + 20.0 * r.next(8) as f32);
    let down = if r.next(4) == 0 { 1.0 } else { -1.0 };
    let mut flies = false;
    match r.next(6) {
        0 => {
            let by = 20.0 * (1 + r.next(60)) as f32;
            h.input.pointer_to(x, y).scroll_pixels(0.0, by * down);
        }
        1 => {
            h.input.pointer_to(x, y).scroll_lines(0.0, 3.0 * down);
            flies = true;
        }
        2 => {
            h.input.pointer_to(x, y).click(P);
        }
        3 if read_only => press(h, NamedKey::End),
        3 => with_mods(h, META, Key::Named(NamedKey::End)),
        4 if read_only => press(h, NamedKey::Home),
        4 => with_mods(h, META, Key::Named(NamedKey::Home)),
        _ => {
            press(h, NamedKey::PageDown);
            flies = true;
        }
    }
    flies
}

/// How often a walk did what it is there to do.
#[derive(Default)]
struct Walked {
    switches: usize,
    /// Switches made with a pane away from the top.
    away: usize,
    /// Switches in the frame after a move.
    close: usize,
    /// Switches of one pane.
    alone: usize,
}

/// One pair of linked panes walked at random: moves, then a new identity on one pane or both.
fn walk(wrap: bool, read_only: bool, second: usize, seed: u64, seen: &mut Walked) {
    let base = CodeOptions {
        wrap,
        read_only,
        ..plain()
    };
    let doc = |n| {
        if wrap { long_lines(n, 30) } else { numbered(n) }
    };
    let mut docs = [doc(200), doc(second)];
    let (mut r, mut ids, mut ms) = (Lcg(seed), [1, 1], 0);
    let mut h = UiHarness::new(WIN);
    let mut frame = |h: &mut UiHarness, by: u64, ids: [u64; 2]| {
        ms += by;
        h.at_ms(ms);
        pair(h, &mut docs, &base, ids)
    };
    frame(&mut h, 16, ids);
    let mut flying = false;
    for step in 0..100 {
        // A jump in the frame after a far one, wrapped, lands off by what that frame measured
        // there: the area's own limit, no document's.
        if wrap {
            frame(&mut h, 16, ids);
        }
        flying |= wander(&mut h, &mut r, read_only);
        let mut last = frame(&mut h, 16, ids).0;
        if r.next(3) != 0 {
            continue;
        }
        // Zero frames is the window: what the move published has not reached the panes.
        let gap = r.next(3);
        for _ in 0..gap {
            last = frame(&mut h, 16, ids).0;
        }
        let which = r.next(3);
        // A pane alone leaves the other's flight on its course, so that one rests first.
        if which != 2 && flying {
            for _ in 0..8 {
                last = frame(&mut h, 100, ids).0;
            }
        }
        flying = false;
        seen.switches += 1;
        seen.away += usize::from(last != [0.0, 0.0]);
        seen.close += usize::from(gap == 0);
        seen.alone += usize::from(which != 2);

        let opens = [which != 1, which != 0];
        ids[0] += u64::from(opens[0]);
        ids[1] += u64::from(opens[1]);
        let at = format!("wrap {wrap} read-only {read_only} {second} seed {seed} step {step}");
        let (mut o, mut repaint) = frame(&mut h, 16, ids);
        for (o, opens) in o.iter().zip(opens) {
            assert!(!opens || *o == 0.0, "{at}: the frame of the switch: {o}");
        }
        for n in 0.. {
            assert!(which != 2 || o == [0.0, 0.0], "{at} frame {n}: {o:?}");
            if repaint == Repaint::Idle {
                break;
            }
            assert!(n < 40, "{at}: the page did not settle");
            (o, repaint) = frame(&mut h, 100, ids);
        }
        assert_eq!(o, [0.0, 0.0], "{at}: at rest");
    }
}

#[test]
fn a_walk_of_scrolls_and_new_identities_opens_every_document_at_the_top_of_every_linked_pane() {
    let mut seen = Walked::default();
    for wrap in [false, true] {
        for read_only in [true, false] {
            // Equal, one that stops short of the other, and one that fits.
            for second in [200, 14, 5] {
                for seed in 0..5 {
                    walk(wrap, read_only, second, seed, &mut seen);
                }
            }
        }
    }
    assert!(seen.away > 900, "the instrument: {}", seen.away);
    assert!(seen.close > 500, "the instrument: {}", seen.close);
    assert!(seen.alone > 800, "the instrument: {}", seen.alone);
    assert!(seen.switches > 1_500, "the instrument: {}", seen.switches);
}

#[test]
fn the_code_editor_sets_no_insert_limit_so_it_takes_more_than_the_shaped_limit() {
    use lune_ui_core::text_edit::MAX_SHAPED_INSERT;
    // It draws only the lines in view, so the adapter's cap is its bound. Lines of 49 bytes: just
    // over the shaped widgets' limit goes in by set-value and by paste.
    let big =
        "0123456789012345678901234567890123456789012345678\n".repeat(MAX_SHAPED_INSERT / 50 + 2);
    assert!(big.len() > MAX_SHAPED_INSERT);

    let mut doc = CodeDoc::new("old");
    let mut h = focused(&mut doc, &plain());
    h.input.access(AccessAction {
        target: id(),
        action: Action::SetValue,
        value: Some(AccessValue::Text(big.clone())),
    });
    let r = run(&mut h, &mut doc, &plain());
    assert!(r.r.changed);
    assert_eq!(doc.text().len(), big.len());

    let mut doc = CodeDoc::new("old");
    let mut h = focused(&mut doc, &plain());
    chord(&mut h, "v");
    run(&mut h, &mut doc, &plain());
    h.input.clipboard_text(&big);
    let r = run(&mut h, &mut doc, &plain());
    assert!(r.r.changed);
    assert_eq!(doc.text().len(), big.len() + 3);
}
