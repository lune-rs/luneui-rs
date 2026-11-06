//! The key-value list (`data.kv-list` acceptance) and its selectable values (D125): the key column
//! is the longest key's measured width, rows of 24, elided and wrapped values, the stacked cells,
//! selection by drag and by press, Copy writing `full`, no tab stop, no input outside a value, the
//! selection clearing with focus, and the `Group` node per pair.

use lune_ui_core::access::{Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{ClipboardRequest, FrameOutput, Key, Modifiers, NamedKey, PointerButton};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape, TextItem};
use lune_ui_core::testing::{FakeText, ScriptedInput, UiHarness};
use lune_ui_core::text::{Spec, Style, elide};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Ui, WidgetId};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::kv_list::{KvMode, KvOptions};
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};

const WIN: (f32, f32) = (600.0, 400.0);
const RECT: Rect = Rect::new(20.0, 30.0, 400.0, 0.0);
/// A 13 px glyph on the fake text is 6.5 px wide.
const GLYPH: f32 = 6.5;

const PAIRS: [(&str, &str); 3] = [
    ("Id", "alpha-beta-gamma"),
    ("Repository", "/home/lune/code/luneui"),
    ("Port", "8080"),
];

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers.iter().flat_map(|l| l.texts.clone()).collect()
}

fn fills(scene: &Scene) -> Vec<Shape> {
    let c = premul(DARK.text_selection);
    scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.color == c)
        .copied()
        .collect()
}

fn kv(
    h: &mut UiHarness,
    rect: Rect,
    pairs: &[(&str, &str)],
    opts: &KvOptions,
) -> (Scene, FrameOutput, f32) {
    let mut height = 0.0;
    let (scene, out) = h.frame(|ui| height = ui.kv_list("kv", rect, pairs, opts));
    (scene, out, height)
}

fn plain(h: &mut UiHarness) -> (Scene, FrameOutput) {
    let (scene, out, _) = kv(h, RECT, &PAIRS, &KvOptions::default());
    (scene, out)
}

fn value_id(i: usize) -> WidgetId {
    WidgetId::ROOT.with("kv").with(i.to_string()).with("value")
}

/// A harness that has drawn the default list once, so its hit regions exist.
fn ready() -> UiHarness {
    let mut h = UiHarness::new(WIN);
    plain(&mut h);
    h
}

fn value_x() -> f32 {
    RECT.x + 10.0 * GLYPH + 16.0
}

/// The row-`i` value's vertical middle.
fn value_mid(i: usize) -> f32 {
    RECT.y + 24.0 * i as f32 + 2.0 + 10.0
}

fn chord(h: &mut UiHarness, key: &str) {
    h.input.set_modifiers(Modifiers {
        logo: true,
        ..Modifiers::default()
    });
    h.input.key_down(Key::Character(key.to_owned()), None);
    h.input.set_modifiers(Modifiers::default());
}

fn selection_of(out: &FrameOutput, i: usize) -> (usize, usize) {
    out.tree
        .get(value_id(i))
        .and_then(|n| n.text.as_ref())
        .expect("the value's text")
        .selection
}

#[test]
fn the_key_column_is_the_longest_keys_measured_width_and_the_gap_is_16() {
    let (scene, _, _) = kv(&mut ready(), RECT, &PAIRS, &KvOptions::default());
    let t = texts(&scene);
    assert_eq!(t.len(), 6, "a key and a value per pair");
    let longest = text_w("Repository");
    for i in 0..3 {
        assert_eq!(t[2 * i].x, RECT.x, "key {i}");
        assert_eq!(t[2 * i + 1].x, RECT.x + longest + 16.0, "value {i}");
    }
    assert_eq!(longest, 65.0);
}

/// The width the fake text system gives `s` at 13 px.
fn text_w(s: &str) -> f32 {
    use lune_ui_core::text::TextSystem;
    FakeText
        .measure(&Spec::plain(s, Style::ui(13.0, DARK.fg_primary)), 1.0)
        .0
}

