//! The `diff-view` page: the diff body (`text.diff-view`, D15) unified and split over generated lines
//! (the live state and the bench use 100,000 rows; the scenes a few dozen). The page plays the app:
//! it owns the lines, the pairing, the mode, word wrap and the hunk actions, composes the header
//! (the mode switch and the wrap toggle) and opens the context menu, whose one row writes the
//! selected lines.

use lune_ui::icon;
use lune_ui::input::{Modifiers, NamedKey};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, ButtonOptions, button_width_with};
use lune_ui::widgets::diff_view::{
    DiffLine, DiffMark, DiffMode, DiffOptions, HunkAction, LineKind, SplitRow, pair_rows,
    widest_chars,
};
use lune_ui::widgets::menu::{MenuOptions, MenuPlace, MenuRow};

use super::{caption, card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "diff-view",
    title: "Diff view",
    kind: PageKind::Ui(UiPage {
        state: big,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The diff, in window coordinates: inside the card, under the header row. 24 rows are in view.
const RECT: Rect = Rect::new(34.0, 116.0, 972.0, 480.0);
/// The header row's top, and its buttons' height.
const BAR_Y: f32 = 82.0;
const BAR_H: f32 = 24.0;

/// Added and removed text, paired by index: the pair differs in the marked chars.
const OLD: [&str; 6] = [
    "    let config = Config::load(path)?;",
    "    if config.verbose { log::set_level(Level::Debug); }",
    "    for item in queue.drain(..) { handle(item, &mut state)?; }",
    "    state.flush().map_err(|e| Error::io(e))?;",
    "    let label = format!(\"{} of {}\", done, total);",
    "    return Ok(Summary { done, total, failed });",
];
const NEW: [&str; 6] = [
    "    let config = Config::load_from(path, &env)?;",
    "    if config.verbose { log::set_level(Level::Trace); }",
    "    for item in queue.drain(..) { handle(item, &mut state, &cache)?; }",
    "    state.flush().map_err(Error::io)?;",
    "    let label = format!(\"{done} of {total}\");",
    "    return Ok(Summary { done, total, failed, skipped });",
];
const CONTEXT: [&str; 6] = [
    "fn run(path: &Path, env: &Env) -> Result<Summary, Error> {",
    "    let mut state = State::new();",
    "    let (mut done, mut total, mut failed) = (0, 0, 0);",
    "",
    "    // Everything below reads the queue once.",
    "}",
];
/// The chars that differ between the first removed line and the first added one.
const OLD_MARKS: [DiffMark; 1] = [DiffMark { start: 24, end: 28 }];
const NEW_MARKS: [DiffMark; 2] = [
    DiffMark { start: 24, end: 33 },
    DiffMark { start: 38, end: 42 },
];

/// The lines of a diff, and what the app derives from them when they change.
struct Doc {
    lines: Vec<DiffLine<'static>>,
    rows: Vec<SplitRow>,
    widest: usize,
}

impl Doc {
    fn of(lines: Vec<DiffLine<'static>>) -> Doc {
        Doc {
            rows: pair_rows(&lines),
            widest: widest_chars(&lines),
            lines,
        }
    }
}

fn line(
    kind: LineKind,
    old: Option<u32>,
    new: Option<u32>,
    text: &'static str,
) -> DiffLine<'static> {
    DiffLine {
        old,
        new,
        ..DiffLine::new(kind, text)
    }
}

/// `n` lines of hunks of twelve lines: a header, three context, two removed, three added, three
/// context. The first header is always `@@ -1,9 +1,10 @@`.
fn build(n: usize) -> Doc {
    let mut lines = Vec::with_capacity(n + 12);
    let (mut old, mut new) = (1u32, 1u32);
    let mut hunk = 0u32;
    while lines.len() < n {
        let head: &'static str = Box::leak(format!("@@ -{old},9 +{new},10 @@").into_boxed_str());
        lines.push(DiffLine {
            hunk: Some(hunk),
            ..DiffLine::new(LineKind::Hunk, head)
        });
        let pick = |k: u32| (hunk + k) as usize % 6;
        for k in 0..3 {
            lines.push(line(
                LineKind::Context,
                Some(old),
                Some(new),
                CONTEXT[pick(k)],
            ));
            old += 1;
            new += 1;
        }
        for k in 0..2 {
            let mut l = line(LineKind::Removed, Some(old), None, OLD[pick(k)]);
            if hunk == 0 && k == 0 {
                l.marks = &OLD_MARKS;
            }
            lines.push(l);
            old += 1;
        }
        for k in 0..3 {
            let mut l = line(LineKind::Added, None, Some(new), NEW[pick(k)]);
            if hunk == 0 && k == 0 {
                l.marks = &NEW_MARKS;
            }
            lines.push(l);
            new += 1;
        }
        for k in 3..6 {
            lines.push(line(
                LineKind::Context,
                Some(old),
                Some(new),
                CONTEXT[pick(k)],
            ));
            old += 1;
            new += 1;
        }
        hunk += 1;
    }
    lines.truncate(n);
    Doc::of(lines)
}

/// A line of 260 chars, among shorter ones.
fn with_long_line() -> Doc {
    let long: &'static str = Box::leak(
        "    let message = format!(\"the request to {host} failed after {tries} tries: {reason}; \
         the proxy tried every upstream in the pool and none of them answered within the \
         deadline, so the call gives up and reports the last error it saw\", host = host);"
            .to_owned()
            .into_boxed_str(),
    );
    let mut doc = build(8);
    doc.lines[5] = line(LineKind::Added, None, Some(5), long);
    doc.lines[4] = line(LineKind::Removed, Some(5), None, long);
    Doc::of(doc.lines)
}

/// The page plays the app.
struct Live {
    doc: Doc,
    split: bool,
    wrap: bool,
    /// How many of the two hunk actions show.
    actions: usize,
    enabled: bool,
    /// The context menu's place while it is open.
    menu: Option<(f32, f32)>,
    /// The last hunk action pressed, shown beside the header row.
    pressed: Option<(u32, usize)>,
}

fn state(doc: Doc) -> Live {
    Live {
        doc,
        split: false,
        wrap: false,
        actions: 0,
        enabled: true,
        menu: None,
        pressed: None,
    }
}

fn big() -> Box<dyn PageState> {
    Box::new(Live {
        actions: 1,
        ..state(build(100_000))
    })
}

fn unified() -> Box<dyn PageState> {
    Box::new(state(build(40)))
}

fn split() -> Box<dyn PageState> {
    Box::new(Live {
        split: true,
        ..state(build(40))
    })
}

fn intraline() -> Box<dyn PageState> {
    Box::new(state(build(12)))
}

fn hunk_actions() -> Box<dyn PageState> {
    Box::new(Live {
        actions: 1,
        ..state(build(40))
    })
}

fn wrapped() -> Box<dyn PageState> {
    Box::new(Live {
        wrap: true,
        ..state(with_long_line())
    })
}

fn wide() -> Box<dyn PageState> {
    Box::new(state(with_long_line()))
}

fn empty() -> Box<dyn PageState> {
    Box::new(state(Doc::of(Vec::new())))
}

fn scrolled() -> Box<dyn PageState> {
    Box::new(state(build(1_000)))
}

fn disabled() -> Box<dyn PageState> {
    Box::new(Live {
        actions: 2,
        enabled: false,
        ..state(build(40))
    })
}

const ACTIONS: [HunkAction<'static>; 2] = [
    HunkAction {
        glyph: icon::ADD,
        label: "Stage hunk",
    },
    HunkAction {
        glyph: icon::REMOVE,
        label: "Unstage hunk",
    },
];

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Diff view");
        heading(ui, "DIFF VIEW", card.x + 24.0, card.y + 24.0);
        self.header(ui, card);

        let mode = if self.split {
            DiffMode::Split(&self.doc.rows)
        } else {
            DiffMode::Unified
        };
        let opts = DiffOptions {
            mode,
            wrap: self.wrap,
            label: "Changes",
            actions: &ACTIONS[..self.actions],
            identity: 1,
            widest_chars: self.doc.widest,
        };
        let lines = &self.doc.lines;
        let r = ui.with_enabled(self.enabled, |ui| {
            ui.diff_view("diff", on_card(card, RECT), lines, &opts)
        });
        if r.hunk.is_some() {
            self.pressed = r.hunk;
        }
        if r.context.is_some() {
            self.menu = ui.pointer_pos();
        }
        if let Some(at) = self.menu {
            let rows = [MenuRow::Item {
                glyph: Some(icon::COPY),
                text: ui.tr("luneui.diff-view.copy-selected-lines"),
                shortcut: "Cmd+C",
            }];
            let place = MenuOptions {
                place: MenuPlace::At(at),
                anchor_id: None,
                name: "Diff actions",
            };
            let m = ui.menu_with("menu", &rows, &place);
            if m.picked == Some(0) {
                let text = ui.diff_selected_text("diff", &self.doc.lines, mode);
                ui.clipboard_write(text);
            }
            if m.picked.is_some() || m.dismissed {
                self.menu = None;
            }
        }
    }
}

