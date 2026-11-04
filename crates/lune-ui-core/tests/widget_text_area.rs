//! The multi-line text box (`widget.text-field` on `layout.scroll-area`): Enter, vertical motion,
//! the caret in view at once, selection per line, the wheel, the node and retention.

use lune_ui_core::access::{AccessAction, AccessValue, Action, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{
    ClipboardRequest, CursorIcon, FrameOutput, ImeEvent, Key, Modifiers, NamedKey, PointerButton,
    Repaint,
};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::field::{FOCUS, FieldResponse};
use lune_ui_core::widgets::text_area::AreaOptions;

const WIN: (f32, f32) = (400.0, 300.0);
const RECT: Rect = Rect::new(20.0, 20.0, 200.0, 100.0);
/// The text's left and top edges: the frame of 1 and the padding.
const TX: f32 = 30.0;
const TY: f32 = 27.0;

fn id() -> WidgetId {
    WidgetId::ROOT.with("a")
}

fn line_h() -> f32 {
    Style::ui(DARK.font_base, DARK.fg_primary).line
}

fn glyph() -> f32 {
    DARK.font_base * 0.5
}

fn run(
    h: &mut UiHarness,
    text: &mut String,
    opts: &AreaOptions<'_>,
) -> (Scene, FieldResponse, FrameOutput) {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| {
        resp = Some(ui.text_area("a", RECT, text, opts));
    });
    (scene, resp.expect("ran"), out)
}

fn plain(h: &mut UiHarness, text: &mut String) -> (Scene, FieldResponse, FrameOutput) {
    run(h, text, &AreaOptions::default())
}

