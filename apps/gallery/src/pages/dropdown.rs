//! The `dropdown` page: a combo and the options list it opens, through the `Ui`. The legacy
//! `dropdown` and `dropdown-flipped` scenes keep the Painter API; this page adds the hover and
//! disabled-row scenes.

use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "dropdown",
    title: "Dropdown",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

const OPTIONS: [(&str, bool); 4] = [
    ("RznSans", true),
    ("Sans Serif", true),
    ("Monospace", true),
    ("Coming soon", false),
];

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        open: false,
        selected: 0,
    })
}

fn opened() -> Box<dyn PageState> {
    Box::new(Live {
        open: true,
        selected: 0,
    })
}

/// The page plays the app: it owns `open` and the selected option.
struct Live {
    open: bool,
    selected: usize,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Dropdown");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "DROPDOWN", x, y);
        let anchor = Rect::new(x, y + 20.0, 210.0, 32.0);
        let combo = ui.combo(
            "combo",
            anchor,
            OPTIONS[self.selected].0,
            &mut self.open,
            "Font",
        );
        if self.open {
            let d = ui.dropdown("dd", anchor, combo.id, &OPTIONS, self.selected);
            if let Some(i) = d.picked {
                self.selected = i;
            }
            if d.picked.is_some() || d.dismissed {
                self.open = false;
            }
        }
    }
}

const COMBO: &[&str] = &["combo"];
/// The second option's row: the overlay `dd`, its list, then the row's index.
const ROW_1: &[&str] = &["dd", "list", "1"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "dropdown-hover",
        scale: 2,
        tier: MotionTier::Full,
        state: opened,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(ROW_1)],
        }],
        capture_ms: 300,
        expect: Expect::Settled,
    },
    // "Coming soon" is disabled: no hover, no hit region.
    UiScene {
        id: "dropdown-disabled",
        scale: 2,
        tier: MotionTier::Full,
        state: opened,
        script: &[],
        capture_ms: 300,
        expect: Expect::Settled,
    },
];

/// What the bench replays: open by click, hover a row, Down, Enter (picks), reopen with Down.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(COMBO)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::PointerOver(ROW_1)],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::Enter)],
    },
];
