//! The text editing engine: caret, selection, navigation, undo and redo, clipboard commands and
//! the preedit an input method composes. It is pure: the app owns the text and passes it to
//! every call, the layout answers the geometry through [`EditLayout`], and time arrives as an
//! argument. A widget maps [`FocusEvent`]s with [`EditCommand::from_event`] and calls
//! [`TextEdit::apply`].
//!
//! The text is read through [`EditText`] and changed through [`EditBuffer`]: a `String` as it
//! always was, a [`CodeDoc`](crate::code_doc::CodeDoc) at a cost that does not follow the
//! document's size. Every rule here serves both (D180).
//!
//! Every offset in this module is a char offset; the `TextSystem` queries use bytes of the spec's
//! text, and [`SpecLayout`] turns one into the other at that seam, with the two private helpers
//! `byte_of` and `char_of` that also index the string when an edit changes it.
//!
//! Offsets are total: any caret, anchor or query offset past the end or inside a grapheme is
//! clamped, never a panic.

use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::time::Duration;

use crate::input::{
    FocusEvent, ImeEvent, Key, Keymap, Modifiers, NamedKey, Primary, WordModifier, WordMotion,
    printable,
};
use crate::motion::Time;
use crate::text::{CaretPos, LineSpan, Spec, TextSystem, Unit, clamp_to_boundary, joined_text};

/// How many undo steps a [`TextEdit`] keeps.
pub const UNDO_DEPTH: usize = 200;

/// The most bytes a widget that shapes its whole text takes in one insert (a paste, an assistive
/// `SetValue`, an input method's commit); a longer one is refused whole, never cut. The text field
/// and the text area build one spec of all their text, so what they take costs in proportion: a
/// field about 25 ms a frame and 290 MB at 1 MiB, an area about 0.4 s to first draw and 430 MB (and
/// at the clipboard adapter's 32 MiB cap 10 to 16 s and 10 to 14 GB, extrapolated; measured and
/// recorded in D232 and D233). They set it through [`EditFlags::max_insert`]; the code editor draws
/// only the lines in view and leaves it off, so it takes what the adapter lets through. An app's own
/// `max_chars` still applies below it.
pub const MAX_SHAPED_INSERT: usize = 1_048_576;

/// The pause that ends a run of typed characters as one undo step.
pub const COALESCE_PAUSE: Duration = Duration::from_millis(500);

/// How many lines a page motion crosses when the layout does not say.
pub const DEFAULT_PAGE_ROWS: usize = 10;

/// The byte offset of char `offset` of `text`; past the end gives `text.len()`.
fn byte_of(text: &str, offset: usize) -> usize {
    text.char_indices().nth(offset).map_or(text.len(), |c| c.0)
}

/// The char offset of byte `offset` of `text`, clamped into it and down to a char's start.
fn char_of(text: &str, offset: usize) -> usize {
    text[..clamp_to_boundary(text, offset)].chars().count()
}

/// [`char_of`] for sorted byte offsets, in one pass over `text`.
fn chars_of_sorted(text: &str, bytes: &[usize]) -> Vec<usize> {
    let (mut at, mut count) = (0, 0);
    bytes
        .iter()
        .map(|b| {
            let to = clamp_to_boundary(text, *b).max(at);
            count += text[at..to].chars().count();
            at = to;
            count
        })
        .collect()
}

/// What the engine asks of the text's layout. Offsets are chars. `caret`, `hit` and `lines` read
/// the laid-out text, which the caller keeps equal to the text it passes to the engine;
/// `boundaries` reads the `text` it is given, so it also serves text not yet in the layout.
pub trait EditLayout {
    /// The sorted boundaries of `unit` in `text`, including 0 and the char count.
    fn boundaries(&mut self, text: &str, unit: Unit) -> Vec<usize>;

    /// Where the caret sits at `offset`.
    fn caret(&mut self, offset: usize) -> CaretPos;

    /// The offset nearest to `pos`, in logical px from the text's top-left corner.
    fn hit(&mut self, pos: (f32, f32)) -> usize;

    /// The visual lines, top to bottom, with `start` and `end` in chars.
    fn lines(&mut self) -> Vec<LineSpan>;

    /// The visual lines around `at`, top to bottom, as [`Self::lines`] gives them: at least the
    /// line that shows `at`, the lines from it to `reach` lines away (negative is up), and one
    /// more on each side where there is one. A layout of a long document answers only these.
    fn lines_near(&mut self, _at: usize, _reach: isize) -> Vec<LineSpan> {
        self.lines()
    }

    /// How many lines a page motion crosses.
    fn page_rows(&self) -> usize {
        DEFAULT_PAGE_ROWS
    }
}

/// What the engine reads of a text. Offsets are chars, and all of them are total: past the end
/// clamps. A reader of a long document answers each call without walking the document.
pub trait EditText {
    /// How many chars the text holds.
    fn char_len(&self) -> usize;

    /// A value that changes whenever the text does, which is how the engine sees an external
    /// change.
    fn fingerprint(&self) -> u64;

    /// A slice around char `at` and the char offset it starts at; it holds `at`, and it starts
    /// and ends where text segmentation does not reach across (the whole text, or whole lines).
    /// Grapheme motions and snapping read this window of the caret. A word motion reads it too,
    /// so a text that answers less than the whole of itself bounds the motion: see
    /// [`CodeDoc`](crate::code_doc::CodeDoc).
    fn window(&self, at: usize) -> (usize, &str);

