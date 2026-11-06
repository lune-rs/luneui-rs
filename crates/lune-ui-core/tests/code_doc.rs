//! The document of the code editor and the engine seam (D179, D180): the line index against a
//! fresh build after every edit of a seeded script, one key script through `TextEdit` over `String`
//! and over `CodeDoc`, undo on a document, and what a call reads of a long document.

use std::cell::Cell;
use std::ops::Range;
use std::time::Duration;

use lune_ui_core::code_doc::CodeDoc;
use lune_ui_core::input::Keymap;
use lune_ui_core::testing::FakeText;
use lune_ui_core::text::{CaretPos, LineSpan, Spec, Style, TextSystem, Unit};
use lune_ui_core::text_edit::{
    COALESCE_PAUSE, EditBuffer, EditCommand, EditFlags, EditLayout, EditOutcome, EditText, Motion,
    SpecLayout, TextEdit, UNDO_DEPTH,
};

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

/// A small seeded generator, so a failing script replays.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
        from[self.below(from.len())]
    }
}

/// The index of `doc` read through its whole public face equals a fresh build of its text.
fn assert_index(doc: &CodeDoc, step: usize) {
    let fresh = CodeDoc::new(doc.text());
    assert_eq!(doc.line_count(), fresh.line_count(), "step {step}: count");
    assert_eq!(doc.char_len(), fresh.char_len(), "step {step}: chars");
    assert_eq!(doc.char_len(), doc.text().chars().count(), "step {step}");
    assert_eq!(
        doc.widest_chars(),
        fresh.widest_chars(),
        "step {step}: widest"
    );
    for i in 0..doc.line_count() {
        assert_eq!(doc.line(i), fresh.line(i), "step {step}: line {i}");
        assert_eq!(doc.line_start(i), fresh.line_start(i), "step {step}: {i}");
    }
}

#[test]
fn the_three_terminators_end_a_line_and_stay_in_the_text() {
    let doc = CodeDoc::new("a\r\nb\rc\n");
    assert_eq!(doc.line_count(), 4);
    let lines: Vec<&str> = (0..4).map(|i| doc.line(i)).collect();
    assert_eq!(lines, ["a", "b", "c", ""]);
    assert_eq!(doc.text(), "a\r\nb\rc\n");
    assert_eq!(doc.line(4), "");
    assert_eq!(doc.line(usize::MAX), "");
}

#[test]
fn an_empty_document_has_one_line() {
    let doc = CodeDoc::new("");
    assert_eq!(doc.line_count(), 1);
    assert_eq!(doc.line(0), "");
    assert_eq!(doc.line_of(9), 0);
    assert_eq!(doc.widest_chars(), 0);
    assert_eq!(CodeDoc::default().line_count(), 1);
}

#[test]
fn a_char_offset_finds_its_line_and_a_line_its_start() {
    let doc = CodeDoc::new("é😀\r\nb\rc\n");
    // Line 0 is chars 0..4 with its `\r\n`: both terminator chars are on the line they end.
    assert_eq!(
        (0..9).map(|at| doc.line_of(at)).collect::<Vec<_>>(),
        [0, 0, 0, 0, 1, 1, 2, 2, 3]
    );
    assert_eq!(doc.line_of(usize::MAX), 3);
    assert_eq!(
        (0..5).map(|i| doc.line_start(i)).collect::<Vec<_>>(),
        [0, 4, 6, 8, 8]
    );
    assert_eq!(doc.char_len(), 8);
}

#[test]
fn the_widest_line_is_counted_in_chars_without_its_terminator() {
    let mut doc = CodeDoc::new("éé\r\nabc\nz");
    assert_eq!(doc.widest_chars(), 3);
    doc.replace(4..7, "");
    assert_eq!(doc.widest_chars(), 2, "the widest line shrank");
    doc.replace(0..0, "wider ");
    assert_eq!(doc.widest_chars(), 8);
}

#[test]
fn the_fingerprint_changes_on_every_change_and_no_two_documents_share_one() {
    let (mut a, mut b) = (CodeDoc::new("same"), CodeDoc::new("same"));
    assert_ne!(a.fingerprint(), b.fingerprint());
    let before = a.fingerprint();
    a.replace(0..0, "x");
    assert_ne!(a.fingerprint(), before);
    let before = a.fingerprint();
    a.set_text("same");
    assert_ne!(a.fingerprint(), before);
    let mut twin = a.clone();
    assert_eq!(twin.fingerprint(), a.fingerprint(), "the same text");
    twin.replace(0..1, "y");
    a.replace(0..1, "z");
    b.replace(0..1, "z");
    assert_ne!(twin.fingerprint(), a.fingerprint());
    assert_ne!(b.fingerprint(), a.fingerprint());
}

