//! The `code-block` page (`text.code-block`, D10): the read-only code view in a frame with a copy
//! button. The page plays the app: it owns the text and the highlight spans (the toolkit never
//! tokenizes, D45) and sizes the block with `code_block::height`. The Wane scene overrides the
//! defaults (13 over 20, padding 12 by 10, a 1 px ring) and colours an env block in the mock's
//! three tones.

use lune_ui::color::Rgba;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::code_block::{CodeBlockOptions, height};
use lune_ui::widgets::text_lines::HighlightSpan;

use super::{card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "code-block",
    title: "Code block",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The block's left and top edges, in window coordinates: inside the card, under its heading.
const AT: (f32, f32) = (34.0, 90.0);
const WIDTH: f32 = 520.0;
/// Where the copy button's centre is with the default padding: 10 in from the right, 10 down, and
/// half of its 21 px.
const BUTTON: (f32, f32) = (AT.0 + WIDTH - 10.0 - 10.5, AT.1 + 10.0 + 10.5);
/// Off the block, so no thumb or hover shows at the capture.
const AWAY: Ev = Ev::PointerAt(5.0, 5.0);

const KEYWORD: Rgba = [0x56, 0x9C, 0xD6, 0xFF];
const STRING: Rgba = [0xCE, 0x91, 0x78, 0xFF];
const COMMENT: Rgba = [0x6A, 0x99, 0x55, 0xFF];
const BAD: Rgba = [0xF1, 0x4C, 0x4C, 0xFF];

const CODE: &str = concat!(
    "// Reads the queue once and reports what it did.\n",
    "fn run(path: &Path, env: &Env) -> Result<Summary, Error> {\n",
    "    let config = Config::load_from(path, &env)?;\n",
    "    let label = format!(\"{done} of {total}\");\n",
    "    Ok(Summary { done, total, failed, label })\n",
    "}",
);

const WIDE: &str = "let line = \"a single line that runs on well past the right edge of the block, \
so the text scrolls sideways and the overlay thumb shows while the pointer is over it\";\n\
let next = 1;";

const ENV: &str = concat!(
    "\"env\": {\n",
    "  \"ANTHROPIC_BASE_URL\": \"http://127.0.0.1:4646\",\n",
    "  \"ANTHROPIC_AUTH_TOKEN\": \"wane-local-key\",\n",
    "  \"ANTHROPIC_DEFAULT_OPUS_MODEL\": \"claude-wane-opus\",\n",
    "  \"ANTHROPIC_DEFAULT_SONNET_MODEL\": \"claude-wane-sonnet\"\n",
    "}",
);

const PLAIN: &str = "cargo build --workspace\ncargo test --workspace\ncargo clippy --all-targets\n\
scripts/check-boundaries.sh\ncargo fmt --all -- --check";

#[derive(Clone, Copy, PartialEq)]
enum Colors {
    Plain,
    Syntax,
    Env,
    Bad,
}

/// The page plays the app.
struct Live {
    text: &'static str,
    colors: Colors,
    /// Wane's look: 13 over 20, padding 12 by 10, a 1 px ring.
    wane: bool,
}

const fn state(text: &'static str, colors: Colors) -> Live {
    Live {
        text,
        colors,
        wane: false,
    }
}

fn live() -> Box<dyn PageState> {
    Box::new(state(WIDE, Colors::Plain))
}

fn rest() -> Box<dyn PageState> {
    Box::new(state(PLAIN, Colors::Plain))
}

fn highlighted() -> Box<dyn PageState> {
    Box::new(state(CODE, Colors::Syntax))
}

fn wide() -> Box<dyn PageState> {
    Box::new(state(WIDE, Colors::Plain))
}

fn empty() -> Box<dyn PageState> {
    Box::new(state("", Colors::Plain))
}

fn bad_spans() -> Box<dyn PageState> {
    Box::new(state("0123456789\nsecond line\nthird", Colors::Bad))
}

fn wane() -> Box<dyn PageState> {
    Box::new(Live {
        wane: true,
        ..state(ENV, Colors::Env)
    })
}

/// A span of chars `start..end` of line `line`.
fn span(line: usize, start: usize, end: usize, color: Rgba) -> (usize, HighlightSpan) {
    (line, HighlightSpan { start, end, color })
}

/// The page's stand-in for a tokenizer: a comment, keywords and a string, by line.
fn syntax(text: &str) -> Vec<(usize, HighlightSpan)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let chars = |bytes: usize| line[..bytes].chars().count();
        if line.trim_start().starts_with("//") {
            out.push(span(i, 0, chars(line.len()), COMMENT));
            continue;
        }
        for word in ["fn", "let", "mut"] {
            for (at, _) in line.match_indices(word) {
                let before = at == 0 || !line.as_bytes()[at - 1].is_ascii_alphanumeric();
                let after = !line.as_bytes()[at + word.len()].is_ascii_alphanumeric();
                if before && after {
                    out.push(span(i, chars(at), chars(at + word.len()), KEYWORD));
                }
            }
        }
        if let (Some(a), Some(b)) = (line.find('"'), line.rfind('"'))
            && a < b
        {
            out.push(span(i, chars(a), chars(b + 1), STRING));
        }
    }
    out
}