fn focused(text: &mut String, opts: &AreaOptions<'_>) -> UiHarness {
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

fn selection(out: &FrameOutput) -> (usize, usize) {
    out.tree
        .get(id())
        .and_then(|n| n.text.as_ref())
        .expect("text")
        .selection
}

fn caret(scene: &Scene) -> [f32; 4] {
    let over = &scene.layers[0].geometry.over;
    over.iter()
        .find(|s| s.rect[2] == 1.5)
        .expect("a caret")
        .rect
}

fn scroll_offset(out: &FrameOutput) -> f64 {
    let area = out.tree.get(id().with("area")).expect("area node");
    match area.value {
        Some(Value::Number { value, .. }) => value,
        _ => panic!("no number"),
    }
}

fn numbered(n: usize) -> String {
    (0..n)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn enter_inserts_a_line_break_and_submits_nothing() {
    let mut text = String::from("ab");
    let mut h = focused(&mut text, &AreaOptions::default());
    press(&mut h, NamedKey::Enter);
    typed(&mut h, "c");
    let (scene, r, _) = plain(&mut h, &mut text);
    assert_eq!(text, "ab\nc");
    assert!(r.response.changed && !r.submitted);
    let c = caret(&scene);
    assert_eq!(c[0], TX + glyph() + 1.0);
    assert_eq!(c[1], TY + line_h());
}

#[test]
fn the_node_is_multiline_text_inside_the_scroll_area() {
    let opts = AreaOptions {
        label: Some("Notes"),
        description: Some("Free text"),
        error: true,
        ..AreaOptions::default()
    };
    let mut text = String::from("a\nb");
    let mut h = UiHarness::new(WIN);
    let (_, _, out) = run(&mut h, &mut text, &opts);
    let n = out.tree.get(id()).expect("node");
    assert_eq!(n.role, Role::MultilineText);
    assert_eq!(n.name, "Notes");
    assert_eq!(n.description.as_deref(), Some("Free text"));
    assert_eq!(n.parent, Some(id().with("area")));
    assert_eq!(n.value, Some(Value::Text(text.clone())));
    assert_eq!(n.text.as_ref().map(|t| t.selection), Some((3, 3)));
    assert!(n.states.contains(State::Invalid) && n.states.contains(State::Focusable));
    let area = out.tree.get(id().with("area")).expect("area");
    assert_eq!(area.role, Role::ScrollArea);
}

#[test]
fn wrap_passes_the_inner_width_to_the_spec() {
    let opts = AreaOptions {
        wrap: true,
        ..AreaOptions::default()
    };
    let mut text = String::from("hello");
    let mut h = UiHarness::new(WIN);
    let (scene, _, _) = run(&mut h, &mut text, &opts);
    let style = Style::ui(DARK.font_base, DARK.fg_primary);
    let inner = RECT.w - 2.0 - 18.0 - 2.5;
    let spec = Spec::plain("hello", style).wrapped(inner);
    assert_eq!(scene.layers[0].texts[0].key, FakeText.prepare(&spec, 1.0).0);
    let (scene, _, _) = plain(&mut h, &mut text);
    let spec = Spec::plain("hello", style);
    assert_eq!(scene.layers[0].texts[0].key, FakeText.prepare(&spec, 1.0).0);
}

#[test]
fn up_and_down_keep_the_column_and_page_keys_cross_a_viewport() {
    let mut text = numbered(40);
    let mut h = focused(&mut text, &AreaOptions::default());
    press(&mut h, NamedKey::End);
    let logo = Modifiers {
        logo: true,
        ..Modifiers::default()
    };
    h.input.set_modifiers(logo);
    press(&mut h, NamedKey::Home);
    h.input.set_modifiers(Modifiers::default());
    press(&mut h, NamedKey::ArrowRight);
    press(&mut h, NamedKey::ArrowDown);
    press(&mut h, NamedKey::ArrowDown);
    let (_, _, out) = plain(&mut h, &mut text);
    let line_len = "line 0\n".len();
    assert_eq!(selection(&out).0, 2 * line_len + 1);
    press(&mut h, NamedKey::PageDown);
    let (_, _, out) = plain(&mut h, &mut text);
    let rows = ((RECT.h - 2.0 - 12.0) / line_h()).floor() as usize;
    assert_eq!(selection(&out).0, (2 + rows) * line_len + 1);
}

#[test]
fn the_caret_comes_into_view_at_once() {
    let mut text = numbered(40);
    let mut h = focused(&mut text, &AreaOptions::default());
    // The caret starts past the viewport: the first move brings it in.
    h.at_ms(1000);
    press(&mut h, NamedKey::ArrowLeft);
    let (scene, _, out) = plain(&mut h, &mut text);
    let c = caret(&scene);
    assert!(c[1] >= RECT.y && c[1] + c[3] <= RECT.bottom(), "{c:?}");
    let at_end = scroll_offset(&out);
    assert!(at_end > 0.0);
    // Typing never animates: nothing is in flight after the one catch-up frame.
    let (_, _, out) = plain(&mut h, &mut text);
    assert_eq!(out.repaint, Repaint::Idle);
    assert_eq!(scroll_offset(&out), at_end);
    // Going up past the top edge moves the least.
    for _ in 0..12 {
        press(&mut h, NamedKey::ArrowUp);
    }
    let (scene, _, out) = plain(&mut h, &mut text);
    let c = caret(&scene);
    assert!(c[1] >= RECT.y && c[1] + c[3] <= RECT.bottom(), "{c:?}");
    assert!(scroll_offset(&out) < at_end);
}

#[test]
fn without_wrap_the_caret_scrolls_sideways_too() {
    let mut text = "w".repeat(80);
    let mut h = focused(&mut text, &AreaOptions::default());
    press(&mut h, NamedKey::ArrowLeft);
    let (scene, _, _) = plain(&mut h, &mut text);
    let c = caret(&scene);
    assert!(c[0] + c[2] <= RECT.right(), "{c:?}");
    press(&mut h, NamedKey::Home);
    let (scene, _, _) = plain(&mut h, &mut text);
    assert_eq!(caret(&scene)[0], TX + 1.0);
}

#[test]
fn the_wheel_scrolls_and_does_not_pull_back_to_the_caret() {
    let mut text = numbered(40);
    let mut h = UiHarness::new(WIN);
    plain(&mut h, &mut text);
    h.input.pointer_to(100.0, 60.0).scroll_pixels(0.0, -90.0);
    let (_, _, out) = plain(&mut h, &mut text);
    assert_eq!(scroll_offset(&out), 90.0);
    let (_, _, out) = plain(&mut h, &mut text);
    assert_eq!(scroll_offset(&out), 90.0);
}

#[test]
fn a_press_lands_on_the_scrolled_line_and_a_drag_selects_per_line() {
    let mut text = numbered(40);
    let mut h = UiHarness::new(WIN);
    plain(&mut h, &mut text);
    h.input
        .pointer_to(TX + 100.0, TY + 2.0)
        .scroll_pixels(0.0, -3.0 * line_h());
    plain(&mut h, &mut text);
    // Row 0 of the viewport is line 3, after the padding's worth of it.
    h.input
        .pointer_to(TX + glyph() + 0.5, TY + 1.0)
        .press(PointerButton::Primary);
    let (_, r, out) = plain(&mut h, &mut text);
    assert!(r.response.focused);
    assert_eq!(selection(&out), (3 * 7 + 1, 3 * 7 + 1));
    h.input
        .pointer_to(TX + 2.0 * glyph() + 0.5, TY + 2.0 * line_h() + 1.0);
    let (scene, _, out) = plain(&mut h, &mut text);
    assert_eq!(selection(&out), (3 * 7 + 1, 5 * 7 + 2));
    let fill = premul(DARK.text_selection);
    let fills: Vec<Shape> = scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.color == fill)
        .copied()
        .collect();
    assert_eq!(fills.len(), 3, "one box per line");
    assert_eq!(fills[0].rect[0], TX + glyph());
    assert_eq!(fills[1].rect[0], TX);
    assert_eq!(fills[1].rect[2], 6.0 * glyph());
    assert_eq!(fills[2].rect[2], 2.0 * glyph());
    assert_eq!(fills[1].rect[1] - fills[0].rect[1], line_h());
}

#[test]
fn the_cursor_is_text_and_a_thumb_press_reaches_no_text() {
    let mut text = numbered(40);
    let mut h = UiHarness::new(WIN);
    plain(&mut h, &mut text);
    h.input.pointer_to(100.0, 60.0);
    let (_, _, out) = plain(&mut h, &mut text);
    assert_eq!(out.cursor, CursorIcon::Text);
    plain(&mut h, &mut text);
    h.input
        .pointer_to(RECT.right() - 4.0, RECT.y + 6.0)
        .press(PointerButton::Primary);
    let (_, r, _) = plain(&mut h, &mut text);
    assert!(!r.response.focused, "the thumb took the press");
}

#[test]
fn read_only_scrolls_selects_and_copies_but_enter_edits_nothing() {
    let opts = AreaOptions {
        read_only: true,
        ..AreaOptions::default()
    };
    let mut text = numbered(3);
    let mut h = focused(&mut text, &opts);
    press(&mut h, NamedKey::Enter);
    typed(&mut h, "x");
    let logo = Modifiers {
        logo: true,
        ..Modifiers::default()
    };
    h.input.set_modifiers(logo);
    h.input.key_down(Key::Character("a".to_owned()), None);
    h.input.key_down(Key::Character("c".to_owned()), None);
    h.input.set_modifiers(Modifiers::default());
    let (_, _, out) = run(&mut h, &mut text, &opts);
    assert_eq!(text, numbered(3));
    assert_eq!(out.clipboard, [ClipboardRequest::Write(numbered(3))]);
    let n = out.tree.get(id()).expect("node");
    assert!(n.states.contains(State::ReadOnly));
    assert_eq!(out.ime_area, None);
}

#[test]
fn paste_keeps_line_breaks_and_max_chars_holds() {
    let opts = AreaOptions {
        max_chars: Some(5),
        ..AreaOptions::default()
    };
    let mut text = String::new();
    let mut h = focused(&mut text, &opts);
    let logo = Modifiers {
        logo: true,
        ..Modifiers::default()
    };
    h.input.set_modifiers(logo);
    h.input.key_down(Key::Character("v".to_owned()), None);
    h.input.set_modifiers(Modifiers::default());
    run(&mut h, &mut text, &opts);
    h.input.clipboard_text("a\r\nb\ncdef");
    run(&mut h, &mut text, &opts);
    assert_eq!(text, "a\nb\nc");
}

#[test]
fn set_value_replaces_the_text_and_marks_changed() {
    let mut text = String::from("old");
    let mut h = UiHarness::new(WIN);
    plain(&mut h, &mut text);
    h.input.access(AccessAction {
        target: id(),
        action: Action::SetValue,
        value: Some(AccessValue::Text("new\ntext".to_owned())),
    });
    let (_, r, _) = plain(&mut h, &mut text);
    assert_eq!(text, "new\ntext");
    assert!(r.response.changed);
}

#[test]
fn a_disabled_area_takes_nothing() {
    let mut text = String::from("x");
    let mut h = UiHarness::new(WIN);
    let area = |h: &mut UiHarness, text: &mut String| {
        let mut resp = None;
        let (_, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                resp = Some(ui.text_area("a", RECT, text, &AreaOptions::default()));
            });
        });
        (resp.expect("ran"), out)
    };
    area(&mut h, &mut text);
    h.input
        .pointer_to(100.0, 60.0)
        .click(PointerButton::Primary);
    typed(&mut h, "y");
    let (r, out) = area(&mut h, &mut text);
    assert!(!r.response.focused && !r.response.enabled);
    assert_eq!(text, "x");
    assert!(
        out.tree
            .get(id())
            .expect("n")
            .states
            .contains(State::Disabled)
    );
}

