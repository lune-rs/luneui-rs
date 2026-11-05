//! Shared plumbing of the text views (D6): highlight spans for a line of text, the runs a line is
//! drawn from, and the boxes of a char range across runs.
//!
//! The toolkit never tokenizes (D45): the app hands over spans, in chars of the line (D95), and
//! this module clamps, sorts and cuts them into the runs of one [`Spec`].

use std::ops::Range;

use super::field::drawn_layout;
use crate::color::Rgba;
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::text_edit::EditLayout;
use crate::ui::Ui;

/// A coloured stretch of one line: chars `start..end` (D6). It need not be tidy: [`line_spec`]
/// clamps it to the line, skips it if empty or inverted, and cuts its overlap with an earlier one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub color: Rgba,
}

/// Colours for the lines in view. It is asked for every line drawn, every frame, so an app that
/// tokenizes caches by line and text.
pub trait Highlighter {
    /// The spans of line `line` (0-based), whose text is `text` without its terminator.
    fn spans(&self, line: usize, text: &str) -> Vec<HighlightSpan>;
}

/// The highlighter that colours nothing: every char takes the base colour.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlainHighlighter;

impl Highlighter for PlainHighlighter {
    fn spans(&self, _line: usize, _text: &str) -> Vec<HighlightSpan> {
        Vec::new()
    }
}

/// The runs that draw `text` in `base` with `spans` coloured. The joined text of the runs always
/// equals `text`. Spans past the end are clamped to it, empty or inverted ones (after the clamp)
/// are skipped, unsorted ones are sorted by start, and where two overlap the earlier keeps its
/// chars. Empty `text` gives one empty run, so the line still carries its style.
#[allow(dead_code, reason = "the code view draws its lines from it (step 4)")]
pub(crate) fn line_spec(text: &str, base: Style, spans: &[HighlightSpan]) -> Vec<(String, Style)> {
    if text.is_empty() {
        return vec![(String::new(), base)];
    }
    // Byte offset of each char boundary, with the end as the last entry.
    let mut at: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    let chars = at.len();
    at.push(text.len());
    let mut sorted: Vec<HighlightSpan> = spans.to_vec();
    sorted.sort_by_key(|s| s.start);
    let mut runs: Vec<(String, Style)> = Vec::new();
    let mut done = 0;
    for span in sorted {
        let (start, end) = (span.start.max(done), span.end.min(chars));
        if start >= end {
            continue;
        }
        if start > done {
            runs.push((text[at[done]..at[start]].to_owned(), base));
        }
        let colored = Style {
            color: span.color,
            ..base
        };
        runs.push((text[at[start]..at[end]].to_owned(), colored));
        done = end;
    }
    if done < chars {
        runs.push((text[at[done]..].to_owned(), base));
    }
    runs
}

/// The runs of `spec` cut to chars `from..to` of its joined text, each with its own style.
fn slice_runs(spec: &Spec, from: usize, to: usize) -> Spec {
    let mut runs = Vec::new();
    let mut pos = 0;
    for (text, style) in &spec.runs {
        let n = text.chars().count();
        let (a, b) = (from.max(pos), to.min(pos + n));
        if a < b {
            let part: String = text.chars().skip(a - pos).take(b - a).collect();
            runs.push((part, *style));
        }
        pos += n;
    }
    Spec { runs, wrap: None }
}