#[test]
fn a_window_runs_from_the_line_before_to_the_line_after_with_terminators() {
    let doc = CodeDoc::new("aa\r\nbb\rcc\ndd\nee");
    assert_eq!(doc.window(0), (0, "aa\r\nbb\r"));
    assert_eq!(doc.window(4), (0, "aa\r\nbb\rcc\n"));
    assert_eq!(doc.window(8), (4, "bb\rcc\ndd\n"));
    assert_eq!(doc.window(14), (10, "dd\nee"));
    assert_eq!(
        doc.window(usize::MAX),
        (10, "dd\nee"),
        "past the end clamps"
    );
    assert_eq!(doc.window(2), doc.window(3), "inside the `\\r\\n`");
}

#[test]
fn a_slice_and_a_replace_take_chars_and_clamp() {
    let mut doc = CodeDoc::new("héllo\nwörld");
    assert_eq!(doc.slice(1..8), "éllo\nwö");
    assert_eq!(doc.slice(8..usize::MAX), "rld");
    assert_eq!(doc.slice(99..100), "");
    let (high, low) = (4, 2);
    assert_eq!(doc.slice(high..low), "", "an inverted range is empty");
    doc.replace(9..99, "!");
    assert_eq!(doc.text(), "héllo\nwör!");
    doc.replace(99..100, "?");
    assert_eq!(doc.text(), "héllo\nwör!?");
    assert_index(&doc, 0);
}

#[test]
fn an_edit_that_makes_or_breaks_a_crlf_moves_the_line_start() {
    // Between the `\r` and the `\n`: two terminators, so one more line.
    let mut doc = CodeDoc::new("a\r\nb");
    doc.replace(2..2, "x");
    assert_eq!(doc.text(), "a\rx\nb");
    assert_eq!(doc.line_count(), 3);
    assert_index(&doc, 0);
    // The `\n` of a `\r\n` goes: a lone `\r` still ends the line.
    let mut doc = CodeDoc::new("a\r\nb");
    doc.replace(2..3, "");
    assert_eq!(doc.line_count(), 2);
    assert_index(&doc, 1);
    // A lone `\r` and a following `\n` meet when what stood between them goes.
    let mut doc = CodeDoc::new("a\rx\nb");
    doc.replace(2..3, "");
    assert_eq!(doc.text(), "a\r\nb");
    assert_eq!(doc.line_count(), 2);
    assert_index(&doc, 2);
    // An insert ending in `\r` before a `\n`, and one starting with `\n` after a `\r`.
    let mut doc = CodeDoc::new("a\nb\rc");
    doc.replace(1..1, "x\r");
    assert_eq!(doc.line_count(), 3);
    assert_index(&doc, 3);
    doc.replace(6..6, "\ny");
    assert_eq!(doc.text(), "ax\r\nb\r\nyc");
    assert_eq!(doc.line_count(), 3);
    assert_index(&doc, 4);
}

#[test]
fn the_incremental_index_equals_a_fresh_one_after_every_edit_of_a_seeded_script() {
    const PIECES: &[&str] = &[
        "\n",
        "\r\n",
        "\r",
        "\n\n",
        "\r\r\n\n",
        "x",
        "word ",
        "é",
        "😀",
        "e\u{301}",
        "👩\u{200D}💻",
        "a\nb",
        "a\r\nb\rc",
        "pasted\nover\nlines\n",
    ];
    for seed in [0x9E37_79B9_7F4A_7C15, 0xD1B5_4A32_D192_ED03, 7] {
        let mut rng = Rng(seed);
        let mut doc = CodeDoc::new("start\r\nof the\rtext\n\nend");
        let mut model = doc.text().to_owned();
        for step in 0..2_000 {
            let len = doc.char_len();
            let line = rng.below(doc.line_count());
            let at = match rng.below(4) {
                // A line's end, on its terminator.
                0 => doc.line_start(line) + doc.line(line).chars().count(),
                // Inside the terminator, when it is a `\r\n`.
                1 => doc.line_start(line) + doc.line(line).chars().count() + 1,
                2 => doc.line_start(line),
                _ => rng.below(len + 1),
            }
            .min(len);
            let span = match rng.below(5) {
                0 | 1 => 0,
                2 => 1,
                3 => rng.below(6),
                // Across lines.
                _ => doc.line_start(line + rng.below(4) + 1).saturating_sub(at),
            };
            let with = match rng.below(4) {
                0 => "",
                _ => rng.pick(PIECES),
            };
            doc.replace(at..at + span, with);
            model.replace(at..at + span, with);
            assert_eq!(doc.text(), model, "seed {seed}, step {step}");
            assert_index(&doc, step);
            if doc.char_len() > 600 {
                let cut = doc.char_len() / 2;
                doc.replace(cut..doc.char_len(), "");
                model.replace(cut..usize::MAX, "");
                assert_index(&doc, step);
            }
        }
    }
}

