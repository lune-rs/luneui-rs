//! The text seam's contract on the glyphon `TextCache`, with the gallery's fonts and with none.

use lune_ui_core::testing::text_system_contract;
use lune_ui_wgpu::text_cache::TextCache;

#[test]
fn text_cache_with_fonts_meets_the_contract() {
    let fonts: [&[u8]; 3] = [
        include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Medium.ttf"),
    ];
    let mut text = TextCache::new(&fonts, "RznSans", "RobotoMono Nerd Font Mono");
    assert_eq!(text_system_contract(&mut text), Ok(()));
}

#[test]
fn text_cache_without_fonts_meets_the_contract() {
    let mut text = TextCache::new(&[], "sans-serif", "monospace");
    assert_eq!(text_system_contract(&mut text), Ok(()));
}

#[test]
fn text_cache_without_fonts_lays_out_nothing() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = TextCache::new(&[], "sans-serif", "monospace");
    let spec = Spec::plain("Hello", Style::ui(13.0, [255; 4]));
    assert_eq!(text.prepare(&spec, 2.0).1, (0.0, 0.0));
    assert_eq!(text.measure(&spec, 2.0), (0.0, 0.0));
}

fn with_fonts() -> TextCache {
    let fonts: [&[u8]; 3] = [
        include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Medium.ttf"),
    ];
    TextCache::new(&fonts, "RznSans", "RobotoMono Nerd Font Mono")
}

/// The contract's hand-worked segmentation table, against `unicode-segmentation` itself.
#[test]
fn the_segment_table_matches_unicode_segmentation() {
    use lune_ui_core::testing::SEGMENT_TABLE;
    use unicode_segmentation::UnicodeSegmentation;
    for case in SEGMENT_TABLE {
        let mut g: Vec<usize> = case.text.grapheme_indices(true).map(|(i, _)| i).collect();
        g.push(case.text.len());
        g.dedup();
        let mut w: Vec<usize> = case
            .text
            .split_word_bound_indices()
            .map(|(i, _)| i)
            .collect();
        w.push(case.text.len());
        w.dedup();
        assert_eq!(g, case.graphemes, "graphemes of {:?}", case.text);
        assert_eq!(w, case.words, "words of {:?}", case.text);
    }
}

#[test]
fn the_caret_at_the_end_is_the_width_and_lines_follow_the_endings() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = with_fonts();
    let style = Style::ui(14.0, [255; 4]);
    for scale in [1.0, 1.5, 2.0] {
        for s in ["Hello world", "Wide WWWW iiii", "fi ffi \u{e9}"] {
            let spec = Spec::plain(s, style);
            let width = text.measure(&spec, scale).0;
            assert!(width > 0.0);
            assert_eq!(
                text.caret(&spec, scale, s.len()).x,
                width,
                "{s:?} at {scale}"
            );
        }
    }
    // Three lines across \n and \r\n, split over two runs.
    let spec = Spec {
        runs: vec![
            ("ab\ncd".to_owned(), style),
            ("\r\nef".to_owned(), style.bold()),
        ],
        wrap: None,
    };
    let lines = text.lines(&spec, 2.0);
    let ranges: Vec<_> = lines.iter().map(|l| (l.start, l.end)).collect();
    assert_eq!(ranges, [(0, 2), (3, 5), (7, 9)]);
    assert!(lines.windows(2).all(|w| w[0].y < w[1].y));
    let c = text.caret(&spec, 2.0, 8);
    assert_eq!(c.line, 2);
    assert_eq!(c.y, lines[2].y);
    assert!(c.x > 0.0);
    // The click lands in the middle line, and the end of the text is the joined length.
    let mid = lines[1].y + lines[1].height / 2.0;
    assert_eq!(text.hit(&spec, 2.0, (0.0, mid)), 3);
    assert_eq!(text.hit(&spec, 2.0, (1000.0, mid)), 5);
    assert_eq!(text.hit(&spec, 2.0, (1000.0, 1000.0)), 9);
}

