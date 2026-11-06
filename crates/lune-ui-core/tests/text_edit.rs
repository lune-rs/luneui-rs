//! The editing engine (`text.text-edit` and `text.ime` acceptance): known answers on `FakeText`,
//! both word motions, undo, clipboard, flags, IME, totality. Every motion also runs on
//! "héllo wörld", so a byte offset leaking through the seam fails.

use std::time::Duration;

use lune_ui_core::input::{FocusEvent, ImeEvent, Key, Keymap, Modifiers, NamedKey};
use lune_ui_core::testing::FakeText;
use lune_ui_core::text::{CaretPos, LineSpan, Spec, Style, TextSystem, Unit};
use lune_ui_core::text_edit::{
    COALESCE_PAUSE, EditCommand, EditFlags, EditLayout, EditOutcome, MAX_ONE_LINE_INSERT, Motion,
    SpecLayout, TextEdit, UNDO_DEPTH,
};

const WORLD: &str = "héllo wörld";

/// Every char 5 px wide, every line 20 px high.
fn spec(text: &str) -> Spec {
    Spec::plain(text, Style::mono(10.0, 20.0, [255; 4]))
}

fn mv(motion: Motion) -> EditCommand {
    EditCommand::Move {
        motion,
        extend: false,
    }
}

fn ext(motion: Motion) -> EditCommand {
    EditCommand::Move {
        motion,
        extend: true,
    }
}

fn ins(s: &str) -> EditCommand {
    EditCommand::Insert(s.to_owned())
}

const BACK: EditCommand = EditCommand::Backspace { word: false };
const DEL: EditCommand = EditCommand::Delete { word: false };

/// An engine with its text, driven the way a widget drives it: a fresh layout per command.
struct Ed {
    edit: TextEdit,
    text: String,
    system: FakeText,
    keymap: Keymap,
    now: Duration,
}

impl Ed {
    fn new(text: &str) -> Self {
        let mut ed = Self {
            edit: TextEdit::new(),
            text: text.to_owned(),
            system: FakeText,
            keymap: Keymap::PC,
            now: Duration::ZERO,
        };
        ed.sync();
        ed
    }

    /// `text` with the caret at `caret`.
    fn at(text: &str, caret: usize) -> Self {
        let mut ed = Self::new(text);
        ed.edit.set_selection(caret, caret);
        ed.sync();
        ed
    }

    fn with_flags(text: &str, flags: EditFlags) -> Self {
        let mut ed = Self::new(text);
        ed.edit.set_flags(flags);
        ed
    }

    fn sync(&mut self) {
        let sp = spec(&self.text);
        self.edit
            .sync(&self.text, &mut SpecLayout::new(&mut self.system, &sp, 1.0));
    }

    fn cmd(&mut self, cmd: EditCommand) -> EditOutcome {
        let sp = spec(&self.text);
        let mut layout = SpecLayout::new(&mut self.system, &sp, 1.0);
        self.edit
            .apply(cmd, &mut self.text, &mut layout, self.keymap, self.now)
    }

    /// Maps an event as a widget does; `None` when it maps to nothing.
    fn event(&mut self, ev: FocusEvent) -> Option<EditOutcome> {
        let cmd = EditCommand::from_event(&ev, self.keymap, self.edit.flags())?;
        Some(self.cmd(cmd))
    }

    fn typed(&mut self, s: &str) {
        for c in s.chars() {
            self.cmd(ins(&c.to_string()));
        }
    }

    fn caret(&self) -> usize {
        self.edit.caret()
    }

    fn sel(&self) -> (usize, usize) {
        let s = self.edit.selection();
        (s.start, s.end)
    }
}

fn named(key: NamedKey, modifiers: Modifiers) -> FocusEvent {
    FocusEvent::Key {
        key: Key::Named(key),
        modifiers,
        repeat: false,
    }
}

fn chord(c: &str, modifiers: Modifiers) -> FocusEvent {
    FocusEvent::Key {
        key: Key::Character(c.to_owned()),
        modifiers,
        repeat: false,
    }
}

const NONE: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: false,
};
const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..NONE
};
const CTRL: Modifiers = Modifiers {
    control: true,
    ..NONE
};
const ALT: Modifiers = Modifiers { alt: true, ..NONE };
const CMD: Modifiers = Modifiers { logo: true, ..NONE };

// Known answers: `text-edit.md` Acceptance.

#[test]
fn typing_puts_the_caret_after_the_text() {
    let mut ed = Ed::new("");
    ed.typed("abc");
    assert_eq!((ed.text.as_str(), ed.caret()), ("abc", 3));
}

#[test]
fn left_twice_then_shift_right_selects_one_to_two() {
    for text in ["abc", "héo"] {
        let mut ed = Ed::at(text, 3);
        ed.cmd(mv(Motion::GraphemeLeft));
        ed.cmd(mv(Motion::GraphemeLeft));
        assert_eq!(ed.caret(), 1, "{text}");
        ed.cmd(ext(Motion::GraphemeRight));
        assert_eq!(ed.sel(), (1, 2), "{text}");
        assert_eq!((ed.edit.anchor(), ed.caret()), (1, 2), "{text}");
    }
}

#[test]
fn select_all_then_typing_replaces() {
    let mut ed = Ed::new(WORLD);
    ed.cmd(EditCommand::SelectAll);
    assert_eq!(ed.sel(), (0, 11));
    ed.typed("x");
    assert_eq!((ed.text.as_str(), ed.caret()), ("x", 1));
}

#[test]
fn backspace_at_zero_and_delete_at_the_end_do_nothing() {
    let mut ed = Ed::at("abc", 0);
    let out = ed.cmd(BACK);
    assert!(!out.changed && out.consumed);
    ed.edit.set_selection(3, 3);
    let out = ed.cmd(DEL);
    assert!(!out.changed && out.consumed);
    assert_eq!(ed.text, "abc");
    assert_eq!(ed.edit.undo_steps(), 0);
}

#[test]
fn one_backspace_takes_a_whole_cluster() {
    for (text, left) in [
        ("xe\u{301}", "x"),
        ("x\u{1F44D}\u{1F3FD}", "x"),
        ("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}", ""),
    ] {
        let end = text.chars().count();
        let mut ed = Ed::at(text, end);
        assert_eq!(ed.caret(), end);
        ed.cmd(BACK);
        assert_eq!(ed.text, left, "{text:?}");
    }
}

#[test]
fn delete_and_motion_step_over_a_whole_cluster() {
    let mut ed = Ed::at("e\u{301}x", 0);
    ed.cmd(mv(Motion::GraphemeRight));
    assert_eq!(ed.caret(), 2);
    ed.cmd(mv(Motion::GraphemeLeft));
    assert_eq!(ed.caret(), 0);
    ed.cmd(DEL);
    assert_eq!(ed.text, "x");
}

#[test]
fn an_arrow_collapses_a_selection_to_its_edge() {
    let mut ed = Ed::new(WORLD);
    ed.edit.set_selection(2, 7);
    ed.cmd(mv(Motion::GraphemeLeft));
    assert_eq!(ed.sel(), (2, 2));
    ed.edit.set_selection(7, 2);
    ed.cmd(mv(Motion::GraphemeRight));
    assert_eq!(ed.sel(), (7, 7));
}

#[test]
fn grapheme_motion_on_non_ascii_counts_chars() {
    let mut ed = Ed::at(WORLD, 0);
    for want in 1..=11 {
        ed.cmd(mv(Motion::GraphemeRight));
        assert_eq!(ed.caret(), want);
    }
    ed.cmd(mv(Motion::GraphemeRight));
    assert_eq!(ed.caret(), 11);
    ed.cmd(mv(Motion::TextStart));
    assert_eq!(ed.caret(), 0);
    ed.cmd(ext(Motion::TextEnd));
    assert_eq!(ed.sel(), (0, 11));
}

