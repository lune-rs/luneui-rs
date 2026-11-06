//! The `code-editor-view` page: the code view (`text.code-editor-view`, D181) read-only and editable
//! over generated code. The live state and the bench hold a document of 1,000,000 lines, built once
//! per process; the scenes a few dozen lines. The page plays the app: it owns the documents, the
//! highlighter, the bands and the notices above the text, and pins its own overlay to a line.

use std::sync::OnceLock;

use lune_ui::code_doc::CodeDoc;
use lune_ui::color::Rgba;
use lune_ui::icon;
use lune_ui::input::{Modifiers, NamedKey};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::{Sense, Ui};
use lune_ui::widgets::code_editor::{ANCHOR_INSET, CodeOptions, LineBand};
use lune_ui::widgets::text_lines::{HighlightSpan, Highlighter, PlainHighlighter};

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "code-editor-view",
    title: "Code editor view",
    kind: PageKind::Ui(UiPage {
        state: big,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The view, in window coordinates: inside the card, under the header row. 24 lines are in view.
const RECT: Rect = Rect::new(34.0, 116.0, 972.0, 480.0);
/// The header row: the live state's toggle, or a notice above the text.
const BAR: Rect = Rect::new(34.0, 82.0, 972.0, 26.0);
/// Four panes side by side.
const PANE_GAP: f32 = 8.0;
const PANE_W: f32 = (RECT.w - 3.0 * PANE_GAP) / 4.0;
/// Two panes side by side.
const HALF_W: f32 = (RECT.w - PANE_GAP) / 2.0;

const KEYWORD: Rgba = [0x56, 0x9C, 0xD6, 0xFF];
const STRING: Rgba = [0xCE, 0x91, 0x78, 0xFF];
const COMMENT: Rgba = [0x6A, 0x99, 0x55, 0xFF];
/// Lunegit's tints that have no token: a modified and a conflicted line.
const MODIFIED: Rgba = [0xE2, 0xB9, 0x3D, 0x1A];
const CONFLICTED: Rgba = [0xF1, 0x4C, 0x4C, 0x1A];

const CODE: [&str; 12] = [
    "// Reads the queue once and reports what it did.",
    "fn run(path: &Path, env: &Env) -> Result<Summary, Error> {",
    "    let config = Config::load_from(path, &env)?;",
    "    let mut state = State::new();",
    "    let (mut done, mut total, mut failed) = (0, 0, 0);",
    "",
    "    for item in queue.drain(..) {",
    "        handle(item, &mut state, &cache)?;",
    "    }",
    "    let label = format!(\"{done} of {total}\");",
    "    Ok(Summary { done, total, failed, label })",
    "}",
];

fn code(lines: usize) -> CodeDoc {
    let all: Vec<&str> = CODE.iter().cycle().take(lines).copied().collect();
    CodeDoc::new(all.join("\n"))
}

/// The live document: clones are cheap beside building it.
fn million() -> CodeDoc {
    static DOC: OnceLock<CodeDoc> = OnceLock::new();
    DOC.get_or_init(|| code(1_000_000)).clone()
}

fn long_note() -> CodeDoc {
    CodeDoc::new(
        "Notes on the proxy\n\nThe request to the host failed after every retry, because the proxy \
         tried each upstream in the pool and none of them answered within the deadline, so the \
         call gave up and reported the last error it saw.\n\nNext: read the upstream's logs, then \
         raise the deadline for the slow pool only, and leave the others as they are.",
    )
}

/// The page's stand-in for a tokenizer: keywords, one string and a comment, by line.
struct Syntax;

impl Highlighter for Syntax {
    fn spans(&self, _line: usize, text: &str) -> Vec<HighlightSpan> {
        let chars = |bytes: usize| text[..bytes].chars().count();
        let trimmed = text.trim_start();
        if trimmed.starts_with("//") {
            return vec![HighlightSpan {
                start: chars(text.len() - trimmed.len()),
                end: chars(text.len()),
                color: COMMENT,
            }];
        }
        let mut out = Vec::new();
        for word in ["fn ", "let ", "mut ", "for ", "in "] {
            for (at, _) in text.match_indices(word) {
                let edge = at == 0 || !text.as_bytes()[at - 1].is_ascii_alphanumeric();
                if edge {
                    out.push(HighlightSpan {
                        start: chars(at),
                        end: chars(at + word.len() - 1),
                        color: KEYWORD,
                    });
                }
            }
        }
        if let (Some(a), Some(b)) = (text.find('"'), text.rfind('"'))
            && a < b
        {
            out.push(HighlightSpan {
                start: chars(a),
                end: chars(b + 1),
                color: STRING,
            });
        }
        out
    }
}

/// Spans no tidy tokenizer gives: past the end, empty, inverted.
struct BadSpans;

impl Highlighter for BadSpans {
    fn spans(&self, _line: usize, _text: &str) -> Vec<HighlightSpan> {
        let span = |start, end| HighlightSpan {
            start,
            end,
            color: STRING,
        };
        vec![span(4, 9_999), span(2, 2), span(9, 1)]
    }
}

/// What the page lays out.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// One view, under `Live::notice`.
    One,
    /// Three read-only panes that scroll together and an editable one.
    Synced,
    /// Two editable panes, each with an overlay on line 7.
    Anchored,
}

#[derive(Clone, Copy, PartialEq)]
enum Colors {
    Plain,
    Syntax,
    Bad,
}

/// The page plays the app.
struct Live {
    docs: Vec<CodeDoc>,
    shape: Shape,
    read_only: bool,
    wrap: bool,
    gutter: bool,
    colors: Colors,
    bands: Vec<(usize, Band)>,
    notice: Option<&'static str>,
    /// The live state's toggle shows.
    toggle: bool,
}

#[derive(Clone, Copy)]
enum Band {
    Added,
    Modified,
    Conflicted,
}

fn state(doc: CodeDoc) -> Live {
    Live {
        docs: vec![doc],
        shape: Shape::One,
        read_only: true,
        wrap: false,
        gutter: true,
        colors: Colors::Plain,
        bands: Vec::new(),
        notice: None,
        toggle: false,
    }
}

fn big() -> Box<dyn PageState> {
    Box::new(Live {
        toggle: true,
        colors: Colors::Syntax,
        ..state(million())
    })
}

fn readonly() -> Box<dyn PageState> {
    Box::new(state(code(40)))
}

fn highlighted() -> Box<dyn PageState> {
    Box::new(Live {
        colors: Colors::Syntax,
        ..state(code(40))
    })
}

fn editable() -> Box<dyn PageState> {
    Box::new(Live {
        read_only: false,
        ..state(code(40))
    })
}

fn wrapped() -> Box<dyn PageState> {
    Box::new(Live {
        read_only: false,
        wrap: true,
        gutter: false,
        ..state(long_note())
    })
}

fn banded() -> Box<dyn PageState> {
    Box::new(Live {
        bands: vec![
            (4, Band::Added),
            (5, Band::Added),
            (6, Band::Added),
            (9, Band::Modified),
            (12, Band::Conflicted),
        ],
        ..state(code(40))
    })
}

fn clamped() -> Box<dyn PageState> {
    Box::new(Live {
        bands: vec![
            (0, Band::Added),
            (3, Band::Added),
            (9_999, Band::Conflicted),
        ],
        ..state(code(5))
    })
}

fn synced() -> Box<dyn PageState> {
    Box::new(Live {
        docs: (0..4).map(|_| code(120)).collect(),
        shape: Shape::Synced,
        ..state(CodeDoc::default())
    })
}

fn anchored() -> Box<dyn PageState> {
    Box::new(Live {
        docs: (0..2).map(|_| code(120)).collect(),
        shape: Shape::Anchored,
        read_only: false,
        ..state(CodeDoc::default())
    })
}

fn truncated() -> Box<dyn PageState> {
    Box::new(Live {
        notice: Some("luneui.code-editor.truncated"),
        ..state(code(40))
    })
}

fn highlight_off() -> Box<dyn PageState> {
    Box::new(Live {
        notice: Some("luneui.code-editor.highlight-disabled"),
        ..state(code(40))
    })
}

fn empty() -> Box<dyn PageState> {
    Box::new(state(CodeDoc::default()))
}

fn bad_spans() -> Box<dyn PageState> {
    Box::new(Live {
        colors: Colors::Bad,
        ..state(code(12))
    })
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Code editor view");
        heading(ui, "CODE EDITOR VIEW", card.x + 24.0, card.y + 24.0);
        if self.toggle {
            let mut edit = !self.read_only;
            let label = "Edit";
            ui.checkbox(
                "edit",
                Rect::new(BAR.x, BAR.y, 120.0, 24.0),
                &mut edit,
                label,
            );
            self.read_only = !edit;
        }
        if let Some(key) = self.notice {
            notice(ui, key);
        }
        match self.shape {
            Shape::One => self.one(ui),
            Shape::Synced => self.synced(ui),
            Shape::Anchored => self.anchored(ui),
        }
    }
}