impl Live {
    /// The mode switch, the wrap toggle and what the last hunk action was.
    fn header(&mut self, ui: &mut Ui<'_>, card: Rect) {
        let rect = on_card(card, RECT);
        let bar_y = on_card(card, Rect::new(0.0, BAR_Y, 0.0, 0.0)).y;
        let mut x = rect.x;
        for (key, text, split) in [
            ("unified", ui.tr("luneui.diff-view.unified"), false),
            ("split", ui.tr("luneui.diff-view.side-by-side"), true),
        ] {
            let kind = if self.split == split {
                ButtonKind::Primary
            } else {
                ButtonKind::Secondary
            };
            let options = ButtonOptions {
                kind,
                small: true,
                icon: None,
            };
            let w = button_width_with(ui.painter(), text, &options);
            let r = ui.button_with(key, Rect::new(x, bar_y, w, BAR_H), text, &options);
            if r.clicked {
                self.split = split;
            }
            x += w + 8.0;
        }
        let wrap = ui.tr("luneui.diff-view.word-wrap");
        ui.checkbox(
            "wrap",
            Rect::new(x + 8.0, bar_y, 140.0, BAR_H),
            &mut self.wrap,
            wrap,
        );
        if let Some((hunk, action)) = self.pressed {
            let text = format!("{}: hunk {hunk}", ACTIONS[action].label);
            caption(ui, &text, rect.x + 560.0, bar_y + 6.0);
        }
    }
}

