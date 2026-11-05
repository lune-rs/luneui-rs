//! The string table: the app's and the toolkit's translated strings, looked up by key with
//! `{n}` and `{n:spec}` placeholders.
//!
//! Keys the toolkit draws itself live in [`DEFAULTS`] under the `luneui` namespace; a table may
//! translate exactly those, and no others (an unknown toolkit key is rejected at insert, so a typo
//! cannot pass silently). Lookup never fails: table value, else the default, else the key.
//!
//! [`format_template`] fills placeholders left to right. The toolkit ships no number or date
//! locale data: `{n:spec}` hands the argument and the spec to the app's [`ArgFormat`] hook.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Write as _};

/// The namespace of the toolkit's own keys (a key is in it when it starts with this, then a dot).
const NAMESPACE: &str = "luneui";

/// The toolkit's own strings, key and English text. A table may translate these and add its own
/// keys outside the toolkit namespace.
pub const DEFAULTS: &[(&str, &str)] = &[
    ("luneui.dialog.cancel", "Cancel"),
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

/// Fills the placeholders of `template` from `args`, left to right.
///
/// `{{` and `}}` are literal braces; `{n}` and `{n:spec}` (n in ASCII digits) go through `f`; a
/// placeholder whose argument is missing stays as written; extra arguments are ignored. Any other
/// `{` and a lone `}` stay literal. Never panics.
pub fn format_template(template: &str, args: &[&dyn Display], f: &dyn ArgFormat) -> String {
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' => {
                if chars.get(i + 1) == Some(&'{') {
                    out.push('{');
                    i += 2;
                } else if let Some((index, spec, end)) = placeholder(&chars, i) {
                    match args.get(index) {
                        Some(arg) => f.format(index, *arg, &spec, &mut out),
                        None => out.extend(&chars[i..end]),
                    }
                    i = end;
                } else {
                    out.push('{');
                    i += 1;
                }
            }
            '}' => {
                out.push('}');
                i += if chars.get(i + 1) == Some(&'}') { 2 } else { 1 };
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Parses `{n}` or `{n:spec}` at `chars[start]` (a `{`): the index, the spec and the position
/// after the closing brace. `None` for anything else (no digits, an overflowing index, no
/// closing brace, a character other than `}` or `:` after the digits).
fn placeholder(chars: &[char], start: usize) -> Option<(usize, String, usize)> {
    let mut j = start + 1;
    let mut index: usize = 0;
    let digits_from = j;
    // `to_digit(10)` accepts ASCII digits only.
    while let Some(d) = chars.get(j).and_then(|c| c.to_digit(10)) {
        index = index.checked_mul(10)?.checked_add(d as usize)?;
        j += 1;
    }
    if j == digits_from {
        return None;
    }
    match chars.get(j)? {
        '}' => Some((index, String::new(), j + 1)),
        ':' => {
            let close = chars[j + 1..].iter().position(|&c| c == '}')? + j + 1;
            Some((index, chars[j + 1..close].iter().collect(), close + 1))
        }
        _ => None,
    }
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