#[test]
fn the_ring_follows_the_focus_row() {
    let mut text = String::new();
    let mut h = UiHarness::new(WIN);
    plain(&mut h, &mut text);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    plain(&mut h, &mut text);
    let duration = DARK
        .motion
        .resolve(&FOCUS, lune_ui_core::motion::MotionTier::Full)
        .expect("a focus motion")
        .duration;
    h.at_ms(duration.as_millis() as u64 + 500);
    plain(&mut h, &mut text);
    let (scene, _, out) = plain(&mut h, &mut text);
    assert_eq!(out.repaint, Repaint::Idle);
    let ring = scene.layers[0].geometry.under[1];
    assert_eq!(ring.color, premul(DARK.accent));
}

#[test]
fn caret_selection_undo_and_offset_survive_600_hidden_frames_under_keep_alive() {
    let mut text = numbered(30);
    let mut h = UiHarness::new(WIN);
    let shown = |h: &mut UiHarness, text: &mut String| {
        h.frame(|ui| {
            ui.keep_alive("tab", |ui| {
                ui.text_area("a", RECT, text, &AreaOptions::default());
            });
        })
        .1
    };
    let tab = WidgetId::ROOT.with("tab");
    shown(&mut h, &mut text);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    shown(&mut h, &mut text);
    typed(&mut h, "z");
    shown(&mut h, &mut text);
    let before = shown(&mut h, &mut text);
    for _ in 0..600 {
        h.frame(|ui| ui.mark_alive("tab"));
    }
    let after = shown(&mut h, &mut text);
    let reading = |o: &FrameOutput| {
        let n = o.tree.get(tab.with("a")).expect("node");
        let a = o.tree.get(tab.with("a").with("area")).expect("area");
        (n.text.as_ref().map(|t| t.selection), a.value.clone())
    };
    assert_eq!(reading(&before), reading(&after));
    let logo = Modifiers {
        logo: true,
        ..Modifiers::default()
    };
    h.input.set_modifiers(logo);
    h.input.key_down(Key::Character("z".to_owned()), None);
    h.input.set_modifiers(Modifiers::default());
    shown(&mut h, &mut text);
    assert_eq!(text, numbered(30));
}

