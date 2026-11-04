//! The text seam's fake and the contract every text system meets.

use crate::text::{
    CaretPos, LineSpan, Spec, Style, TextKey, TextSystem, Unit, clamp_to_boundary, joined_text,
};

/// A deterministic [`TextSystem`] with no fonts, for tests.
///
/// - key: `TextKey(spec.cache_key(scale))`, so it matches any real system's identity rule;
/// - width: the sum over runs of char count × size × 0.5 (logical px), so every char is half its
///   run's size wide;
/// - lines break at `\n`, `\r\n` and a lone `\r` (the ending is not part of a line); with `wrap: Some(w)`,
///   `w > 0` and a widest line above `w`, each line above `w` shows as `ceil(line width / w)` rows
///   (at most one per char, the chars spread evenly over them) and the width is `w`; otherwise the
///   width is the widest line's;
/// - height: rows × the first run's line height, or 17.0 with no runs (as the shaping cache gives
///   an empty spec);
/// - queries: the caret and hit follow the half-size chars and the rows `prepare` counts, as
///   `lines` lists them (a row's end is the next one's start, where the caret belongs to the later
///   row); `segments` groups combining marks U+0300 to U+036F, U+FE0F, U+200D and
///   the skin tones U+1F3FB to U+1F3FF onto the char before them, a U+200D also joins the
///   pictographic char after it, and `\r\n` is one cluster; a word is a run of clusters led by an
///   alphanumeric char, a run of clusters led by whitespace, or any other single cluster.
///
/// Never panics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FakeText;

/// One char of a spec: its byte offset in the joined text, its width and its line height.
struct Glyph {
    at: usize,
    ch: char,
    w: f32,
}

/// The fake's layout: chars per row, each row's byte range, the line height and the width.
struct Layout {
    lines: Vec<(usize, usize, Vec<Glyph>)>,
    line_h: f32,
    width: f32,
}

/// Pushes the ending-separated line `start..end`, cut into rows when it is wider than `wrap`.
fn push_line(
    out: &mut Vec<(usize, usize, Vec<Glyph>)>,
    wrap: Option<f32>,
    start: usize,
    end: usize,
    glyphs: Vec<Glyph>,
) {
    let width: f32 = glyphs.iter().map(|g| g.w).sum();
    let count = glyphs.len();
    let rows = match wrap {
        Some(w) if w > 0.0 && width > w => ((width / w).ceil() as usize).min(count),
        _ => 1,
    };
    if rows <= 1 {
        out.push((start, end, glyphs));
        return;
    }
    let mut cut: Vec<Vec<Glyph>> = (0..rows).map(|_| Vec::new()).collect();
    for (i, g) in glyphs.into_iter().enumerate() {
        cut[i * rows / count].push(g);
    }
    let starts: Vec<usize> = cut.iter().map(|r| r[0].at).collect();
    for (i, row) in cut.into_iter().enumerate() {
        out.push((starts[i], starts.get(i + 1).copied().unwrap_or(end), row));
    }
}

impl FakeText {
    fn layout(spec: &Spec) -> Layout {
        let mut chars = Vec::new();
        let mut at = 0;
        for (text, style) in &spec.runs {
            for ch in text.chars() {
                chars.push(Glyph {
                    at,
                    ch,
                    w: style.size * 0.5,
                });
                at += ch.len_utf8();
            }
        }
        let len = at;
        let mut lines = Vec::new();
        let (mut start, mut cur) = (0, Vec::new());
        let mut widest: f32 = 0.0;
        let mut i = 0;
        while i < chars.len() {
            let g = &chars[i];
            let ending = match g.ch {
                '\n' => 1,
                '\r' if chars.get(i + 1).is_some_and(|n| n.ch == '\n') => 2,
                '\r' => 1,
                _ => 0,
            };
            if ending == 0 {
                cur.push(Glyph {
                    at: g.at,
                    ch: g.ch,
                    w: g.w,
                });
                i += 1;
                continue;
            }
            let end = g.at;
            let line = std::mem::take(&mut cur);
            widest = widest.max(line.iter().map(|g| g.w).sum());
            push_line(&mut lines, spec.wrap, start, end, line);
            i += ending;
            start = chars.get(i).map_or(len, |n| n.at);
        }
        widest = widest.max(cur.iter().map(|g| g.w).sum());
        push_line(&mut lines, spec.wrap, start, len, cur);
        let line_h = spec.runs.first().map_or(17.0, |(_, st)| st.line);
        let width = match spec.wrap {
            Some(w) if w > 0.0 && widest > w => w,
            _ => widest,
        };
        Layout {
            lines,
            line_h,
            width,
        }
    }
}

fn pictographic(c: char) -> bool {
    c as u32 >= 0x1F000 || (0x2600..=0x27BF).contains(&(c as u32))
}