/// An engine with its text, driven the way a widget drives it: a fresh layout per command.
struct Ed<T> {
    edit: TextEdit,
    text: T,
    system: FakeText,
    keymap: Keymap,
    now: Duration,
}

impl<T: EditBuffer> Ed<T> {
    fn new(text: T, shown: &str, keymap: Keymap) -> Self {
        let mut ed = Self {
            edit: TextEdit::new(),
            text,
            system: FakeText,
            keymap,
            now: Duration::ZERO,
        };
        ed.edit.set_flags(EditFlags {
            multiline: true,
            ..EditFlags::default()
        });
        ed.sync(shown);
        ed
    }

    fn sync(&mut self, shown: &str) {
        let sp = spec(shown);
        self.edit
            .sync_text(&self.text, &mut SpecLayout::new(&mut self.system, &sp, 1.0));
    }

    fn cmd(&mut self, cmd: EditCommand, shown: &str) -> EditOutcome {
        let sp = spec(shown);
        let mut layout = SpecLayout::new(&mut self.system, &sp, 1.0);
        self.edit
            .apply(cmd, &mut self.text, &mut layout, self.keymap, self.now)
    }
}

fn doc_ed(text: &str, keymap: Keymap) -> Ed<CodeDoc> {
    Ed::new(CodeDoc::new(text), text, keymap)
}

impl Ed<CodeDoc> {
    fn run(&mut self, cmd: EditCommand) -> EditOutcome {
        let shown = self.text.text().to_owned();
        self.cmd(cmd, &shown)
    }

    fn at(mut self, caret: usize) -> Self {
        self.edit.set_selection(caret, caret);
        let shown = self.text.text().to_owned();
        self.sync(&shown);
        self
    }
}

/// No two lines in a row hold no word, where a word motion's reach is the same as a `String`'s.
fn single_blank_lines(text: &str) -> bool {
    let blank: Vec<bool> = text
        .split('\n')
        .map(|l| !l.chars().any(char::is_alphanumeric))
        .collect();
    !blank.windows(2).any(|w| w[0] && w[1])
}

fn script_command(rng: &mut Rng, text: &str) -> EditCommand {
    const TYPED: &[&str] = &["a", "b", "z", " ", "é", "😀", "e\u{301}", "\n"];
    const PASTED: &[&str] = &["x y", "one\ntwo", "a\r\nb\rc", "\nz\n", "😀 é"];
    const MOTIONS: &[Motion] = &[
        Motion::GraphemeLeft,
        Motion::GraphemeRight,
        Motion::LineStart,
        Motion::LineEnd,
        Motion::TextStart,
        Motion::TextEnd,
        Motion::LineUp,
        Motion::LineDown,
        Motion::PageUp,
        Motion::PageDown,
    ];
    let motion = MOTIONS[rng.below(MOTIONS.len())];
    let words = single_blank_lines(text);
    match rng.below(16) {
        0..=3 => ins(rng.pick(TYPED)),
        4 => EditCommand::Backspace { word: false },
        5 => EditCommand::Delete { word: false },
        6 => mv(motion),
        7 => ext(motion),
        8 if words => mv(if rng.below(2) == 0 {
            Motion::WordLeft
        } else {
            Motion::WordRight
        }),
        9 if words => ext(if rng.below(2) == 0 {
            Motion::WordLeft
        } else {
            Motion::WordRight
        }),
        10 if words => {
            if rng.below(2) == 0 {
                EditCommand::Backspace { word: true }
            } else {
                EditCommand::Delete { word: true }
            }
        }
        11 => EditCommand::Undo,
        12 => EditCommand::Redo,
        13 => match rng.below(3) {
            0 => EditCommand::Cut,
            1 => EditCommand::Copy,
            _ => EditCommand::PasteText(rng.pick(PASTED).to_owned()),
        },
        14 => EditCommand::SelectAll,
        15 => EditCommand::PlaceCaret((rng.below(120) as f32, rng.below(200) as f32)),
        _ => mv(motion),
    }
}