/// Wane's tones: the key `fg_secondary`, the punctuation `fg_quaternary`, the value `fg_primary`
/// (no span).
fn env(text: &str) -> Vec<(usize, HighlightSpan)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let len = line.chars().count();
        let quotes: Vec<usize> = line
            .chars()
            .enumerate()
            .filter(|(_, c)| *c == '"')
            .map(|(at, _)| at)
            .collect();
        let tone = |from, to, color| (from < to).then(|| span(i, from, to, color));
        let spans = match quotes.as_slice() {
            [key_start, key_end, value_start, value_end] => [
                tone(*key_start, key_end + 1, THEME.fg_secondary),
                tone(key_end + 1, *value_start, THEME.fg_quaternary),
                tone(value_end + 1, len, THEME.fg_quaternary),
            ],
            [key_start, key_end] => [
                tone(*key_start, key_end + 1, THEME.fg_secondary),
                tone(key_end + 1, len, THEME.fg_quaternary),
                None,
            ],
            _ => [tone(0, len, THEME.fg_quaternary), None, None],
        };
        out.extend(spans.into_iter().flatten());
    }
    out
}

/// Spans no tidy tokenizer gives: past the end, empty, inverted, and a line that does not exist.
fn bad(_text: &str) -> Vec<(usize, HighlightSpan)> {
    vec![
        span(0, 3, 99, BAD),
        span(1, 5, 5, BAD),
        span(1, 9, 2, BAD),
        span(50, 0, 4, BAD),
    ]
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Code block");
        heading(ui, "CODE BLOCK", card.x + 24.0, card.y + 24.0);
        let spans = match self.colors {
            Colors::Plain => Vec::new(),
            Colors::Syntax => syntax(self.text),
            Colors::Env => env(self.text),
            Colors::Bad => bad(self.text),
        };
        let base = CodeBlockOptions {
            label: "Code",
            spans: &spans,
            ..CodeBlockOptions::default()
        };
        let opts = if self.wane {
            CodeBlockOptions {
                size: 13.0,
                line: 20.0,
                pad: (12.0, 10.0),
                border: true,
                ..base
            }
        } else {
            base
        };
        let h = height(ui, self.text, &opts);
        ui.code_block(
            "block",
            on_card(card, Rect::new(AT.0, AT.1, WIDTH, h)),
            self.text,
            &opts,
        );
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let samples: [(&str, &'static str, Colors); 6] = [
            ("Wide line", WIDE, Colors::Plain),
            ("Commands", PLAIN, Colors::Plain),
            ("Highlighted code", CODE, Colors::Syntax),
            ("Environment block", ENV, Colors::Env),
            (
                "Spans past the text",
                "0123456789\nsecond line\nthird",
                Colors::Bad,
            ),
            ("Empty", "", Colors::Plain),
        ];
        let names: Vec<&str> = samples.iter().map(|(n, ..)| *n).collect();
        let mut at = samples
            .iter()
            .position(|(_, t, c)| *t == self.text && *c == self.colors)
            .unwrap_or(0);
        if panel.choice(ui, "Text", &names, &mut at) {
            self.text = samples[at].1;
            self.colors = samples[at].2;
        }
        panel.toggle(ui, "Compact look", &mut self.wane);
    }
}