#[test]
fn rows_are_24_high_with_the_text_on_a_20_px_line_2_px_down() {
    let (scene, _, height) = kv(&mut ready(), RECT, &PAIRS, &KvOptions::default());
    assert_eq!(height, 72.0);
    let t = texts(&scene);
    for i in 0..3 {
        let y = RECT.y + 24.0 * i as f32 + 2.0;
        assert_eq!((t[2 * i].y, t[2 * i + 1].y), (y, y), "row {i}");
    }
}

#[test]
fn key_and_value_colours_and_the_mono_option() {
    let (scene, _, _) = kv(&mut ready(), RECT, &PAIRS[..1], &KvOptions::default());
    let t = texts(&scene);
    let key = Spec::plain(
        "Id",
        Style {
            line: 20.0,
            ..Style::ui(13.0, DARK.fg_tertiary)
        },
    );
    let value = Spec::plain(
        "alpha-beta-gamma",
        Style {
            line: 20.0,
            ..Style::ui(13.0, DARK.fg_primary)
        },
    );
    assert_eq!(t[0].key.0, key.cache_key(1.0));
    assert_eq!(t[1].key.0, value.cache_key(1.0));
    let mono = KvOptions {
        mono: true,
        ..KvOptions::default()
    };
    let (scene, _, _) = kv(&mut UiHarness::new(WIN), RECT, &PAIRS[..1], &mono);
    let mono_value = Spec::plain("alpha-beta-gamma", Style::mono(13.0, 20.0, DARK.fg_primary));
    assert_eq!(texts(&scene)[1].key.0, mono_value.cache_key(1.0));
    assert_eq!(texts(&scene)[0].key.0, key.cache_key(1.0), "keys stay sans");
}

#[test]
fn a_long_value_is_elided_to_the_value_column_and_a_key_never_is() {
    let long = "0123456789".repeat(4);
    let rect = Rect::new(20.0, 30.0, 200.0, 0.0);
    let pairs = [("A very long key name", long.as_str())];
    let (scene, _, _) = kv(
        &mut UiHarness::new(WIN),
        rect,
        &pairs,
        &KvOptions::default(),
    );
    let t = texts(&scene);
    let key_w = text_w("A very long key name");
    let col = 200.0 - key_w - 16.0;
    let style = Style {
        line: 20.0,
        ..Style::ui(13.0, DARK.fg_primary)
    };
    let want = elide(&mut FakeText, &Spec::plain(long.clone(), style), 1.0, col);
    assert_eq!(
        t[1].key.0,
        want.cache_key(1.0),
        "the value is cut to the column"
    );
    assert_ne!(t[1].key.0, Spec::plain(long, style).cache_key(1.0));
    let key = Spec::plain(
        "A very long key name",
        Style {
            line: 20.0,
            ..Style::ui(13.0, DARK.fg_tertiary)
        },
    );
    assert_eq!(t[0].key.0, key.cache_key(1.0), "the key is whole");
}

#[test]
fn a_wrap_value_wraps_and_its_row_is_the_text_plus_2_and_2() {
    let long = "word ".repeat(30);
    let long = long.trim();
    let rect = Rect::new(20.0, 30.0, 300.0, 0.0);
    let opts = KvOptions {
        mode: KvMode::Wrap,
        ..KvOptions::default()
    };
    let pairs = [("Note", long), ("Port", "8080")];
    let (scene, out, height) = kv(&mut UiHarness::new(WIN), rect, &pairs, &opts);
    let first = height - 24.0;
    assert!(first > 24.0);
    assert_eq!(first % 20.0, 4.0, "whole 20 px lines plus 2 + 2: {first}");
    let t = texts(&scene);
    assert_eq!(t[3].y, 30.0 + first + 2.0, "the next row follows");
    let group = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Group)
        .expect("group");
    assert_eq!(group.bounds.h, first);
    // A short wrap value still takes a whole row.
    let (_, _, h) = kv(&mut UiHarness::new(WIN), rect, &pairs[1..], &opts);
    assert_eq!(h, 24.0);
}