    /// The text of chars `chars`.
    fn slice(&self, chars: Range<usize>) -> String;
}

/// What the engine changes of a text.
pub trait EditBuffer: EditText {
    /// Puts `with` in place of chars `chars`; offsets clamp as [`EditText`]'s do.
    fn replace(&mut self, chars: Range<usize>, with: &str);
}

impl EditText for str {
    fn char_len(&self) -> usize {
        self.chars().count()
    }

    fn fingerprint(&self) -> u64 {
        hash_of(self)
    }

    fn window(&self, _at: usize) -> (usize, &str) {
        (0, self)
    }

    fn slice(&self, chars: Range<usize>) -> String {
        let from = byte_of(self, chars.start);
        self[from..byte_of(self, chars.end).max(from)].to_owned()
    }
}

impl EditText for String {
    fn char_len(&self) -> usize {
        self.as_str().char_len()
    }

    fn fingerprint(&self) -> u64 {
        self.as_str().fingerprint()
    }

    fn window(&self, at: usize) -> (usize, &str) {
        self.as_str().window(at)
    }

    fn slice(&self, chars: Range<usize>) -> String {
        self.as_str().slice(chars)
    }
}

impl EditBuffer for String {
    fn replace(&mut self, chars: Range<usize>, with: &str) {
        let from = byte_of(self, chars.start);
        let to = byte_of(self, chars.end).max(from);
        self.replace_range(from..to, with);
    }
}

/// [`EditLayout`] over a [`TextSystem`]: the one place char offsets meet the seam's byte offsets.
/// `spec` holds the text being edited; build a new one after an edit changes it.
pub struct SpecLayout<'a> {
    system: &'a mut dyn TextSystem,
    spec: &'a Spec,
    scale: f32,
    joined: String,
    page_rows: usize,
}

impl<'a> SpecLayout<'a> {
    pub fn new(system: &'a mut dyn TextSystem, spec: &'a Spec, scale: f32) -> Self {
        Self {
            system,
            spec,
            scale,
            joined: joined_text(spec),
            page_rows: DEFAULT_PAGE_ROWS,
        }
    }

    /// Sets how many lines a page motion crosses: what a viewport shows.
    pub fn with_page_rows(mut self, rows: usize) -> Self {
        self.page_rows = rows;
        self
    }
}

impl std::fmt::Debug for SpecLayout<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpecLayout")
            .field("scale", &self.scale)
            .field("page_rows", &self.page_rows)
            .finish_non_exhaustive()
    }
}

impl EditLayout for SpecLayout<'_> {
    fn boundaries(&mut self, text: &str, unit: Unit) -> Vec<usize> {
        chars_of_sorted(text, &self.system.segments(text, unit))
    }

    fn caret(&mut self, offset: usize) -> CaretPos {
        let at = byte_of(&self.joined, offset);
        self.system.caret(self.spec, self.scale, at)
    }

    fn hit(&mut self, pos: (f32, f32)) -> usize {
        char_of(&self.joined, self.system.hit(self.spec, self.scale, pos))
    }

    fn lines(&mut self) -> Vec<LineSpan> {
        let mut lines = self.system.lines(self.spec, self.scale);
        let ends: Vec<usize> = lines.iter().flat_map(|l| [l.start, l.end]).collect();
        let chars = chars_of_sorted(&self.joined, &ends);
        for (line, range) in lines.iter_mut().zip(chars.chunks_exact(2)) {
            line.start = range[0];
            line.end = range[1];
        }
        lines
    }

    fn page_rows(&self) -> usize {
        self.page_rows
    }
}

/// A caret motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    GraphemeLeft,
    GraphemeRight,
    /// To the start of the previous word.
    WordLeft,
    /// To the end of the word, or the start of the next one, as the keymap's [`WordMotion`] says.
    WordRight,
    LineStart,
    LineEnd,
    TextStart,
    TextEnd,
    /// One visual line up, keeping the preferred x.
    LineUp,
    LineDown,
    /// [`EditLayout::page_rows`] visual lines up, keeping the preferred x.
    PageUp,
    PageDown,
}

/// One thing [`TextEdit::apply`] can do. Offsets are chars; points are logical px from the text's
/// top-left corner.
#[derive(Debug, Clone, PartialEq)]
pub enum EditCommand {
    /// Moves the caret; `extend` keeps the anchor, so the selection grows.
    Move {
        motion: Motion,
        extend: bool,
    },
    SelectAll,
    /// Selects the word segment at the offset.
    SelectWordAt(usize),
    /// Selects the visual line at the offset.
    SelectLineAt(usize),
    /// Typed text, replacing the selection.
    Insert(String),
    /// Removes the selection, else the grapheme or word before the caret.
    Backspace {
        word: bool,
    },
    /// Removes the selection, else the grapheme or word after the caret.
    Delete {
        word: bool,
    },
    Undo,
    Redo,
    /// Writes the selection to the clipboard and removes it.
    Cut,
    /// Writes the selection to the clipboard.
    Copy,
    /// Asks for the clipboard's text; the answer comes back as [`EditCommand::PasteText`].
    Paste,
    PasteText(String),
    /// Puts the caret at the point.
    PlaceCaret((f32, f32)),
    /// Moves the caret to the point, keeping the anchor.
    ExtendTo((f32, f32)),
    /// One line: reports `submitted`. Many lines: inserts a line break.
    Enter,
    /// Composing text; `cursor` is byte offsets of `text`, as the platform gives them. An empty
    /// `text` clears the preedit.
    ImePreedit {
        text: String,
        cursor: Option<(usize, usize)>,
    },
    /// The composed text, replacing the selection, as one undo step.
    ImeCommit(String),
    /// Drops the preedit: `ImeEvent::Disabled`, or the widget lost focus.
    ImeCancel,
}