const fn scene(
    id: &'static str,
    state: fn() -> Box<dyn PageState>,
    script: &'static [Step],
    capture_ms: u32,
    expect: Expect,
) -> UiScene {
    UiScene {
        id,
        scale: 2.0,
        tier: MotionTier::Full,
        state,
        script,
        capture_ms,
        expect,
    }
}

const fn key(at_ms: u32, events: &'static [Ev]) -> Step {
    Step { at_ms, events }
}

const OVER_BUTTON: Ev = Ev::PointerAt(BUTTON.0, BUTTON.1);

/// A point on line `n` (from 0) of the text, `col` chars in, with the 11 px mono advance near 6.6.
const fn at(n: usize, col: usize) -> Ev {
    Ev::PointerAt(
        AT.0 + 10.0 + col as f32 * 6.6,
        AT.1 + 10.0 + (n as f32 + 0.5) * 14.0,
    )
}

const SCENES: &[UiScene] = &[
    scene("code-block-rest", rest, &[], 600, Expect::Settled),
    scene(
        "code-block-highlighted",
        highlighted,
        &[],
        600,
        Expect::Settled,
    ),
    // A line wider than the block, the pointer over it: the thumb.
    scene(
        "code-block-overflow",
        wide,
        &[key(0, &[Ev::PointerAt(AT.0 + 200.0, AT.1 + 40.0)])],
        600,
        Expect::Settled,
    ),
    scene(
        "code-block-hover-copy",
        rest,
        &[key(0, &[OVER_BUTTON])],
        600,
        Expect::Settled,
    ),
    // Tab to the text, Tab to the copy button: its ring.
    scene(
        "code-block-focus-copy",
        rest,
        &[key(0, &[Ev::Tab]), key(50, &[Ev::Tab, AWAY])],
        600,
        Expect::Settled,
    ),
    // From line 2 to line 3 by a drag.
    scene(
        "code-block-selection",
        rest,
        &[
            key(0, &[at(1, 4), Ev::Press]),
            key(100, &[at(2, 14)]),
            key(200, &[Ev::Release, AWAY]),
        ],
        700,
        Expect::Settled,
    ),
    // The check mark shows until 1,500 ms after the click: a frame is due then.
    scene(
        "code-block-copied",
        rest,
        &[
            key(0, &[OVER_BUTTON]),
            key(100, &[Ev::Click]),
            key(200, &[AWAY]),
        ],
        500,
        Expect::Waiting,
    ),
    scene("code-block-empty", empty, &[], 600, Expect::Settled),
    scene("code-block-bad-spans", bad_spans, &[], 600, Expect::Settled),
    // HOVER and the crossfade are `instant` (120 ms): half way at 60 ms.
    scene(
        "code-block-copy-hover-fade",
        rest,
        &[key(0, &[OVER_BUTTON])],
        60,
        Expect::Moving,
    ),
    scene(
        "code-block-copied-fade",
        rest,
        &[key(0, &[OVER_BUTTON]), key(100, &[Ev::Click])],
        160,
        Expect::Moving,
    ),
    scene("code-block-wane", wane, &[], 600, Expect::Settled),
];

/// What the bench replays: the pointer over the wide line, a sideways swipe both ways, a copy and
/// the end of its feedback.
const BENCH: &[Step] = &[
    key(0, &[Ev::PointerAt(AT.0 + 200.0, AT.1 + 40.0)]),
    key(
        100,
        &[Ev::Scroll {
            x: -200.0,
            y: 0.0,
            precise: true,
        }],
    ),
    key(
        300,
        &[Ev::Scroll {
            x: 200.0,
            y: 0.0,
            precise: true,
        }],
    ),
    key(500, &[OVER_BUTTON]),
    key(600, &[Ev::Click]),
    key(2_300, &[AWAY]),
];
