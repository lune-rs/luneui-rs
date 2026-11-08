//! The string table: the app's and the toolkit's translated strings, looked up by key with
//! `{n}` and `{n:spec}` placeholders.
//!
//! Keys the toolkit draws itself live in [`DEFAULTS`] under the `luneui` namespace; a table may
//! translate exactly those, and no others (an unknown toolkit key is rejected at insert, so a typo
//! cannot pass silently). Lookup never fails: table value, else the default, else the key.
//!
//! [`format_template`] fills placeholders left to right. The toolkit ships no number or date
//! locale data: `{n:spec}` hands the argument and the spec to the app's [`ArgFormat`] hook.
//!
//! A [`Catalog`] is a language file (JSON, read by the bounded reader in [`crate::json`]) that
//! holds a `StringTable`: the app overlays it on its base table ([`StringTable::overlay`]), so a key
//! a translation omits falls back. The templates are data from a file: [`format_template`] returns
//! for any string and any arguments, in time linear in the template, and its output is at most the
//! template's length plus what the hook writes for each placeholder the template names.
//! [`audit_template`] counts a template's placeholders with the same scanner.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Write as _};

use crate::json::{self, JsonError, Value};

/// The namespace of the toolkit's own keys (a key is in it when it starts with this, then a dot).
const NAMESPACE: &str = "luneui";

/// The toolkit's own strings, key and English text. A table may translate these and add its own
/// keys outside the toolkit namespace.
pub const DEFAULTS: &[(&str, &str)] = &[
    ("luneui.dialog.cancel", "Cancel"),
    ("luneui.window.minimize", "Minimize"),
    ("luneui.window.maximize", "Maximize"),
    ("luneui.window.restore", "Restore"),
    ("luneui.window.close", "Close"),
    ("luneui.text-field.clear", "Clear"),
    ("luneui.tab-strip.new", "New tab"),
    ("luneui.tab-strip.close", "Close tab"),
    ("luneui.tab-strip.menu.close", "Close"),
    ("luneui.tab-strip.menu.close-others", "Close Others"),
    ("luneui.tab-strip.menu.close-all", "Close All"),
    ("luneui.tab-strip.menu.rename", "Rename\u{2026}"),
    ("luneui.verdict-row.ok", "OK"),
    ("luneui.verdict-row.warn", "WARN"),
    ("luneui.verdict-row.no", "NO"),
    ("luneui.stat.no-data", "No data"),
    ("luneui.menu.name", "Menu"),
    ("luneui.toast-notifications.clear-all", "Clear all"),
    ("luneui.toast-notifications.history-header", "NOTIFICATIONS"),
    ("luneui.toast-notifications.dismiss", "Dismiss"),
    ("luneui.command-palette.show-more", "Show more results"),
    (
        "luneui.command-palette.footer-right",
        "Enter open    Esc close",
    ),
    (
        "luneui.command-palette.destructive-tooltip",
        "Destructive action \u{2014} requires confirmation",
    ),
    (
        "luneui.settings-page.restart-message",
        "Some changes require a restart to take effect.",
    ),
    ("luneui.settings-page.restart-action", "Restart now"),
    ("luneui.bar-chart.name", "Bar chart"),
    ("luneui.pie-chart.name", "Pie chart"),
    ("luneui.kanban-board.add-column", "Add column"),
    ("luneui.kanban-board.add-card", "Add card"),
    ("luneui.kanban-board.column-actions", "Column actions"),
    ("luneui.kanban-board.empty-column", "No cards yet."),
    ("luneui.code-block.copy", "Copy code"),
    ("luneui.code-block.copied", "Copied"),
    ("luneui.kv-list.copy", "Copy {0}"),
    ("luneui.kv-list.copied", "Copied"),
    (
        "luneui.code-editor.truncated",
        "File truncated \u{2014} showing partial content",
    ),
    (
        "luneui.code-editor.highlight-disabled",
        "Syntax highlighting disabled for large files",
    ),
    ("luneui.code-editor.read-only", "Read-only"),
    (
        "luneui.markdown.image-remote-blocked",
        "Remote image not loaded",
    ),
    (
        "luneui.markdown.mermaid-failed",
        "Diagram could not be rendered \u{2014} showing source",
    ),
    (
        "luneui.markdown.math-failed",
        "Formula could not be rendered \u{2014} showing source",
    ),
    ("luneui.diff-view.unified", "Unified diff"),
    ("luneui.diff-view.side-by-side", "Side-by-side diff"),
    ("luneui.diff-view.options", "Diff options"),
    ("luneui.diff-view.word-wrap", "Word wrap"),
    (
        "luneui.diff-view.copy-selected-lines",
        "Copy selected lines",
    ),
    ("luneui.diff-view.pane-old", "old"),
    ("luneui.diff-view.pane-new", "new"),
    ("luneui.diff-view.row-added", "Added"),
    ("luneui.diff-view.row-removed", "Removed"),
    ("luneui.diff-view.row-context", "Unchanged"),
];