#[test]
fn word_right_lands_per_the_keymap() {
    // (keymap, from, to)
    for (keymap, stops) in [
        (Keymap::MAC, [(0, 5), (3, 5), (5, 11), (6, 11), (11, 11)]),
        (Keymap::PC, [(0, 6), (3, 6), (5, 6), (6, 11), (11, 11)]),
    ] {
        for (from, to) in stops {
            let mut ed = Ed::at(WORLD, from);
            ed.keymap = keymap;
            ed.cmd(mv(Motion::WordRight));
            assert_eq!(ed.caret(), to, "{keymap:?} from {from}");
        }
    }
}

#[test]
fn word_left_goes_to_the_previous_word_start_on_both_keymaps() {
    for keymap in [Keymap::MAC, Keymap::PC] {
        for (from, to) in [(11, 6), (8, 6), (6, 0), (5, 0), (2, 0), (0, 0)] {
            let mut ed = Ed::at(WORLD, from);
            ed.keymap = keymap;
            ed.cmd(mv(Motion::WordLeft));
            assert_eq!(ed.caret(), to, "{keymap:?} from {from}");
        }
    }
}

#[test]
fn word_motion_skips_punctuation_and_space() {
    let mut ed = Ed::at("ab,  cd", 2);
    ed.cmd(mv(Motion::WordRight));
    assert_eq!(ed.caret(), 5);
    ed.cmd(mv(Motion::WordLeft));
    assert_eq!(ed.caret(), 0);
}

#[test]
fn word_motion_extends() {
    let mut ed = Ed::at(WORLD, 0);
    ed.cmd(ext(Motion::WordRight));
    assert_eq!(ed.sel(), (0, 6));
    let mut ed = Ed::at(WORLD, 11);
    ed.cmd(ext(Motion::WordLeft));
    assert_eq!(ed.sel(), (6, 11));
}

#[test]
fn word_delete_follows_the_word_motions() {
    let mut ed = Ed::at(WORLD, 11);
    ed.cmd(EditCommand::Backspace { word: true });
    assert_eq!(ed.text, "héllo ");
    assert_eq!(ed.caret(), 6);

    let mut ed = Ed::at(WORLD, 0);
    ed.cmd(EditCommand::Delete { word: true });
    assert_eq!(ed.text, "wörld");

    let mut ed = Ed::at(WORLD, 0);
    ed.keymap = Keymap::MAC;
    ed.cmd(EditCommand::Delete { word: true });
    assert_eq!(ed.text, " wörld");
}

#[test]
fn delete_removes_a_selection_on_both_keys() {
    for cmd in [BACK, DEL, EditCommand::Backspace { word: true }] {
        let mut ed = Ed::new(WORLD);
        ed.edit.set_selection(8, 1);
        ed.cmd(cmd);
        assert_eq!(ed.text, "hrld");
        assert_eq!(ed.sel(), (1, 1));
    }
}

#[test]
fn line_ends_stay_on_the_line() {
    let text = "héllo\nwörld\n";
    let mut ed = Ed::at(text, 8);
    ed.cmd(mv(Motion::LineEnd));
    assert_eq!(ed.caret(), 11);
    ed.cmd(mv(Motion::LineStart));
    assert_eq!(ed.caret(), 6);
    ed.cmd(ext(Motion::LineEnd));
    assert_eq!(ed.sel(), (6, 11));
    // The empty line after the final break.
    ed.cmd(mv(Motion::TextEnd));
    assert_eq!(ed.caret(), 12);
    ed.cmd(mv(Motion::LineStart));
    assert_eq!(ed.caret(), 12);
}

#[test]
fn line_end_in_one_line_is_the_text_end() {
    let mut ed = Ed::at(WORLD, 3);
    ed.cmd(mv(Motion::LineEnd));
    assert_eq!(ed.caret(), 11);
    ed.cmd(mv(Motion::LineStart));
    assert_eq!(ed.caret(), 0);
}

#[test]
fn up_and_down_keep_the_preferred_x_through_a_short_line() {
    let text = "héllo wörld\nab\nhéllo wörld";
    let mut ed = Ed::at(text, 9);
    ed.cmd(mv(Motion::LineDown));
    assert_eq!(ed.caret(), 14, "clamped to the short line's end");
    ed.cmd(mv(Motion::LineDown));
    assert_eq!(ed.caret(), 15 + 9, "the x came back");
    ed.cmd(mv(Motion::LineUp));
    ed.cmd(mv(Motion::LineUp));
    assert_eq!(ed.caret(), 9);
}

#[test]
fn a_sideways_move_or_an_edit_resets_the_preferred_x() {
    let text = "héllo wörld\nab\nhéllo wörld";
    let mut ed = Ed::at(text, 9);
    ed.cmd(mv(Motion::LineDown));
    ed.cmd(mv(Motion::GraphemeLeft));
    ed.cmd(mv(Motion::LineDown));
    assert_eq!(ed.caret(), 15 + 1);

    let mut ed = Ed::at(text, 9);
    ed.cmd(mv(Motion::LineDown));
    ed.typed("c");
    ed.cmd(mv(Motion::LineDown));
    assert_eq!(ed.caret(), 16 + 3);
}

#[test]
fn up_on_the_first_line_and_down_on_the_last_stay() {
    let mut ed = Ed::at("ab\ncd", 1);
    ed.cmd(mv(Motion::LineUp));
    assert_eq!(ed.caret(), 1);
    ed.edit.set_selection(4, 4);
    ed.cmd(mv(Motion::LineDown));
    assert_eq!(ed.caret(), 4);
}

#[test]
fn vertical_motion_extends() {
    let mut ed = Ed::at("héllo\nwörld", 2);
    ed.cmd(ext(Motion::LineDown));
    assert_eq!(ed.sel(), (2, 8));
    assert_eq!(ed.edit.anchor(), 2);
}

