//! The document of the code editor: one `String` and a line index (D179). The app owns a
//! [`CodeDoc`] as it owns a `String` for a field and hands it to the engine, which reads and
//! changes it through [`EditText`] and [`EditBuffer`].
//!
//! The text is kept verbatim. `\n`, `\r\n` and a lone `\r` each end a line, and a line's terminator
//! belongs to the line it ends but is not in [`CodeDoc::line`]. Every offset is a char offset. The
//! index holds where each line starts, in bytes and in chars, so no call walks the document: an
//! offset costs a binary search and a walk inside one line, and an edit re-reads only what it
//! changed. An edit still moves the starts after it, plain arithmetic that measures 0.9 ms at
//! 1,000,000 lines (D179).
//!
//! Word motions land where a `String`'s do. [`EditText::window`] holds the lines either side of
//! the caret's; a word motion that finds no word there reads the window past its edge, and so on,
//! so it crosses any run of blank lines to the next word or the text's edge, reading as many
//! windows as the lines it crosses.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::text_edit::{EditBuffer, EditText};

/// Where a document's next revision comes from. It is process-wide so that two documents never
/// share a fingerprint: a [`TextEdit`](crate::text_edit::TextEdit) moved to another document sees
/// an external change. It is never drawn, so frames stay deterministic.
static REVISION: AtomicU64 = AtomicU64::new(0);

fn next_revision() -> u64 {
    REVISION.fetch_add(1, Ordering::Relaxed)
}

/// Text with a line index. See the [module docs](self).
#[derive(Debug, Clone)]
pub struct CodeDoc {
    text: String,
    /// Where each line starts, in bytes of `text`. Line 0 starts at 0, so there is always one.
    bytes: Vec<usize>,
    /// Where each line starts, in chars.
    chars: Vec<usize>,
    char_len: usize,
    /// How many lines have each width in chars, so the widest is known as an edit shrinks it.
    widths: BTreeMap<usize, usize>,
    revision: u64,
    /// The lines the last [`EditBuffer::replace`] changed, kept until the next change.
    edit: Option<LineEdit>,
}

/// The lines one [`EditBuffer::replace`] changed: from line `at`, `removed` lines gave way to
/// `added`. The line the edit starts in keeps its number, so `at` is the line after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LineEdit {
    /// The revision the edit was made on.
    since: u64,
    /// The revision it made.
    made: u64,
    pub(crate) at: usize,
    pub(crate) removed: usize,
    pub(crate) added: usize,
}

impl Default for CodeDoc {
    fn default() -> Self {
        Self::new("")
    }
}

/// Whether a line starts at byte `at` of `text`, a char boundary.
fn line_starts_at(text: &[u8], at: usize) -> bool {
    match at.checked_sub(1).map(|before| text[before]) {
        None | Some(b'\n') => true,
        Some(b'\r') => text.get(at) != Some(&b'\n'),
        Some(_) => false,
    }
}

impl CodeDoc {
    /// A document holding `text`, line terminators as given.
    pub fn new(text: impl Into<String>) -> Self {
        let mut doc = Self {
            text: text.into(),
            bytes: Vec::new(),
            chars: Vec::new(),
            char_len: 0,
            widths: BTreeMap::new(),
            revision: 0,
            edit: None,
        };
        doc.reindex();
        doc
    }