#[test]
fn stacked_cells_run_in_two_columns_key_over_a_mono_value() {
    let pairs = [("One", "1"), ("Two", "2"), ("Three", "3")];
    let opts = KvOptions {
        mode: KvMode::Stacked,
        ..KvOptions::default()
    };
    let (scene, _, height) = kv(&mut UiHarness::new(WIN), RECT, &pairs, &opts);
    let col = (400.0 - 16.0) / 2.0;
    assert_eq!(height, 36.0 + 12.0 + 36.0, "two rows of 36 and a gap of 12");
    let t = texts(&scene);
    assert_eq!(t.len(), 6);
    let cell = |i: usize| {
        (
            RECT.x + (i % 2) as f32 * (col + 16.0),
            RECT.y + (i / 2) as f32 * 48.0,
        )
    };
    for i in 0..3 {
        let (x, y) = cell(i);
        assert_eq!((t[2 * i].x, t[2 * i].y), (x, y), "key {i}");
        assert_eq!((t[2 * i + 1].x, t[2 * i + 1].y), (x, y + 16.0), "value {i}");
    }
    let key = Spec::plain(
        "One",
        Style {
            line: 16.0,
            ..Style::ui(11.0, DARK.fg_tertiary)
        },
    );
    assert_eq!(t[0].key.0, key.cache_key(1.0));
    let value = Spec::plain("1", Style::mono(13.0, 20.0, DARK.fg_primary));
    assert_eq!(t[1].key.0, value.cache_key(1.0));
}

#[test]
fn an_empty_list_draws_nothing_and_is_0_high() {
    let (scene, out, h) = kv(&mut UiHarness::new(WIN), RECT, &[], &KvOptions::default());
    assert_eq!(h, 0.0);
    assert!(texts(&scene).is_empty() && out.tree.nodes.is_empty());
}

#[test]
fn each_pair_is_a_group_over_a_key_and_a_selectable_value() {
    let (_, out) = plain(&mut ready());
    let groups: Vec<_> = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Group)
        .collect();
    assert_eq!(groups.len(), 3);
    for (i, (k, v)) in PAIRS.iter().enumerate() {
        assert_eq!(groups[i].name, *k);
        assert_eq!(groups[i].children.len(), 2);
        let key = out.tree.get(groups[i].children[0]).expect("key");
        assert_eq!((key.role, key.name.as_str()), (Role::StaticText, *k));
        assert_eq!(key.value, Some(Value::Text((*k).to_owned())));
        let value = out.tree.get(groups[i].children[1]).expect("value");
        assert_eq!(value.id, value_id(i));
        assert_eq!((value.role, value.name.as_str()), (Role::StaticText, *k));
        assert_eq!(value.value, Some(Value::Text((*v).to_owned())));
        assert!(value.states.contains(State::ReadOnly));
        assert!(value.states.contains(State::Focusable));
    }
}

#[test]
fn a_drag_across_a_value_then_copy_writes_the_substring() {
    let mut h = ready();
    let x = value_x();
    h.input.pointer_to(x + 1.0, value_mid(0));
    plain(&mut h);
    h.input.press(PointerButton::Primary);
    plain(&mut h);
    h.input.pointer_to(x + 5.0 * GLYPH + 0.5, value_mid(0));
    plain(&mut h);
    let (scene, out) = plain(&mut h);
    assert_eq!(selection_of(&out, 0), (0, 5));
    let f = fills(&scene);
    assert_eq!(f.len(), 1, "one selection box");
    assert_eq!(f[0].rect, [x, RECT.y + 2.0, 5.0 * GLYPH, 20.0]);
    h.input.release(PointerButton::Primary);
    plain(&mut h);
    chord(&mut h, "c");
    let (_, out) = plain(&mut h);
    assert_eq!(out.clipboard, [ClipboardRequest::Write("alpha".to_owned())]);
}