fn extends(c: char) -> bool {
    matches!(c, '\u{300}'..='\u{36F}' | '\u{FE0F}' | '\u{200D}' | '\u{1F3FB}'..='\u{1F3FF}')
}

/// Cluster boundaries of the fake's grapheme rule, with 0 and the end.
fn clusters(text: &str) -> Vec<usize> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = vec![0];
    let mut i = 0;
    while i < chars.len() {
        let base = chars[i].1;
        i += 1;
        if base == '\r' && chars.get(i).is_some_and(|c| c.1 == '\n') {
            i += 1;
        } else {
            while let Some(&(_, c)) = chars.get(i) {
                if extends(c) {
                    i += 1;
                    if c == '\u{200D}'
                        && pictographic(base)
                        && chars.get(i).is_some_and(|n| pictographic(n.1))
                    {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
        }
        out.push(chars.get(i).map_or(text.len(), |c| c.0));
    }
    out.dedup();
    out
}

impl TextSystem for FakeText {
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
        let key = TextKey(spec.cache_key(scale));
        let layout = Self::layout(spec);
        (
            key,
            (layout.width, layout.lines.len() as f32 * layout.line_h),
        )
    }

    fn caret(&mut self, spec: &Spec, _scale: f32, offset: usize) -> CaretPos {
        let layout = Self::layout(spec);
        let text = joined_text(spec);
        let off = clamp_to_boundary(&text, offset);
        let line = layout
            .lines
            .iter()
            .rposition(|l| l.0 <= off)
            .unwrap_or_default();
        let (_, end, glyphs) = &layout.lines[line];
        let off = off.min(*end);
        CaretPos {
            x: glyphs.iter().take_while(|g| g.at < off).map(|g| g.w).sum(),
            y: line as f32 * layout.line_h,
            height: layout.line_h,
            line,
        }
    }

    fn hit(&mut self, spec: &Spec, _scale: f32, pos: (f32, f32)) -> usize {
        if !(pos.0.is_finite() && pos.1.is_finite()) {
            return 0;
        }
        let layout = Self::layout(spec);
        let row = (pos.1 / layout.line_h).floor().max(0.0) as usize;
        let (start, end, glyphs) = &layout.lines[row.min(layout.lines.len() - 1)];
        let mut x = 0.0;
        for g in glyphs {
            if pos.0 < x + g.w / 2.0 {
                return g.at;
            }
            x += g.w;
        }
        if glyphs.is_empty() { *start } else { *end }
    }

    fn lines(&mut self, spec: &Spec, _scale: f32) -> Vec<LineSpan> {
        let layout = Self::layout(spec);
        layout
            .lines
            .iter()
            .enumerate()
            .map(|(i, l)| LineSpan {
                start: l.0,
                end: l.1,
                y: i as f32 * layout.line_h,
                height: layout.line_h,
            })
            .collect()
    }

    fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
        let graphemes = clusters(text);
        if unit == Unit::Grapheme {
            return graphemes;
        }
        #[derive(PartialEq, Clone, Copy)]
        enum Kind {
            Word,
            Space,
            Other,
        }
        let kind = |at: usize| match text[at..].chars().next() {
            Some(c) if c.is_alphanumeric() => Kind::Word,
            Some(c) if c.is_whitespace() => Kind::Space,
            _ => Kind::Other,
        };
        let mut out = vec![0];
        let mut prev: Option<Kind> = None;
        for &at in &graphemes[..graphemes.len() - 1] {
            let k = kind(at);
            if at != 0 && (k == Kind::Other || prev != Some(k)) {
                out.push(at);
            }
            prev = Some(k);
        }
        if text.is_empty() {
            return out;
        }
        out.push(text.len());
        out
    }
}

fn check(ok: bool, property: &str, detail: impl FnOnce() -> String) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(format!("{property}: {}", detail()))
    }
}

fn finite_size(property: &str, size: (f32, f32)) -> Result<(), String> {
    check(
        size.0.is_finite() && size.1.is_finite() && size.0 >= 0.0 && size.1 >= 0.0,
        property,
        || format!("size {size:?} is not finite and non-negative"),
    )
}

