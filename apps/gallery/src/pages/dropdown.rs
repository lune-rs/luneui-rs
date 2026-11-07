//! The `dropdown` page: a combo and the options list it opens, through the `Ui`. The legacy
//! `dropdown` and `dropdown-flipped` scenes keep the Painter API; this page adds the hover and
//! disabled-row scenes, the glass surface and a long list that scrolls.

use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::dropdown::DropdownOptions;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

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

/// Thirty options, the twenty-first selected: a list that scrolls and opens at its selection.
const LONG_SELECTED: usize = 20;

fn options(long: bool) -> Vec<(String, bool)> {
    if long {
        (1..=30).map(|i| (format!("Font {i}"), true)).collect()
    } else {
        OPTIONS.iter().map(|(t, e)| ((*t).to_owned(), *e)).collect()
    }
}

fn live() -> Box<dyn PageState> {
    Box::new(Live::new(false, 0, DropdownOptions::default()))
}

fn opened() -> Box<dyn PageState> {
    Box::new(Live::new(true, 0, DropdownOptions::default()))
}

/// The glass surface of the menu, open.
fn glass() -> Box<dyn PageState> {
    let options = DropdownOptions {
        glass: true,
        ..DropdownOptions::default()
    };
    Box::new(Live::new(true, 0, options))
}

/// Thirty options, eight rows, the selection low in the list.
fn scrolling() -> Box<dyn PageState> {
    let options = DropdownOptions {
        glass: true,
        max_rows: Some(8),
    };
    let mut page = Live::new(true, LONG_SELECTED, options);
    page.items = self::options(true);
    Box::new(page)
}

/// The page plays the app: it owns `open` and the selected option.
struct Live {
    open: bool,
    selected: usize,
    items: Vec<(String, bool)>,
    options: DropdownOptions,
}

impl Live {
    fn new(open: bool, selected: usize, options: DropdownOptions) -> Self {
        Self {
            open,
            selected,
            items: self::options(false),
            options,
        }
    }
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Dropdown");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "DROPDOWN", x, y);
        let anchor = Rect::new(x, y + 20.0, 210.0, 32.0);
        let combo = ui.combo(
            "combo",
            anchor,
            &self.items[self.selected].0,
            &mut self.open,
            "Font",
        );
        if self.open {
            let d = ui.dropdown_with(
                "dd",
                anchor,
                combo.id,
                &self.items,
                self.selected,
                &self.options,
            );
            if let Some(i) = d.picked {
                self.selected = i;
            }
            if d.picked.is_some() || d.dismissed {
                self.open = false;
            }
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "Glass surface", &mut self.options.glass);
        let mut rows = self.options.max_rows.unwrap_or(0) as f32;
        if panel.number(ui, "Rows shown (0: all)", &mut rows, (0.0, 12.0, 1.0)) {
            self.options.max_rows = (rows >= 1.0).then_some(rows as usize);
        }
        let mut long = self.items.len() > OPTIONS.len();
        if panel.toggle(ui, "Thirty options", &mut long) {
            self.items = options(long);
            self.selected = if long { LONG_SELECTED } else { 0 };
        }
        // Last: the open list is dismissed by a press outside it.
        panel.toggle(ui, "Open", &mut self.open);
    }
}

const COMBO: &[&str] = &["combo"];
/// The second option's row: the overlay `dd`, its list, then the row's index.
const ROW_1: &[&str] = &["dd", "list", "1"];
/// A row of the long list that is in view at open.
const ROW_SCROLLED: &[&str] = &["dd", "list", "19"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "dropdown-hover",
        scale: 2.0,
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
        scale: 2.0,
        tier: MotionTier::Full,
        state: opened,
        script: &[],
        capture_ms: 300,
        expect: Expect::Settled,
    },
    // The glass surface: the blurred page shows through the fill.
    UiScene {
        id: "dropdown-glass",
        scale: 2.0,
        tier: MotionTier::Full,
        state: glass,
        script: &[],
        capture_ms: 300,
        expect: Expect::Settled,
    },
    // Thirty options in eight rows, opened on the twenty-first: it shows, the thumb is over the
    // list's right edge while the pointer is in it.
    UiScene {
        id: "dropdown-scroll",
        scale: 2.0,
        tier: MotionTier::Full,
        state: scrolling,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(ROW_SCROLLED)],
        }],
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