#[test]
fn the_selection_draws_while_focused_and_a_focus_loss_clears_it() {
    let go = Rect::new(400.0, 200.0, 80.0, 32.0);
    let with_button = |h: &mut UiHarness| {
        h.frame(|ui| {
            ui.kv_list("kv", RECT, &PAIRS, &KvOptions::default());
            ui.button("go", go, "Go", ButtonKind::Secondary);
        })
    };
    let mut h = ready();
    h.input.pointer_to(value_x() + 1.0, value_mid(0));
    plain(&mut h);
    h.input.press(PointerButton::Primary);
    plain(&mut h);
    h.input.release(PointerButton::Primary);
    plain(&mut h);
    chord(&mut h, "a");
    let (scene, out) = plain(&mut h);
    assert_eq!(selection_of(&out, 0), (0, 16));
    assert_eq!(fills(&scene).len(), 1);
    // A press on another control takes the focus away.
    h.input.pointer_to(440.0, 216.0);
    with_button(&mut h);
    h.input.click(PointerButton::Primary);
    with_button(&mut h);
    let (scene, out) = with_button(&mut h);
    assert!(fills(&scene).is_empty(), "no fill without focus");
    let (a, b) = selection_of(&out, 0);
    assert_eq!(a, b, "the selection is cleared");
    // Focused again by request: still nothing selected.
    h.frame(|ui| {
        ui.request_focus(value_id(0));
        ui.kv_list("kv", RECT, &PAIRS, &KvOptions::default());
    });
    let (scene, out) = plain(&mut h);
    assert!(
        out.tree
            .get(value_id(0))
            .is_some_and(|n| n.states.contains(State::Focused))
    );
    assert!(fills(&scene).is_empty());
    let (a, b) = selection_of(&out, 0);
    assert_eq!(a, b);
}

#[test]
fn a_value_is_not_a_tab_stop() {
    let mut h = UiHarness::new(WIN);
    let mut focused = Vec::new();
    let run = |h: &mut UiHarness| {
        let mut f = false;
        h.frame(|ui| {
            ui.kv_list("kv", RECT, &PAIRS, &KvOptions::default());
            f = ui
                .button(
                    "go",
                    Rect::new(20.0, 200.0, 80.0, 32.0),
                    "Go",
                    ButtonKind::Secondary,
                )
                .focused;
        });
        f
    };
    run(&mut h);
    for _ in 0..3 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        focused.push(run(&mut h));
    }
    assert_eq!(
        focused,
        [true, true, true],
        "Tab only ever lands on the button"
    );
    // And Tab never selected a value.
    let (scene, _) = plain(&mut h);
    assert!(fills(&scene).is_empty());
}

#[test]
fn a_press_outside_a_value_does_nothing() {
    let mut h = ready();
    // On the key, in the gap, past the end of a short value, and in the row's blank right side.
    let spots = [
        (RECT.x + 3.0, value_mid(0)),
        (value_x() - 8.0, value_mid(0)),
        (value_x() + 4.0 * GLYPH + 30.0, value_mid(2)),
        (RECT.x + 380.0, value_mid(1)),
    ];
    for (x, y) in spots {
        h.input.pointer_to(x, y);
        plain(&mut h);
        h.input.click(PointerButton::Primary);
        plain(&mut h);
        chord(&mut h, "a");
        plain(&mut h);
        chord(&mut h, "c");
        let (scene, out) = plain(&mut h);
        assert!(out.clipboard.is_empty(), "({x}, {y})");
        assert!(fills(&scene).is_empty(), "({x}, {y})");
    }
}