/// Checks the properties every [`TextSystem`] must have; the error names the first that fails.
///
/// - P1: preparing the same spec at the same scale twice gives the same key and size;
/// - P2: the same spec at scales 1 and 2 gives different keys;
/// - P3: different text gives different keys;
/// - P4: `measure` equals `prepare`'s size;
/// - P5: an empty spec (no runs) has width 0 and a finite height ≥ 0;
/// - P6: "aa bb cc dd ee ff gg" at `Style::ui(13.0, …)` wrapped at 40 is at most 41 wide;
/// - P7: every size is finite and ≥ 0;
/// - P8: the queries on an empty spec and on empty text answer finite values (the caret at 0, one
///   line) and a `pos` that is not finite hits 0;
/// - P9: an offset past the end clamps to the end, one inside a char to the char's start;
/// - P10: the caret at 0 has `x == 0`, and at the end of one unwrapped line it has exactly the
///   width `prepare` returns;
/// - P11: for a system that lays text out (width above 0), `hit` of the caret of each boundary of
///   an ASCII line is that boundary;
/// - P12: on a three-line spec joined with `\n` and `\r\n` the lines start at the joined offsets, in
///   order, top to bottom, and the caret of each start is on its line at the line's top; a final
///   terminator opens one more, empty line;
/// - P13: `segments` of every [`SEGMENT_TABLE`] text is the table's boundaries, for both units;
/// - P14: `hit` keeps the column outside the lines: a point above the first line answers as that
///   line at the same `x`, a point below the last answers as the last, left and right of a line
///   are its first and last offset;
/// - P15: the queries and `prepare` agree on the lines, for hard and soft breaks and a final
///   terminator: `measure`'s height is the last row's bottom, the rows stack with no gap, the caret
///   of each row's start is on that row, the caret at the end is on the last row, and `hit` at each
///   row's left edge is its start;
/// - P16: a line break is one break, whatever the rest of the text holds: the rows of a table of
///   ASCII and non-ASCII texts are the table's byte ranges, and around each break the caret at and
///   inside it is at the row's end and the caret after it at the next row's start;
/// - P17: along a row the caret never moves left as the offset grows, also through a run of spaces
///   at a wrap;
/// - P18: on any row, `hit` of the caret of a grapheme boundary is that boundary.
pub fn text_system_contract(ts: &mut dyn TextSystem) -> Result<(), String> {
    let white = [255; 4];
    let samples = [
        Spec::plain("Hello", Style::ui(13.0, white)),
        Spec::plain("Hello world", Style::ui(15.0, white).bold()),
        Spec::plain("fn main() {}", Style::mono(12.0, 16.0, white)),
        Spec {
            runs: vec![
                ("Two ".to_owned(), Style::ui(13.0, white)),
                ("runs".to_owned(), Style::ui(13.0, white).medium()),
            ],
            wrap: None,
        },
        Spec::plain("A longer line that has to wrap", Style::ui(13.0, white)).wrapped(80.0),
    ];
    let mut sizes = Vec::new();

    for spec in &samples {
        let (k1, s1) = ts.prepare(spec, 1.0);
        let (k2, s2) = ts.prepare(spec, 1.0);
        check(k1 == k2 && s1 == s2, "P1 same spec and scale", || {
            format!("{k1:?} {s1:?} then {k2:?} {s2:?}")
        })?;
        sizes.push(s1);
    }

    for spec in &samples {
        let (k1, _) = ts.prepare(spec, 1.0);
        let (k2, s2) = ts.prepare(spec, 2.0);
        check(k1 != k2, "P2 scales 1 and 2", || format!("both {k1:?}"))?;
        sizes.push(s2);
    }

    let a = Spec::plain("alpha", Style::ui(13.0, white));
    let b = Spec::plain("bravo", Style::ui(13.0, white));
    let (ka, _) = ts.prepare(&a, 1.0);
    let (kb, _) = ts.prepare(&b, 1.0);
    check(ka != kb, "P3 different text", || format!("both {ka:?}"))?;

    for spec in &samples {
        for scale in [1.0, 2.0] {
            let (_, prepared) = ts.prepare(spec, scale);
            let measured = ts.measure(spec, scale);
            check(measured == prepared, "P4 measure equals prepare", || {
                format!("measure {measured:?}, prepare {prepared:?} at scale {scale}")
            })?;
        }
    }

    let empty = Spec {
        runs: Vec::new(),
        wrap: None,
    };
    let (_, (w, h)) = ts.prepare(&empty, 1.0);
    check(
        w == 0.0 && h.is_finite() && h >= 0.0,
        "P5 empty spec",
        || format!("size ({w}, {h})"),
    )?;
    sizes.push((w, h));

    let wrapped = Spec::plain("aa bb cc dd ee ff gg", Style::ui(13.0, white)).wrapped(40.0);
    for scale in [1.0, 2.0] {
        let (_, size) = ts.prepare(&wrapped, scale);
        check(size.0 <= 41.0, "P6 wrap at 40", || {
            format!("width {} at scale {scale}", size.0)
        })?;
        sizes.push(size);
    }

    for size in sizes {
        finite_size("P7 finite sizes", size)?;
    }
    query_contract(ts)?;
    segment_contract(ts)
}

/// A text, and where its grapheme and word boundaries are, in bytes.
#[derive(Debug, Clone, Copy)]
pub struct SegmentCase {
    pub text: &'static str,
    pub graphemes: &'static [usize],
    pub words: &'static [usize],
}