#[test]
fn one_key_script_gives_the_same_text_caret_and_anchor_over_a_string_and_a_document() {
    const START: &str = "fn main() {\n    let héllo = 1;\n\n    wörld(héllo);\n}\n\nend of it";
    for (seed, keymap) in [
        (0x2545_F491_4F6C_DD1D, Keymap::PC),
        (0x2545_F491_4F6C_DD1D, Keymap::MAC),
        (0xA076_1D64_78BD_642F, Keymap::PC),
        (0xE703_7ED1_A0B4_28DB, Keymap::MAC),
    ] {
        let mut rng = Rng(seed);
        let mut string = Ed::new(START.to_owned(), START, keymap);
        let mut doc = doc_ed(START, keymap);
        for step in 0..3_000 {
            let text = string.text.clone();
            let cmd = script_command(&mut rng, &text);
            let pause = [10, 100, 600][rng.below(3)];
            string.now += Duration::from_millis(pause);
            doc.now += Duration::from_millis(pause);
            let a = string.cmd(cmd.clone(), &text);
            let b = doc.cmd(cmd.clone(), &text);
            let at = format!("seed {seed:x}, step {step}, {cmd:?}");
            assert_eq!(a, b, "{at}: outcome");
            assert_eq!(string.text, doc.text.text(), "{at}: text");
            assert_eq!(string.edit.caret(), doc.edit.caret(), "{at}: caret");
            assert_eq!(string.edit.anchor(), doc.edit.anchor(), "{at}: anchor");
            assert_eq!(string.edit.undo_steps(), doc.edit.undo_steps(), "{at}");
            assert_eq!(string.edit.redo_steps(), doc.edit.redo_steps(), "{at}");
            if string.text.chars().count() > 400 {
                for ed_cmd in [EditCommand::SelectAll, EditCommand::Cut] {
                    let text = string.text.clone();
                    string.cmd(ed_cmd.clone(), &text);
                    doc.cmd(ed_cmd, &text);
                }
                let text = string.text.clone();
                string.cmd(ins("restart\n\nhere"), &text);
                doc.cmd(ins("restart\n\nhere"), &text);
            }
        }
        assert_index(&doc.text, 0);
    }
}

#[test]
fn a_word_motion_crosses_one_blank_line_as_a_string_does() {
    const TEXT: &str = "foo\n\nbar";
    let cases = [
        (Keymap::PC, Motion::WordRight, 3, 5),
        (Keymap::MAC, Motion::WordRight, 3, 8),
        (Keymap::PC, Motion::WordLeft, 5, 0),
        (Keymap::MAC, Motion::WordLeft, 5, 0),
    ];
    for (keymap, motion, from, to) in cases {
        let mut ed = doc_ed(TEXT, keymap).at(from);
        ed.run(mv(motion));
        assert_eq!(ed.edit.caret(), to, "{motion:?}");
        let mut string = Ed::new(TEXT.to_owned(), TEXT, keymap);
        string.edit.set_selection(from, from);
        string.cmd(mv(motion), TEXT);
        assert_eq!(string.edit.caret(), to, "{motion:?} on a String");
    }
}

#[test]
fn across_two_or_more_blank_lines_a_word_motion_stops_at_the_blank_line() {
    // Lines: foo, "", "", "", bar.
    const TEXT: &str = "foo\n\n\n\nbar";
    for keymap in [Keymap::PC, Keymap::MAC] {
        let mut ed = doc_ed(TEXT, keymap).at(3);
        ed.run(mv(Motion::WordRight));
        assert_eq!(ed.edit.caret(), 5, "the second blank line");
        ed.run(mv(Motion::WordRight));
        let bar = if keymap == Keymap::PC { 7 } else { 10 };
        assert_eq!(ed.edit.caret(), bar, "one blank line is left: it crosses");

        let mut ed = doc_ed(TEXT, keymap).at(7);
        ed.run(mv(Motion::WordLeft));
        assert_eq!(ed.edit.caret(), 6, "the blank line before bar");
        ed.run(mv(Motion::WordLeft));
        assert_eq!(ed.edit.caret(), 5);
        ed.run(mv(Motion::WordLeft));
        assert_eq!(ed.edit.caret(), 0, "foo is one blank line away");
    }
}

