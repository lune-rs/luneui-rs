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

fn base() -> Live {
    Live {
        count: 1000,
        seed: &[],
        seeded: false,
        enabled: true,
    }
}

fn million() -> Box<dyn PageState> {
    Box::new(Live {
        count: 1_000_000,
        ..base()
    })
}

fn thousand() -> Box<dyn PageState> {
    Box::new(base())
}

fn two_and_five() -> Box<dyn PageState> {
    Box::new(Live {
        seed: &[2, 5],
        ..base()
    })
}

fn two() -> Box<dyn PageState> {
    Box::new(Live {
        seed: &[2],
        ..base()
    })
}

fn disabled() -> Box<dyn PageState> {
    Box::new(Live {
        enabled: false,
        ..base()
    })
}

fn empty() -> Box<dyn PageState> {
    Box::new(Live { count: 0, ..base() })
}

fn far() -> Box<dyn PageState> {
    Box::new(Live {
        seed: &[900],
        ..base()
    })
}

/// The page plays the app: it owns the data (a count) and draws each row from its index.
struct Live {
    count: usize,
    /// Rows selected before the first frame draws.
    seed: &'static [usize],
    seeded: bool,
    enabled: bool,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Virtualized list");
        heading(ui, "VIRTUALIZED LIST", card.x + 24.0, card.y + 24.0);
        if !self.seeded {
            self.seeded = true;
            let state = ui.list_state("list");
            state.selection.extend(self.seed.iter().map(|&i| key(i)));
        }
        let opts = ListOptions {
            count: self.count,
            heights: Heights::Fixed(ROW),
            selection: SelectionMode::Multi,
            index_of: None,
            label: "Rows",
        };
        ui.with_enabled(self.enabled, |ui| {
            ui.virtual_list("list", LIST, &opts, key, |ui, row| {
                let spec = Spec::plain(
                    format!("Row {}", row.index),
                    Style::ui(THEME.font_base, THEME.fg_primary),
                );
                ui.painter()
                    .text(&spec, row.rect.x + 16.0, row.rect.y + 12.0);
                ROW
            });
        });
        if self.count == 0 {
            let spec = Spec::plain("No rows", Style::ui(THEME.font_base, THEME.fg_tertiary));
            ui.painter().text_centered(&spec, LIST);
        }
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

/// A plain list scene: the events at their times, then a capture.
const fn scene(
    id: &'static str,
    state: fn() -> Box<dyn PageState>,
    script: &'static [Step],
    capture_ms: u32,
    expect: Expect,
) -> UiScene {
    UiScene {
        id,
        scale: 2,
        tier: MotionTier::Full,
        state,
        script,
        capture_ms,
        expect,
    }
}

const SCENES: &[UiScene] = &[
    scene("virtualized-list-rest", thousand, &[], 0, Expect::Settled),
    scene(
        "virtualized-list-hover",
        thousand,
        &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(row(3).0, row(3).1)],
        }],
        600,
        Expect::Settled,
    ),
    scene(
        "virtualized-list-selected",
        two_and_five,
        &[],
        600,
        Expect::Settled,
    ),
    scene(
        "virtualized-list-selected-hover",
        two,
        &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(row(2).0, row(2).1)],
        }],
        600,
        Expect::Settled,
    ),
    // Click row 2 (the anchor), then Shift-click row 6: rows 2 to 6 are selected, the pointer is on
    // the last, which shows the selected-hover fill.
    scene(
        "virtualized-list-range",
        thousand,
        &[
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
        900,
        Expect::Settled,
    ),
    // Tab focuses the list; five Down presses walk the cursor onto row 4 (the first Down lands on
    // the first visible row), and the ring shows because the keys moved it.
    scene(
        "virtualized-list-keyboard",
        thousand,
        &[
            Step {
                at_ms: 0,
                events: &[Ev::Tab],
            },
            Step {
                at_ms: 50,
                events: &[Ev::Key(NamedKey::ArrowDown)],
            },
            Step {
                at_ms: 100,
                events: &[Ev::Key(NamedKey::ArrowDown)],
            },
            Step {
                at_ms: 150,
                events: &[Ev::Key(NamedKey::ArrowDown)],
            },
            Step {
                at_ms: 200,
                events: &[Ev::Key(NamedKey::ArrowDown)],
            },
            Step {
                at_ms: 250,
                events: &[Ev::Key(NamedKey::ArrowDown)],
            },
        ],
        900,
        Expect::Settled,
    ),
    scene(
        "virtualized-list-disabled",
        disabled,
        &[],
        600,
        Expect::Settled,
    ),
    scene("virtualized-list-empty", empty, &[], 600, Expect::Settled),
    // Row 900 is selected before the scroll; the pointer is over the list and the content moves
    // 40000 px at once (a trackpad delta, 1:1).
    scene(
        "virtualized-list-scrolled",
        far,
        &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(row(3).0, row(3).1)],
            },
            Step {
                at_ms: 100,
                events: &[Ev::Scroll {
                    x: 0.0,
                    y: -40000.0,
                    precise: true,
                }],
            },
        ],
        900,
        Expect::Settled,
    ),
    // The pointer enters row 3 at 300 ms: the fill follows over `instant` (120 ms).
    scene(
        "virtualized-list-hover-fade",
        thousand,
        &[Step {
            at_ms: 300,
            events: &[Ev::PointerAt(row(3).0, row(3).1)],
        }],
        360,
        Expect::Moving,
    ),
    // The pointer rests on row 5; the click at 300 ms selects it: the fill follows over `fast`.
    scene(
        "virtualized-list-select-fade",
        thousand,
        &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(row(5).0, row(5).1)],
            },
            Step {
                at_ms: 300,
                events: &[Ev::Click],
            },
        ],
        390,
        Expect::Moving,
    ),
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