/// Row `i`'s centre (the text column) while the offset is 0.
const fn row(i: usize) -> (f32, f32) {
    (RECT.x + 400.0, RECT.y + (i as f32 + 0.5) * 20.0)
}

/// Row `i` of the left or the right pane.
const fn pane_row(right: bool, i: usize) -> (f32, f32) {
    let w = ((RECT.w - 1.0) / 2.0) as u32 as f32;
    let x = RECT.x + if right { w + 1.0 } else { 0.0 } + 200.0;
    (x, RECT.y + (i as f32 + 0.5) * 20.0)
}

/// A point on the first hunk's first action. The button's place follows the header's text width
/// (the fake text and the real font differ by up to 8 px), so the point is 12 px into it from the
/// fake text's left edge: inside the 18 px button for both.
const fn first_action() -> (f32, f32) {
    // pill = 16 chars at 4.5 plus 24; the group is the pill, 8 and 18; centred in the diff.
    let pill = 16.0 * 4.5 + 24.0;
    let left = RECT.x + RECT.w / 2.0 + pill / 2.0 - 5.0;
    (left + 12.0, RECT.y + 10.0)
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

const SCENES: &[UiScene] = &[
    scene("diff-view-unified", unified, &[], 600, Expect::Settled),
    scene("diff-view-split", split, &[], 600, Expect::Settled),
    // One action per hunk, the first hovered.
    scene(
        "diff-view-hunk-actions",
        hunk_actions,
        &[key(0, &[Ev::PointerAt(first_action().0, first_action().1)])],
        600,
        Expect::Settled,
    ),
    scene(
        "diff-view-hunk-pressed",
        hunk_actions,
        &[
            key(0, &[Ev::PointerAt(first_action().0, first_action().1)]),
            key(100, &[Ev::Press]),
        ],
        900,
        Expect::Settled,
    ),
    // Rows 4 to 7: a click on 4, a Shift-click on 7.
    scene(
        "diff-view-selection",
        unified,
        &[
            key(0, &[Ev::PointerAt(row(4).0, row(4).1), Ev::Click]),
            key(
                100,
                &[
                    Ev::Mods(SHIFT),
                    Ev::PointerAt(row(7).0, row(7).1),
                    Ev::Click,
                ],
            ),
            key(200, &[Ev::Mods(NO_MODS), Ev::PointerAt(5.0, 5.0)]),
        ],
        800,
        Expect::Settled,
    ),
    scene(
        "diff-view-wrapped",
        wrapped,
        &[key(100, &[]), key(200, &[])],
        600,
        Expect::Settled,
    ),
    scene(
        "diff-view-wide",
        wide,
        &[key(0, &[Ev::PointerAt(row(3).0, row(3).1)])],
        600,
        Expect::Settled,
    ),
    scene("diff-view-intraline", intraline, &[], 600, Expect::Settled),
    scene("diff-view-empty", empty, &[], 600, Expect::Settled),
    scene(
        "diff-view-scrolled",
        scrolled,
        &[
            key(0, &[Ev::PointerAt(row(3).0, row(3).1)]),
            key(
                100,
                &[Ev::Scroll {
                    x: 0.0,
                    y: -4000.0,
                    precise: true,
                }],
            ),
        ],
        1400,
        Expect::Settled,
    ),
    // HOVER is `instant` (120 ms): half way at 60 ms. PRESS is `fast` (180 ms): at 90 ms.
    scene(
        "diff-view-hunk-hover-fade",
        hunk_actions,
        &[key(0, &[Ev::PointerAt(first_action().0, first_action().1)])],
        60,
        Expect::Moving,
    ),
    scene(
        "diff-view-hunk-press-fade",
        hunk_actions,
        &[
            key(0, &[Ev::PointerAt(first_action().0, first_action().1)]),
            key(300, &[Ev::Press]),
        ],
        390,
        Expect::Moving,
    ),
    // The header's three stops (two modes, the wrap toggle), the diff body, then the first
    // action: its ring.
    scene(
        "diff-view-hunk-focus",
        hunk_actions,
        &[
            key(0, &[Ev::Tab]),
            key(50, &[Ev::Tab]),
            key(100, &[Ev::Tab]),
            key(150, &[Ev::Tab]),
            key(200, &[Ev::Tab]),
        ],
        600,
        Expect::Settled,
    ),
    scene(
        "diff-view-hunk-disabled",
        disabled,
        &[],
        600,
        Expect::Settled,
    ),
];

/// What the bench replays, on 100,000 rows: a click, wheel bursts and keys in unified, the switch to
/// split and the same in two panes, then word wrap.
const BENCH: &[Step] = &[
    key(0, &[Ev::PointerAt(row(3).0, row(3).1)]),
    key(100, &[Ev::Click]),
    key(
        200,
        &[Ev::Scroll {
            x: 0.0,
            y: -5.0,
            precise: false,
        }],
    ),
    key(
        260,
        &[Ev::Scroll {
            x: 0.0,
            y: -500.0,
            precise: true,
        }],
    ),
    key(500, &[Ev::Key(NamedKey::ArrowDown)]),
    key(560, &[Ev::Key(NamedKey::PageDown)]),
    key(700, &[Ev::Key(NamedKey::End)]),
    key(1000, &[Ev::Key(NamedKey::Home)]),
    key(1200, &[Ev::PointerOver(&["split"]), Ev::Click]),
    key(
        1400,
        &[Ev::PointerAt(pane_row(false, 3).0, pane_row(false, 3).1)],
    ),
    key(1500, &[Ev::Click]),
    key(
        1600,
        &[Ev::Scroll {
            x: 0.0,
            y: -5.0,
            precise: false,
        }],
    ),
    key(
        1700,
        &[Ev::Scroll {
            x: 0.0,
            y: -800.0,
            precise: true,
        }],
    ),
    key(
        2000,
        &[Ev::PointerAt(pane_row(true, 3).0, pane_row(true, 3).1)],
    ),
    key(2100, &[Ev::Click]),
    key(2200, &[Ev::Key(NamedKey::PageDown)]),
    key(2400, &[Ev::Key(NamedKey::End)]),
    key(2700, &[Ev::PointerOver(&["wrap"]), Ev::Click]),
    key(
        2900,
        &[
            Ev::PointerAt(pane_row(false, 3).0, pane_row(false, 3).1),
            Ev::Scroll {
                x: 0.0,
                y: -600.0,
                precise: true,
            },
        ],
    ),
    key(3200, &[Ev::PointerOver(&["unified"]), Ev::Click]),
];