#[test]
fn a_word_motion_stops_at_a_blank_line_ended_by_any_terminator() {
    for text in ["foo\r\n\r\n\r\nbar", "foo\r\r\rbar", "foo\n\r\n\rbar"] {
        let start = CodeDoc::new(text).line_start(3);
        let mut ed = doc_ed(text, Keymap::PC).at(start);
        ed.run(mv(Motion::WordLeft));
        assert_eq!(
            ed.edit.caret(),
            ed.text.line_start(2),
            "{text:?}: the blank line before bar"
        );
    }
}

#[test]
fn a_word_motion_always_moves_until_the_end_of_the_text() {
    let text = "\n\n\n\n a \n\n\n\n\n;;\n\n\n\n";
    let end = text.chars().count();
    let mut ed = doc_ed(text, Keymap::MAC).at(0);
    for _ in 0..end {
        let at = ed.edit.caret();
        ed.run(mv(Motion::WordRight));
        assert!(ed.edit.caret() > at || at == end, "stuck at {at}");
    }
    assert_eq!(ed.edit.caret(), end);
    for _ in 0..end {
        let at = ed.edit.caret();
        ed.run(mv(Motion::WordLeft));
        assert!(ed.edit.caret() < at || at == 0, "stuck at {at}");
    }
    assert_eq!(ed.edit.caret(), 0);
}

#[test]
fn selecting_a_word_and_a_line_reads_around_the_offset() {
    let mut ed = doc_ed("one\ntwo three\nfour\nfive", Keymap::PC);
    ed.run(EditCommand::SelectWordAt(9));
    assert_eq!(ed.edit.selection(), 8..13);
    ed.run(EditCommand::SelectLineAt(9));
    assert_eq!(ed.edit.selection(), 4..13);
    ed.run(EditCommand::SelectAll);
    assert_eq!(ed.edit.selection(), 0..23);
    assert_eq!(
        ed.run(EditCommand::Copy).clipboard_write.as_deref(),
        Some("one\ntwo three\nfour\nfive")
    );
}

#[test]
fn caret_and_anchor_a_document_apart_each_snap_on_their_own_line() {
    let mut text = String::new();
    text.push_str("e\u{301}x\n");
    text.push_str(&"line\n".repeat(50));
    text.push_str("e\u{301}y");
    let last = text.chars().count() - 2;
    let mut ed = doc_ed(&text, Keymap::PC);
    ed.edit.set_selection(1, last);
    ed.sync(&text);
    assert_eq!(ed.edit.anchor(), 0, "down off the combining mark");
    assert_eq!(ed.edit.caret(), last - 1);
}

#[test]
fn undo_keeps_two_hundred_steps_on_a_document() {
    let mut ed = doc_ed("", Keymap::PC);
    for i in 0..UNDO_DEPTH + 50 {
        ed.now += COALESCE_PAUSE;
        ed.run(ins(if i % 2 == 0 { "a\n" } else { "b" }));
    }
    assert_eq!(ed.edit.undo_steps(), UNDO_DEPTH);
    for _ in 0..UNDO_DEPTH {
        assert!(ed.run(EditCommand::Undo).changed);
        assert_index(&ed.text, 0);
    }
    assert!(!ed.run(EditCommand::Undo).changed);
    assert_eq!(ed.text.char_len(), 75, "the 50 oldest steps stay");
    assert_eq!(ed.edit.redo_steps(), UNDO_DEPTH);
    for _ in 0..UNDO_DEPTH {
        ed.run(EditCommand::Redo);
    }
    assert_eq!(ed.text.char_len(), 375);
    assert_index(&ed.text, 1);
}