#[test]
fn a_wrapped_line_breaks_into_rows_and_the_break_offset_starts_the_next() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = with_fonts();
    let spec = Spec::plain("aaaa bbbb cccc dddd", Style::ui(14.0, [255; 4])).wrapped(60.0);
    let lines = text.lines(&spec, 1.0);
    assert!(lines.len() >= 3, "{lines:?}");
    assert_eq!(lines[0].start, 0);
    let second = lines[1].start;
    assert!(second > 0);
    let at_break = text.caret(&spec, 1.0, second);
    assert_eq!((at_break.line, at_break.x), (1, 0.0));
    let before = text.caret(&spec, 1.0, second - 1);
    assert_eq!(before.line, 0);
    assert_eq!(lines.last().expect("lines").end, spec.runs[0].0.len());
}

#[test]
fn right_to_left_and_mixed_text_answer_without_panicking() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = with_fonts();
    let spec = Spec::plain("abc \u{5d0}\u{5d1}\u{5d2} def", Style::ui(14.0, [255; 4]));
    let len = spec.runs[0].0.len();
    for b in 0..=len + 2 {
        let c = text.caret(&spec, 1.0, b);
        assert!(c.x.is_finite() && c.y.is_finite() && c.height > 0.0);
    }
    assert!(text.hit(&spec, 1.0, (30.0, 5.0)) <= len);
}

#[test]
fn the_height_counts_the_line_a_final_terminator_opens() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = with_fonts();
    let style = Style::ui(13.0, [255; 4]);
    for (spec, rows) in [
        ("a\nb", 2.0),
        ("a\nb\n", 3.0),
        ("a\nb\n\n", 4.0),
        ("a\r\n", 2.0),
    ] {
        let s = Spec::plain(spec, style);
        for scale in [1.0, 2.0] {
            assert_eq!(text.measure(&s, scale).1, rows * style.line, "{spec:?}");
            let end = text.caret(&s, scale, spec.len());
            assert_eq!(end.y, (rows - 1.0) * style.line, "{spec:?}");
        }
    }
}

#[test]
fn a_point_below_the_text_keeps_its_column_and_the_open_line_is_the_end() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = with_fonts();
    let style = Style::ui(13.0, [255; 4]);
    let spec = Spec::plain("Hello\nworld", style);
    let x = text.caret(&spec, 2.0, 2).x;
    let bottom = text.measure(&spec, 2.0).1;
    assert_eq!(text.hit(&spec, 2.0, (x, bottom + 30.0)), 8);
    assert_eq!(text.hit(&spec, 2.0, (x, -30.0)), 2);
    let open = Spec::plain("Hello\n", style);
    let bottom = text.measure(&open, 2.0).1;
    assert_eq!(text.hit(&open, 2.0, (x, bottom - 1.0)), 6);
    assert_eq!(text.hit(&open, 2.0, (x, bottom + 30.0)), 6);
}

fn rows(text: &mut TextCache, spec: &lune_ui_core::text::Spec, scale: f32) -> Vec<(usize, usize)> {
    use lune_ui_core::text::TextSystem;
    let lines = text.lines(spec, scale);
    lines.iter().map(|l| (l.start, l.end)).collect()
}

/// The row of a line whose first space was dropped owns that space's offset, in `lines` as in
/// `caret`, and `hit` at its left edge shows the same caret.
#[test]
fn a_dropped_leading_space_belongs_to_its_lines_row() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = with_fonts();
    let style = Style::ui(13.0, [255; 4]);
    let word = "w".repeat(20);
    let spec = Spec::plain(format!("ww\n {word}\nnext"), style).wrapped(200.0);
    assert_eq!(rows(&mut text, &spec, 2.0), [(0, 2), (3, 24), (25, 29)]);
    let (space, glyph) = (text.caret(&spec, 2.0, 3), text.caret(&spec, 2.0, 4));
    assert_eq!((space.line, space.x), (1, 0.0));
    assert_eq!((glyph.line, glyph.x), (1, 0.0));
    let mid = space.y + space.height / 2.0;
    let at = text.hit(&spec, 2.0, (0.0, mid));
    let left = text.caret(&spec, 2.0, at);
    assert_eq!((left.line, left.x), (1, 0.0));
    let first = Spec::plain(format!(" {word}"), style).wrapped(200.0);
    assert_eq!(rows(&mut text, &first, 2.0), [(0, 21)]);
}