/// Lunegit's strip above the text: padding 12 and 6, `surface_input`, a 13 px icon and 11 px text.
fn notice(ui: &mut Ui<'_>, key: &str) {
    let t = *ui.theme();
    ui.painter().fill(BAR, 0.0, t.surface_input);
    let mark = icon::glyph(icon::WARNING, 13.0, t.fg_secondary);
    let at = Rect::new(BAR.x + 12.0, BAR.y, 13.0, BAR.h);
    ui.painter().text_centered(&mark, at);
    let text = ui.tr(key);
    let spec = Spec::plain(text, Style::ui(11.0, t.fg_secondary));
    let h = ui.painter().measure(&spec).1;
    ui.painter()
        .text(&spec, BAR.x + 31.0, BAR.y + (BAR.h - h) / 2.0);
}

impl Live {
    fn one(&mut self, ui: &mut Ui<'_>) {
        let t = *ui.theme();
        let bands: Vec<LineBand> = self
            .bands
            .iter()
            .map(|(line, band)| LineBand {
                line: *line,
                color: match band {
                    Band::Added => t.diff_added,
                    Band::Modified => MODIFIED,
                    Band::Conflicted => CONFLICTED,
                },
            })
            .collect();
        let highlighter: &dyn Highlighter = match self.colors {
            Colors::Plain => &PlainHighlighter,
            Colors::Syntax => &Syntax,
            Colors::Bad => &BadSpans,
        };
        let opts = CodeOptions {
            read_only: self.read_only,
            wrap: self.wrap,
            gutter: self.gutter,
            tab_inserts: !self.read_only,
            label: "Code",
            highlighter,
            bands: &bands,
            identity: Some(1),
            ..CodeOptions::default()
        };
        ui.code_editor("code", RECT, &mut self.docs[0], &opts);
    }