/// Boundaries every [`TextSystem::segments`] must give. The expectations are worked out by hand;
/// the text cache's tests check them against the real UAX #29 implementation.
pub const SEGMENT_TABLE: &[SegmentCase] = &[
    SegmentCase {
        text: "",
        graphemes: &[0],
        words: &[0],
    },
    // e + combining acute (2 bytes), x
    SegmentCase {
        text: "e\u{301}x",
        graphemes: &[0, 3, 4],
        words: &[0, 4],
    },
    // thumbs up + medium skin tone
    SegmentCase {
        text: "\u{1F44D}\u{1F3FD}",
        graphemes: &[0, 8],
        words: &[0, 8],
    },
    // man ZWJ woman ZWJ girl
    SegmentCase {
        text: "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
        graphemes: &[0, 18],
        words: &[0, 18],
    },
    SegmentCase {
        text: "h\u{e9}llo w\u{f6}rld",
        graphemes: &[0, 1, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13],
        words: &[0, 6, 7, 13],
    },
    SegmentCase {
        text: "a  b",
        graphemes: &[0, 1, 2, 3, 4],
        words: &[0, 1, 3, 4],
    },
    SegmentCase {
        text: "a\r\nb",
        graphemes: &[0, 1, 3, 4],
        words: &[0, 1, 3, 4],
    },
    SegmentCase {
        text: "ab, cd",
        graphemes: &[0, 1, 2, 3, 4, 5, 6],
        words: &[0, 2, 3, 4, 6],
    },
];

fn segment_contract(ts: &dyn TextSystem) -> Result<(), String> {
    for case in SEGMENT_TABLE {
        for (unit, want) in [(Unit::Grapheme, case.graphemes), (Unit::Word, case.words)] {
            let got = ts.segments(case.text, unit);
            check(got == want, "P13 segments", || {
                format!("{:?} {unit:?}: {got:?}, expected {want:?}", case.text)
            })?;
        }
    }
    Ok(())
}

fn query_contract(ts: &mut dyn TextSystem) -> Result<(), String> {
    let white = [255; 4];
    let style = Style::ui(14.0, white);

    // P8
    let empty_spec = Spec {
        runs: Vec::new(),
        wrap: None,
    };
    for spec in [&empty_spec, &Spec::plain("", style)] {
        let c = ts.caret(spec, 1.0, 0);
        check(
            c.x == 0.0 && c.y == 0.0 && c.height.is_finite() && c.height >= 0.0 && c.line == 0,
            "P8 caret on empty text",
            || format!("{c:?}"),
        )?;
        let lines = ts.lines(spec, 1.0);
        check(
            lines.len() == 1 && lines[0].start == 0 && lines[0].end == 0,
            "P8 lines of empty text",
            || format!("{lines:?}"),
        )?;
        check(
            ts.hit(spec, 1.0, (5.0, 5.0)) == 0,
            "P8 hit on empty text",
            String::new,
        )?;
    }
    let hello = Spec::plain("Hello world", style);
    for pos in [
        (f32::NAN, 0.0),
        (0.0, f32::NAN),
        (f32::INFINITY, 0.0),
        (0.0, f32::NEG_INFINITY),
    ] {
        check(ts.hit(&hello, 1.0, pos) == 0, "P8 non-finite pos", || {
            format!("{pos:?}")
        })?;
    }

    // P9: "aéb" has the 2-byte é at 1..3.
    let acc = Spec::plain("a\u{e9}b", style);
    let at = |ts: &mut dyn TextSystem, o| ts.caret(&acc, 1.0, o);
    let (end, inside, start) = (at(ts, 4), at(ts, 2), at(ts, 1));
    check(
        at(ts, 99) == end && at(ts, 4) == end,
        "P9 past the end",
        || format!("{:?} vs {end:?}", at(ts, 99)),
    )?;
    check(inside == start, "P9 inside a char", || {
        format!("{inside:?} vs {start:?}")
    })?;

    // P10
    for scale in [1.0, 2.0] {
        for text in ["Hello world", "fn main() {}", "a\u{e9}b"] {
            let spec = Spec::plain(text, style);
            let width = ts.prepare(&spec, scale).1.0;
            let first = ts.caret(&spec, scale, 0);
            let last = ts.caret(&spec, scale, text.len());
            check(first.x == 0.0, "P10 caret at 0", || {
                format!("{text:?} at {scale}: {first:?}")
            })?;
            check(last.x == width, "P10 caret at the end", || {
                format!("{text:?} at {scale}: x {} vs width {width}", last.x)
            })?;
        }
    }

    // P11
    let line = Spec::plain("Hello world", style);
    if ts.prepare(&line, 1.0).1.0 > 0.0 {
        for scale in [1.0, 2.0] {
            for b in 0..=line.runs[0].0.len() {
                let c = ts.caret(&line, scale, b);
                let got = ts.hit(&line, scale, (c.x, c.y + c.height / 2.0));
                check(got == b, "P11 hit of a caret", || {
                    format!("boundary {b} at {scale}: caret {c:?} hits {got}")
                })?;
            }
        }
    }

    // P12
    let three = Spec::plain("ab\ncd\r\nef", style);
    let lines = ts.lines(&three, 1.0);
    let shaped = ts.prepare(&three, 1.0).1.0 > 0.0;
    if shaped {
        let starts: Vec<usize> = lines.iter().map(|l| l.start).collect();
        let ends: Vec<usize> = lines.iter().map(|l| l.end).collect();
        check(
            starts == [0, 3, 7] && ends == [2, 5, 9],
            "P12 line ranges",
            || format!("{lines:?}"),
        )?;
        check(
            lines.windows(2).all(|w| w[0].y < w[1].y) && lines.iter().all(|l| l.height > 0.0),
            "P12 line order",
            || format!("{lines:?}"),
        )?;
        for (i, l) in lines.iter().enumerate() {
            let c = ts.caret(&three, 1.0, l.start);
            check(
                c.line == i && c.x == 0.0 && c.y == l.y,
                "P12 caret at a line start",
                || format!("line {i} {l:?}: {c:?}"),
            )?;
        }
        // A final terminator opens one more, empty line.
        let open = Spec::plain("ab\n", style);
        let lines = ts.lines(&open, 1.0);
        let ranges: Vec<(usize, usize)> = lines.iter().map(|l| (l.start, l.end)).collect();
        check(
            ranges == [(0, 2), (3, 3)],
            "P12 trailing terminator",
            || format!("{lines:?}"),
        )?;
        let c = ts.caret(&open, 1.0, 3);
        check(
            c.line == 1 && c.x == 0.0 && c.y == lines[1].y,
            "P12 caret after a trailing terminator",
            || format!("{c:?}"),
        )?;
        line_contract(ts, style)?;
    } else {
        check(
            !lines.is_empty()
                && lines
                    .iter()
                    .all(|l| l.y.is_finite() && l.height.is_finite()),
            "P12 lines without layout",
            || format!("{lines:?}"),
        )?;
    }
    Ok(())
}