/// The default text of a toolkit key, if it is one.
pub fn default_text(key: &str) -> Option<&'static str> {
    DEFAULTS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

fn in_toolkit_namespace(key: &str) -> bool {
    key.strip_prefix(NAMESPACE)
        .is_some_and(|rest| rest.starts_with('.'))
}

/// Why an insert was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringError {
    /// The key is empty.
    EmptyKey,
    /// The key is in the toolkit namespace but is not one of [`DEFAULTS`].
    UnknownToolkitKey(String),
}

impl Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringError::EmptyKey => f.write_str("empty string key"),
            StringError::UnknownToolkitKey(k) => write!(f, "unknown toolkit string key `{k}`"),
        }
    }
}

impl std::error::Error for StringError {}

/// Formats one argument for a `{n}` or `{n:spec}` placeholder (the app's number and date rules).
pub trait ArgFormat {
    /// Appends the text of `arg` to `out`. `index` is the placeholder's argument index and `spec`
    /// the text after the colon (empty for `{n}`).
    fn format(&self, index: usize, arg: &dyn Display, spec: &str, out: &mut String);
}

/// The default hook: the argument's `Display`, the spec ignored.
#[derive(Clone, Copy, Debug, Default)]
pub struct DisplayFormat;

impl ArgFormat for DisplayFormat {
    fn format(&self, _index: usize, arg: &dyn Display, _spec: &str, out: &mut String) {
        // Writing to a String cannot fail unless the argument's Display errors; then keep what it wrote.
        let _ = write!(out, "{arg}");
    }
}

/// One piece of a template, as the scanner reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Piece<'t> {
    /// Text with no brace in it.
    Text(&'t str),
    /// `{{`: a literal `{`.
    OpenEscape,
    /// `}}`: a literal `}`.
    CloseEscape,
    /// `{n}` or `{n:spec}`; `raw` is all of it, `colon` says whether it had a spec part.
    Field {
        index: usize,
        spec: &'t str,
        raw: &'t str,
        colon: bool,
    },
    /// A `{` that starts no placeholder (no digits, an index past `usize`, no closing brace, any
    /// other character after the digits): kept literally.
    StrayOpen,
    /// A `}` that closes nothing: kept literally.
    StrayClose,
}

/// Splits a template into [`Piece`]s, left to right. [`format_template`] and [`audit_template`]
/// both read the template through it, so they cannot disagree about what a placeholder is. Linear
/// in the template: a search for a closing brace that finds none is not repeated, and a run of
/// digits is read once.
struct Scanner<'t> {
    text: &'t str,
    pos: usize,
    /// A search for `}` ran to the end and found none: no later placeholder can close.
    no_more_closers: bool,
}

impl<'t> Scanner<'t> {
    fn new(text: &'t str) -> Self {
        Self {
            text,
            pos: 0,
            no_more_closers: false,
        }
    }

    /// The placeholder starting at the `{` at `self.pos`, or `None` when it is none.
    fn field(&mut self) -> Option<Piece<'t>> {
        let start = self.pos;
        let bytes = self.text.as_bytes();
        let mut j = start + 1;
        let mut index: Option<usize> = Some(0);
        while let Some(d) = bytes.get(j).filter(|b| b.is_ascii_digit()) {
            index = index
                .and_then(|i| i.checked_mul(10))
                .and_then(|i| i.checked_add(usize::from(d - b'0')));
            j += 1;
        }
        if j == start + 1 {
            return None;
        }
        // An index past `usize` is read to its end (the run is skipped once) but is no placeholder.
        let index = index?;
        let (spec, end, colon) = match bytes.get(j)? {
            b'}' => ("", j + 1, false),
            b':' if !self.no_more_closers => {
                let from = j + 1;
                let Some(len) = self.text.get(from..)?.find('}') else {
                    self.no_more_closers = true;
                    return None;
                };
                (self.text.get(from..from + len)?, from + len + 1, true)
            }
            _ => return None,
        };
        Some(Piece::Field {
            index,
            spec,
            raw: self.text.get(start..end)?,
            colon,
        })
    }
}

impl<'t> Iterator for Scanner<'t> {
    type Item = Piece<'t>;

