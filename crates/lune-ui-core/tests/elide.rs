//! `text::elide` on the fake text system, where every char is half its run's size wide: the cut,
//! the width bound, what comes back unchanged and how little gets shaped.

use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{
    CaretPos, LineSpan, Spec, Style, TextKey, TextSystem, Unit, elide, joined_text,
};

const WHITE: [u8; 4] = [255; 4];

/// 5 px a char.
fn small() -> Style {
    Style::ui(10.0, WHITE)
}

/// 10 px a char.
fn big() -> Style {
    Style::ui(20.0, [1, 2, 3, 255])
}

fn cut(spec: &Spec, max_w: f32) -> Spec {
    elide(&mut FakeText, spec, 1.0, max_w)
}

fn text_of(spec: &Spec) -> String {
    joined_text(spec)
}

fn width(spec: &Spec) -> f32 {
    FakeText.measure(spec, 1.0).0
}

/// A [`FakeText`] that lists what it shaped and counts its caret queries.
#[derive(Default)]
struct Counting {
    shaped: Vec<TextKey>,
    carets: usize,
}

impl TextSystem for Counting {
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
        let out = FakeText.prepare(spec, scale);
        if !self.shaped.contains(&out.0) {
            self.shaped.push(out.0);
        }
        out
    }

    fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos {
        self.carets += 1;
        FakeText.caret(spec, scale, offset)
    }

    fn hit(&mut self, _: &Spec, _: f32, _: (f32, f32)) -> usize {
        panic!("elide never asks `hit`: it is not monotonic inside a cluster")
    }

    fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan> {
        FakeText.lines(spec, scale)
    }

    fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
        FakeText.segments(text, unit)
    }
}

#[test]
fn a_spec_that_fits_comes_back_equal_and_nothing_else_is_shaped() {
    let spec = Spec::plain("Hello", small());
    let mut text = Counting::default();
    assert_eq!(elide(&mut text, &spec, 1.0, 25.0), spec);
    assert_eq!(text.shaped, [TextKey(spec.cache_key(1.0))]);
    assert_eq!(text.carets, 0);
}

#[test]
fn a_text_too_wide_ends_in_an_ellipsis_within_the_width() {
    let spec = Spec::plain("Hello world", small());
    for (max_w, want) in [
        (54.0, "Hello wor\u{2026}"),
        (30.0, "Hello\u{2026}"),
        (5.0, "\u{2026}"),
    ] {
        let got = cut(&spec, max_w);
        assert_eq!(text_of(&got), want, "at {max_w}");
        assert!(width(&got) <= max_w);
        assert_eq!(got.wrap, None);
    }
}

#[test]
fn trailing_whitespace_goes_with_the_cut() {
    let spec = Spec::plain("Hello    world", small());
    for max_w in [30.0, 35.0, 40.0, 45.0, 50.0] {
        assert_eq!(text_of(&cut(&spec, max_w)), "Hello\u{2026}", "at {max_w}");
    }
    assert_eq!(
        text_of(&cut(&Spec::plain("    world", small()), 20.0)),
        "\u{2026}"
    );
}

#[test]
fn with_no_room_for_the_ellipsis_the_result_is_the_ellipsis() {
    let spec = Spec::plain("Hello", small());
    for max_w in [4.9, 0.0, -3.0] {
        assert_eq!(cut(&spec, max_w), Spec::plain("\u{2026}", small()));
    }
}

#[test]
fn a_width_that_is_not_finite_and_a_wrapped_spec_come_back_as_they_are() {
    let spec = Spec::plain("Hello\nworld", small());
    for max_w in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(cut(&spec, max_w), spec);
    }
    let wrapped = Spec::plain("Hello world", small()).wrapped(20.0);
    assert_eq!(cut(&wrapped, 10.0), wrapped);
}

#[test]
fn an_empty_text_comes_back_as_it_is() {
    let none = Spec {
        runs: Vec::new(),
        wrap: None,
    };
    let empty = Spec::plain("", small());
    for max_w in [10.0, 0.0, -1.0] {
        assert_eq!(cut(&none, max_w), none);
        assert_eq!(cut(&empty, max_w), empty);
    }
}

#[test]
fn a_line_break_cuts_there_and_adds_the_ellipsis() {
    for text in ["ab\ncd", "ab\r\ncd", "ab\rcd", "ab\n", "ab  \ncd"] {
        let got = cut(&Spec::plain(text, small()), 100.0);
        assert_eq!(text_of(&got), "ab\u{2026}", "{text:?}");
    }
    let split = Spec {
        runs: vec![("ab\r".to_owned(), small()), ("\ncd".to_owned(), big())],
        wrap: None,
    };
    assert_eq!(cut(&split, 100.0), Spec::plain("ab\u{2026}", small()));
    assert_eq!(
        cut(&Spec::plain("\nab", small()), 100.0),
        Spec::plain("\u{2026}", small())
    );
    // The first line too wide: the cut is inside it, and the rest never counts.
    assert_eq!(
        text_of(&cut(&Spec::plain("abcdef\nxy", small()), 15.0)),
        "ab\u{2026}"
    );
}