/// The texts of P14 to P18: hard breaks of each kind, with and without a final terminator, each
/// in ASCII and with a non-ASCII char.
const LINE_TEXTS: &[&str] = &[
    "a",
    "a\nb",
    "a\nb\n",
    "a\nb\n\n",
    "\n",
    "\n\n",
    "a\n\nb",
    "a\r\n",
    "a\r\nb\r\n",
    "Hello world\nsecond",
    "\u{e9}\r\nb",
    "\u{e9}\r\nb\r\n",
    "\u{e9}b\r\ncd\r\nef",
    "\r\n\u{e9}\r\n\r\n",
    "a\rb",
    "\u{e9}\rb",
    "a\r",
    "a\r\r\nb",
    "\u{e9}\n\rb",
];

/// The rows of a text as byte ranges, for P16.
const BREAK_ROWS: &[(&str, &[(usize, usize)])] = &[
    ("ab\r\ncd", &[(0, 2), (4, 6)]),
    ("\u{e9}b\r\ncd", &[(0, 3), (5, 7)]),
    ("\u{e9}b\r\ncd\r\nef", &[(0, 3), (5, 7), (9, 11)]),
    ("\u{e9}\r\nb\r\n", &[(0, 2), (4, 5), (7, 7)]),
    ("a\rb", &[(0, 1), (2, 3)]),
    ("\u{e9}\rb", &[(0, 2), (3, 4)]),
    ("a\r\r\nb", &[(0, 1), (2, 2), (4, 5)]),
    ("\u{e9}\r\r\nb", &[(0, 2), (3, 3), (5, 6)]),
];

/// The specs of P14 to P18.
fn line_specs(style: Style) -> Vec<Spec> {
    let mut specs: Vec<Spec> = LINE_TEXTS.iter().map(|t| Spec::plain(*t, style)).collect();
    specs.push(Spec::plain("aa bb cc dd ee ff gg", style).wrapped(40.0));
    specs.push(Spec::plain("aa bb cc dd ee ff gg\n", style).wrapped(40.0));
    specs.push(Spec::plain("hello world again      x", style).wrapped(120.0));
    specs.push(Spec::plain("hello w\u{f6}rld again        x y", style).wrapped(120.0));
    specs.push(split_break(style));
    specs
}

/// A CRLF split across two runs.
fn split_break(style: Style) -> Spec {
    Spec {
        runs: vec![("\u{e9}\r".to_owned(), style), ("\nb".to_owned(), style)],
        wrap: None,
    }
}