    fn next(&mut self) -> Option<Piece<'t>> {
        let rest = self.text.get(self.pos..).filter(|r| !r.is_empty())?;
        let bytes = rest.as_bytes();
        let piece = match bytes.first()? {
            b'{' if bytes.get(1) == Some(&b'{') => {
                self.pos += 2;
                Piece::OpenEscape
            }
            b'{' => match self.field() {
                Some(field) => {
                    if let Piece::Field { raw, .. } = field {
                        self.pos += raw.len();
                    }
                    field
                }
                None => {
                    self.pos += 1;
                    Piece::StrayOpen
                }
            },
            b'}' if bytes.get(1) == Some(&b'}') => {
                self.pos += 2;
                Piece::CloseEscape
            }
            b'}' => {
                self.pos += 1;
                Piece::StrayClose
            }
            _ => {
                // Both braces are ASCII, so the run ends on a character boundary.
                let len = rest.find(['{', '}']).unwrap_or(rest.len());
                self.pos += len;
                Piece::Text(rest.get(..len)?)
            }
        };
        Some(piece)
    }
}

/// Fills the placeholders of `template` from `args`, left to right.
///
/// `{{` and `}}` are literal braces; `{n}` and `{n:spec}` (n in ASCII digits, within `usize`) go
/// through `f`; a placeholder whose argument is missing stays as written; extra arguments are
/// ignored. Any other `{` and a lone `}` stay literal. Never panics, whatever the template, and
/// takes time linear in it; the output is the template's text plus what `f` writes for each
/// placeholder that has an argument.
pub fn format_template(template: &str, args: &[&dyn Display], f: &dyn ArgFormat) -> String {
    let mut out = String::with_capacity(template.len());
    for piece in Scanner::new(template) {
        match piece {
            Piece::Text(t) => out.push_str(t),
            Piece::OpenEscape | Piece::StrayOpen => out.push('{'),
            Piece::CloseEscape | Piece::StrayClose => out.push('}'),
            Piece::Field {
                index, spec, raw, ..
            } => match args.get(index) {
                Some(arg) => f.format(index, *arg, spec, &mut out),
                None => out.push_str(raw),
            },
        }
    }
    out
}

/// What [`audit_template`] counted in one template.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TemplateAudit {
    /// `{n}` placeholders.
    pub plain: usize,
    /// `{n:spec}` placeholders (an empty spec counts: the colon is there).
    pub spec: usize,
    /// Stray braces: a `{` that starts no placeholder (no digits, an index past `usize`, no
    /// closing brace) and a `}` that closes nothing. `{{` and `}}` are escapes, not these.
    pub malformed: usize,
}

/// Counts the placeholders of `template` as [`format_template`] reads them: the same scanner.
pub fn audit_template(template: &str) -> TemplateAudit {
    let mut audit = TemplateAudit::default();
    for piece in Scanner::new(template) {
        match piece {
            Piece::Field { colon: false, .. } => audit.plain += 1,
            Piece::Field { colon: true, .. } => audit.spec += 1,
            Piece::StrayOpen | Piece::StrayClose => audit.malformed += 1,
            Piece::Text(_) | Piece::OpenEscape | Piece::CloseEscape => {}
        }
    }
    audit
}

/// Translated strings by key. `Default` holds no entries, so every lookup falls back to the
/// toolkit default or the key itself.
#[derive(Clone, Debug, Default)]
pub struct StringTable {
    entries: BTreeMap<String, String>,
}