#[test]
fn a_press_on_an_elided_value_selects_it_whole_and_copy_writes_all_of_it() {
    let long = "0123456789".repeat(4);
    let rect = Rect::new(20.0, 30.0, 200.0, 0.0);
    let pairs = [("Key", long.as_str())];
    let opts = KvOptions::default();
    let mut h = UiHarness::new(WIN);
    let (scene, ..) = kv(&mut h, rect, &pairs, &opts);
    let drawn = texts(&scene)[1];
    let vx = 20.0 + text_w("Key") + 16.0;
    // The press lands in the middle of what is drawn, not at an offset of the full text.
    h.input.pointer_to(vx + 20.0, 30.0 + 12.0);
    kv(&mut h, rect, &pairs, &opts);
    h.input.click(PointerButton::Primary);
    let (scene, out, _) = kv(&mut h, rect, &pairs, &opts);
    let f = fills(&scene);
    assert_eq!(f.len(), 1);
    let w = f[0].rect[2];
    assert_eq!((f[0].rect[0], f[0].rect[1], f[0].rect[3]), (vx, 32.0, 20.0));
    assert!(
        w < 200.0 - text_w("Key") - 16.0 + 0.01,
        "the fill covers what is drawn, not the full text"
    );
    assert_eq!(texts(&scene)[1], drawn);
    assert_eq!(selection_of(&out, 0), (0, 40));
    chord(&mut h, "c");
    let (_, out, _) = kv(&mut h, rect, &pairs, &opts);
    assert_eq!(out.clipboard, [ClipboardRequest::Write(long.clone())]);
}

#[test]
fn navigation_cannot_make_a_partial_selection_of_an_elided_value() {
    let long = "0123456789".repeat(4);
    let rect = Rect::new(20.0, 30.0, 200.0, 0.0);
    let pairs = [("Key", long.as_str())];
    let opts = KvOptions::default();
    let mut h = UiHarness::new(WIN);
    kv(&mut h, rect, &pairs, &opts);
    let vx = 20.0 + text_w("Key") + 16.0;
    h.input.pointer_to(vx + 20.0, 42.0);
    kv(&mut h, rect, &pairs, &opts);
    h.input.click(PointerButton::Primary);
    kv(&mut h, rect, &pairs, &opts);
    // Arrows, Shift-arrows and Copy in one frame: Copy still writes everything.
    h.input.key_down(Key::Named(NamedKey::ArrowLeft), None);
    h.input.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    h.input.key_down(Key::Named(NamedKey::ArrowRight), None);
    h.input.set_modifiers(Modifiers::default());
    chord(&mut h, "c");
    let (scene, out, _) = kv(&mut h, rect, &pairs, &opts);
    assert_eq!(out.clipboard, [ClipboardRequest::Write(long.clone())]);
    assert_eq!(fills(&scene).len(), 1);
    assert_eq!(selection_of(&out, 0), (0, 40));
    // Select all works as it does everywhere; a drag cannot narrow it.
    chord(&mut h, "a");
    let (_, out, _) = kv(&mut h, rect, &pairs, &opts);
    assert_eq!(selection_of(&out, 0), (0, 40));
}

#[test]
fn copy_with_nothing_selected_is_declined_for_an_elided_value_too() {
    let long = "0123456789".repeat(4);
    let rect = Rect::new(20.0, 30.0, 200.0, 0.0);
    let pairs = [("Key", long.as_str())];
    let mut h = UiHarness::new(WIN);
    h.shortcuts.add(lune_ui_core::shortcuts::Binding::new(
        lune_ui_core::shortcuts::Scope::Global,
        lune_ui_core::shortcuts::Chord::key(Key::Character("c".into())).primary(),
        "copy",
    ));
    let opts = KvOptions::default();
    kv(&mut h, rect, &pairs, &opts);
    chord(&mut h, "c");
    let (_, out, _) = kv(&mut h, rect, &pairs, &opts);
    assert!(out.clipboard.is_empty());
    assert_eq!(out.fired.len(), 1, "the binding has it");
}