fn row_ranges(ts: &mut dyn TextSystem, spec: &Spec, scale: f32) -> Vec<(usize, usize)> {
    ts.lines(spec, scale)
        .iter()
        .map(|l| (l.start, l.end))
        .collect()
}

/// P16's table and P14 to P18 on the contract's specs.
fn line_contract(ts: &mut dyn TextSystem, style: Style) -> Result<(), String> {
    for (text, want) in BREAK_ROWS {
        let got = row_ranges(ts, &Spec::plain(*text, style), 1.0);
        check(got == *want, "P16 rows of a text", || {
            format!("{text:?}: {got:?}, expected {want:?}")
        })?;
    }
    let got = row_ranges(ts, &split_break(style), 1.0);
    check(
        got == [(0, 2), (4, 5)],
        "P16 a CRLF across two runs",
        || format!("{got:?}"),
    )?;
    for spec in line_specs(style) {
        spec_line_contract(ts, &spec)?;
    }
    Ok(())
}

/// P14, P15, the caret around each break of P16, P17 and P18 for one `spec`, at scales 1 and 2. For a
/// system that lays text out; [`text_system_contract`] runs it on its own specs, and an adapter's
/// tests may run it on generated ones.
pub fn spec_line_contract(ts: &mut dyn TextSystem, spec: &Spec) -> Result<(), String> {
    for scale in [1.0, 2.0] {
        hit_outside_the_lines(ts, spec, scale)?;
        rows_agree(ts, spec, scale)?;
        caret_around_breaks(ts, spec, scale)?;
        caret_along_rows(ts, spec, scale)?;
        hit_of_the_caret(ts, spec, scale)?;
    }
    Ok(())
}

fn hit_of_the_caret(ts: &mut dyn TextSystem, spec: &Spec, scale: f32) -> Result<(), String> {
    let text = joined_text(spec);
    let lines = ts.lines(spec, scale);
    for at in ts.segments(&text, Unit::Grapheme) {
        let c = ts.caret(spec, scale, at);
        let row = lines[c.line];
        // Inside a break, and in whitespace dropped at a wrap, no point answers.
        if at < row.start || at > row.end {
            continue;
        }
        let got = ts.hit(spec, scale, (c.x, c.y + c.height / 2.0));
        check(got == at, "P18 hit of a caret on a row", || {
            format!("{text:?} at {scale}, offset {at}: {c:?} hits {got}")
        })?;
    }
    Ok(())
}

fn hit_outside_the_lines(ts: &mut dyn TextSystem, spec: &Spec, scale: f32) -> Result<(), String> {
    let text = joined_text(spec);
    let lines = ts.lines(spec, scale);
    let width = ts.prepare(spec, scale).1.0;
    let (first, last) = (lines[0], lines[lines.len() - 1]);
    for x in [0.0, width / 3.0, width / 2.0, width + 40.0] {
        let inside = ts.hit(spec, scale, (x, first.y + first.height / 2.0));
        let above = ts.hit(spec, scale, (x, first.y - 40.0));
        check(above == inside, "P14 hit above the first line", || {
            format!("{text:?} at {scale}, x {x}: {above}, on the line {inside}")
        })?;
        let inside = ts.hit(spec, scale, (x, last.y + last.height / 2.0));
        for y in [last.y + last.height, last.y + last.height + 40.0] {
            let below = ts.hit(spec, scale, (x, y));
            check(below == inside, "P14 hit below the last line", || {
                format!("{text:?} at {scale}, ({x}, {y}): {below}, on the line {inside}")
            })?;
        }
    }
    if spec.wrap.is_none() {
        for l in &lines {
            let mid = l.y + l.height / 2.0;
            let (left, right) = (
                ts.hit(spec, scale, (-40.0, mid)),
                ts.hit(spec, scale, (width + 40.0, mid)),
            );
            check(left == l.start, "P14 hit left of a line", || {
                format!("{text:?} at {scale}, {l:?}: {left}")
            })?;
            check(right == l.end, "P14 hit right of a line", || {
                format!("{text:?} at {scale}, {l:?}: {right}")
            })?;
        }
    }
    Ok(())
}