#[test]
fn the_cut_falls_on_grapheme_boundaries() {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    for text in [
        format!("a{family}b{family}c"),
        "e\u{301}e\u{301}e\u{301}e\u{301}".to_owned(),
        "\u{1F44D}\u{1F3FD}\u{1F44D}\u{1F3FD}\u{1F44D}\u{1F3FD}".to_owned(),
    ] {
        let spec = Spec::plain(text.clone(), small());
        let bounds = FakeText.segments(&text, Unit::Grapheme);
        for max_w in 0..60 {
            let got = text_of(&cut(&spec, max_w as f32));
            if got == text {
                continue;
            }
            let kept = got.strip_suffix('\u{2026}').expect("ends in the ellipsis");
            assert!(bounds.contains(&kept.len()), "{got:?} at {max_w}");
            assert!(text.starts_with(kept));
        }
    }
}

#[test]
fn a_cut_inside_run_two_keeps_run_one_whole_and_each_style() {
    let spec = Spec {
        runs: vec![
            ("abc".to_owned(), small()),
            ("defgh".to_owned(), big()),
            ("xyz".to_owned(), small()),
        ],
        wrap: None,
    };
    // abc 15, d 10, e 10, the ellipsis in run two's style 10.
    let got = cut(&spec, 45.0);
    assert_eq!(
        got.runs,
        [
            ("abc".to_owned(), small()),
            ("de\u{2026}".to_owned(), big())
        ]
    );
    assert!(width(&got) <= 45.0);
}

/// Measured in the style it will carry: in the wider style there is room for one char less.
#[test]
fn the_ellipsis_is_measured_in_the_style_it_carries() {
    let wide = Spec::plain("defgh", big());
    assert_eq!(text_of(&cut(&wide, 30.0)), "de\u{2026}");
    assert_eq!(text_of(&cut(&wide, 29.0)), "d\u{2026}");
}

#[test]
fn a_cut_at_the_end_of_a_run_carries_that_runs_style() {
    let spec = Spec {
        runs: vec![("abc".to_owned(), small()), ("  defgh".to_owned(), big())],
        wrap: None,
    };
    // The spaces go, and with them run two: the last kept run is run one.
    assert_eq!(cut(&spec, 45.0).runs, [("abc\u{2026}".to_owned(), small())]);
}

#[test]
fn the_runs_before_the_cut_stay_also_when_empty() {
    let spec = Spec {
        runs: vec![("".to_owned(), big()), ("abcdef".to_owned(), small())],
        wrap: None,
    };
    let got = cut(&spec, 15.0);
    assert_eq!(
        got.runs,
        [("".to_owned(), big()), ("ab\u{2026}".to_owned(), small())]
    );
    // Nothing kept: the ellipsis takes the first style that has text.
    assert_eq!(cut(&spec, 5.0), Spec::plain("\u{2026}", small()));
}

/// O(log n) caret queries, and one shaped result: never a shaped prefix per offset.
#[test]
fn a_long_text_costs_a_few_queries() {
    let spec = Spec::plain("x".repeat(4096), small());
    let mut text = Counting::default();
    let got = elide(&mut text, &spec, 1.0, 1000.0);
    assert!(width(&got) <= 1000.0);
    assert!(text.carets <= 13, "{} carets", text.carets);
    // The spec, the ellipsis and the result.
    assert_eq!(text.shaped.len(), 3);
}

/// A system whose carets run ahead of the shaped result's width, as a cut that breaks a ligature
/// or joins differently does: the result is measured, and still meets the bound.
#[derive(Default)]
struct Understated;

impl TextSystem for Understated {
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
        FakeText.prepare(spec, scale)
    }

    fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos {
        let c = FakeText.caret(spec, scale, offset);
        CaretPos { x: c.x * 0.25, ..c }
    }

    fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize {
        FakeText.hit(spec, scale, pos)
    }

    fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan> {
        FakeText.lines(spec, scale)
    }

    fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
        FakeText.segments(text, unit)
    }
}

#[test]
fn carets_that_understate_the_cut_never_break_the_bound() {
    let spec = Spec::plain("x".repeat(100), small());
    for max_w in [5.0, 6.0, 12.0, 40.0, 333.0] {
        let got = elide(&mut Understated, &spec, 1.0, max_w);
        assert!(width(&got) <= max_w, "{got:?} at {max_w}");
        assert!(text_of(&got).ends_with('\u{2026}'));
    }
    assert_eq!(
        text_of(&elide(&mut Understated, &spec, 1.0, 40.0))
            .chars()
            .count(),
        8
    );
}

#[test]
fn nothing_panics_on_a_sweep_of_widths() {
    let specs = [
        Spec::plain("a b  c\n\r\n", small()),
        Spec::plain(" ", small()),
        Spec::plain("\n", small()),
        Spec {
            runs: vec![(" ".to_owned(), big()), ("\u{301}x".to_owned(), small())],
            wrap: None,
        },
    ];
    for spec in &specs {
        for w in -2..40 {
            let got = cut(spec, w as f32 * 1.5);
            assert!(!got.runs.is_empty());
        }
    }
}

#[test]
fn the_painter_elides_at_the_frames_scale() {
    let spec = Spec::plain("Hello world", small());
    let mut h = UiHarness::new((100.0, 100.0));
    h.frame(|ui| {
        let got = ui.painter().elide(&spec, 30.0);
        assert_eq!(text_of(&got), "Hello\u{2026}");
    });
}
