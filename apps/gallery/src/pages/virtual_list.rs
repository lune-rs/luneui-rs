//! The `virtual-list` page: rows of 45 px on the virtualized list (`layout.virtualized-list`).
//! The scenes use 1000 rows keyed by index; the window and the bench use 1,000,000, where only the
//! rows in view are built. Rows hold their number and are addressed in scenes by position, because
//! a row's id derives from its key, not from a string path.

use lune_ui::input::{Modifiers, NamedKey};
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::virtual_list::{Heights, ListOptions, RowKey, SelectionMode};

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "virtual-list",
    title: "Virtualized list",
    kind: PageKind::Ui(UiPage {
        state: million,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The list, in window coordinates: inside the card, under the heading.
const LIST: Rect = Rect::new(34.0, 82.0, 520.0, 360.0);
const ROW: f32 = 45.0;

fn key(i: usize) -> RowKey {
    RowKey(i as u64)
}

fn million() -> Box<dyn PageState> {
    Box::new(Live { count: 1_000_000 })
}

fn thousand() -> Box<dyn PageState> {
    Box::new(Live { count: 1000 })
}

/// The page plays the app: it owns the data (a count) and draws each row from its index.
struct Live {
    count: usize,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Virtualized list");
        heading(ui, "VIRTUALIZED LIST", card.x + 24.0, card.y + 24.0);
        let opts = ListOptions {
            count: self.count,
            heights: Heights::Fixed(ROW),
            selection: SelectionMode::Multi,
            index_of: None,
            label: "Rows",
        };
        ui.virtual_list("list", LIST, &opts, key, |ui, row| {
            let spec = Spec::plain(
                format!("Row {}", row.index),
                Style::ui(THEME.font_base, THEME.fg_primary),
            );
            ui.painter()
                .text(&spec, row.rect.x + 16.0, row.rect.y + 12.0);
            ROW
        });
    }
}

/// The centre of row `i` while the list is at offset 0.
const fn row(i: usize) -> (f32, f32) {
    (LIST.x + 100.0, LIST.y + (i as f32 + 0.5) * ROW)
}

const SHIFT: Modifiers = Modifiers {
    shift: true,
    control: false,
    alt: false,
    logo: false,
};
const NONE: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: false,
};

const SCENES: &[UiScene] = &[
    // Click row 2 (the anchor), then Shift-click row 6: rows 2 to 6 are selected, the pointer is on
    // the last, which shows the selected-hover fill.
    UiScene {
        id: "virtualized-list-range",
        scale: 2,
        tier: MotionTier::Full,
        state: thousand,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(row(2).0, row(2).1)],
            },
            Step {
                at_ms: 100,
                events: &[Ev::Click],
            },
            Step {
                at_ms: 200,
                events: &[Ev::Mods(SHIFT), Ev::PointerAt(row(6).0, row(6).1)],
            },
            Step {
                at_ms: 300,
                events: &[Ev::Click],
            },
            Step {
                at_ms: 400,
                events: &[Ev::Mods(NONE)],
            },
        ],
        capture_ms: 900,
        expect: Expect::Settled,
    },
];

/// What the bench replays, on a million rows: a click, a wheel burst, keys, a jump to the end.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(row(3).0, row(3).1)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -5.0,
            precise: false,
        }],
    },
    Step {
        at_ms: 260,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -5.0,
            precise: false,
        }],
    },
    Step {
        at_ms: 320,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -500.0,
            precise: true,
        }],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 560,
        events: &[Ev::Key(NamedKey::PageDown)],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Key(NamedKey::End)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Key(NamedKey::Home)],
    },
];