fn rows_agree(ts: &mut dyn TextSystem, spec: &Spec, scale: f32) -> Result<(), String> {
    let text = joined_text(spec);
    let lines = ts.lines(spec, scale);
    let last = lines[lines.len() - 1];
    let height = ts.measure(spec, scale).1;
    check(
        (height - (last.y + last.height)).abs() < 1e-3,
        "P15 measure and lines",
        || format!("{text:?} at {scale}: height {height}, {lines:?}"),
    )?;
    check(
        lines
            .windows(2)
            .all(|w| (w[0].y + w[0].height - w[1].y).abs() < 1e-3),
        "P15 rows stack",
        || format!("{text:?} at {scale}: {lines:?}"),
    )?;
    let end = ts.caret(spec, scale, text.len());
    check(
        end.line == lines.len() - 1 && end.y == last.y,
        "P15 caret at the end",
        || format!("{text:?} at {scale}: {end:?}, {lines:?}"),
    )?;
    for (i, l) in lines.iter().enumerate() {
        let c = ts.caret(spec, scale, l.start);
        check(c.line == i && c.y == l.y, "P15 caret at a row", || {
            format!("{text:?} at {scale}, row {i} {l:?}: {c:?}")
        })?;
        let got = ts.hit(spec, scale, (0.0, l.y + l.height / 2.0));
        check(got == l.start, "P15 hit at a row's left edge", || {
            format!("{text:?} at {scale}, row {i} {l:?}: {got}")
        })?;
    }
    Ok(())
}

/// The terminators of `text` as byte ranges: `\n`, `\r\n` or a lone `\r`.
fn breaks(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let len = match (bytes[i], bytes.get(i + 1)) {
            (b'\r', Some(b'\n')) => 2,
            (b'\r' | b'\n', _) => 1,
            _ => 0,
        };
        if len > 0 {
            out.push((i, i + len));
        }
        i += len.max(1);
    }
    out
}

fn caret_around_breaks(ts: &mut dyn TextSystem, spec: &Spec, scale: f32) -> Result<(), String> {
    let text = joined_text(spec);
    for (at, after) in breaks(&text) {
        let end = ts.caret(spec, scale, at);
        for inside in at + 1..after {
            let c = ts.caret(spec, scale, inside);
            check(c == end, "P16 caret inside a break", || {
                format!("{text:?} at {scale}, offset {inside}: {c:?}, the break's {end:?}")
            })?;
        }
        let next = ts.caret(spec, scale, after);
        check(
            next.x == 0.0 && next.line == end.line + 1,
            "P16 caret after a break",
            || format!("{text:?} at {scale}, offset {after}: {next:?}, before it {end:?}"),
        )?;
    }
    Ok(())
}