impl StringTable {
    /// Sets the text of `key`, replacing an earlier one. Refuses an empty key and a toolkit-namespace
    /// key that is not in [`DEFAULTS`].
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), StringError> {
        let key = key.into();
        if key.is_empty() {
            return Err(StringError::EmptyKey);
        }
        if in_toolkit_namespace(&key) && default_text(&key).is_none() {
            return Err(StringError::UnknownToolkitKey(key));
        }
        self.entries.insert(key, value.into());
        Ok(())
    }

    /// Copies every entry of `other` into this table; where both have a key, `other`'s wins. The
    /// entries were checked when they were inserted, so none is refused here.
    pub fn overlay(&mut self, other: &StringTable) {
        self.entries
            .extend(other.entries.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    /// The table's own entries, `(key, text)`, in key order. Defaults are not listed.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// The table's own text for `key`, if it has one.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// The text for `key`: the table's, else the toolkit default, else the key itself.
    pub fn tr<'s>(&'s self, key: &'s str) -> &'s str {
        self.get(key).or_else(|| default_text(key)).unwrap_or(key)
    }

    /// [`Self::tr`] with the placeholders filled from `args` through `f`.
    pub fn tr_fmt(&self, key: &str, args: &[&dyn Display], f: &dyn ArgFormat) -> String {
        format_template(self.tr(key), args, f)
    }
}

/// Why a language file did not load. A file that fails loads nothing: the app keeps its table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// The bytes are not JSON within the reader's limits.
    Json(JsonError),
    /// The top-level value is not an object.
    NotAnObject,
    /// A required field is absent.
    Missing(&'static str),
    /// A field has the wrong type (`strings` not a map of strings included).
    WrongType(&'static str),
    /// `code` or `displayName` is empty or only whitespace.
    Empty(&'static str),
    /// A key of `strings` was refused (D6: an empty key, or a `luneui.` key that is not a default).
    Key(StringError),
}

/// A language file: `{ "code", "displayName", "version", "strings": { key: text } }`. The strings
/// are a [`StringTable`] like any other; load it, then overlay it on the app's base table.
#[derive(Debug, Clone)]
pub struct Catalog {
    pub code: String,
    pub display_name: String,
    /// A monotonic integer an app uses to refresh bundled files; 0 when absent. The loader does
    /// not read it further.
    pub version: i64,
    table: StringTable,
}

impl Catalog {
    /// Reads a language file through the bounded JSON reader. `code`, `displayName` (neither empty
    /// nor blank) and `strings` (an object of strings) are required; `version` is an exact integer
    /// or absent. Unknown top-level keys are ignored. Every key goes through
    /// [`StringTable::insert`], and its refusal fails the whole load (D6): a typo in a `luneui.`
    /// key does not ship. Nothing here reads a file or panics.
    pub fn from_json(bytes: &[u8]) -> Result<Catalog, CatalogError> {
        let root = json::parse(bytes).map_err(CatalogError::Json)?;
        if !matches!(root, Value::Object(_)) {
            return Err(CatalogError::NotAnObject);
        }
        let required = |key: &'static str| root.get(key).map_err(|_| CatalogError::Missing(key));
        let name = |key: &'static str| -> Result<String, CatalogError> {
            let text = required(key)?
                .as_str()
                .map_err(|_| CatalogError::WrongType(key))?;
            if text.trim().is_empty() {
                return Err(CatalogError::Empty(key));
            }
            Ok(text.to_string())
        };
        let code = name("code")?;
        let display_name = name("displayName")?;
        let version = match root.get("version") {
            Ok(v) => v.as_int().map_err(|_| CatalogError::WrongType("version"))?,
            Err(_) => 0,
        };
        let strings = required("strings")?
            .string_map()
            .map_err(|_| CatalogError::WrongType("strings"))?;
        let mut table = StringTable::default();
        for (key, text) in strings {
            table.insert(key, text).map_err(CatalogError::Key)?;
        }
        Ok(Catalog {
            code,
            display_name,
            version,
            table,
        })
    }

    /// The catalog's strings.
    pub fn table(&self) -> &StringTable {
        &self.table
    }

    /// The catalog's strings, by value.
    pub fn into_table(self) -> StringTable {
        self.table
    }
}

#[cfg(test)]
mod tests {
    use super::{Piece, Scanner};

    fn pieces(t: &str) -> Vec<Piece<'_>> {
        Scanner::new(t).collect()
    }

    #[test]
    fn the_scanner_splits_a_template_into_pieces() {
        assert_eq!(
            pieces("a{{b}}{0}{1:x}{c}"),
            [
                Piece::Text("a"),
                Piece::OpenEscape,
                Piece::Text("b"),
                Piece::CloseEscape,
                Piece::Field {
                    index: 0,
                    spec: "",
                    raw: "{0}",
                    colon: false
                },
                Piece::Field {
                    index: 1,
                    spec: "x",
                    raw: "{1:x}",
                    colon: true
                },
                Piece::StrayOpen,
                Piece::Text("c"),
                Piece::StrayClose,
            ]
        );
        assert!(pieces("").is_empty());
    }

    #[test]
    fn pieces_cover_the_template_exactly() {
        // Every piece consumes at least one byte and the scanner ends at the end of the text.
        for t in [
            "{{{",
            "}}}",
            "{0:",
            "{99999999999999999999}",
            "\u{e9}{\u{e9}}",
            "{0:{}}",
        ] {
            let mut s = Scanner::new(t);
            let mut last = 0;
            while s.next().is_some() {
                assert!(s.pos > last, "{t:?} stalled at {last}");
                last = s.pos;
            }
            assert_eq!(s.pos, t.len(), "{t:?}");
        }
    }
}