#[test]
fn typing_coalesces_and_the_pause_ends_a_step_on_a_document() {
    let mut ed = doc_ed("", Keymap::PC);
    for c in ["a", "b", "c"] {
        ed.now += Duration::from_millis(100);
        ed.run(ins(c));
    }
    assert_eq!(ed.edit.undo_steps(), 1);
    ed.now += COALESCE_PAUSE;
    ed.run(ins("d"));
    assert_eq!(ed.edit.undo_steps(), 2);
    ed.run(EditCommand::Undo);
    assert_eq!(ed.text.text(), "abc");
    ed.run(EditCommand::Undo);
    assert_eq!(ed.text.text(), "");
    assert_eq!(ed.edit.redo_steps(), 2);
}

#[test]
fn set_text_clears_both_logs_and_clamps() {
    let mut ed = doc_ed("", Keymap::PC);
    ed.run(ins("long text"));
    ed.now += COALESCE_PAUSE;
    ed.run(ins(" more"));
    ed.run(EditCommand::Undo);
    assert_eq!((ed.edit.undo_steps(), ed.edit.redo_steps()), (1, 1));
    ed.text.set_text("ab");
    ed.sync("ab");
    assert_eq!((ed.edit.undo_steps(), ed.edit.redo_steps()), (0, 0));
    assert_eq!(ed.edit.caret(), 2);
}

#[test]
fn swapping_one_document_for_another_clears_both_logs() {
    let mut ed = doc_ed("same", Keymap::PC);
    ed.run(mv(Motion::TextEnd));
    ed.run(ins("!"));
    ed.now += COALESCE_PAUSE;
    ed.run(ins("?"));
    ed.run(EditCommand::Undo);
    assert_eq!((ed.edit.undo_steps(), ed.edit.redo_steps()), (1, 1));
    // Equal text, another document.
    ed.text = CodeDoc::new("same!");
    ed.sync("same!");
    assert_eq!((ed.edit.undo_steps(), ed.edit.redo_steps()), (0, 0));
}

/// A document that counts the bytes the engine reads of it.
struct Counted {
    doc: CodeDoc,
    read: Cell<usize>,
}

impl Counted {
    fn take(&self) -> usize {
        self.read.replace(0)
    }
}

impl EditText for Counted {
    fn char_len(&self) -> usize {
        self.doc.char_len()
    }

    fn fingerprint(&self) -> u64 {
        self.doc.fingerprint()
    }

    fn window(&self, at: usize) -> (usize, &str) {
        let window = self.doc.window(at);
        self.read.set(self.read.get() + window.1.len());
        window
    }

    fn slice(&self, chars: Range<usize>) -> String {
        let slice = self.doc.slice(chars);
        self.read.set(self.read.get() + slice.len());
        slice
    }
}

impl EditBuffer for Counted {
    fn replace(&mut self, chars: Range<usize>, with: &str) {
        self.doc.replace(chars, with);
    }
}

/// A layout that shows nothing of the document but the lines near the caret, as the editor's does.
/// The text is ASCII, so bytes are chars. Its `lines` fails: the engine must ask for `lines_near`.
struct Near<'a> {
    doc: &'a CodeDoc,
    segmented: usize,
}

impl EditLayout for Near<'_> {
    fn boundaries(&mut self, text: &str, unit: Unit) -> Vec<usize> {
        self.segmented += text.len();
        FakeText.segments(text, unit)
    }

    fn caret(&mut self, offset: usize) -> CaretPos {
        let line = self.doc.line_of(offset);
        CaretPos {
            x: (offset - self.doc.line_start(line)) as f32 * 5.0,
            y: line as f32 * 20.0,
            height: 20.0,
            line,
        }
    }

    fn hit(&mut self, pos: (f32, f32)) -> usize {
        let line = (pos.1 / 20.0) as usize;
        let width = self.doc.line(line).len();
        self.doc.line_start(line) + width.min((pos.0 / 5.0) as usize)
    }

    fn lines(&mut self) -> Vec<LineSpan> {
        panic!("the engine read every line of the document");
    }

    fn lines_near(&mut self, at: usize, reach: isize) -> Vec<LineSpan> {
        let line = self.doc.line_of(at);
        let first = line.saturating_add_signed(reach.min(0)).saturating_sub(1);
        let last = line.saturating_add_signed(reach.max(0)) + 1;
        (first..=last.min(self.doc.line_count() - 1))
            .map(|i| LineSpan {
                start: self.doc.line_start(i),
                end: self.doc.line_start(i) + self.doc.line(i).len(),
                y: i as f32 * 20.0,
                height: 20.0,
            })
            .collect()
    }
}

