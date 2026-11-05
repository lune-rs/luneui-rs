//! The `split-panes` page: a column split (the first pane sized, 160 to 600 px, the second keeping
//! 200) with a button that hides the first pane, and a row split below it (the second pane sized,
//! 60 to 220 px, the first keeping 60). The strip is transparent at rest: drag it, or Tab to it and
//! use the arrows, Home and End. The sizes the panes print are the page's own bookkeeping; the
//! widget stores them and the app reads them.

use lune_ui::input::NamedKey;
use lune_ui::layout::Axis;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, ButtonOptions};
use lune_ui::widgets::split_panes::{Pane, SplitOptions, SplitSide};

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "split-panes",
    title: "Split panes",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
/// The column split, and the row split under it.
const COLS: Rect = Rect::new(X, Y + 56.0, 940.0, 220.0);
const ROWS: Rect = Rect::new(X, Y + 296.0, 940.0, 240.0);
const TOGGLE: Rect = Rect::new(X, Y + 20.0, 150.0, 24.0);

const COLS_DEFAULT: f32 = 300.0;
const ROWS_DEFAULT: f32 = 100.0;

const COLS_OPTS: SplitOptions<'static> = SplitOptions {
    axis: Axis::Horizontal,
    sized: SplitSide::First,
    default: COLS_DEFAULT,
    min: 160.0,
    max: 600.0,
    min_other: 200.0,
    hidden: false,
    name: "Column width",
};
const ROWS_OPTS: SplitOptions<'static> = SplitOptions {
    axis: Axis::Vertical,
    sized: SplitSide::Second,
    default: ROWS_DEFAULT,
    min: 60.0,
    max: 220.0,
    min_other: 60.0,
    hidden: false,
    name: "Row height",
};

fn live() -> Box<dyn PageState> {
    Box::new(Live::new(false))
}

fn collapsed() -> Box<dyn PageState> {
    Box::new(Live::new(true))
}

struct Live {
    /// The column split's first pane is hidden.
    hidden: bool,
    /// The sizes as of the last frame, for the panes to print.
    cols: f32,
    rows: f32,
}

impl Live {
    fn new(hidden: bool) -> Live {
        Live {
            hidden,
            cols: COLS_DEFAULT,
            rows: ROWS_DEFAULT,
        }
    }
}

/// A pane's tint and its two lines, inset 12 px.
fn pane(ui: &mut Ui<'_>, rect: Rect, tint: [u8; 4], title: &str, detail: &str) {
    ui.painter().fill(rect, 0.0, tint);
    let first = Spec::plain(title, Style::ui(THEME.font_base, THEME.fg_bright).medium());
    let second = Spec::plain(detail, Style::ui(THEME.font_s, THEME.fg_secondary));
    let p = ui.painter();
    p.text(&first, rect.x + 12.0, rect.y + 12.0);
    p.text(&second, rect.x + 12.0, rect.y + 34.0);
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        card(ui, "Split panes");
        heading(ui, "SPLIT PANES", X, Y);
        let ghost = ButtonOptions {
            kind: ButtonKind::Secondary,
            small: true,
            icon: None,
        };
        let label = if self.hidden {
            "Show first pane"
        } else {
            "Hide first pane"
        };
        if ui.button_with("toggle", TOGGLE, label, &ghost).clicked {
            self.hidden = !self.hidden;
        }

        let cols = SplitOptions {
            hidden: self.hidden,
            ..COLS_OPTS
        };
        let (width, height) = (self.cols, self.rows);
        let r = ui.split_panes("cols", COLS, &cols, |ui, which, rect| match which {
            Pane::First => pane(
                ui,
                rect,
                THEME.surface_overlay,
                "First pane",
                &format!("Width {width:.0}"),
            ),
            Pane::Second => pane(
                ui,
                rect,
                THEME.surface_hover,
                "Second pane",
                "Keeps at least 200",
            ),
        });
        self.cols = r.size;

        let r = ui.split_panes("rows", ROWS, &ROWS_OPTS, |ui, which, rect| match which {
            Pane::First => pane(
                ui,
                rect,
                THEME.surface_hover,
                "Content",
                "Keeps at least 60",
            ),
            Pane::Second => pane(
                ui,
                rect,
                THEME.surface_overlay,
                "Panel below",
                &format!("Height {height:.0}"),
            ),
        });
        self.rows = r.size;
    }
}

/// The centre of the column strip at its default, and of the row strip.
const COLS_STRIP: (f32, f32) = (X + COLS_DEFAULT + 2.0, Y + 56.0 + 110.0);
const ROWS_STRIP: (f32, f32) = (X + 470.0, Y + 296.0 + 240.0 - ROWS_DEFAULT - 2.0);

const SCENES: &[UiScene] = &[
    // Two columns and two rows, the strips transparent, settled.
    UiScene {
        id: "split-panes-two",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The first column hidden: the second takes the whole width, no strip.
    UiScene {
        id: "split-panes-collapsed",
        scale: 2,
        tier: MotionTier::Full,
        state: collapsed,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The column strip pressed and dragged 200 px to the right, held.
    UiScene {
        id: "split-panes-drag-mid",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(COLS_STRIP.0, COLS_STRIP.1)],
            },
            Step {
                at_ms: 100,
                events: &[Ev::Press],
            },
            Step {
                at_ms: 200,
                events: &[Ev::PointerAt(COLS_STRIP.0 + 200.0, COLS_STRIP.1)],
            },
            // The panes follow the strip on the catch-up frame.
            Step {
                at_ms: 250,
                events: &[],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // The row strip dragged 70 px up: the panel below grows.
    UiScene {
        id: "split-panes-rows",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(ROWS_STRIP.0, ROWS_STRIP.1)],
            },
            Step {
                at_ms: 100,
                events: &[Ev::Press],
            },
            Step {
                at_ms: 200,
                events: &[Ev::PointerAt(ROWS_STRIP.0, ROWS_STRIP.1 - 70.0)],
            },
            Step {
                at_ms: 300,
                events: &[Ev::Release],
            },
        ],
        capture_ms: 500,
        expect: Expect::Settled,
    },
    // Tab past the button onto the column strip: it fills with the accent.
    UiScene {
        id: "split-panes-focus",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::Tab],
            },
            Step {
                at_ms: 50,
                events: &[Ev::Tab],
            },
        ],
        capture_ms: 500,
        expect: Expect::Settled,
    },
];

/// What the bench replays: drag the column strip out and back, Tab to it, arrows, Home and End,
/// then drag the row strip.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(COLS_STRIP.0, COLS_STRIP.1)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Press],
    },
    Step {
        at_ms: 200,
        events: &[Ev::PointerAt(COLS_STRIP.0 + 120.0, COLS_STRIP.1)],
    },
    Step {
        at_ms: 300,
        events: &[Ev::PointerAt(COLS_STRIP.0 - 60.0, COLS_STRIP.1)],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Release],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::ArrowRight)],
    },
    Step {
        at_ms: 900,
        events: &[Ev::Key(NamedKey::ArrowLeft)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Key(NamedKey::End)],
    },
    Step {
        at_ms: 1100,
        events: &[Ev::Key(NamedKey::Home)],
    },
    Step {
        at_ms: 1300,
        events: &[Ev::PointerAt(ROWS_STRIP.0, ROWS_STRIP.1)],
    },
    Step {
        at_ms: 1400,
        events: &[Ev::Press],
    },
    Step {
        at_ms: 1500,
        events: &[Ev::PointerAt(ROWS_STRIP.0, ROWS_STRIP.1 - 50.0)],
    },
    Step {
        at_ms: 1600,
        events: &[Ev::Release],
    },
];