    /// The whole text, terminators as they were given.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the whole text. It is an external change: an engine on this document clears its
    /// undo and redo logs and clamps its caret at the next sync.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.reindex();
    }

    /// How many lines the text holds: at least 1, and a final terminator opens one more.
    pub fn line_count(&self) -> usize {
        self.bytes.len()
    }

    /// Line `i` without its terminator; past the end, `""`.
    pub fn line(&self, i: usize) -> &str {
        let Some(from) = self.bytes.get(i) else {
            return "";
        };
        let line = &self.text[*from..self.line_end(i)];
        line.strip_suffix("\r\n")
            .or_else(|| line.strip_suffix(['\n', '\r']))
            .unwrap_or(line)
    }

    /// The line that holds char `at`: a terminator's chars are on the line they end, and past the
    /// end is the last line.
    pub fn line_of(&self, at: usize) -> usize {
        self.chars.partition_point(|c| *c <= at).saturating_sub(1)
    }

    /// The char offset line `i` starts at; past the end, the char count.
    pub fn line_start(&self, i: usize) -> usize {
        self.chars.get(i).copied().unwrap_or(self.char_len)
    }

    /// The width in chars of the widest line, terminators not counted.
    pub fn widest_chars(&self) -> usize {
        self.widths.last_key_value().map_or(0, |w| *w.0)
    }

    /// The lines changed since revision `since` (a [`EditText::fingerprint`]), when one
    /// [`EditBuffer::replace`] made every change since: `None` after any other change, or none.
    pub(crate) fn line_edit(&self, since: u64) -> Option<LineEdit> {
        self.edit
            .filter(|e| e.since == since && e.made == self.revision)
    }

    /// The byte where line `i` ends, past its terminator.
    fn line_end(&self, i: usize) -> usize {
        self.bytes.get(i + 1).copied().unwrap_or(self.text.len())
    }

    /// The width in chars of line `i`, which exists. A terminator is ASCII, so its bytes are its
    /// chars, and only the line's end is read.
    fn width(&self, i: usize) -> usize {
        let chars = self.chars.get(i + 1).copied().unwrap_or(self.char_len) - self.chars[i];
        let tail = &self.text.as_bytes()[self.bytes[i]..self.line_end(i)];
        let terminator = match tail {
            [.., b'\r', b'\n'] => 2,
            [.., b'\n' | b'\r'] => 1,
            _ => 0,
        };
        chars - terminator
    }

    fn count_width(&mut self, i: usize) {
        *self.widths.entry(self.width(i)).or_default() += 1;
    }

    fn forget_width(&mut self, i: usize) {
        let width = self.width(i);
        if let Some(count) = self.widths.get_mut(&width) {
            *count -= 1;
            if *count == 0 {
                self.widths.remove(&width);
            }
        }
    }

    /// Builds the index from the text, as a change of the whole text.
    fn reindex(&mut self) {
        self.bytes = vec![0];
        self.chars = vec![0];
        self.char_len = 0;
        self.widths.clear();
        let text = self.text.as_bytes();
        for (at, c) in self.text.char_indices() {
            self.char_len += 1;
            let next = at + c.len_utf8();
            if line_starts_at(text, next) {
                self.bytes.push(next);
                self.chars.push(self.char_len);
            }
        }
        for i in 0..self.bytes.len() {
            self.count_width(i);
        }
        self.revision = next_revision();
    }

    /// The byte offset of char `at`; past the end gives the text's length.
    fn byte_of(&self, at: usize) -> usize {
        if at >= self.char_len {
            return self.text.len();
        }
        let line = self.line_of(at);
        let from = self.bytes[line];
        self.text[from..]
            .char_indices()
            .nth(at - self.chars[line])
            .map_or(self.text.len(), |c| from + c.0)
    }
}

impl EditText for CodeDoc {
    fn char_len(&self) -> usize {
        self.char_len
    }

    fn fingerprint(&self) -> u64 {
        self.revision
    }

    /// From the start of the line before `at`'s to the end of the line after it, terminators
    /// included. See the [module docs](self) for what that does to a word motion.
    fn window(&self, at: usize) -> (usize, &str) {
        let line = self.line_of(at);
        let first = line.saturating_sub(1);
        let (from, to) = (self.bytes[first], self.line_end(line + 1));
        (self.chars[first], &self.text[from..to])
    }

    fn slice(&self, chars: Range<usize>) -> String {
        let from = self.byte_of(chars.start);
        self.text[from..self.byte_of(chars.end).max(from)].to_owned()
    }
}

impl EditBuffer for CodeDoc {
    /// Whether a line starts at a byte depends on that byte and the one before it, so only the
    /// starts from the edit's first byte to the byte after it can change. That covers a `\r\n`
    /// the edit makes or breaks at either end; the starts outside only move.
    fn replace(&mut self, chars: Range<usize>, with: &str) {
        let end = chars.end.min(self.char_len);
        let start = chars.start.min(end);
        let (from, to) = (self.byte_of(start), self.byte_of(end));
        if from == to && with.is_empty() {
            return;
        }
        // Lines `kept..past` start inside the edit; the line before them runs into it.
        let kept = self.bytes.partition_point(|b| *b < from);
        let past = self.bytes.partition_point(|b| *b <= to);
        let changed = kept.saturating_sub(1);
        for i in changed..past {
            self.forget_width(i);
        }
        self.text.replace_range(from..to, with);

        let text = self.text.as_bytes();
        let (mut bytes, mut starts) = (Vec::new(), Vec::new());
        if line_starts_at(text, from) {
            bytes.push(from);
            starts.push(start);
        }
        let mut inserted = 0;
        for (at, c) in with.char_indices() {
            inserted += 1;
            let next = from + at + c.len_utf8();
            if line_starts_at(text, next) {
                bytes.push(next);
                starts.push(start + inserted);
            }
        }
        let added = bytes.len();
        self.bytes.splice(kept..past, bytes);
        self.chars.splice(kept..past, starts);
        let moved = kept + added;
        for b in &mut self.bytes[moved..] {
            *b = *b - (to - from) + with.len();
        }
        for c in &mut self.chars[moved..] {
            *c = *c - (end - start) + inserted;
        }
        self.char_len = self.char_len - (end - start) + inserted;
        for i in changed..moved {
            self.count_width(i);
        }
        let since = self.revision;
        self.revision = next_revision();
        self.edit = Some(LineEdit {
            since,
            made: self.revision,
            at: changed + 1,
            removed: past - changed - 1,
            added: moved - changed - 1,
        });
    }
}