    fn synced(&mut self, ui: &mut Ui<'_>) {
        let t = *ui.theme();
        let side = CodeOptions {
            read_only: true,
            label: "Pane",
            color: Some(t.fg_secondary),
            scroll_group: Some("sides"),
            ..CodeOptions::default()
        };
        let result = CodeOptions {
            label: "Result",
            ..CodeOptions::default()
        };
        for (i, doc) in self.docs.iter_mut().enumerate() {
            let rect = pane(i, PANE_W);
            let opts = if i == 3 { &result } else { &side };
            ui.code_editor(("pane", i), rect, doc, opts);
        }
    }

    fn anchored(&mut self, ui: &mut Ui<'_>) {
        let t = *ui.theme();
        let opts = CodeOptions {
            label: "Result",
            ..CodeOptions::default()
        };
        for (i, doc) in self.docs.iter_mut().enumerate() {
            let key = ("pane", i);
            ui.code_editor(key, pane(i, HALF_W), doc, &opts);
            // Out of view there is no rect: nothing draws and nothing registers.
            if let Some(line) = ui.code_line_rect(key, 7) {
                let strip = Rect::new(line.right() - ANCHOR_INSET - 96.0, line.y, 96.0, line.h);
                let id = ui.id(("strip", i));
                ui.interact(id, strip, Sense::CLICK);
                ui.painter().fill(strip, 3.0, t.surface_input);
                let spec = Spec::plain("Accept", Style::ui(11.0, t.fg_primary));
                ui.painter().text_centered(&spec, strip);
            }
        }
    }
}

/// Pane `i` of panes `w` wide.
const fn pane(i: usize, w: f32) -> Rect {
    Rect::new(RECT.x + i as f32 * (w + PANE_GAP), RECT.y, w, RECT.h)
}

/// A point on line `n` (from 1) of the view at offset 0, `col` chars into the text.
const fn line(n: usize, col: usize) -> (f32, f32) {
    // The gutter of two digits at 6.5 a digit in the fake text; the real font is near it.
    (
        RECT.x + 27.0 + col as f32 * 6.5 + 1.0,
        RECT.y + (n as f32 - 0.5) * 20.0,
    )
}

const SHIFT: Modifiers = Modifiers {
    shift: true,
    control: false,
    alt: false,
    logo: false,
};
const NO_MODS: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: false,
};
/// Off the view, so no thumb shows at the capture.
const AWAY: Ev = Ev::PointerAt(5.0, 5.0);

const fn scene(
    id: &'static str,
    state: fn() -> Box<dyn PageState>,
    script: &'static [Step],
    capture_ms: u32,
) -> UiScene {
    UiScene {
        id,
        scale: 2.0,
        tier: MotionTier::Full,
        state,
        script,
        capture_ms,
        expect: Expect::Settled,
    }
}

const fn key(at_ms: u32, events: &'static [Ev]) -> Step {
    Step { at_ms, events }
}