/// The most lines a command reads, whatever the document's size: sync's windows of the caret and
/// the anchor and the command's two, of three lines each.
const READS: usize = 4 * 3;

/// Applies `cmd` to the document and answers the bytes it read through the text and the layout.
fn cost(text: &mut Counted, edit: &mut TextEdit, cmd: EditCommand) -> (usize, usize) {
    let shown = text.doc.clone();
    let mut layout = Near {
        doc: &shown,
        segmented: 0,
    };
    text.take();
    edit.apply(cmd, text, &mut layout, Keymap::PC, Duration::ZERO);
    (text.take(), layout.segmented)
}

fn counted(lines: usize) -> (Counted, TextEdit) {
    let text = Counted {
        doc: CodeDoc::new("some words on a line\n".repeat(lines)),
        read: Cell::new(0),
    };
    let mut edit = TextEdit::new();
    edit.set_flags(EditFlags {
        multiline: true,
        ..EditFlags::default()
    });
    (text, edit)
}

#[test]
fn a_settled_sync_reads_nothing() {
    let (text, mut edit) = counted(2_000);
    let shown = text.doc.clone();
    let mut layout = Near {
        doc: &shown,
        segmented: 0,
    };
    edit.set_selection(21_000, 21_000);
    edit.sync_text(&text, &mut layout);
    assert!(text.take() > 0, "the first sync snaps the caret");
    layout.segmented = 0;
    for _ in 0..3 {
        edit.sync_text(&text, &mut layout);
    }
    assert_eq!(text.take(), 0);
    assert_eq!(layout.segmented, 0);
}

#[test]
fn a_motion_reads_the_window_only() {
    let (mut text, mut edit) = counted(2_000);
    let line = "some words on a line\n".len();
    let doc = text.doc.text().len();
    edit.set_selection(1_000 * line, 1_000 * line);
    // A first motion, so the next ones start from a settled state and a real caret.
    cost(&mut text, &mut edit, mv(Motion::GraphemeRight));
    for motion in [
        Motion::GraphemeLeft,
        Motion::GraphemeRight,
        Motion::WordLeft,
        Motion::WordRight,
        Motion::LineStart,
        Motion::LineEnd,
        Motion::LineUp,
        Motion::LineDown,
        Motion::PageUp,
        Motion::PageDown,
    ] {
        let (read, segmented) = cost(&mut text, &mut edit, mv(motion));
        assert!(read <= READS * line, "{motion:?} read {read} of {doc}");
        assert!(segmented <= READS * line, "{motion:?} {segmented} of {doc}");
    }
    let (read, _) = cost(&mut text, &mut edit, mv(Motion::TextEnd));
    assert!(read <= READS * line, "TextEnd read {read} of {doc}");
}

#[test]
fn an_edit_reads_the_window_and_what_it_replaces() {
    let (mut text, mut edit) = counted(2_000);
    let line = "some words on a line\n".len();
    edit.set_selection(1_000 * line, 1_000 * line);
    let (read, _) = cost(&mut text, &mut edit, ins("x"));
    assert!(read <= READS * line, "read {read}");
    let (read, _) = cost(&mut text, &mut edit, EditCommand::Backspace { word: false });
    assert!(read <= READS * line, "read {read}");
    let (read, _) = cost(&mut text, &mut edit, EditCommand::Undo);
    assert!(read <= READS * line, "read {read}");
}

#[test]
fn grapheme_motion_crosses_a_line_start_at_the_second_and_the_millionth_line() {
    let mut text = Counted {
        doc: CodeDoc::new("x\n".repeat(1_000_000)),
        read: Cell::new(0),
    };
    let mut edit = TextEdit::new();
    for line in [1, 999_999] {
        let start = text.doc.line_start(line);
        edit.set_selection(start, start);
        let (read, _) = cost(&mut text, &mut edit, mv(Motion::GraphemeLeft));
        assert_eq!(edit.caret(), start - 1, "line {line}");
        assert!(read < 64, "read {read}");
        let (_, _) = cost(&mut text, &mut edit, mv(Motion::GraphemeRight));
        assert_eq!(edit.caret(), start, "line {line}");
    }
    assert_eq!(text.char_len(), 2_000_000);
}