#[test]
fn a_preedit_on_line_two_shifts_that_line_and_reports_an_area_in_the_viewport() {
    let mut text = String::from("ab\ncd");
    let mut h = focused(&mut text, &AreaOptions::default());
    press(&mut h, NamedKey::ArrowLeft);
    h.input.ime(ImeEvent::Preedit {
        text: "xy".to_owned(),
        cursor: Some((2, 2)),
    });
    let (scene, r, out) = plain(&mut h, &mut text);
    assert_eq!(text, "ab\ncd", "the buffer is untouched");
    let underline = scene.layers[0]
        .geometry
        .over
        .iter()
        .find(|s| s.rect[3] == 1.0)
        .expect("underline")
        .rect;
    assert_eq!(underline[0], TX + glyph());
    assert_eq!(underline[2], 2.0 * glyph());
    assert_eq!(underline[1], TY + 2.0 * line_h() - 1.0);
    let area = r.ime_area.expect("an area");
    assert_eq!(out.ime_area, Some(area));
    assert_eq!(area.x, TX + 3.0 * glyph() + 1.0);
    assert_eq!(area.y, TY + line_h());
}

#[test]
fn the_ime_area_in_a_scrolled_area_is_on_screen() {
    let mut text = numbered(40);
    let mut h = focused(&mut text, &AreaOptions::default());
    press(&mut h, NamedKey::ArrowLeft);
    let (_, r, _) = plain(&mut h, &mut text);
    let area = r.ime_area.expect("an area");
    assert!(
        area.y >= RECT.y && area.bottom() <= RECT.bottom(),
        "{area:?}"
    );
    assert!(area.h > 0.0);
}