fn caret_along_rows(ts: &mut dyn TextSystem, spec: &Spec, scale: f32) -> Result<(), String> {
    let text = joined_text(spec);
    let mut prev: Option<(usize, CaretPos)> = None;
    for at in ts.segments(&text, Unit::Grapheme) {
        let c = ts.caret(spec, scale, at);
        if let Some((from, p)) = prev {
            check(c.line >= p.line, "P17 caret goes down the rows", || {
                format!("{text:?} at {scale}: {from} {p:?}, {at} {c:?}")
            })?;
            check(
                c.line != p.line || c.x >= p.x - 1e-3,
                "P17 caret along a row",
                || format!("{text:?} at {scale}: {from} {p:?}, {at} {c:?}"),
            )?;
        }
        prev = Some((at, c));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_text_meets_the_contract() {
        assert_eq!(text_system_contract(&mut FakeText), Ok(()));
    }

    #[test]
    fn fake_text_queries_follow_its_formula() {
        let style = Style::ui(10.0, [255; 4]);
        let mut t = FakeText;
        let spec = Spec::plain("ab\ncd\r\nef", style);
        let lines = t.lines(&spec, 1.0);
        assert_eq!(
            lines
                .iter()
                .map(|l| (l.start, l.end, l.y))
                .collect::<Vec<_>>(),
            [(0, 2, 0.0), (3, 5, 13.0), (7, 9, 26.0)]
        );
        // Two chars of 5 px, one line of 13 px, on three lines; the width is the widest line.
        assert_eq!(t.prepare(&spec, 1.0).1, (10.0, 39.0));
        let c = t.caret(&spec, 1.0, 4);
        assert_eq!((c.x, c.y, c.line), (5.0, 13.0, 1));
        // Left and right halves of the 'd', then past the end of the line, then below.
        assert_eq!(t.hit(&spec, 1.0, (7.0, 14.0)), 4);
        assert_eq!(t.hit(&spec, 1.0, (8.0, 14.0)), 5);
        assert_eq!(t.hit(&spec, 1.0, (50.0, 14.0)), 5);
        assert_eq!(t.hit(&spec, 1.0, (-3.0, 99.0)), 7);
    }

    #[test]
    fn fake_text_sizes_follow_its_formula() {
        let style = Style::ui(10.0, [255; 4]);
        let mut t = FakeText;
        assert_eq!(t.prepare(&Spec::plain("abcd", style), 1.0).1, (20.0, 13.0));
        assert_eq!(
            t.prepare(&Spec::plain("abcd", style).wrapped(8.0), 1.0).1,
            (8.0, 39.0)
        );
        let empty = Spec {
            runs: Vec::new(),
            wrap: Some(0.0),
        };
        assert_eq!(t.prepare(&empty, 2.0).1, (0.0, 17.0));
    }

    /// A system that answers the queries from a fixed table of nonsense is caught.
    struct Broken(FakeText);

    impl TextSystem for Broken {
        fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
            self.0.prepare(spec, scale)
        }
        fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos {
            // Forgets to clamp offsets inside a char.
            let mut c = self.0.caret(spec, scale, offset);
            if offset == 2 {
                c.x += 1.0;
            }
            c
        }
        fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize {
            self.0.hit(spec, scale, pos)
        }
        fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan> {
            self.0.lines(spec, scale)
        }
        fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
            self.0.segments(text, unit)
        }
    }

    /// What a shaping system does if nobody pins it: it forgets the line a final terminator opens,
    /// or answers the end of the text for a point below the last line.
    struct Drifting {
        short_extent: bool,
    }

    impl TextSystem for Drifting {
        fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
            let (key, (w, h)) = FakeText.prepare(spec, scale);
            let open = joined_text(spec).ends_with('\n');
            let short = self.short_extent && open;
            (key, (w, if short { h - spec.runs[0].1.line } else { h }))
        }
        fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos {
            FakeText.caret(spec, scale, offset)
        }
        fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize {
            let bottom = FakeText.measure(spec, scale).1;
            if !self.short_extent && pos.0.is_finite() && pos.1 >= bottom {
                return joined_text(spec).len();
            }
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
    fn the_contract_catches_a_hit_that_loses_the_column() {
        let mut t = Drifting {
            short_extent: false,
        };
        let err = text_system_contract(&mut t).expect_err("caught");
        assert!(err.starts_with("P14 hit below the last line"), "{err}");
    }

    #[test]
    fn the_contract_catches_a_height_without_the_open_line() {
        let mut t = Drifting { short_extent: true };
        let err = text_system_contract(&mut t).expect_err("caught");
        assert!(err.starts_with("P15 measure and lines"), "{err}");
    }

    /// A caret that falls back to the row's left edge in the last bytes of a row.
    struct Jumping;

    impl TextSystem for Jumping {
        fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
            FakeText.prepare(spec, scale)
        }
        fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos {
            let mut c = FakeText.caret(spec, scale, offset);
            let row = FakeText.lines(spec, scale)[c.line];
            if spec.wrap.is_some() && offset > row.start && offset + 1 == row.end {
                c.x = 0.0;
            }
            c
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
    fn the_contract_catches_a_caret_that_jumps_back_along_a_row() {
        let err = text_system_contract(&mut Jumping).expect_err("caught");
        assert!(err.starts_with("P17 caret along a row"), "{err}");
    }

    /// A shaper that makes a CRLF two breaks once the text is not ASCII, as `BidiInfo` does.
    struct DoubleBreak;

    impl DoubleBreak {
        fn twisted(spec: &Spec) -> Spec {
            let text = joined_text(spec);
            if text.is_ascii() || spec.runs.len() != 1 {
                return spec.clone();
            }
            Spec::plain(text.replace("\r\n", "\n\n"), spec.runs[0].1)
        }
    }

    impl TextSystem for DoubleBreak {
        fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
            FakeText.prepare(&Self::twisted(spec), scale)
        }
        fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos {
            FakeText.caret(&Self::twisted(spec), scale, offset)
        }
        fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize {
            FakeText.hit(&Self::twisted(spec), scale, pos)
        }
        fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan> {
            FakeText.lines(&Self::twisted(spec), scale)
        }
        fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
            FakeText.segments(text, unit)
        }
    }

    #[test]
    fn the_contract_catches_a_crlf_that_breaks_twice_in_non_ascii_text() {
        let err = text_system_contract(&mut DoubleBreak).expect_err("caught");
        assert!(err.starts_with("P16 rows of a text"), "{err}");
    }

    #[test]
    fn fake_text_rows_agree_with_its_height() {
        let style = Style::ui(10.0, [255; 4]);
        let mut t = FakeText;
        let spec = Spec::plain("abcd", style).wrapped(8.0);
        let rows: Vec<(usize, usize)> = t
            .lines(&spec, 1.0)
            .iter()
            .map(|l| (l.start, l.end))
            .collect();
        assert_eq!(rows, [(0, 2), (2, 3), (3, 4)]);
        // The offset where a row breaks belongs to the later row.
        assert_eq!(t.caret(&spec, 1.0, 2).line, 1);
        assert_eq!(t.hit(&spec, 1.0, (0.0, 14.0)), 2);
    }

    #[test]
    fn the_contract_catches_a_caret_that_does_not_clamp() {
        let err = text_system_contract(&mut Broken(FakeText)).expect_err("caught");
        assert!(err.starts_with("P9 inside a char"), "{err}");
    }
}