#[test]
fn the_shown_text_may_differ_from_what_is_copied_only_by_being_selected_whole() {
    // A caller's own spec for a value: the drawn text is not `full`.
    let mut h = UiHarness::new(WIN);
    let spec = Spec::plain("ab\u{2026}", Style::ui(13.0, DARK.fg_primary));
    let rect = Rect::new(20.0, 30.0, 200.0, 20.0);
    let run = |h: &mut UiHarness| {
        h.frame(|ui| {
            ui.selectable_text("s", rect, &spec, "abcdef");
        })
    };
    run(&mut h);
    h.input.pointer_to(24.0, 40.0);
    run(&mut h);
    h.input.click(PointerButton::Primary);
    run(&mut h);
    chord(&mut h, "c");
    let (_, out) = run(&mut h);
    assert_eq!(
        out.clipboard,
        [ClipboardRequest::Write("abcdef".to_owned())]
    );
}

#[test]
fn a_drag_selects_in_a_value_inside_a_scrolled_parent() {
    let mut h = UiHarness::new(WIN);
    let spec = Spec::plain("0123456789", Style::ui(13.0, DARK.fg_primary));
    // Content (35, 300) at offset (15, 200) is screen (20, 100).
    let rect = Rect::new(35.0, 300.0, 140.0, 20.0);
    let opts = ScrollOptions {
        extent: Some((1000.0, 1000.0)),
        ..ScrollOptions::default()
    };
    let area = Rect::new(0.0, 0.0, 380.0, 280.0);
    let run = |h: &mut UiHarness, ms: u64| {
        h.at_ms(ms)
            .frame(|ui: &mut Ui<'_>| {
                ui.scroll_area("o", area, &opts, |ui, _| {
                    ui.selectable_text("s", rect, &spec, "0123456789");
                    ((), Extent::default())
                });
            })
            .1
    };
    h.input.pointer_to(370.0, 270.0);
    run(&mut h, 0);
    h.input.scroll_pixels(-15.0, -200.0);
    for ms in [5, 10, 20] {
        run(&mut h, ms);
    }
    h.input.pointer_to(20.0 + 2.0 * GLYPH + 0.5, 110.0);
    run(&mut h, 30);
    h.input.press(PointerButton::Primary);
    run(&mut h, 40);
    h.input.pointer_to(20.0 + 6.0 * GLYPH + 0.5, 110.0);
    run(&mut h, 50);
    let out = run(&mut h, 60);
    let node = out.tree.get(WidgetId::ROOT.with("s")).expect("node");
    assert_eq!(node.text.as_ref().expect("text").selection, (2, 6));
    h.input.release(PointerButton::Primary);
    run(&mut h, 70);
    chord(&mut h, "c");
    let out = run(&mut h, 80);
    assert_eq!(out.clipboard, [ClipboardRequest::Write("2345".to_owned())]);
}

#[test]
fn a_disabled_list_takes_no_press() {
    let mut h = UiHarness::new(WIN);
    let run = |h: &mut UiHarness| {
        h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                ui.kv_list("kv", RECT, &PAIRS, &KvOptions::default());
            });
        })
    };
    run(&mut h);
    h.input.pointer_to(value_x() + 3.0, value_mid(0));
    run(&mut h);
    h.input.click(PointerButton::Primary);
    run(&mut h);
    chord(&mut h, "a");
    run(&mut h);
    chord(&mut h, "c");
    let (scene, out) = run(&mut h);
    assert!(out.clipboard.is_empty() && fills(&scene).is_empty());
}

#[test]
fn the_list_is_static_and_settles() {
    let mut h = UiHarness::new(WIN);
    for ms in [0, 16, 700] {
        h.at_ms(ms);
        let (_, out) = plain(&mut h);
        assert_eq!(out.repaint, lune_ui_core::input::Repaint::Idle, "{ms}");
    }
}

#[test]
fn at_scale_2_the_row_is_48_px_and_the_gap_32_px() {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let (scene, _, height) = kv(&mut h, RECT, &PAIRS, &KvOptions::default());
    assert_eq!(height * 2.0, 3.0 * 48.0);
    let t = texts(&scene);
    let key_col = text_w("Repository") * 2.0;
    assert_eq!((t[1].x - t[0].x) * 2.0, key_col + 32.0);
}