/// How a field restricts its engine; [`TextEdit::set_flags`] sets them. [`EditCommand::from_event`]
/// takes them as [`TextEdit::flags`] gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EditFlags {
    /// The caret and selection work, edits do not.
    pub read_only: bool,
    /// Copy and cut do nothing, and no undo step is kept.
    pub password: bool,
    /// Enter inserts a line break, and vertical motion works.
    pub multiline: bool,
    /// The most chars an edit may leave in the text.
    pub max_chars: Option<usize>,
    /// The most bytes one insert may bring, else it is refused whole: set by a widget that shapes
    /// its whole text ([`MAX_SHAPED_INSERT`]), left `None` by one that draws only the lines in view.
    pub max_insert: Option<usize>,
    /// A preedit exists, so keys belong to the input method. Set by [`TextEdit::flags`];
    /// [`TextEdit::set_flags`] ignores it.
    pub composing: bool,
}

/// What one command did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditOutcome {
    /// The text changed.
    pub changed: bool,
    /// Enter was pressed in a one-line field.
    pub submitted: bool,
    /// Text for the clipboard.
    pub clipboard_write: Option<String>,
    /// The widget should ask for the clipboard's text.
    pub clipboard_read: bool,
    /// The engine took the command; otherwise the event is free for a shortcut binding.
    pub consumed: bool,
}

impl EditCommand {
    /// The command an event asks for, if any. `flags` comes from [`TextEdit::flags`]. Keys map per
    /// the catalogue's `text.text-edit`; while `flags.composing` keys and typed text yield nothing,
    /// because they belong to the input method. Escape and Tab map to nothing.
    pub fn from_event(event: &FocusEvent, keymap: Keymap, flags: EditFlags) -> Option<EditCommand> {
        let edits = !flags.read_only;
        match event {
            FocusEvent::Key { key, modifiers, .. } if !flags.composing => {
                Self::from_key(key, *modifiers, keymap, flags)
            }
            FocusEvent::Key { .. } => None,
            FocusEvent::Text(t) => (edits && !flags.composing).then(|| Self::Insert(t.clone())),
            FocusEvent::ClipboardText(t) => edits.then(|| Self::PasteText(t.clone())),
            FocusEvent::ClipboardUnavailable => None,
            FocusEvent::Ime(ImeEvent::Enabled) => None,
            FocusEvent::Ime(ImeEvent::Preedit { text, cursor }) => {
                edits.then(|| Self::ImePreedit {
                    text: text.clone(),
                    cursor: *cursor,
                })
            }
            FocusEvent::Ime(ImeEvent::Commit(t)) => edits.then(|| Self::ImeCommit(t.clone())),
            FocusEvent::Ime(ImeEvent::Disabled) => Some(Self::ImeCancel),
        }
    }

