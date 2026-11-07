//! The JSON lab: text typed in is read by the toolkit's bounded JSON reader
//! (`lune_ui::json::parse`) and the value or the error is shown beside it. The reader's bounds
//! are the ones listed under the boxes; a text over them is an error, never a crash.

use std::fmt::Write as _;

use lune_ui::json::{
    JsonError, JsonErrorKind, MAX_DEPTH, MAX_INPUT, MAX_KEY, MAX_MEMBERS, MAX_STRING, MAX_VALUES,
    Value, parse,
};
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::text_area::AreaOptions;

use super::text;
use crate::page::{Page, PageKind, PageState, Stage, UiPage};
use crate::pages::{caption, card, heading};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "lab-json",
    title: "JSON lab",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: &[],
        bench: &[],
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Json::new())
}

const SAMPLE: &str = r#"{
  "name": "luneui",
  "version": 1,
  "glass": ["window", "panel", "card"],
  "opacity": { "tint": 0.8, "material": 0.7 },
  "dark": true,
  "note": null
}"#;

/// What the boxes hold at most, and the lines of the result that are shown.
const MAX_CHARS: usize = 20_000;
const MAX_LINES: usize = 400;
const BOX: (f32, f32) = (470.0, 380.0);

/// The texts the knob loads, in the order shown.
const SAMPLES: [(&str, &str); 4] = [
    ("Sample", SAMPLE),
    ("An array", "[1, 2, 3]"),
    ("A mistake", "[1, 2,]"),
    ("A duplicate key", r#"{"a": 1, "a": 2}"#),
];

struct Json {
    /// The sample the knob last loaded (an index into [`SAMPLES`]).
    loaded: usize,
    source: String,
    /// The result: the value pretty-printed, or the error.
    result: String,
    /// One line on what the reader made of the text.
    verdict: String,
    ok: bool,
    /// The text `result` was made from; `None` before the first read.
    read: Option<String>,
}

impl Json {
    fn new() -> Self {
        Self {
            loaded: 0,
            source: SAMPLE.to_owned(),
            result: String::new(),
            verdict: String::new(),
            ok: true,
            read: None,
        }
    }

    /// Reads the text again when it changed.
    fn refresh(&mut self) {
        if self.read.as_deref() == Some(self.source.as_str()) {
            return;
        }
        match parse(self.source.as_bytes()) {
            Ok(value) => {
                self.verdict = format!("Valid: {}", describe(&value));
                self.result = pretty(&value);
                self.ok = true;
            }
            Err(e) => {
                self.verdict = format!("Error: {}", error_text(&e));
                self.result = String::new();
                self.ok = false;
            }
        }
        self.read = Some(self.source.clone());
    }
}

/// What the error says, in words, with its place.
fn error_text(e: &JsonError) -> String {
    let what = match &e.kind {
        JsonErrorKind::TooLarge => format!("longer than {MAX_INPUT} bytes"),
        JsonErrorKind::TooDeep => format!("nested more than {MAX_DEPTH} deep"),
        JsonErrorKind::StringTooLong => format!("a string longer than {MAX_STRING} bytes"),
        JsonErrorKind::KeyTooLong => format!("a key longer than {MAX_KEY} bytes"),
        JsonErrorKind::TooManyMembers => format!("more than {MAX_MEMBERS} members"),
        JsonErrorKind::TooManyValues => format!("more than {MAX_VALUES} values"),
        JsonErrorKind::InvalidUtf8 => "not UTF-8".to_owned(),
        JsonErrorKind::Syntax(why) => (*why).to_owned(),
        JsonErrorKind::InvalidEscape => "a bad escape in a string".to_owned(),
        JsonErrorKind::TrailingData => "something after the value".to_owned(),
        JsonErrorKind::DuplicateKey(k) => format!("the key \"{k}\" twice"),
        JsonErrorKind::NumberOutOfRange => "a number out of range".to_owned(),
        JsonErrorKind::WrongType => "the wrong kind of value".to_owned(),
        JsonErrorKind::Missing(k) => format!("no member \"{k}\""),
    };
    if e.line > 0 {
        format!("{what}, line {} column {}", e.line, e.column)
    } else {
        what
    }
}

/// The top value in a few words.
fn describe(v: &Value) -> String {
    match v {
        Value::Null => "null".to_owned(),
        Value::Bool(b) => format!("a boolean, {b}"),
        Value::Number(n) => format!("a number, {}", n.value()),
        Value::String(s) => format!("a string of {} characters", s.chars().count()),
        Value::Array(a) => format!("an array of {}", a.len()),
        Value::Object(o) => format!("an object of {} members", o.len()),
    }
}

/// `v` indented two spaces a level, one value a line; cut at [`MAX_LINES`] lines.
fn pretty(v: &Value) -> String {
    let mut w = Writer {
        out: String::new(),
        lines: 0,
    };
    w.value(v, 0);
    if w.lines > MAX_LINES {
        let _ = write!(w.out, "\n... {} more lines", w.lines - MAX_LINES);
    }
    w.out
}

/// The lines of a pretty-printed value: past the cap only the count grows, so a big tree costs
/// a walk (bounded by the reader's limits) and no more text.
struct Writer {
    out: String,
    lines: usize,
}

impl Writer {
    fn line(&mut self, depth: usize, s: &str) {
        if self.lines < MAX_LINES {
            if !self.out.is_empty() {
                self.out.push('\n');
            }
            for _ in 0..depth {
                self.out.push_str("  ");
            }
            self.out.push_str(s);
        }
        self.lines += 1;
    }

    fn value(&mut self, v: &Value, depth: usize) {
        match v {
            Value::Null => self.line(depth, "null"),
            Value::Bool(b) => self.line(depth, &b.to_string()),
            Value::Number(n) => self.line(depth, &n.value().to_string()),
            Value::String(s) => self.line(depth, &format!("{s:?}")),
            Value::Array(items) => {
                self.line(depth, &format!("[{}]", items.len()));
                for item in items {
                    self.value(item, depth + 1);
                }
            }
            Value::Object(members) => {
                self.line(depth, &format!("{{{}}}", members.len()));
                for (key, value) in members {
                    self.line(depth + 1, &format!("{key:?}:"));
                    self.value(value, depth + 2);
                }
            }
        }
    }
}

impl PageState for Json {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "JSON lab");
        let t = *ui.theme();
        let (x, y) = (card.x + 24.0, card.y + 24.0);

        heading(ui, "JSON TEXT", x, y);
        let source = AreaOptions {
            placeholder: "Type JSON",
            label: Some("JSON text"),
            max_chars: Some(MAX_CHARS),
            error: !self.ok,
            ..AreaOptions::default()
        };
        ui.text_area(
            "source",
            Rect::new(x, y + 20.0, BOX.0, BOX.1),
            &mut self.source,
            &source,
        );
        self.refresh();

        heading(ui, "WHAT THE READER MADE OF IT", x + BOX.0 + 20.0, y);
        let result = AreaOptions {
            placeholder: "No value",
            label: Some("Result"),
            read_only: true,
            ..AreaOptions::default()
        };
        ui.text_area(
            "result",
            Rect::new(x + BOX.0 + 20.0, y + 20.0, BOX.0, BOX.1),
            &mut self.result,
            &result,
        );
        let colour = if self.ok { t.ok } else { t.err };
        text(
            ui,
            &self.verdict,
            x,
            y + 20.0 + BOX.1 + 14.0,
            t.font_base,
            colour,
        );
        caption(
            ui,
            &format!(
                "Bounds: {MAX_INPUT} bytes of input, {MAX_DEPTH} levels, {MAX_STRING}-byte strings, {MAX_KEY}-byte keys, {MAX_MEMBERS} members, {MAX_VALUES} values."
            ),
            x,
            y + 20.0 + BOX.1 + 44.0,
        );
        caption(
            ui,
            &format!(
                "The box takes up to {MAX_CHARS} characters; the result shows up to {MAX_LINES} lines."
            ),
            x,
            y + 20.0 + BOX.1 + 64.0,
        );
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let names: Vec<&str> = SAMPLES.iter().map(|(n, _)| *n).collect();
        if panel.choice(ui, "Load a text", &names, &mut self.loaded) {
            self.source = SAMPLES[self.loaded].1.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::rig::player;
    use super::*;
    use crate::page::Ev;
    use lune_ui::access::Role;
    use lune_ui::input::FrameOutput;
    use lune_ui::testing::FakeText;

    fn box_text(out: &FrameOutput, name: &str) -> String {
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == Role::MultilineText && n.name == name)
            .and_then(|n| n.text.as_ref())
            .map(|t| t.text.clone())
            .expect("a text box")
    }

    #[test]
    fn the_sample_reads_and_shows_its_value() {
        let mut p = player("lab-json", live());
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        let (_, out) = p.frame(&mut FakeText, 16, &[]).expect("frame");
        let result = box_text(&out, "Result");
        assert!(result.starts_with("{6}"), "{result}");
        assert!(result.contains("\"glass\":"), "{result}");
        assert!(result.contains("\"panel\""), "{result}");
        assert!(result.contains("null"), "{result}");
        assert_eq!(box_text(&out, "JSON text"), SAMPLE);
    }

    #[test]
    fn typing_changes_the_value_and_a_mistake_shows_its_place() {
        let mut j = Json::new();
        j.source = "[1, 2,]".to_owned();
        j.refresh();
        assert!(!j.ok);
        assert!(j.verdict.starts_with("Error: "), "{}", j.verdict);
        assert!(j.verdict.contains("line 1 column"), "{}", j.verdict);
        assert!(j.result.is_empty());
        j.source = "[1, 2]".to_owned();
        j.refresh();
        assert!(j.ok);
        assert_eq!(j.verdict, "Valid: an array of 2");
        assert_eq!(j.result, "[2]\n  1\n  2");
        // The reader's bounds are errors, not crashes.
        j.source = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        j.refresh();
        assert!(j.verdict.contains("nested more than 16"), "{}", j.verdict);
        j.source = r#"{"a":1,"a":2}"#.to_owned();
        j.refresh();
        assert!(j.verdict.contains("the key \"a\" twice"), "{}", j.verdict);
    }

    #[test]
    fn a_big_tree_is_cut_to_a_screenful() {
        let mut j = Json::new();
        j.source = format!("[{}]", vec!["1"; 1000].join(","));
        j.refresh();
        assert!(j.ok);
        assert_eq!(
            j.result.lines().count(),
            MAX_LINES + 1,
            "the cap and the note"
        );
        assert!(
            j.result.ends_with("more lines"),
            "{}",
            &j.result[j.result.len() - 30..]
        );
        assert!(
            j.result.contains("601 more lines"),
            "{}",
            &j.result[j.result.len() - 30..]
        );
    }

    /// Typing in the box reaches the reader: the result follows the keys.
    #[test]
    fn keys_typed_in_the_box_change_the_result() {
        let mut p = player("lab-json", live());
        let all = lune_ui::input::Modifiers {
            logo: true,
            ..lune_ui::input::Modifiers::default()
        };
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        p.frame(
            &mut FakeText,
            16,
            &[Ev::PointerOver(&["source"]), Ev::Click],
        )
        .expect("frame");
        p.frame(&mut FakeText, 32, &[Ev::KeyChar("a", all)])
            .expect("frame");
        p.frame(&mut FakeText, 48, &[Ev::Type("[7]")])
            .expect("frame");
        let (_, out) = p.frame(&mut FakeText, 64, &[]).expect("frame");
        assert_eq!(box_text(&out, "JSON text"), "[7]");
        assert_eq!(box_text(&out, "Result"), "[1]\n  7");
    }
}