/// Class 1 of D177: a value that holds the key focus when its scope turns disabled takes the keys
/// it takes enabled and changes nothing, in both the positioned and the whole-selection path.
#[test]
fn a_value_focused_then_disabled_takes_its_keys_and_changes_nothing() {
    use lune_ui_core::shortcuts::{Binding, Chord, Scope};
    type Probe = (&'static str, Key, bool);
    let named = |n: &'static str, k| (n, Key::Named(k), false);
    let logo = |n: &'static str, c: &str| (n, Key::Character(c.to_owned()), true);
    // (the spec drawn, the keys that fire nothing with it enabled)
    let cases: [(&str, &str, Vec<Probe>); 2] = [
        (
            "abcdef",
            "positioned",
            vec![
                named("Enter", NamedKey::Enter),
                named("Left", NamedKey::ArrowLeft),
                named("Right", NamedKey::ArrowRight),
                named("Home", NamedKey::Home),
                named("End", NamedKey::End),
                logo("Command+A", "a"),
                logo("Command+C", "c"),
            ],
        ),
        (
            "ab\u{2026}",
            "elided",
            vec![logo("Command+A", "a"), logo("Command+C", "c")],
        ),
    ];
    let rect = Rect::new(20.0, 30.0, 200.0, 20.0);
    for (drawn, mode, probes) in cases {
        let spec = Spec::plain(drawn, Style::ui(13.0, DARK.fg_primary));
        for (name, key, with_logo) in probes {
            let mut h = UiHarness::new(WIN);
            let chord_of = Chord::key(key.clone());
            h.shortcuts.add(Binding::new(
                Scope::Global,
                if with_logo {
                    chord_of.primary()
                } else {
                    chord_of
                },
                name,
            ));
            h.shortcuts.add(Binding::new(
                Scope::Global,
                Chord::key(Key::Named(NamedKey::F5)),
                "f5",
            ));
            let run = |h: &mut UiHarness, enabled: bool| {
                h.frame(|ui| {
                    ui.with_enabled(enabled, |ui| {
                        ui.selectable_text("s", rect, &spec, "abcdef");
                    });
                })
                .1
            };
            let sel = |out: &FrameOutput| {
                out.tree
                    .get(WidgetId::ROOT.with("s"))
                    .and_then(|n| n.text.as_ref())
                    .expect("text")
                    .selection
            };
            let press = |h: &mut UiHarness| {
                if with_logo {
                    h.input.set_modifiers(Modifiers {
                        logo: true,
                        ..Modifiers::default()
                    });
                }
                h.input.key_down(key.clone(), None);
                h.input.set_modifiers(Modifiers::default());
            };
            run(&mut h, true);
            h.input.pointer_to(24.0, 40.0);
            run(&mut h, true);
            h.input.click(PointerButton::Primary);
            run(&mut h, true);
            chord(&mut h, "a");
            let before = sel(&run(&mut h, true));
            assert_eq!(before.0.min(before.1), 0, "{mode} {name}");
            assert_ne!(before.0, before.1, "{mode} {name}: selected");
            // Enabled, the same key fires nothing (so the probe proves something).
            press(&mut h);
            let out = run(&mut h, true);
            assert!(out.fired.is_empty(), "{mode} {name}: enabled it fired");
            // A fresh start, disabled.
            let mut h2 = UiHarness::new(WIN);
            h2.shortcuts = h.shortcuts.clone();
            run(&mut h2, true);
            h2.input.pointer_to(24.0, 40.0);
            run(&mut h2, true);
            h2.input.click(PointerButton::Primary);
            run(&mut h2, true);
            chord(&mut h2, "a");
            run(&mut h2, true);
            run(&mut h2, false);
            press(&mut h2);
            let out = run(&mut h2, false);
            assert!(out.fired.is_empty(), "{mode} {name}: disabled, it fired");
            assert!(out.clipboard.is_empty(), "{mode} {name}: clipboard used");
            h2.input.key_down(Key::Named(NamedKey::F5), None);
            let out = run(&mut h2, false);
            assert_eq!(out.fired.len(), 1, "{mode} {name}: F5");
            // Enabled again: the selection is what it was, and Copy writes it.
            let out = run(&mut h2, true);
            assert_eq!(sel(&out), before, "{mode} {name}: the selection moved");
        }
    }
}

#[test]
fn select_all_on_an_elided_value_focused_then_disabled_selects_nothing() {
    use lune_ui_core::access::{AccessAction, Action};
    use lune_ui_core::shortcuts::{Binding, Chord, Scope};
    let spec = Spec::plain("ab\u{2026}", Style::ui(13.0, DARK.fg_primary));
    let rect = Rect::new(20.0, 30.0, 200.0, 20.0);
    let mut h = UiHarness::new(WIN);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Character("a".into())).primary(),
        "all",
    ));
    let run = |h: &mut UiHarness, enabled: bool| {
        h.frame(|ui| {
            ui.with_enabled(enabled, |ui| {
                ui.selectable_text("s", rect, &spec, "abcdef");
            });
        })
        .1
    };
    run(&mut h, true);
    // Focus by the assistive route, so nothing is selected.
    h.input.access(AccessAction {
        target: WidgetId::ROOT.with("s"),
        action: Action::Focus,
        value: None,
    });
    let out = run(&mut h, true);
    assert_eq!(out.focused, Some(WidgetId::ROOT.with("s")));
    run(&mut h, false);
    chord(&mut h, "a");
    let out = run(&mut h, false);
    assert!(out.fired.is_empty(), "the key is the value's");
    let out = run(&mut h, true);
    let sel = out
        .tree
        .get(WidgetId::ROOT.with("s"))
        .and_then(|n| n.text.as_ref())
        .expect("text")
        .selection;
    assert_eq!(sel, (0, 0), "nothing was selected while disabled");
}

