//! The public half of the text views' shared plumbing (D6): the span type and the highlighter
//! trait. `line_spec` and `range_boxes` are crate-private; their cases are in the unit tests of
//! `widgets/text_lines.rs`.

use lune_ui_core::widgets::text_lines::{HighlightSpan, Highlighter, PlainHighlighter};

/// Colours every `x` of a line.
struct Xs;

impl Highlighter for Xs {
    fn spans(&self, _line: usize, text: &str) -> Vec<HighlightSpan> {
        text.chars()
            .enumerate()
            .filter(|(_, c)| *c == 'x')
            .map(|(i, _)| HighlightSpan {
                start: i,
                end: i + 1,
                color: [1, 2, 3, 255],
            })
            .collect()
    }
}

#[test]
fn a_highlighter_is_object_safe_and_the_plain_one_colours_nothing() {
    let all: [&dyn Highlighter; 2] = [&PlainHighlighter, &Xs];
    assert!(all[0].spans(0, "xx").is_empty());
    let spans = all[1].spans(3, "axb");
    assert_eq!(spans.len(), 1);
    assert_eq!((spans[0].start, spans[0].end), (1, 2));
}

#[test]
fn spans_are_chars_of_the_line() {
    // "é" is two bytes, one char: the x after it is char 1.
    let spans = Xs.spans(0, "\u{e9}x");
    assert_eq!((spans[0].start, spans[0].end), (1, 2));
}