/// The boxes of chars `range` of `spec`, one per visual line it touches, from the text's top-left.
/// A run is measured with its own style, so a range across runs is right too.
pub(super) fn range_boxes(ui: &mut Ui<'_>, spec: &Spec, range: &Range<usize>) -> Vec<Rect> {
    if range.is_empty() || spec.runs.is_empty() {
        return Vec::new();
    }
    let parts = drawn_layout(ui, spec, |l| {
        let rows = l.lines();
        let spans = rows.iter().enumerate().filter_map(|(i, row)| {
            let (a, b) = (range.start.max(row.start), range.end.min(row.end));
            (a < b).then_some((i, *row, a, b))
        });
        let spans: Vec<_> = spans.collect();
        spans
            .into_iter()
            .map(|(i, row, a, b)| (i, row, a, b, l.caret(a), l.caret(b)))
            .collect::<Vec<_>>()
    });
    let mut boxes = Vec::with_capacity(parts.len());
    for (i, row, a, b, from, to) in parts {
        // A soft wrap's end offset shows on the next row, so this row's part is measured.
        let to_x = if to.line == i {
            to.x
        } else {
            from.x + ui.painter().measure(&slice_runs(spec, a, b)).0
        };
        let (x0, x1) = (from.x.min(to_x), from.x.max(to_x));
        boxes.push(Rect::new(x0, row.y, x1 - x0, row.height));
    }
    boxes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::UiHarness;

    const BASE: Style = Style {
        font: crate::text::Font::Mono,
        size: 10.0,
        line: 20.0,
        weight: 400,
        color: [255; 4],
        italic: false,
    };
    const RED: Rgba = [255, 0, 0, 255];
    const BLUE: Rgba = [0, 0, 255, 255];

    fn span(start: usize, end: usize, color: Rgba) -> HighlightSpan {
        HighlightSpan { start, end, color }
    }

    fn joined(runs: &[(String, Style)]) -> String {
        runs.iter().map(|(t, _)| t.as_str()).collect()
    }

    /// The text and colour of each run.
    fn shape(runs: &[(String, Style)]) -> Vec<(&str, Rgba)> {
        runs.iter().map(|(t, s)| (t.as_str(), s.color)).collect()
    }

    #[test]
    fn a_span_past_the_end_is_clamped_to_the_line() {
        let runs = line_spec("0123456789", BASE, &[span(3, 99, RED)]);
        assert_eq!(shape(&runs), [("012", BASE.color), ("3456789", RED)]);
    }

    #[test]
    fn an_empty_or_inverted_span_adds_nothing() {
        for s in [span(5, 5, RED), span(7, 2, RED), span(20, 30, RED)] {
            let runs = line_spec("0123456789", BASE, &[s]);
            assert_eq!(shape(&runs), [("0123456789", BASE.color)], "{s:?}");
        }
    }

    #[test]
    fn a_span_over_a_multibyte_char_lands_on_chars_not_bytes() {
        // "é" is two bytes: chars 1..2 is the é alone.
        let runs = line_spec("a\u{e9}b", BASE, &[span(1, 2, RED)]);
        assert_eq!(
            shape(&runs),
            [("a", BASE.color), ("\u{e9}", RED), ("b", BASE.color)]
        );
        assert_eq!(joined(&runs), "a\u{e9}b");
    }

    #[test]
    fn where_spans_overlap_the_earlier_keeps_its_chars() {
        let runs = line_spec("0123456789", BASE, &[span(0, 6, RED), span(4, 8, BLUE)]);
        assert_eq!(
            shape(&runs),
            [("012345", RED), ("67", BLUE), ("89", BASE.color)]
        );
        // A span inside an earlier one is cut away.
        let runs = line_spec("0123456789", BASE, &[span(0, 8, RED), span(2, 4, BLUE)]);
        assert_eq!(shape(&runs), [("01234567", RED), ("89", BASE.color)]);
    }

    #[test]
    fn unsorted_spans_are_sorted_by_start() {
        let runs = line_spec("0123456789", BASE, &[span(6, 8, BLUE), span(1, 3, RED)]);
        assert_eq!(
            shape(&runs),
            [
                ("0", BASE.color),
                ("12", RED),
                ("345", BASE.color),
                ("67", BLUE),
                ("89", BASE.color)
            ]
        );
    }

    #[test]
    fn empty_text_gives_one_empty_run_in_the_base_style() {
        let runs = line_spec("", BASE, &[span(0, 4, RED)]);
        assert_eq!(runs, [(String::new(), BASE)]);
    }

    #[test]
    fn the_joined_text_always_equals_the_input() {
        let texts = ["", "a", "0123456789", "a\u{e9}\u{1f600}b", "tab\there"];
        let spans = [
            span(0, 0, RED),
            span(0, 1, RED),
            span(2, 99, BLUE),
            span(9, 3, RED),
            span(1, 3, BLUE),
            span(0, 50, RED),
        ];
        for t in texts {
            for n in 0..=spans.len() {
                for rot in 0..spans.len() {
                    let mut set = spans[..n].to_vec();
                    let by = rot.min(set.len());
                    set.rotate_left(by);
                    let runs = line_spec(t, BASE, &set);
                    assert_eq!(joined(&runs), t, "{t:?} {set:?}");
                    assert!(
                        t.is_empty() || runs.iter().all(|(r, _)| !r.is_empty()),
                        "{t:?}: no empty run in a non-empty line"
                    );
                }
            }
        }
    }

    #[test]
    fn plain_highlighter_colours_nothing() {
        assert!(PlainHighlighter.spans(0, "anything").is_empty());
    }

    fn boxes_of(spec: &Spec, range: Range<usize>) -> Vec<Rect> {
        let mut h = UiHarness::new((400.0, 300.0));
        let mut out = Vec::new();
        h.frame(|ui| out = range_boxes(ui, spec, &range));
        out
    }

    #[test]
    fn range_boxes_on_a_two_run_wrapped_spec_equal_the_one_run_spec() {
        let one = Spec::plain("abcdefghijklmnopqrstuvwxyz", BASE).wrapped(60.0);
        let red = Style { color: RED, ..BASE };
        let two = Spec {
            runs: vec![
                ("abcdefghijklm".to_owned(), BASE),
                ("nopqrstuvwxyz".to_owned(), red),
            ],
            wrap: Some(60.0),
        };
        for range in [2..20, 0..26, 11..15, 5..6, 13..26] {
            let a = boxes_of(&one, range.clone());
            let b = boxes_of(&two, range.clone());
            assert!(!a.is_empty(), "{range:?}: the instrument sees boxes");
            assert_eq!(a, b, "{range:?}");
        }
        // Several rows really were touched, so the soft-wrap branch ran.
        assert!(boxes_of(&one, 0..26).len() > 2);
    }

    #[test]
    fn a_wrapped_range_across_runs_of_different_size_measures_each_part_by_its_run() {
        // Both runs are narrower per char than the first: the part on the second row must be
        // measured with the second run's style, not the first's.
        let big = Style { size: 20.0, ..BASE };
        let two = Spec {
            runs: vec![("abcd".to_owned(), BASE), ("efghijklmnop".to_owned(), big)],
            wrap: Some(50.0),
        };
        let boxes = boxes_of(&two, 0..16);
        let one_style = boxes_of(&Spec::plain("abcdefghijklmnop", BASE).wrapped(50.0), 0..16);
        assert_ne!(boxes, one_style, "the runs' own widths count");
        assert!(boxes.iter().all(|b| b.w > 0.0 && b.h > 0.0));
    }
}