/// D11: a rich spec whose runs join to `full` selects by position, across its runs.
#[test]
fn a_two_run_spec_equal_to_full_selects_part_of_it_and_copies_that_part() {
    let a = Style::ui(13.0, DARK.fg_primary);
    let b = Style {
        color: DARK.fg_secondary,
        ..a
    };
    let spec = Spec {
        runs: vec![("abc".to_owned(), a), ("defg".to_owned(), b)],
        wrap: None,
    };
    let rect = Rect::new(20.0, 30.0, 200.0, 20.0);
    let mut h = UiHarness::new(WIN);
    let run = |h: &mut UiHarness| {
        h.frame(|ui| {
            ui.selectable_text("s", rect, &spec, "abcdefg");
        })
    };
    run(&mut h);
    // From between "b" and "c" (char 2) to between "e" and "f" (char 5), across both runs.
    h.input.pointer_to(20.0 + 2.0 * GLYPH + 0.5, 40.0);
    run(&mut h);
    h.input.press(PointerButton::Primary);
    run(&mut h);
    h.input.pointer_to(20.0 + 5.0 * GLYPH + 0.5, 40.0);
    run(&mut h);
    let (scene, out) = run(&mut h);
    let node = out.tree.get(WidgetId::ROOT.with("s")).expect("node");
    assert_eq!(node.text.as_ref().expect("text").selection, (2, 5));
    let f = fills(&scene);
    assert_eq!(f.len(), 1, "one fill: {f:?}");
    assert!((f[0].rect[0] - (20.0 + 2.0 * GLYPH)).abs() < 0.01, "{f:?}");
    assert!((f[0].rect[2] - 3.0 * GLYPH).abs() < 0.01, "{f:?}");
    h.input.release(PointerButton::Primary);
    run(&mut h);
    chord(&mut h, "c");
    let (_, out) = run(&mut h);
    assert_eq!(out.clipboard, [ClipboardRequest::Write("cde".to_owned())]);
}