const SCENES: &[UiScene] = &[
    scene("code-editor-view-readonly", readonly, &[], 600),
    scene("code-editor-view-highlighted", highlighted, &[], 600),
    // The caret on line 3.
    scene(
        "code-editor-view-editable",
        editable,
        &[
            key(0, &[Ev::PointerAt(line(3, 8).0, line(3, 8).1), Ev::Click]),
            key(100, &[AWAY]),
        ],
        600,
    ),
    // From line 3 to line 5.
    scene(
        "code-editor-view-selection",
        editable,
        &[
            key(0, &[Ev::PointerAt(line(3, 4).0, line(3, 4).1), Ev::Click]),
            key(
                100,
                &[
                    Ev::Mods(SHIFT),
                    Ev::PointerAt(line(5, 12).0, line(5, 12).1),
                    Ev::Click,
                ],
            ),
            key(200, &[Ev::Mods(NO_MODS), AWAY]),
        ],
        800,
    ),
    scene(
        "code-editor-view-wrapped",
        wrapped,
        &[key(100, &[]), key(200, &[])],
        600,
    ),
    scene("code-editor-view-bands", banded, &[], 600),
    scene("code-editor-view-bands-clamped", clamped, &[], 600),
    // The first pane scrolled: the next two follow, the editable one stays.
    scene(
        "code-editor-view-synced",
        synced,
        &[
            key(0, &[Ev::PointerAt(RECT.x + 100.0, RECT.y + 100.0)]),
            key(
                50,
                &[Ev::Scroll {
                    x: 0.0,
                    y: -130.0,
                    precise: true,
                }],
            ),
            key(150, &[AWAY]),
        ],
        800,
    ),
    // The left pane shows its overlay; the right one has scrolled line 7 out of view.
    scene(
        "code-editor-view-anchor",
        anchored,
        &[
            key(
                0,
                &[Ev::PointerAt(
                    RECT.x + HALF_W + PANE_GAP + 100.0,
                    RECT.y + 100.0,
                )],
            ),
            key(
                50,
                &[Ev::Scroll {
                    x: 0.0,
                    y: -200.0,
                    precise: true,
                }],
            ),
            key(150, &[AWAY]),
        ],
        800,
    ),
    scene("code-editor-view-truncated", truncated, &[], 600),
    scene("code-editor-view-highlight-off", highlight_off, &[], 600),
    scene("code-editor-view-empty", empty, &[], 600),
    scene("code-editor-view-bad-spans", bad_spans, &[], 600),
    // A Latin preedit (D90), so the bundled font shows it.
    scene(
        "code-editor-view-ime",
        editable,
        &[
            key(0, &[Ev::PointerAt(line(4, 8).0, line(4, 8).1), Ev::Click]),
            key(100, &[AWAY, Ev::ImePreedit("compose", Some((7, 7)))]),
        ],
        600,
    ),
    // Read-only with keyboard focus: the ring, no caret.
    scene(
        "code-editor-view-focus",
        readonly,
        &[key(0, &[Ev::Tab])],
        600,
    ),
];

const WHEEL: Ev = Ev::Scroll {
    x: 0.0,
    y: -5.0,
    precise: false,
};
const WHEEL_BACK: Ev = Ev::Scroll {
    x: 0.0,
    y: 5.0,
    precise: false,
};

/// What the bench replays on 1,000,000 lines: wheel steps through the document, a far jump, then the
/// edit toggle and typing on a line.
const BENCH: &[Step] = &[
    key(0, &[Ev::PointerAt(line(6, 8).0, line(6, 8).1)]),
    key(100, &[Ev::Click]),
    key(200, &[WHEEL]),
    key(260, &[WHEEL]),
    key(320, &[WHEEL]),
    key(380, &[WHEEL_BACK]),
    key(700, &[Ev::Key(NamedKey::End)]),
    key(800, &[Ev::Key(NamedKey::Home)]),
    key(900, &[Ev::PointerAt(BAR.x + 8.0, BAR.y + 12.0), Ev::Click]),
    key(
        1_000,
        &[Ev::PointerAt(line(6, 8).0, line(6, 8).1), Ev::Click],
    ),
    key(1_100, &[Ev::Type("let ")]),
    key(1_200, &[Ev::Type("value = 1;")]),
    key(1_300, &[Ev::Key(NamedKey::Enter)]),
    key(1_400, &[Ev::Key(NamedKey::ArrowDown)]),
    key(1_500, &[Ev::Key(NamedKey::PageDown)]),
    key(1_600, &[Ev::Key(NamedKey::Backspace)]),
];