#[test]
fn page_motion_crosses_page_rows_and_stops_at_the_ends() {
    let text = (0..30)
        .map(|i| format!("l{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut ed = Ed::at(&text, 1);
    ed.cmd(mv(Motion::PageDown));
    assert_eq!(ed.caret(), 41, "ten lines of four chars");
    ed.cmd(ext(Motion::PageUp));
    assert_eq!(ed.caret(), 1);
    assert_eq!(ed.edit.anchor(), 41);
    let mut system = FakeText;
    let sp = spec(&text);
    let mut layout = SpecLayout::new(&mut system, &sp, 1.0).with_page_rows(100);
    let mut owned = text.clone();
    ed.edit.apply(
        mv(Motion::PageDown),
        &mut owned,
        &mut layout,
        Keymap::PC,
        Duration::ZERO,
    );
    assert_eq!(ed.caret(), 29 * 4 + 1, "stopped on the last line");
}

/// Rows that run into each other, as a soft wrap gives: "abc " and "def", 5 px chars.
struct Wrapped;

impl EditLayout for Wrapped {
    fn boundaries(&mut self, text: &str, _unit: Unit) -> Vec<usize> {
        (0..=text.chars().count()).collect()
    }

    fn caret(&mut self, offset: usize) -> CaretPos {
        let (row, x) = if offset >= 4 {
            (1, offset - 4)
        } else {
            (0, offset)
        };
        CaretPos {
            x: x as f32 * 5.0,
            y: row as f32 * 20.0,
            height: 20.0,
            line: row,
        }
    }

    fn hit(&mut self, pos: (f32, f32)) -> usize {
        let col = (pos.0 / 5.0).round() as usize;
        if pos.1 < 20.0 {
            col.min(4)
        } else {
            4 + col.min(3)
        }
    }

    fn lines(&mut self) -> Vec<LineSpan> {
        let row = |start, end, y| LineSpan {
            start,
            end,
            y,
            height: 20.0,
        };
        vec![row(0, 4, 0.0), row(4, 7, 20.0)]
    }
}

#[test]
fn at_a_soft_wrap_the_caret_stays_on_its_row() {
    let mut edit = TextEdit::new();
    let mut text = "abc def".to_owned();
    let mut run = |edit: &mut TextEdit, cmd| {
        edit.apply(cmd, &mut text, &mut Wrapped, Keymap::PC, Duration::ZERO);
        edit.caret()
    };
    edit.set_selection(1, 1);
    assert_eq!(run(&mut edit, mv(Motion::LineEnd)), 3, "before the wrap");
    assert_eq!(run(&mut edit, mv(Motion::LineEnd)), 3, "and End stays");
    assert_eq!(run(&mut edit, mv(Motion::LineStart)), 0);
    edit.set_selection(6, 6);
    assert_eq!(
        run(&mut edit, mv(Motion::LineStart)),
        4,
        "the wrap point is the later row"
    );
    assert_eq!(run(&mut edit, mv(Motion::LineUp)), 0);
    edit.set_selection(7, 7);
    assert_eq!(
        run(&mut edit, mv(Motion::LineUp)),
        3,
        "a far x stays on the upper row"
    );
}

#[test]
fn select_word_at_an_offset() {
    let mut ed = Ed::new(WORLD);
    ed.cmd(EditCommand::SelectWordAt(2));
    assert_eq!(ed.sel(), (0, 5));
    ed.cmd(EditCommand::SelectWordAt(5));
    assert_eq!(ed.sel(), (5, 6), "the space");
    ed.cmd(EditCommand::SelectWordAt(6));
    assert_eq!(ed.sel(), (6, 11));
    ed.cmd(EditCommand::SelectWordAt(11));
    assert_eq!(
        ed.sel(),
        (6, 11),
        "the end of the text belongs to the last word"
    );
    ed.cmd(EditCommand::SelectWordAt(999));
    assert_eq!(ed.sel(), (6, 11));
    let mut ed = Ed::new("");
    ed.cmd(EditCommand::SelectWordAt(0));
    assert_eq!(ed.sel(), (0, 0));
}

#[test]
fn select_line_at_an_offset() {
    let mut ed = Ed::new("héllo\nwörld\nx");
    ed.cmd(EditCommand::SelectLineAt(7));
    assert_eq!(ed.sel(), (6, 11));
    ed.cmd(EditCommand::SelectLineAt(0));
    assert_eq!(ed.sel(), (0, 5));
    ed.cmd(EditCommand::SelectLineAt(999));
    assert_eq!(ed.sel(), (12, 13));
}

#[test]
fn a_point_places_and_extends() {
    let mut ed = Ed::new(WORLD);
    ed.cmd(EditCommand::PlaceCaret((11.0, 5.0)));
    assert_eq!(ed.sel(), (2, 2));
    ed.cmd(EditCommand::ExtendTo((42.0, 5.0)));
    assert_eq!(ed.sel(), (2, 8));
    assert_eq!(ed.edit.anchor(), 2);
    ed.cmd(EditCommand::ExtendTo((1000.0, 5.0)));
    assert_eq!(ed.caret(), 11);
    ed.cmd(EditCommand::PlaceCaret((f32::NAN, f32::INFINITY)));
    assert_eq!(ed.sel(), (0, 0));
}

#[test]
fn a_point_inside_a_cluster_snaps_down() {
    let mut ed = Ed::new("e\u{301}x");
    ed.cmd(EditCommand::PlaceCaret((6.0, 5.0)));
    assert_eq!(ed.caret(), 0, "char 1 sits inside the cluster");
}

// Undo.

#[test]
fn typing_undoes_in_one_step_and_redoes() {
    let mut ed = Ed::new("");
    ed.typed("abc");
    assert_eq!(ed.edit.undo_steps(), 1);
    let out = ed.cmd(EditCommand::Undo);
    assert!(out.changed);
    assert_eq!((ed.text.as_str(), ed.caret()), ("", 0));
    assert_eq!(ed.edit.redo_steps(), 1);
    let out = ed.cmd(EditCommand::Redo);
    assert!(out.changed);
    assert_eq!((ed.text.as_str(), ed.caret()), ("abc", 3));
}

#[test]
fn undo_on_empty_logs_changes_nothing() {
    let mut ed = Ed::new("abc");
    let out = ed.cmd(EditCommand::Undo);
    assert!(!out.changed && out.consumed);
    let out = ed.cmd(EditCommand::Redo);
    assert!(!out.changed && out.consumed);
}

#[test]
fn a_pause_ends_a_typed_run() {
    let mut ed = Ed::new("");
    ed.typed("ab");
    ed.now += COALESCE_PAUSE - Duration::from_millis(1);
    ed.typed("c");
    assert_eq!(ed.edit.undo_steps(), 1, "just under the pause joins");
    ed.now += COALESCE_PAUSE;
    ed.typed("d");
    assert_eq!(
        ed.edit.undo_steps(),
        2,
        "the pause counts from the last char"
    );
    ed.cmd(EditCommand::Undo);
    assert_eq!(ed.text, "abc");
}

#[test]
fn a_caret_move_ends_a_typed_run() {
    let mut ed = Ed::new("");
    ed.typed("ab");
    ed.cmd(mv(Motion::GraphemeLeft));
    ed.typed("c");
    assert_eq!(ed.text, "acb");
    assert_eq!(ed.edit.undo_steps(), 2);
    ed.cmd(EditCommand::Undo);
    assert_eq!((ed.text.as_str(), ed.caret()), ("ab", 1));
}

#[test]
fn moving_there_and_back_still_ends_the_run() {
    let mut ed = Ed::new("");
    ed.typed("ab");
    ed.cmd(mv(Motion::GraphemeLeft));
    ed.cmd(mv(Motion::GraphemeRight));
    ed.typed("c");
    assert_eq!(ed.edit.undo_steps(), 2);
}

#[test]
fn another_kind_of_edit_ends_a_typed_run() {
    let mut ed = Ed::new("");
    ed.typed("ab");
    ed.cmd(BACK);
    ed.typed("c");
    assert_eq!(ed.text, "ac");
    assert_eq!(ed.edit.undo_steps(), 3);
    ed.cmd(EditCommand::Undo);
    ed.cmd(EditCommand::Undo);
    assert_eq!(ed.text, "ab");
    ed.cmd(EditCommand::Undo);
    assert_eq!(ed.text, "");
}

#[test]
fn typing_after_undo_starts_a_new_step_and_clears_redo() {
    let mut ed = Ed::new("");
    ed.typed("ab");
    ed.cmd(EditCommand::Undo);
    ed.typed("x");
    assert_eq!(ed.edit.redo_steps(), 0);
    assert_eq!(ed.edit.undo_steps(), 1);
    ed.typed("y");
    ed.cmd(EditCommand::Undo);
    assert_eq!(ed.text, "");
}

#[test]
fn typing_over_a_selection_restores_it_on_undo() {
    let mut ed = Ed::new(WORLD);
    ed.edit.set_selection(11, 6);
    ed.typed("ab");
    assert_eq!(ed.text, "héllo ab");
    assert_eq!(ed.edit.undo_steps(), 1);
    ed.cmd(EditCommand::Undo);
    assert_eq!(ed.text, WORLD);
    assert_eq!((ed.edit.anchor(), ed.caret()), (11, 6));
    ed.cmd(EditCommand::Redo);
    assert_eq!((ed.text.as_str(), ed.caret()), ("héllo ab", 8));
}

#[test]
fn undo_keeps_two_hundred_steps() {
    let mut ed = Ed::new("");
    for i in 0..UNDO_DEPTH + 5 {
        ed.now += COALESCE_PAUSE;
        ed.typed(&(i % 10).to_string());
    }
    assert_eq!(ed.edit.undo_steps(), UNDO_DEPTH);
    while ed.edit.undo_steps() > 0 {
        ed.cmd(EditCommand::Undo);
    }
    assert_eq!(ed.text, "01234", "the oldest five stay");
    while ed.edit.redo_steps() > 0 {
        ed.cmd(EditCommand::Redo);
    }
    assert_eq!(ed.text.chars().count(), UNDO_DEPTH + 5);
}

#[test]
fn an_external_change_clears_history_and_clamps() {
    let mut ed = Ed::new("");
    ed.typed("abcdef");
    ed.cmd(EditCommand::Undo);
    ed.cmd(EditCommand::Redo);
    ed.cmd(BACK);
    ed.cmd(EditCommand::Undo);
    assert!(ed.edit.undo_steps() > 0 && ed.edit.redo_steps() > 0);
    ed.text = "xy".to_owned();
    ed.sync();
    assert_eq!((ed.edit.undo_steps(), ed.edit.redo_steps()), (0, 0));
    assert_eq!(ed.caret(), 2);
    let out = ed.cmd(EditCommand::Undo);
    assert!(!out.changed);
    assert_eq!(ed.text, "xy");
}

#[test]
fn apply_notices_an_external_change_without_a_sync() {
    let mut ed = Ed::new("");
    ed.typed("abc");
    ed.text = "z".to_owned();
    let out = ed.cmd(EditCommand::Undo);
    assert!(!out.changed);
    assert_eq!((ed.text.as_str(), ed.caret()), ("z", 1));
}

#[test]
fn an_external_change_ends_a_typed_run() {
    let mut ed = Ed::new("");
    ed.typed("ab");
    ed.text = "ab!".to_owned();
    ed.sync();
    ed.typed("c");
    assert_eq!(ed.edit.undo_steps(), 1);
    ed.cmd(EditCommand::Undo);
    assert_eq!(ed.text, "ab!");
}

#[test]
fn the_engines_own_edits_are_not_external() {
    let mut ed = Ed::new("");
    ed.typed("abc");
    ed.sync();
    assert_eq!(ed.edit.undo_steps(), 1);
    ed.cmd(EditCommand::Undo);
    ed.sync();
    assert_eq!(ed.edit.redo_steps(), 1);
}

#[test]
fn sync_snaps_a_caret_inside_a_cluster_down_and_past_the_end_to_the_end() {
    let mut ed = Ed::new("e\u{301}x");
    ed.edit.set_selection(1, 999);
    ed.sync();
    assert_eq!((ed.edit.anchor(), ed.caret()), (0, 3));
}

// Clipboard.

#[test]
fn cut_copy_and_paste() {
    let mut ed = Ed::new(WORLD);
    ed.edit.set_selection(1, 5);
    let out = ed.cmd(EditCommand::Copy);
    assert_eq!(out.clipboard_write.as_deref(), Some("éllo"));
    assert!(out.consumed && !out.changed);
    assert_eq!(ed.text, WORLD);

    let out = ed.cmd(EditCommand::Cut);
    assert_eq!(out.clipboard_write.as_deref(), Some("éllo"));
    assert!(out.changed);
    assert_eq!((ed.text.as_str(), ed.caret()), ("h wörld", 1));

    let out = ed.cmd(EditCommand::Paste);
    assert!(out.clipboard_read && out.consumed && !out.changed);
    let out = ed.cmd(EditCommand::PasteText("éllo".to_owned()));
    assert!(out.changed);
    assert_eq!((ed.text.as_str(), ed.caret()), (WORLD, 5));
    ed.cmd(EditCommand::Undo);
    assert_eq!(ed.text, "h wörld");
    ed.cmd(EditCommand::Undo);
    assert_eq!(ed.text, WORLD);
}

#[test]
fn copy_with_no_selection_is_not_consumed() {
    let mut ed = Ed::at("abc", 1);
    let out = ed.cmd(EditCommand::Copy);
    assert_eq!(out, EditOutcome::default());
}

#[test]
fn a_paste_is_one_step_apart_from_typing() {
    let mut ed = Ed::new("");
    ed.typed("a");
    ed.cmd(EditCommand::PasteText("b".to_owned()));
    ed.typed("c");
    assert_eq!(ed.edit.undo_steps(), 3);
}

#[test]
fn one_line_pastes_flatten() {
    let mut ed = Ed::new("");
    ed.cmd(EditCommand::PasteText(
        "a\r\nb\nc\rd\te\u{7}f\u{2028}g".to_owned(),
    ));
    assert_eq!(ed.text, "a b c def g");
    assert_eq!(ed.caret(), 11);
}

#[test]
fn many_line_pastes_keep_breaks_and_unify_them() {
    let mut ed = Ed::with_flags(
        "",
        EditFlags {
            multiline: true,
            ..EditFlags::default()
        },
    );
    ed.cmd(EditCommand::PasteText("a\r\nb\nc\td".to_owned()));
    assert_eq!(ed.text, "a\nb\nc\td");
}

#[test]
fn text_that_cleans_to_nothing_keeps_the_selection() {
    let mut ed = Ed::new("abc");
    ed.edit.set_selection(0, 3);
    let out = ed.cmd(EditCommand::PasteText("\u{7}".to_owned()));
    assert!(!out.changed && out.consumed);
    assert_eq!(ed.text, "abc");
    assert_eq!(ed.sel(), (0, 3));
}

#[test]
fn a_one_line_editor_refuses_an_insert_over_the_cap_whole() {
    // A paste and an assistive set-value both come in as `PasteText`; typing and an input method's
    // commit as `Insert`. Past 1 MiB a one-line field would shape the lot every frame.
    let at_cap = "a".repeat(MAX_ONE_LINE_INSERT);
    let over = "a".repeat(MAX_ONE_LINE_INSERT + 1);
    for make in [EditCommand::PasteText, EditCommand::Insert] {
        let mut ed = Ed::new("ab");
        ed.edit.set_selection(1, 1);
        ed.sync();
        let out = ed.cmd(make(over.clone()));
        assert!(!out.changed, "refused");
        assert_eq!(
            (ed.text.as_str(), ed.sel()),
            ("ab", (1, 1)),
            "nothing cut, nothing moved"
        );

        // The same on a selection: it is not replaced.
        ed.edit.set_selection(0, 2);
        ed.sync();
        assert!(!ed.cmd(make(over.clone())).changed);
        assert_eq!((ed.text.as_str(), ed.sel()), ("ab", (0, 2)));

        // One byte less is a normal insert.
        let mut ed = Ed::new("");
        assert!(ed.cmd(make(at_cap.clone())).changed);
        assert_eq!(ed.text.len(), MAX_ONE_LINE_INSERT);
    }
    // Bytes, not chars: a 3-byte char 349,526 times is over the cap in bytes.
    let wide = "\u{20AC}".repeat(MAX_ONE_LINE_INSERT / 3 + 1);
    assert!(wide.len() > MAX_ONE_LINE_INSERT);
    let mut ed = Ed::new("");
    assert!(!ed.cmd(EditCommand::PasteText(wide)).changed);
    assert_eq!(ed.text, "");
    // A password field is one line too.
    let mut ed = Ed::with_flags(
        "",
        EditFlags {
            password: true,
            ..EditFlags::default()
        },
    );
    assert!(!ed.cmd(EditCommand::PasteText(over.clone())).changed);
    assert_eq!(ed.text, "");
}

#[test]
fn a_multi_line_editor_is_not_bound_by_the_one_line_cap() {
    let big = "line\n".repeat(MAX_ONE_LINE_INSERT / 5 * 2);
    assert!(big.len() > MAX_ONE_LINE_INSERT);
    let mut ed = Ed::with_flags(
        "",
        EditFlags {
            multiline: true,
            ..EditFlags::default()
        },
    );
    assert!(ed.cmd(EditCommand::PasteText(big.clone())).changed);
    assert_eq!(ed.text.len(), big.len());
}

#[test]
fn max_chars_still_applies_to_a_one_line_insert_under_the_cap() {
    let mut ed = Ed::with_flags(
        "",
        EditFlags {
            max_chars: Some(5),
            ..EditFlags::default()
        },
    );
    ed.cmd(EditCommand::PasteText("abcdefgh".to_owned()));
    assert_eq!(ed.text, "abcde");
}

#[test]
fn max_chars_truncates_on_a_grapheme_boundary() {
    let flags = EditFlags {
        max_chars: Some(5),
        ..EditFlags::default()
    };
    let mut ed = Ed::with_flags("abc", flags);
    ed.edit.set_selection(3, 3);
    ed.cmd(EditCommand::PasteText("de\u{301}f".to_owned()));
    assert_eq!(
        ed.text, "abcd",
        "the cluster would need two chars and only one is left"
    );

    let mut ed = Ed::with_flags("abc", flags);
    ed.edit.set_selection(1, 3);
    ed.cmd(EditCommand::PasteText("wxyzq".to_owned()));
    assert_eq!(ed.text, "awxyz", "the selection's room counts");

    let mut ed = Ed::with_flags("abcde", flags);
    ed.edit.set_selection(5, 5);
    let out = ed.cmd(ins("f"));
    assert!(!out.changed && out.consumed);
    assert_eq!(ed.text, "abcde");
}

// Flags.

fn flagged(read_only: bool, password: bool) -> EditFlags {
    EditFlags {
        read_only,
        password,
        ..EditFlags::default()
    }
}

#[test]
fn read_only_moves_and_selects_but_never_edits() {
    let mut ed = Ed::with_flags("abc", flagged(true, false));
    ed.cmd(mv(Motion::TextEnd));
    ed.cmd(ext(Motion::GraphemeLeft));
    assert_eq!(ed.sel(), (2, 3));
    for cmd in [
        ins("x"),
        BACK,
        DEL,
        EditCommand::Cut,
        EditCommand::Paste,
        EditCommand::PasteText("x".to_owned()),
        EditCommand::Undo,
        EditCommand::Redo,
        EditCommand::ImeCommit("x".to_owned()),
        EditCommand::ImePreedit {
            text: "x".to_owned(),
            cursor: None,
        },
    ] {
        let out = ed.cmd(cmd.clone());
        assert_eq!(out, EditOutcome::default(), "{cmd:?}");
    }
    assert_eq!(ed.text, "abc");
    assert_eq!(ed.edit.preedit(), None);
    let out = ed.cmd(EditCommand::Copy);
    assert_eq!(out.clipboard_write.as_deref(), Some("c"));
}

#[test]
fn read_only_many_lines_ignores_enter_and_one_line_submits() {
    let mut ed = Ed::with_flags(
        "abc",
        EditFlags {
            read_only: true,
            multiline: true,
            ..EditFlags::default()
        },
    );
    assert_eq!(ed.cmd(EditCommand::Enter), EditOutcome::default());
    assert_eq!(ed.text, "abc");
    let mut ed = Ed::with_flags("abc", flagged(true, false));
    assert!(ed.cmd(EditCommand::Enter).submitted);
}

#[test]
fn a_password_copies_and_cuts_nothing() {
    let mut ed = Ed::with_flags("secret", flagged(false, true));
    ed.edit.set_selection(0, 6);
    let out = ed.cmd(EditCommand::Copy);
    assert_eq!(out.clipboard_write, None);
    let out = ed.cmd(EditCommand::Cut);
    assert_eq!(out.clipboard_write, None);
    assert!(!out.changed);
    assert_eq!(ed.text, "secret");
}

#[test]
fn a_password_keeps_no_undo_steps() {
    let mut ed = Ed::new("");
    ed.typed("ab");
    ed.cmd(EditCommand::Undo);
    ed.typed("c");
    ed.cmd(mv(Motion::GraphemeLeft));
    ed.typed("d");
    ed.cmd(EditCommand::Undo);
    assert!(ed.edit.undo_steps() > 0 && ed.edit.redo_steps() > 0);
    ed.edit.set_flags(flagged(false, true));
    assert_eq!((ed.edit.undo_steps(), ed.edit.redo_steps()), (0, 0));
    ed.typed("xyz");
    ed.cmd(BACK);
    assert_eq!((ed.edit.undo_steps(), ed.edit.redo_steps()), (0, 0));
    let before = ed.text.clone();
    assert!(!ed.cmd(EditCommand::Undo).changed);
    assert_eq!(ed.text, before);
}

#[test]
fn enter_submits_one_line_and_breaks_many() {
    let mut ed = Ed::new("abc");
    let out = ed.cmd(EditCommand::Enter);
    assert!(out.submitted && out.consumed && !out.changed);
    assert_eq!(ed.text, "abc");

    let mut ed = Ed::with_flags(
        "ab",
        EditFlags {
            multiline: true,
            ..EditFlags::default()
        },
    );
    ed.edit.set_selection(1, 1);
    let out = ed.cmd(EditCommand::Enter);
    assert!(out.changed && !out.submitted);
    assert_eq!((ed.text.as_str(), ed.caret()), ("a\nb", 2));
}

// Events.

fn map(ev: &FocusEvent, keymap: Keymap, flags: EditFlags) -> Option<EditCommand> {
    EditCommand::from_event(ev, keymap, flags)
}

#[test]
fn the_primary_chords_map_on_each_keymap() {
    let f = EditFlags::default();
    for (keymap, primary, other) in [(Keymap::MAC, CMD, CTRL), (Keymap::PC, CTRL, CMD)] {
        for (c, cmd) in [
            ("a", EditCommand::SelectAll),
            ("z", EditCommand::Undo),
            ("y", EditCommand::Redo),
            ("x", EditCommand::Cut),
            ("c", EditCommand::Copy),
            ("v", EditCommand::Paste),
        ] {
            assert_eq!(map(&chord(c, primary), keymap, f), Some(cmd.clone()), "{c}");
            let upper = c.to_uppercase();
            assert_eq!(
                map(&chord(&upper, primary), keymap, f),
                Some(cmd),
                "{upper}"
            );
            assert_eq!(map(&chord(c, other), keymap, f), None, "{c} other");
            assert_eq!(map(&chord(c, NONE), keymap, f), None, "{c} plain");
        }
        let shifted = Modifiers {
            shift: true,
            ..primary
        };
        assert_eq!(
            map(&chord("Z", shifted), keymap, f),
            Some(EditCommand::Redo)
        );
        assert_eq!(
            map(&chord("z", shifted), keymap, f),
            Some(EditCommand::Redo)
        );
        assert_eq!(map(&chord("a", shifted), keymap, f), None);
    }
}

#[test]
fn the_word_modifier_maps_arrows_backspace_and_delete() {
    let f = EditFlags::default();
    for (keymap, word, other) in [(Keymap::MAC, ALT, CTRL), (Keymap::PC, CTRL, ALT)] {
        let on = |key| map(&named(key, word), keymap, f);
        assert_eq!(on(NamedKey::ArrowLeft), Some(mv(Motion::WordLeft)));
        assert_eq!(on(NamedKey::ArrowRight), Some(mv(Motion::WordRight)));
        assert_eq!(
            on(NamedKey::Backspace),
            Some(EditCommand::Backspace { word: true })
        );
        assert_eq!(
            on(NamedKey::Delete),
            Some(EditCommand::Delete { word: true })
        );
        let extending = Modifiers {
            shift: true,
            ..word
        };
        assert_eq!(
            map(&named(NamedKey::ArrowRight, extending), keymap, f),
            Some(ext(Motion::WordRight))
        );
        assert_eq!(map(&named(NamedKey::ArrowLeft, other), keymap, f), None);
        assert_eq!(map(&named(NamedKey::Backspace, other), keymap, f), None);
    }
}

#[test]
fn plain_keys_map() {
    let f = EditFlags::default();
    let k = Keymap::PC;
    let on = |key, m| map(&named(key, m), k, f);
    assert_eq!(
        on(NamedKey::ArrowLeft, NONE),
        Some(mv(Motion::GraphemeLeft))
    );
    assert_eq!(
        on(NamedKey::ArrowRight, SHIFT),
        Some(ext(Motion::GraphemeRight))
    );
    assert_eq!(on(NamedKey::Home, NONE), Some(mv(Motion::LineStart)));
    assert_eq!(on(NamedKey::End, SHIFT), Some(ext(Motion::LineEnd)));
    assert_eq!(on(NamedKey::Home, CTRL), Some(mv(Motion::TextStart)));
    assert_eq!(
        on(
            NamedKey::End,
            Modifiers {
                control: true,
                shift: true,
                ..NONE
            }
        ),
        Some(ext(Motion::TextEnd))
    );
    assert_eq!(on(NamedKey::Backspace, NONE), Some(BACK));
    assert_eq!(on(NamedKey::Delete, NONE), Some(DEL));
    assert_eq!(on(NamedKey::Enter, NONE), Some(EditCommand::Enter));
    assert_eq!(on(NamedKey::Enter, CTRL), None, "free for a binding");
    assert_eq!(on(NamedKey::Escape, NONE), None);
    assert_eq!(on(NamedKey::Tab, NONE), None);
    assert_eq!(on(NamedKey::Space, NONE), None, "the Text event types it");
    assert_eq!(
        map(&chord("a", NONE), k, f),
        None,
        "the Text event types it"
    );
}

#[test]
fn vertical_keys_map_only_in_many_lines() {
    let one = EditFlags::default();
    let many = EditFlags {
        multiline: true,
        ..EditFlags::default()
    };
    for (key, motion) in [
        (NamedKey::ArrowUp, Motion::LineUp),
        (NamedKey::ArrowDown, Motion::LineDown),
        (NamedKey::PageUp, Motion::PageUp),
        (NamedKey::PageDown, Motion::PageDown),
    ] {
        assert_eq!(map(&named(key, NONE), Keymap::PC, one), None);
        assert_eq!(map(&named(key, NONE), Keymap::PC, many), Some(mv(motion)));
        assert_eq!(map(&named(key, SHIFT), Keymap::PC, many), Some(ext(motion)));
    }
}

#[test]
fn text_and_clipboard_events_map() {
    let f = EditFlags::default();
    let k = Keymap::MAC;
    assert_eq!(map(&FocusEvent::Text("é".to_owned()), k, f), Some(ins("é")));
    assert_eq!(
        map(&FocusEvent::ClipboardText("x".to_owned()), k, f),
        Some(EditCommand::PasteText("x".to_owned()))
    );
    assert_eq!(map(&FocusEvent::ClipboardUnavailable, k, f), None);
}

#[test]
fn read_only_maps_no_edit_key() {
    let f = flagged(true, false);
    let k = Keymap::PC;
    for ev in [
        chord("x", CTRL),
        chord("v", CTRL),
        chord("z", CTRL),
        chord("y", CTRL),
        named(NamedKey::Backspace, NONE),
        named(NamedKey::Delete, CTRL),
        FocusEvent::Text("a".to_owned()),
        FocusEvent::ClipboardText("a".to_owned()),
    ] {
        assert_eq!(map(&ev, k, f), None, "{ev:?}");
    }
    assert_eq!(map(&chord("c", CTRL), k, f), Some(EditCommand::Copy));
    assert_eq!(map(&chord("a", CTRL), k, f), Some(EditCommand::SelectAll));
    assert_eq!(
        map(&named(NamedKey::ArrowLeft, NONE), k, f),
        Some(mv(Motion::GraphemeLeft))
    );
}

#[test]
fn a_full_key_sequence_through_events() {
    let mut ed = Ed::new("");
    ed.keymap = Keymap::MAC;
    for c in "abc".chars() {
        ed.event(FocusEvent::Text(c.to_string()));
    }
    ed.event(named(NamedKey::ArrowLeft, NONE));
    ed.event(named(NamedKey::ArrowLeft, NONE));
    ed.event(named(NamedKey::ArrowRight, SHIFT));
    assert_eq!(ed.sel(), (1, 2));
    let out = ed.event(chord("c", CMD));
    assert_eq!(out.and_then(|o| o.clipboard_write).as_deref(), Some("b"));
    ed.event(chord("a", CMD));
    ed.event(FocusEvent::Text("z".to_owned()));
    assert_eq!(ed.text, "z");
    ed.event(chord("z", CMD));
    assert_eq!(ed.text, "abc");
}

// IME.

fn preedit(text: &str, cursor: Option<(usize, usize)>) -> EditCommand {
    EditCommand::ImePreedit {
        text: text.to_owned(),
        cursor,
    }
}

#[test]
fn a_preedit_cursor_is_bytes_converted_to_chars() {
    let mut ed = Ed::new("");
    ed.cmd(preedit("héllo", Some((3, 3))));
    assert_eq!(ed.edit.preedit_cursor(), Some((2, 2)));
    ed.cmd(preedit("か", Some((0, 3))));
    assert_eq!(ed.edit.preedit_cursor(), Some((0, 1)));
}

#[test]
fn a_preedit_cursor_inside_a_char_or_past_the_end_clamps() {
    let mut ed = Ed::new("");
    ed.cmd(preedit("か", Some((1, 1))));
    assert_eq!(ed.edit.preedit_cursor(), Some((0, 0)));
    ed.cmd(preedit("か", Some((2, 999))));
    assert_eq!(ed.edit.preedit_cursor(), Some((0, 1)));
    ed.cmd(preedit("か", None));
    assert_eq!(ed.edit.preedit_cursor(), None);
}

#[test]
fn a_preedit_never_touches_the_text() {
    let mut ed = Ed::at("lune", 2);
    let out = ed.cmd(preedit("かな", Some((0, 3))));
    assert!(!out.changed && out.consumed);
    assert_eq!(ed.text, "lune");
    assert_eq!(ed.edit.undo_steps(), 0);
    assert_eq!(ed.caret(), 2);
    assert_eq!(
        ed.edit.display(&ed.text),
        ("luかなne".to_owned(), Some(2..4))
    );
}

#[test]
fn display_is_the_text_without_a_preedit() {
    let ed = Ed::at("lune", 2);
    assert_eq!(ed.edit.display(&ed.text), ("lune".to_owned(), None));
}

#[test]
fn display_shows_the_preedit_in_place_of_the_selection() {
    let mut ed = Ed::new("héllo");
    ed.edit.set_selection(4, 1);
    ed.cmd(preedit("か", None));
    assert_eq!(ed.edit.display(&ed.text), ("hかo".to_owned(), Some(1..2)));
    ed.cmd(EditCommand::ImeCancel);
    assert_eq!(ed.sel(), (1, 4), "a cancel brings the selection back");
}

#[test]
fn display_is_total() {
    let mut ed = Ed::new("abc");
    ed.cmd(preedit("か", None));
    ed.edit.set_selection(999, 999);
    assert_eq!(ed.edit.display("ab"), ("abか".to_owned(), Some(2..3)));
}

#[test]
fn an_empty_preedit_clears() {
    let mut ed = Ed::new("abc");
    ed.cmd(preedit("か", Some((0, 3))));
    ed.cmd(preedit("", Some((0, 0))));
    assert_eq!(ed.edit.preedit(), None);
    assert_eq!(ed.edit.preedit_cursor(), None);
    assert_eq!(ed.edit.display(&ed.text).1, None);
}

#[test]
fn the_scripted_sequence_leaves_one_character() {
    let mut ed = Ed::new("");
    for e in [
        ImeEvent::Enabled,
        ImeEvent::Preedit {
            text: "か".to_owned(),
            cursor: Some((0, 3)),
        },
        ImeEvent::Commit("か".to_owned()),
        ImeEvent::Disabled,
    ] {
        ed.event(FocusEvent::Ime(e));
    }
    assert_eq!((ed.text.as_str(), ed.caret()), ("か", 1));
    assert_eq!(ed.edit.preedit(), None);
}

#[test]
fn commit_after_an_empty_preedit_inserts_once() {
    let mut ed = Ed::new("");
    ed.cmd(preedit("か", Some((0, 3))));
    ed.cmd(preedit("", None));
    let out = ed.cmd(EditCommand::ImeCommit("か".to_owned()));
    assert!(out.changed);
    assert_eq!((ed.text.as_str(), ed.caret()), ("か", 1));
    // And the other order leaves one insertion too.
    let mut ed = Ed::new("");
    ed.cmd(preedit("か", None));
    ed.cmd(EditCommand::ImeCommit("か".to_owned()));
    ed.cmd(preedit("", None));
    assert_eq!(ed.text, "か");
}

#[test]
fn commit_replaces_the_selection_as_one_step() {
    let mut ed = Ed::new("lune");
    ed.typed("");
    ed.edit.set_selection(1, 3);
    ed.cmd(preedit("か", None));
    ed.cmd(EditCommand::ImeCommit("かな".to_owned()));
    assert_eq!((ed.text.as_str(), ed.caret()), ("lかなe", 3));
    assert_eq!(ed.edit.undo_steps(), 1);
    ed.cmd(EditCommand::Undo);
    assert_eq!(ed.text, "lune");
    assert_eq!(ed.sel(), (1, 3));
}

#[test]
fn a_commit_joins_no_typed_run() {
    let mut ed = Ed::new("");
    ed.typed("a");
    ed.cmd(EditCommand::ImeCommit("か".to_owned()));
    ed.typed("b");
    assert_eq!(ed.edit.undo_steps(), 3);
}

#[test]
fn an_empty_commit_keeps_the_selection() {
    let mut ed = Ed::new("abc");
    ed.edit.set_selection(0, 3);
    ed.cmd(preedit("か", None));
    let out = ed.cmd(EditCommand::ImeCommit(String::new()));
    assert!(!out.changed);
    assert_eq!(ed.text, "abc");
    assert_eq!(ed.edit.preedit(), None);
}

#[test]
fn commit_follows_the_line_and_length_rules() {
    let mut ed = Ed::with_flags(
        "",
        EditFlags {
            max_chars: Some(2),
            ..EditFlags::default()
        },
    );
    ed.cmd(EditCommand::ImeCommit("か\nなた".to_owned()));
    assert_eq!(ed.text, "か ");
}

#[test]
fn disabled_and_cancel_drop_the_preedit_and_commit_nothing() {
    let mut ed = Ed::new("abc");
    ed.cmd(preedit("か", None));
    ed.event(FocusEvent::Ime(ImeEvent::Disabled));
    assert_eq!(ed.edit.preedit(), None);
    assert_eq!(ed.text, "abc");
    ed.cmd(preedit("か", None));
    ed.cmd(EditCommand::ImeCancel);
    assert_eq!(ed.edit.preedit(), None);
    assert_eq!(ed.text, "abc");
}

#[test]
fn keys_and_typed_text_yield_nothing_while_composing() {
    let mut ed = Ed::at("abc", 1);
    ed.cmd(preedit("か", None));
    assert!(ed.edit.flags().composing);
    for ev in [
        named(NamedKey::Enter, NONE),
        named(NamedKey::ArrowLeft, NONE),
        named(NamedKey::ArrowRight, SHIFT),
        named(NamedKey::Backspace, NONE),
        named(NamedKey::Escape, NONE),
        chord("a", CTRL),
        chord("z", CTRL),
        FocusEvent::Text("x".to_owned()),
    ] {
        assert_eq!(ed.event(ev.clone()), None, "{ev:?}");
    }
    assert_eq!((ed.text.as_str(), ed.caret()), ("abc", 1));
    ed.cmd(EditCommand::ImeCancel);
    assert!(!ed.edit.flags().composing);
    assert!(ed.event(named(NamedKey::ArrowRight, NONE)).is_some());
}

#[test]
fn apply_refuses_key_and_pointer_commands_while_composing() {
    let mut ed = Ed::at("abc", 1);
    ed.cmd(preedit("か", None));
    for cmd in [
        mv(Motion::TextEnd),
        EditCommand::Enter,
        BACK,
        EditCommand::SelectAll,
        EditCommand::PlaceCaret((0.0, 0.0)),
        EditCommand::SelectWordAt(0),
    ] {
        assert_eq!(ed.cmd(cmd.clone()), EditOutcome::default(), "{cmd:?}");
    }
    assert_eq!((ed.text.as_str(), ed.sel()), ("abc", (1, 1)));
    assert!(ed.edit.preedit().is_some());
}

#[test]
fn a_clipboard_answer_still_lands_while_composing() {
    let mut ed = Ed::at("ab", 1);
    ed.cmd(preedit("か", None));
    let out = ed.event(FocusEvent::ClipboardText("x".to_owned()));
    assert!(out.is_some_and(|o| o.changed));
    assert_eq!(ed.edit.display(&ed.text), ("axかb".to_owned(), Some(2..3)));
}

#[test]
fn turning_read_only_drops_the_preedit() {
    let mut ed = Ed::new("abc");
    ed.cmd(preedit("か", None));
    ed.edit.set_flags(flagged(true, false));
    assert_eq!(ed.edit.preedit(), None);
}

// The whole state.

#[test]
fn debug_prints_no_text() {
    let mut ed = Ed::new("");
    ed.typed("hunter2");
    ed.cmd(EditCommand::Undo);
    ed.cmd(EditCommand::Redo);
    ed.cmd(BACK);
    ed.cmd(EditCommand::Undo);
    ed.cmd(preedit("swordfish", Some((0, 3))));
    let edit = format!("{:?}", ed.edit);
    assert!(
        !edit.contains("hunter") && !edit.contains("swordfish"),
        "{edit}"
    );
    assert!(
        edit.contains("caret") && edit.contains("preedit_chars: Some(9)"),
        "{edit}"
    );
    let sp = spec("hunter2");
    let mut system = FakeText;
    let layout = SpecLayout::new(&mut system, &sp, 1.0);
    assert!(!format!("{layout:?}").contains("hunter"));
}

#[test]
fn spec_layout_answers_in_chars_on_a_multi_byte_text() {
    let mut system = FakeText;
    let sp = spec("héllo\nwörld");
    let mut layout = SpecLayout::new(&mut system, &sp, 1.0);
    assert_eq!(layout.caret(2).x, 10.0);
    assert_eq!(layout.caret(8).x, 10.0);
    assert_eq!(layout.caret(8).line, 1);
    assert_eq!(layout.caret(999).x, 25.0);
    assert_eq!(layout.hit((10.0, 5.0)), 2);
    assert_eq!(layout.hit((10.0, 25.0)), 8);
    let lines = layout.lines();
    assert_eq!(
        lines.iter().map(|l| (l.start, l.end)).collect::<Vec<_>>(),
        [(0, 5), (6, 11)]
    );
    assert_eq!(layout.boundaries("é x", Unit::Word), [0, 1, 2, 3]);
    assert_eq!(layout.boundaries("e\u{301}x", Unit::Grapheme), [0, 2, 3]);
}

/// A small fixed generator, so a failure repeats.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

fn random_command(r: &mut Lcg) -> EditCommand {
    const MOTIONS: [Motion; 12] = [
        Motion::GraphemeLeft,
        Motion::GraphemeRight,
        Motion::WordLeft,
        Motion::WordRight,
        Motion::LineStart,
        Motion::LineEnd,
        Motion::TextStart,
        Motion::TextEnd,
        Motion::LineUp,
        Motion::LineDown,
        Motion::PageUp,
        Motion::PageDown,
    ];
    const SNIPPETS: [&str; 8] = [
        "a",
        "é",
        "e\u{301}",
        "\u{1F44D}\u{1F3FD}",
        " ",
        "\n",
        "か",
        "x\r\ny",
    ];
    let snippet = SNIPPETS[r.next(SNIPPETS.len())].to_owned();
    let offset = [0, 1, 3, 7, 40, usize::MAX][r.next(6)];
    let pos = [
        (0.0, 0.0),
        (13.0, 25.0),
        (1e9, -1e9),
        (f32::NAN, 0.0),
        (f32::INFINITY, f32::NEG_INFINITY),
    ][r.next(5)];
    match r.next(19) {
        0..=3 => EditCommand::Move {
            motion: MOTIONS[r.next(12)],
            extend: r.next(2) == 0,
        },
        4 => EditCommand::SelectAll,
        5 => EditCommand::SelectWordAt(offset),
        6 => EditCommand::SelectLineAt(offset),
        7 | 8 => EditCommand::Insert(snippet),
        9 => EditCommand::Backspace {
            word: r.next(2) == 0,
        },
        10 => EditCommand::Delete {
            word: r.next(2) == 0,
        },
        11 => EditCommand::Undo,
        12 => EditCommand::Redo,
        13 => EditCommand::Cut,
        14 => EditCommand::PasteText(snippet),
        15 => EditCommand::PlaceCaret(pos),
        16 => EditCommand::ExtendTo(pos),
        17 => EditCommand::Enter,
        _ => match r.next(3) {
            0 => EditCommand::ImePreedit {
                text: snippet,
                cursor: Some((offset, r.next(9))),
            },
            1 => EditCommand::ImeCommit(snippet),
            _ => EditCommand::ImeCancel,
        },
    }
}

#[test]
fn any_command_sequence_keeps_offsets_valid_and_undo_exact() {
    for seed in 0..40 {
        let mut r = Lcg(seed);
        let flags = EditFlags {
            multiline: seed % 2 == 0,
            max_chars: (seed % 5 == 0).then_some(12),
            ..EditFlags::default()
        };
        let mut ed = Ed::with_flags("héllo\nwörld e\u{301}", flags);
        ed.keymap = if seed % 3 == 0 {
            Keymap::MAC
        } else {
            Keymap::PC
        };
        // Each undo step's text, oldest first: what undo must walk back through.
        let mut history = vec![ed.text.clone()];
        for _ in 0..400 {
            ed.now += Duration::from_millis(r.next(700) as u64);
            let cmd = random_command(&mut r);
            let undoing = matches!(cmd, EditCommand::Undo);
            let redoing = matches!(cmd, EditCommand::Redo);
            let out = ed.cmd(cmd);
            ed.sync();
            let count = ed.text.chars().count();
            let sp = spec(&ed.text);
            let bounds =
                SpecLayout::new(&mut ed.system, &sp, 1.0).boundaries(&ed.text, Unit::Grapheme);
            assert!(bounds.contains(&ed.edit.caret()), "seed {seed} caret");
            assert!(bounds.contains(&ed.edit.anchor()), "seed {seed} anchor");
            assert!(ed.edit.caret() <= count && ed.edit.anchor() <= count);
            let (shown, range) = ed.edit.display(&ed.text);
            if let Some(range) = range {
                assert!(range.end <= shown.chars().count());
            }
            assert!(ed.edit.undo_steps() <= UNDO_DEPTH);
            if !out.changed {
                continue;
            }
            if undoing {
                let at = ed.edit.undo_steps();
                assert_eq!(ed.text, history[at], "seed {seed} undo");
            } else if redoing {
                let at = ed.edit.undo_steps();
                assert_eq!(ed.text, history[at], "seed {seed} redo");
            } else {
                // A new edit: a typed char may have joined the top step.
                history.truncate(ed.edit.undo_steps());
                history.push(ed.text.clone());
            }
        }
        ed.cmd(EditCommand::ImeCancel);
        while ed.edit.undo_steps() > 0 {
            ed.cmd(EditCommand::Undo);
            assert_eq!(
                ed.text,
                history[ed.edit.undo_steps()],
                "seed {seed} unwinding"
            );
        }
    }
}

#[test]
fn any_stale_or_wild_offset_never_panics_on_an_empty_text() {
    let mut r = Lcg(7);
    let mut ed = Ed::new("");
    for _ in 0..500 {
        ed.edit
            .set_selection(r.next(5) * 1000, usize::MAX - r.next(3));
        ed.text.clear();
        let cmd = random_command(&mut r);
        ed.cmd(cmd);
    }
}

#[test]
fn a_layout_with_no_lines_or_boundaries_is_still_total() {
    struct Empty;
    impl EditLayout for Empty {
        fn boundaries(&mut self, _: &str, _: Unit) -> Vec<usize> {
            Vec::new()
        }
        fn caret(&mut self, _: usize) -> CaretPos {
            CaretPos {
                x: 0.0,
                y: 0.0,
                height: 0.0,
                line: 0,
            }
        }
        fn hit(&mut self, _: (f32, f32)) -> usize {
            usize::MAX
        }
        fn lines(&mut self) -> Vec<LineSpan> {
            Vec::new()
        }
    }
    let mut r = Lcg(11);
    let mut edit = TextEdit::new();
    let mut text = "abc".to_owned();
    for _ in 0..500 {
        let cmd = random_command(&mut r);
        edit.apply(cmd, &mut text, &mut Empty, Keymap::PC, Duration::ZERO);
    }
}

#[test]
fn offsets_survive_a_text_system_that_only_shapes() {
    // `TextSystem` is object safe at the seam `SpecLayout` needs.
    let mut system = FakeText;
    let dynamic: &mut dyn TextSystem = &mut system;
    let sp = spec("abc");
    let layout = SpecLayout::new(dynamic, &sp, 2.0).with_page_rows(3);
    assert_eq!(layout.page_rows(), 3);
}

/// Every `EditCommand` variant at least once; the `match` stops compiling when a variant is added
/// without a case here.
fn every_command() -> Vec<EditCommand> {
    let all = vec![
        mv(Motion::GraphemeRight),
        ext(Motion::LineUp),
        EditCommand::SelectAll,
        EditCommand::SelectWordAt(2),
        EditCommand::SelectLineAt(2),
        ins("x"),
        BACK,
        EditCommand::Backspace { word: true },
        DEL,
        EditCommand::Delete { word: true },
        EditCommand::Undo,
        EditCommand::Redo,
        EditCommand::Cut,
        EditCommand::Copy,
        EditCommand::Paste,
        EditCommand::PasteText("p".to_owned()),
        EditCommand::PlaceCaret((7.0, 3.0)),
        EditCommand::ExtendTo((12.0, 3.0)),
        EditCommand::Enter,
        EditCommand::ImePreedit {
            text: "か".to_owned(),
            cursor: None,
        },
        EditCommand::ImePreedit {
            text: String::new(),
            cursor: None,
        },
        EditCommand::ImeCommit("c".to_owned()),
        EditCommand::ImeCancel,
    ];
    for c in &all {
        match c {
            EditCommand::Move { .. }
            | EditCommand::SelectAll
            | EditCommand::SelectWordAt(_)
            | EditCommand::SelectLineAt(_)
            | EditCommand::Insert(_)
            | EditCommand::Backspace { .. }
            | EditCommand::Delete { .. }
            | EditCommand::Undo
            | EditCommand::Redo
            | EditCommand::Cut
            | EditCommand::Copy
            | EditCommand::Paste
            | EditCommand::PasteText(_)
            | EditCommand::PlaceCaret(_)
            | EditCommand::ExtendTo(_)
            | EditCommand::Enter
            | EditCommand::ImePreedit { .. }
            | EditCommand::ImeCommit(_)
            | EditCommand::ImeCancel => {}
        }
    }
    all
}

#[test]
fn takes_is_what_apply_consumes_for_every_command_and_state() {
    let mut checked = 0;
    let mut taken = 0;
    for cmd in every_command() {
        for read_only in [false, true] {
            for multiline in [false, true] {
                for selected in [false, true] {
                    for composing in [false, true] {
                        let mut ed = Ed::at("one two\nthree", 4);
                        ed.edit.set_flags(EditFlags {
                            read_only,
                            multiline,
                            ..EditFlags::default()
                        });
                        if selected {
                            ed.edit.set_selection(2, 6);
                            ed.sync();
                        }
                        if composing && !read_only {
                            // A preedit exists: only the IME commands and a clipboard answer run.
                            ed.cmd(EditCommand::ImePreedit {
                                text: "か".to_owned(),
                                cursor: None,
                            });
                            assert!(ed.edit.preedit().is_some());
                        }
                        let takes = ed.edit.takes(&cmd);
                        let consumed = ed.cmd(cmd.clone()).consumed;
                        assert_eq!(
                            takes, consumed,
                            "{cmd:?} read_only={read_only} multiline={multiline} \
                             selected={selected} composing={composing}"
                        );
                        checked += 1;
                        taken += usize::from(takes);
                    }
                }
            }
        }
    }
    // The instrument saw both answers.
    assert!(
        checked > 0 && taken > 0 && taken < checked,
        "{taken}/{checked}"
    );
}

#[test]
fn takes_changes_nothing() {
    for cmd in every_command() {
        let mut ed = Ed::at("one two", 3);
        ed.edit.set_selection(1, 5);
        ed.sync();
        let before = (ed.sel(), ed.text.clone(), ed.edit.undo_steps());
        ed.edit.takes(&cmd);
        assert_eq!((ed.sel(), ed.text.clone(), ed.edit.undo_steps()), before);
    }
}