/// `run.glyphs` is not in visual order on a right-to-left row: the row's left edge is a min.
#[test]
fn a_dropped_space_on_a_right_to_left_row_is_at_the_rows_left_edge() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = with_fonts();
    let style = Style::ui(13.0, [255; 4]);
    let spec = Spec::plain(
        "\u{5d0}\u{5d1}\u{5d2} \u{5d3}\u{5d4}\u{5d5}      \u{5d0}\u{5d1}",
        style,
    )
    .wrapped(40.0);
    assert_eq!(rows(&mut text, &spec, 2.0)[1], (7, 17));
    let edge = text.caret(&spec, 2.0, 17);
    let dropped = text.caret(&spec, 2.0, 18);
    assert_eq!((dropped.line, dropped.x), (1, edge.x));
    assert!(text.caret(&spec, 2.0, 14).x > edge.x);
}

/// A soft row with no glyphs in mixed text, and a cluster of two glyphs split over two rows, keep
/// the rows in text order, which every query relies on.
#[test]
fn the_rows_stay_in_text_order() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = with_fonts();
    let style = Style::ui(13.0, [255; 4]);
    let hebrew = "\u{5d3}\u{5d4}\u{5d5}\u{5d0}\u{5d1}\u{5d2}";
    let mixed = format!("word \u{5d0}\u{5d1}\u{5d2} {hebrew}{hebrew}{hebrew} {hebrew}");
    let cases = [
        (Spec::plain(mixed, style).wrapped(25.0), 3.0),
        (
            Spec::plain("\u{1F44D}\u{1F3FD}w\nword", style).wrapped(9.0),
            1.0,
        ),
    ];
    for (spec, scale) in cases {
        let got = rows(&mut text, &spec, scale);
        assert!(got.windows(2).all(|w| w[0].1 <= w[1].0), "{got:?}");
        assert_eq!(text.caret(&spec, scale, 0).line, 0, "{got:?}");
    }
}

/// Texts ending in a cluster of several glyphs: the caret is after all of them, at the text's width.
const CLUSTER_ENDINGS: [&str; 5] = [
    "ok \u{1F44D}\u{1F3FD}",
    "\u{1F1F2}\u{1F1FE}",
    "fix \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    "\u{e17}\u{e33}",
    "\u{915}\u{93f}",
];

fn end_caret_is_the_width(text: &mut TextCache, s: &str) {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let spec = Spec::plain(s, Style::ui(13.0, [255; 4]));
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 3.0] {
        let width = text.measure(&spec, scale).0;
        assert!(width > 0.0, "{s:?}");
        assert_eq!(
            text.caret(&spec, scale, s.len()).x,
            width,
            "{s:?} at {scale}"
        );
    }
}

#[test]
fn the_caret_after_a_cluster_of_several_glyphs_is_at_the_width() {
    let mut text = with_fonts();
    for s in CLUSTER_ENDINGS {
        end_caret_is_the_width(&mut text, s);
    }
}