    fn from_key(key: &Key, m: Modifiers, keymap: Keymap, flags: EditFlags) -> Option<EditCommand> {
        let plain = !m.control && !m.alt && !m.logo;
        let primary = !m.alt
            && match keymap.primary {
                Primary::Logo => m.logo && !m.control,
                Primary::Control => m.control && !m.logo,
            };
        let word = !m.logo
            && match keymap.word {
                WordModifier::Alt => m.alt && !m.control,
                WordModifier::Control => m.control && !m.alt,
            };
        let edits = !flags.read_only;
        let moving = |motion| {
            Some(Self::Move {
                motion,
                extend: m.shift,
            })
        };
        let vertical = |motion| {
            if plain && flags.multiline {
                moving(motion)
            } else {
                None
            }
        };
        match key {
            Key::Character(c) if primary => match (c.to_ascii_lowercase().as_str(), m.shift) {
                ("a", false) => Some(Self::SelectAll),
                ("c", false) => Some(Self::Copy),
                ("z", false) if edits => Some(Self::Undo),
                ("z", true) | ("y", false) if edits => Some(Self::Redo),
                ("x", false) if edits => Some(Self::Cut),
                ("v", false) if edits => Some(Self::Paste),
                _ => None,
            },
            Key::Named(named) => match named {
                NamedKey::ArrowLeft if plain => moving(Motion::GraphemeLeft),
                NamedKey::ArrowRight if plain => moving(Motion::GraphemeRight),
                NamedKey::ArrowLeft if word => moving(Motion::WordLeft),
                NamedKey::ArrowRight if word => moving(Motion::WordRight),
                NamedKey::ArrowUp => vertical(Motion::LineUp),
                NamedKey::ArrowDown => vertical(Motion::LineDown),
                NamedKey::PageUp => vertical(Motion::PageUp),
                NamedKey::PageDown => vertical(Motion::PageDown),
                NamedKey::Home if plain => moving(Motion::LineStart),
                NamedKey::End if plain => moving(Motion::LineEnd),
                NamedKey::Home if primary => moving(Motion::TextStart),
                NamedKey::End if primary => moving(Motion::TextEnd),
                NamedKey::Backspace if edits && (plain || word) => Some(Self::Backspace { word }),
                NamedKey::Delete if edits && (plain || word) => Some(Self::Delete { word }),
                NamedKey::Enter if plain && (edits || !flags.multiline) => Some(Self::Enter),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether this command still runs while a preedit exists. A clipboard answer is not a key:
    /// it was asked for before the composition began.
    fn runs_while_composing(&self) -> bool {
        matches!(
            self,
            Self::ImePreedit { .. } | Self::ImeCommit(_) | Self::ImeCancel | Self::PasteText(_)
        )
    }
}

/// One undoable edit: chars `at..` held `removed` before it and hold `inserted` after it.
struct Step {
    at: usize,
    removed: String,
    inserted: String,
    /// Caret and anchor before and after.
    before: (usize, usize),
    after: (usize, usize),
    /// When the step last grew, for the coalescing pause.
    last_at: Time,
}

/// The composing text, kept beside the buffer until a commit.
struct Preedit {
    text: String,
    /// The selected clause, in chars of `text`.
    cursor: Option<(usize, usize)>,
}

/// The edit state of one text: caret and anchor as char offsets, the undo and redo logs, the
/// preferred x of vertical motion, the preedit, and the fingerprint of the last text it saw. The
/// text stays with the app.
#[derive(Default)]
pub struct TextEdit {
    caret: usize,
    anchor: usize,
    undo: VecDeque<Step>,
    redo: Vec<Step>,
    preferred_x: Option<f32>,
    preedit: Option<Preedit>,
    text_hash: Option<u64>,
    flags: EditFlags,
    /// The last undo step is a run of typed chars that the next typed char may join.
    typing: bool,
    /// Caret and anchor sit on boundaries of the hashed text, so `sync` has nothing to clamp.
    snapped: bool,
}

/// Offsets and counts only: the logs and the preedit hold text, and this type edits passwords.
impl std::fmt::Debug for TextEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextEdit")
            .field("caret", &self.caret)
            .field("anchor", &self.anchor)
            .field("undo", &self.undo.len())
            .field("redo", &self.redo.len())
            .field("preferred_x", &self.preferred_x)
            .field(
                "preedit_chars",
                &self.preedit.as_ref().map(|p| p.text.chars().count()),
            )
            .field("preedit_cursor", &self.preedit_cursor())
            .finish_non_exhaustive()
    }
}

fn hash_of(text: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

/// The last of the sorted `bounds` at or before `at`.
fn snap(bounds: &[usize], at: usize) -> usize {
    let i = bounds.partition_point(|b| *b <= at);
    i.checked_sub(1)
        .and_then(|i| bounds.get(i))
        .map_or(0, |b| *b)
}

/// The last of the sorted `bounds` before `at`.
fn before(bounds: &[usize], at: usize) -> usize {
    snap(bounds, at.saturating_sub(1))
}

/// The first of the sorted `bounds` after `at`, or `at` when there is none.
fn after(bounds: &[usize], at: usize) -> usize {
    let i = bounds.partition_point(|b| *b <= at);
    bounds.get(i).map_or(at, |b| *b)
}

/// The boundaries of `unit` in a [`EditText::window`], as offsets of the whole text.
fn window_bounds(window: (usize, &str), unit: Unit, layout: &mut dyn EditLayout) -> Vec<usize> {
    let (base, text) = window;
    let mut bounds = layout.boundaries(text, unit);
    if base != 0 {
        for b in &mut bounds {
            *b += base;
        }
    }
    bounds
}

/// The boundaries of `unit` around `at`.
fn bounds_near<T: EditText + ?Sized>(
    text: &T,
    at: usize,
    unit: Unit,
    layout: &mut dyn EditLayout,
) -> Vec<usize> {
    window_bounds(text.window(at), unit, layout)
}

/// The char ranges of the word segments around `at` that hold a word (an alphanumeric char), and
/// the window they were read from.
fn words<'a, T: EditText + ?Sized>(
    text: &'a T,
    at: usize,
    layout: &mut dyn EditLayout,
) -> (Vec<(usize, usize)>, (usize, &'a str)) {
    let window = text.window(at);
    let bounds = window_bounds(window, Unit::Word, layout);
    let mut chars = window.1.chars();
    let spans = bounds
        .windows(2)
        .filter(|w| {
            let mut word = false;
            for c in chars.by_ref().take(w[1].saturating_sub(w[0])) {
                word |= c.is_alphanumeric();
            }
            word
        })
        .map(|w| (w[0], w[1]))
        .collect();
    (spans, window)
}

/// Whether the line whose terminator ends at char `edge` is blank.
fn blank_line_before<T: EditText + ?Sized>(text: &T, edge: usize) -> bool {
    let tail = text.slice(edge.saturating_sub(3)..edge);
    let body = tail
        .strip_suffix("\r\n")
        .or_else(|| tail.strip_suffix(['\n', '\r']))
        .unwrap_or(&tail);
    body.is_empty() || body.ends_with(['\n', '\r'])
}

/// Whether the line that starts at char `edge` is blank.
fn blank_line_at<T: EditText + ?Sized>(text: &T, edge: usize) -> bool {
    let next = text.slice(edge..edge + 1);
    next.is_empty() || next.starts_with(['\n', '\r'])
}

/// The word motions read [`EditText::window`]. Finding no word in it, they stop at its edge, unless
/// the line past the edge holds text: a single blank line does not stop them, so they read one
/// window more and stop at that one's edge. A text whose window is all of it never gets that far.
fn word_left<T: EditText + ?Sized>(text: &T, at: usize, layout: &mut dyn EditLayout) -> usize {
    let found = |spans: &[(usize, usize)]| spans.iter().rev().find(|w| w.0 < at).map(|w| w.0);
    let (spans, (base, _)) = words(text, at, layout);
    if let Some(word) = found(&spans) {
        return word;
    }
    if base == 0 || blank_line_before(text, base) {
        return base;
    }
    let (spans, (base, _)) = words(text, base - 1, layout);
    found(&spans).unwrap_or(base)
}

/// See [`word_left`].
fn word_right<T: EditText + ?Sized>(
    text: &T,
    at: usize,
    keymap: Keymap,
    layout: &mut dyn EditLayout,
) -> usize {
    let found = |spans: &[(usize, usize)]| match keymap.word_motion {
        WordMotion::EndOfWord => spans.iter().find(|w| w.1 > at).map(|w| w.1),
        WordMotion::StartOfNext => spans.iter().find(|w| w.0 > at).map(|w| w.0),
    };
    let (spans, (base, window)) = words(text, at, layout);
    if let Some(word) = found(&spans) {
        return word;
    }
    let end = base + window.chars().count();
    if end == text.char_len() || blank_line_at(text, end) {
        return end;
    }
    let (spans, (base, window)) = words(text, end, layout);
    found(&spans).unwrap_or_else(|| base + window.chars().count())
}

/// The index of the visual line that shows `at`: at a soft wrap, the later one.
fn row_of(rows: &[LineSpan], at: usize) -> usize {
    rows.iter().rposition(|r| r.start <= at).unwrap_or(0)
}

/// The last offset the caret can take on row `i`. Where a wrapped row runs straight into the next
/// one, its end offset shows on the next row, so the last offset is the grapheme before it.
fn row_end(rows: &[LineSpan], i: usize, bounds: &[usize]) -> usize {
    let row = rows[i];
    let wrapped = row.end > row.start && rows.get(i + 1).is_some_and(|n| n.start == row.end);
    if wrapped {
        before(bounds, row.end).max(row.start)
    } else {
        row.end
    }
}

/// `text` as an insertion: in one line, line breaks become a space and control chars drop; in many
/// lines every line break becomes `\n` (a lone `\r` too) and other chars stay.
fn clean(text: &str, multiline: bool) -> String {
    let unified = str::replace(&text.replace("\r\n", "\n"), '\r', "\n");
    if multiline {
        return unified;
    }
    let spaced = str::replace(
        &unified,
        ['\n', '\u{b}', '\u{c}', '\u{85}', '\u{2028}', '\u{2029}'],
        " ",
    );
    printable(&spaced).unwrap_or_default()
}

impl TextEdit {
    pub fn new() -> Self {
        Self::default()
    }

    /// The caret: the moving end of the selection, in chars.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The fixed end of the selection, in chars.
    pub fn anchor(&self) -> usize {
        self.anchor
    }

    /// The selection, low end first; empty when there is none.
    pub fn selection(&self) -> Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    /// Sets the selection. The next [`Self::sync`] clamps it into the text.
    pub fn set_selection(&mut self, anchor: usize, caret: usize) {
        self.anchor = anchor;
        self.caret = caret;
        self.preferred_x = None;
        self.typing = false;
        self.snapped = false;
    }

    /// The composing text, if any.
    pub fn preedit(&self) -> Option<&str> {
        self.preedit.as_ref().map(|p| p.text.as_str())
    }

    /// The selected clause of the preedit, in chars of the preedit.
    pub fn preedit_cursor(&self) -> Option<(usize, usize)> {
        self.preedit.as_ref().and_then(|p| p.cursor)
    }

    /// How many steps undo can take.
    pub fn undo_steps(&self) -> usize {
        self.undo.len()
    }

    /// How many steps redo can take.
    pub fn redo_steps(&self) -> usize {
        self.redo.len()
    }

    /// The flags, with `composing` telling whether a preedit exists.
    pub fn flags(&self) -> EditFlags {
        EditFlags {
            composing: self.preedit.is_some(),
            ..self.flags
        }
    }

    /// Sets the flags [`Self::apply`] works under. A password clears both logs at once; read-only
    /// drops the preedit.
    pub fn set_flags(&mut self, flags: EditFlags) {
        self.flags = EditFlags {
            composing: false,
            ..flags
        };
        if flags.password {
            self.undo.clear();
            self.redo.clear();
            self.typing = false;
        }
        if flags.read_only {
            self.preedit = None;
        }
    }

    /// Takes the app's text for this frame. A text that differs from the last result is an
    /// external change: both logs go, since they only replay against the text they were made on.
    /// Caret and anchor clamp into the text, down onto a grapheme boundary.
    pub fn sync(&mut self, text: &str, layout: &mut dyn EditLayout) {
        self.sync_text(text, layout);
    }

    /// [`Self::sync`] for any [`EditText`]. A settled text costs one [`EditText::fingerprint`].
    pub fn sync_text<T: EditText + ?Sized>(&mut self, text: &T, layout: &mut dyn EditLayout) {
        let print = text.fingerprint();
        if self.text_hash != Some(print) {
            self.text_hash = Some(print);
            self.undo.clear();
            self.redo.clear();
            self.typing = false;
            self.preferred_x = None;
        } else if self.snapped {
            return;
        }
        // They may sit a document apart; in one window, one read serves both.
        let (near, far) = (text.window(self.caret), text.window(self.anchor));
        let bounds = window_bounds(near, Unit::Grapheme, layout);
        let caret = snap(&bounds, self.caret);
        let anchor = if (near.0, near.1.len()) == (far.0, far.1.len()) {
            snap(&bounds, self.anchor)
        } else {
            snap(&window_bounds(far, Unit::Grapheme, layout), self.anchor)
        };
        if (caret, anchor) != (self.caret, self.anchor) {
            self.caret = caret;
            self.anchor = anchor;
            self.typing = false;
            self.preferred_x = None;
        }
        self.snapped = true;
    }

    /// Whether [`Self::apply`] would consume `cmd` from this state, without running it: exactly
    /// `apply(cmd).consumed`, for a state that is synced (a caret and anchor on boundaries, as the
    /// widgets keep it). A widget that must take a key and act on none (a disabled scope, D140)
    /// asks this instead of applying.
    pub fn takes(&self, cmd: &EditCommand) -> bool {
        if self.preedit.is_some() && !cmd.runs_while_composing() {
            return false;
        }
        let edits = !self.flags.read_only;
        match cmd {
            EditCommand::Move { .. }
            | EditCommand::SelectAll
            | EditCommand::SelectWordAt(_)
            | EditCommand::SelectLineAt(_)
            | EditCommand::PlaceCaret(_)
            | EditCommand::ExtendTo(_)
            | EditCommand::ImeCancel => true,
            EditCommand::Copy => !self.selection().is_empty(),
            EditCommand::Cut => edits && !self.selection().is_empty(),
            EditCommand::Enter => !self.flags.multiline || edits,
            EditCommand::Insert(_)
            | EditCommand::PasteText(_)
            | EditCommand::Backspace { .. }
            | EditCommand::Delete { .. }
            | EditCommand::Undo
            | EditCommand::Redo
            | EditCommand::Paste
            | EditCommand::ImePreedit { .. }
            | EditCommand::ImeCommit(_) => edits,
        }
    }

    /// Runs one command on `text`. It syncs first, so a text changed behind the engine is an
    /// external change here too. While a preedit exists only IME commands and a clipboard answer
    /// run; anything else does nothing and is not consumed.
    pub fn apply<T: EditBuffer + ?Sized>(
        &mut self,
        cmd: EditCommand,
        text: &mut T,
        layout: &mut dyn EditLayout,
        keymap: Keymap,
        now: Time,
    ) -> EditOutcome {
        self.sync_text(&*text, layout);
        if self.preedit.is_some() && !cmd.runs_while_composing() {
            return EditOutcome::default();
        }
        if !matches!(cmd, EditCommand::Insert(_)) {
            self.typing = false;
        }
        let edits = !self.flags.read_only;
        let mut out = EditOutcome {
            consumed: true,
            ..EditOutcome::default()
        };
        match cmd {
            EditCommand::Move { motion, extend } => {
                let at = self.motion_target(motion, &*text, layout, keymap, extend);
                self.place(at, extend);
            }
            EditCommand::SelectAll => {
                self.anchor = 0;
                self.caret = text.char_len();
                self.preferred_x = None;
            }
            EditCommand::SelectWordAt(at) => self.select_word(at, &*text, layout),
            EditCommand::SelectLineAt(at) => {
                let rows = layout.lines_near(at, 0);
                if !rows.is_empty() {
                    let row = rows[row_of(&rows, at)];
                    self.anchor = row.start;
                    self.caret = row.end;
                    self.preferred_x = None;
                }
            }
            EditCommand::Insert(s) if edits => {
                out.changed = self.insert(&s, text, layout, true, now);
            }
            EditCommand::PasteText(s) if edits => {
                out.changed = self.insert(&s, text, layout, false, now);
            }
            EditCommand::Backspace { word } if edits => {
                let sel = self.selection();
                let from = match (sel.is_empty(), word) {
                    (false, _) => sel.start,
                    (true, true) => word_left(&*text, self.caret, layout),
                    (true, false) => before(
                        &bounds_near(&*text, self.caret, Unit::Grapheme, layout),
                        self.caret,
                    ),
                };
                out.changed = self.replace(text, from, sel.end, "", false, now);
            }
            EditCommand::Delete { word } if edits => {
                let sel = self.selection();
                let to = match (sel.is_empty(), word) {
                    (false, _) => sel.end,
                    (true, true) => word_right(&*text, self.caret, keymap, layout),
                    (true, false) => after(
                        &bounds_near(&*text, self.caret, Unit::Grapheme, layout),
                        self.caret,
                    ),
                };
                out.changed = self.replace(text, sel.start, to, "", false, now);
            }
            EditCommand::Undo if edits => out.changed = self.undo(text),
            EditCommand::Redo if edits => out.changed = self.redo(text),
            EditCommand::Copy | EditCommand::Cut if self.selection().is_empty() => {
                out.consumed = false;
            }
            EditCommand::Copy => out.clipboard_write = self.selected_text(&*text),
            EditCommand::Cut if edits => {
                out.clipboard_write = self.selected_text(&*text);
                if out.clipboard_write.is_some() {
                    let sel = self.selection();
                    out.changed = self.replace(text, sel.start, sel.end, "", false, now);
                }
            }
            EditCommand::Paste if edits => out.clipboard_read = true,
            EditCommand::PlaceCaret(pos) => {
                let at = self.hit(pos, &*text, layout);
                self.place(at, false);
            }
            EditCommand::ExtendTo(pos) => {
                let at = self.hit(pos, &*text, layout);
                self.place(at, true);
            }
            EditCommand::Enter if self.flags.multiline => {
                if edits {
                    out.changed = self.insert("\n", text, layout, false, now);
                } else {
                    out.consumed = false;
                }
            }
            EditCommand::Enter => out.submitted = true,
            EditCommand::ImePreedit { text: p, cursor } if edits => self.set_preedit(p, cursor),
            EditCommand::ImeCommit(s) if edits => {
                self.preedit = None;
                out.changed = self.insert(&s, text, layout, false, now);
            }
            EditCommand::ImeCancel => self.preedit = None,
            // An edit in a read-only text: the key stays free for a binding.
            _ => out.consumed = false,
        }
        self.snapped = false;
        out
    }

    /// The string to draw and the preedit's range in it, in chars. It is `text` with the preedit
    /// in place of the selection, which is what a commit would leave; the selection itself is
    /// untouched and comes back if the composition is cancelled.
    pub fn display(&self, text: &str) -> (String, Option<Range<usize>>) {
        let Some(preedit) = &self.preedit else {
            return (text.to_owned(), None);
        };
        let sel = self.selection();
        let (from, to) = (byte_of(text, sel.start), byte_of(text, sel.end));
        let start = char_of(text, from);
        let shown = format!("{}{}{}", &text[..from], preedit.text, &text[to..]);
        let range = start..start + preedit.text.chars().count();
        (shown, Some(range))
    }

    fn selected_text<T: EditText + ?Sized>(&self, text: &T) -> Option<String> {
        if self.flags.password {
            return None;
        }
        Some(text.slice(self.selection()))
    }

    /// Puts the caret at `at`; without `extend` the anchor follows. Keeps the preferred x: a
    /// motion that ends it has cleared it already.
    fn place(&mut self, at: usize, extend: bool) {
        self.caret = at;
        if !extend {
            self.anchor = at;
        }
    }

    fn hit<T: EditText + ?Sized>(
        &mut self,
        pos: (f32, f32),
        text: &T,
        layout: &mut dyn EditLayout,
    ) -> usize {
        self.preferred_x = None;
        let at = layout.hit(pos);
        snap(&bounds_near(text, at, Unit::Grapheme, layout), at)
    }

    fn select_word<T: EditText + ?Sized>(
        &mut self,
        at: usize,
        text: &T,
        layout: &mut dyn EditLayout,
    ) {
        let bounds = bounds_near(text, at, Unit::Word, layout);
        let i = bounds.partition_point(|b| *b <= at);
        let (from, to) = match (i.checked_sub(1).and_then(|i| bounds.get(i)), bounds.get(i)) {
            (Some(from), Some(to)) => (*from, *to),
            // At the end of the text: the last segment.
            _ if bounds.len() >= 2 => (bounds[bounds.len() - 2], bounds[bounds.len() - 1]),
            _ => return,
        };
        self.anchor = from;
        self.caret = to;
        self.preferred_x = None;
    }

    fn motion_target<T: EditText + ?Sized>(
        &mut self,
        motion: Motion,
        text: &T,
        layout: &mut dyn EditLayout,
        keymap: Keymap,
        extend: bool,
    ) -> usize {
        let vertical = match motion {
            Motion::LineUp => Some(-1),
            Motion::LineDown => Some(1),
            Motion::PageUp => Some(-(layout.page_rows().max(1) as isize)),
            Motion::PageDown => Some(layout.page_rows().max(1) as isize),
            _ => None,
        };
        if let Some(rows) = vertical {
            return self.vertical_target(rows, text, layout);
        }
        self.preferred_x = None;
        let head = self.caret;
        let bounds = bounds_near(text, head, Unit::Grapheme, layout);
        let sel = self.selection();
        match motion {
            Motion::GraphemeLeft if !extend && !sel.is_empty() => sel.start,
            Motion::GraphemeRight if !extend && !sel.is_empty() => sel.end,
            Motion::GraphemeLeft => before(&bounds, head),
            Motion::GraphemeRight => after(&bounds, head),
            Motion::WordLeft => word_left(text, head, layout),
            Motion::WordRight => word_right(text, head, keymap, layout),
            Motion::LineStart | Motion::LineEnd => {
                let rows = layout.lines_near(head, 0);
                if rows.is_empty() {
                    return head;
                }
                let i = row_of(&rows, head);
                if motion == Motion::LineStart {
                    rows[i].start
                } else {
                    row_end(&rows, i, &bounds)
                }
            }
            Motion::TextStart => 0,
            Motion::TextEnd => text.char_len(),
            Motion::LineUp | Motion::LineDown | Motion::PageUp | Motion::PageDown => head,
        }
    }

    /// The offset `delta` visual lines from the caret, at the preferred x; past the first or last
    /// line it stops there, and with no line to go to the caret stays.
    fn vertical_target<T: EditText + ?Sized>(
        &mut self,
        delta: isize,
        text: &T,
        layout: &mut dyn EditLayout,
    ) -> usize {
        let rows = layout.lines_near(self.caret, delta);
        let head = self.caret;
        if rows.is_empty() {
            return head;
        }
        let x = *self.preferred_x.get_or_insert_with(|| layout.caret(head).x);
        let from = row_of(&rows, head);
        let to = from.saturating_add_signed(delta).min(rows.len() - 1);
        if to == from {
            return head;
        }
        let row = rows[to];
        let at = layout.hit((x, row.y + row.height / 2.0));
        let bounds = bounds_near(text, row.start, Unit::Grapheme, layout);
        snap(&bounds, at.min(row_end(&rows, to, &bounds)).max(row.start))
    }

    /// Replaces the selection with `insert`, cleaned and cut to fit; `typing` lets it join the
    /// typed run before it. Empty text after cleaning changes nothing, selection included. An
    /// insert over [`EditFlags::max_insert`] bytes is refused whole.
    fn insert<T: EditBuffer + ?Sized>(
        &mut self,
        insert: &str,
        text: &mut T,
        layout: &mut dyn EditLayout,
        typing: bool,
        now: Time,
    ) -> bool {
        if self.flags.max_insert.is_some_and(|max| insert.len() > max) {
            return false;
        }
        let sel = self.selection();
        let mut insert = clean(insert, self.flags.multiline);
        if let Some(max) = self.flags.max_chars {
            let kept = text.char_len().saturating_sub(sel.len());
            let room = max.saturating_sub(kept);
            if insert.chars().count() > room {
                let cut = snap(&layout.boundaries(&insert, Unit::Grapheme), room);
                insert.truncate(byte_of(&insert, cut));
            }
        }
        if insert.is_empty() {
            return false;
        }
        self.replace(text, sel.start, sel.end, &insert, typing, now)
    }

    /// The one place an edit changes `text`: chars `start..end` become `insert`, the caret lands
    /// after it, and the undo log learns of it. A password keeps no step.
    fn replace<T: EditBuffer + ?Sized>(
        &mut self,
        text: &mut T,
        start: usize,
        end: usize,
        insert: &str,
        typing: bool,
        now: Time,
    ) -> bool {
        if start == end && insert.is_empty() {
            return false;
        }
        let removed = text.slice(start..end);
        text.replace(start..end, insert);
        let before = (self.caret, self.anchor);
        let landed = start + insert.chars().count();
        self.caret = landed;
        self.anchor = landed;
        self.preferred_x = None;
        self.redo.clear();
        // `self.typing` holds only while the caret stayed where the last typed char left it.
        let joins = typing && self.typing && removed.is_empty();
        let top = self
            .undo
            .back_mut()
            .filter(|top| joins && now.saturating_sub(top.last_at) < COALESCE_PAUSE);
        if self.flags.password {
            self.undo.clear();
        } else if let Some(top) = top {
            top.inserted.push_str(insert);
            top.after = (landed, landed);
            top.last_at = now;
        } else {
            self.undo.push_back(Step {
                at: start,
                removed,
                inserted: insert.to_owned(),
                before,
                after: (landed, landed),
                last_at: now,
            });
            if self.undo.len() > UNDO_DEPTH {
                self.undo.pop_front();
            }
        }
        self.typing = typing;
        self.text_hash = Some(text.fingerprint());
        true
    }

    fn undo<T: EditBuffer + ?Sized>(&mut self, text: &mut T) -> bool {
        let Some(step) = self.undo.pop_back() else {
            return false;
        };
        Self::swap(text, step.at, &step.inserted, &step.removed);
        (self.caret, self.anchor) = step.before;
        self.preferred_x = None;
        self.text_hash = Some(text.fingerprint());
        self.redo.push(step);
        true
    }

    fn redo<T: EditBuffer + ?Sized>(&mut self, text: &mut T) -> bool {
        let Some(step) = self.redo.pop() else {
            return false;
        };
        Self::swap(text, step.at, &step.removed, &step.inserted);
        (self.caret, self.anchor) = step.after;
        self.preferred_x = None;
        self.text_hash = Some(text.fingerprint());
        self.undo.push_back(step);
        true
    }

    /// Puts `to` in place of `from`, which sits at char `at`.
    fn swap<T: EditBuffer + ?Sized>(text: &mut T, at: usize, from: &str, to: &str) {
        text.replace(at..at + from.chars().count(), to);
    }

    fn set_preedit(&mut self, text: String, cursor: Option<(usize, usize)>) {
        if text.is_empty() {
            self.preedit = None;
            return;
        }
        let cursor = cursor.map(|(a, b)| {
            let (a, b) = (char_of(&text, a), char_of(&text, b));
            (a.min(b), a.max(b))
        });
        self.preedit = Some(Preedit { text, cursor });
    }
}