/// The cut text stays within the width at each contract scale, on a cluster boundary, for every
/// width from the ellipsis alone to the whole text. The cut changes the shaping of what stays, so
/// the carets the search reads are not the result's width: only the result is measured.
fn elided_stays_within_the_width(text: &mut TextCache, s: &str) {
    use lune_ui_core::text::{Spec, Style, TextSystem, Unit, elide, joined_text};
    let style = Style::ui(13.0, [255; 4]);
    let body = format!("{s} {s} {s}");
    let spec = Spec::plain(body.clone(), style);
    let bounds = text.segments(&body, Unit::Grapheme);
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 3.0] {
        let dots = text.measure(&Spec::plain("\u{2026}", style), scale).0;
        let full = text.measure(&spec, scale).0;
        assert!(dots > 0.0 && full > dots, "{s:?}");
        let mut w = dots;
        while w < full {
            let got = elide(text, &spec, scale, w);
            let kept = joined_text(&got);
            let kept = kept.strip_suffix('\u{2026}').expect("cut");
            assert!(text.measure(&got, scale).0 <= w, "{s:?} at {scale}, {w}");
            assert!(bounds.contains(&kept.len()), "{s:?} at {scale}, {w}");
            assert!(body.starts_with(kept), "{s:?} at {scale}, {w}");
            w += 1.7;
        }
        assert_eq!(elide(text, &spec, scale, full), spec, "{s:?} at {scale}");
        let tight = elide(text, &spec, scale, dots * 0.5);
        assert_eq!(joined_text(&tight), "\u{2026}", "{s:?} at {scale}");
    }
}

#[test]
fn elided_cluster_text_stays_within_the_width() {
    let mut text = with_fonts();
    for s in CLUSTER_ENDINGS {
        elided_stays_within_the_width(&mut text, s);
    }
}

/// Mixed-direction text: a caret's x does not grow with the offset, so the caret search can miss.
#[test]
fn elided_mixed_direction_text_stays_within_the_width() {
    let mut text = with_fonts();
    elided_stays_within_the_width(&mut text, "word \u{5d0}\u{5d1}\u{5d2} more \u{5d3}\u{5d4}");
}

/// The same with fonts that cover the script, where the system has them: the gallery's draw Thai
/// and Devanagari as boxes.
#[test]
fn the_caret_after_a_shaped_cluster_is_at_the_width() {
    let fonts = [
        ("Ayuthaya.ttf", "Ayuthaya", "\u{e17}\u{e33}"),
        (
            "Devanagari Sangam MN.ttc",
            "Devanagari Sangam MN",
            "\u{915}\u{93f}",
        ),
        (
            "Devanagari Sangam MN.ttc",
            "Devanagari Sangam MN",
            "\u{915}\u{940}",
        ),
    ];
    for (file, family, s) in fonts {
        let Ok(data) = std::fs::read(format!("/System/Library/Fonts/Supplemental/{file}")) else {
            println!("skipped: no {file}");
            continue;
        };
        let mut text = TextCache::new(&[&data], family, family);
        end_caret_is_the_width(&mut text, s);
        elided_stays_within_the_width(&mut text, s);
    }
}

/// A seeded walk over the texts that trip shaping: wide and narrow chars, spaces, each break.
fn generated(seed: u64, style: lune_ui_core::text::Style) -> lune_ui_core::text::Spec {
    use lune_ui_core::text::Spec;
    const PIECES: &[&str] = &[
        "a",
        "w",
        "\u{e9}",
        " ",
        "  ",
        "     ",
        "\n",
        "\r\n",
        "\r",
        "word",
        "wwwwwwwwwwwwwwwwwwww",
        "\u{1F44D}\u{1F3FD}",
        "\u{1F1F2}\u{1F1FE}",
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    ];
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = |n: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n) as usize
    };
    let text: String = (0..next(14))
        .map(|_| PIECES[next(PIECES.len() as u64)])
        .collect();
    let spec = Spec::plain(text, style);
    match next(3) {
        0 => spec,
        1 => spec.wrapped(120.0),
        _ => spec.wrapped(200.0),
    }
}

#[test]
fn generated_text_meets_the_line_properties() {
    use lune_ui_core::testing::spec_line_contract;
    use lune_ui_core::text::Style;
    let mut text = with_fonts();
    let style = Style::ui(13.0, [255; 4]);
    let mut failures = Vec::new();
    for seed in 1..=6000 {
        let spec = generated(seed, style);
        if let Err(e) = spec_line_contract(&mut text, &spec) {
            failures.push(e);
        }
    }
    let n = failures.len();
    failures.truncate(12);
    assert_eq!(n, 0, "{failures:#?}");
}
