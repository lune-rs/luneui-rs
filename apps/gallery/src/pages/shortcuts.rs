//! The `shortcuts` page: the shortcut hints a menu shows, formatted by `Chord::hint` (`infra.shortcut-
//! routing`). Two menus are drawn side by side: one with `Keymap::MAC` ("Cmd+N") and one with
//! `Keymap::PC` ("Ctrl+N").

use lune_ui::icon;
use lune_ui::input::{Key, Keymap};
use lune_ui::motion::MotionTier;
use lune_ui::shortcuts::Chord;
use lune_ui::ui::Ui;
use lune_ui::widgets::menu::MenuRow;

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "shortcuts",
    title: "Shortcuts",
    kind: PageKind::Ui(UiPage {
        state: menus,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The three hints the menu shows, formatted for one keymap.
struct Hints {
    new: String,
    copy: String,
    settings: String,
}

fn hints(keymap: &Keymap) -> Hints {
    let chord = |key: &str| {
        Chord::key(Key::Character(key.to_owned()))
            .primary()
            .hint(keymap)
    };
    Hints {
        new: chord("N"),
        copy: chord("C"),
        settings: chord(","),
    }
}

fn rows(h: &Hints) -> [MenuRow<'_>; 4] {
    [
        MenuRow::Item {
            glyph: Some(icon::ADD),
            text: "New Window",
            shortcut: &h.new,
        },
        MenuRow::Item {
            glyph: Some(icon::COPY),
            text: "Copy",
            shortcut: &h.copy,
        },
        MenuRow::Separator,
        MenuRow::Item {
            glyph: Some(icon::GEAR),
            text: "Settings...",
            shortcut: &h.settings,
        },
    ]
}

/// Both menus, open from the start of the scene.
struct Menus;

impl PageState for Menus {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Shortcuts");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "SHORTCUT HINTS", x, y);
        let mac = hints(&Keymap::MAC);
        let pc = hints(&Keymap::PC);
        caption(ui, "macOS: Keymap::MAC", x, y + 24.0);
        caption(ui, "Windows: Keymap::PC", x + 280.0, y + 24.0);
        ui.menu("mac", &rows(&mac), (x, y + 48.0));
        ui.menu("pc", &rows(&pc), (x + 280.0, y + 48.0));
    }
}

fn menus() -> Box<dyn PageState> {
    Box::new(Menus)
}

/// The menus open at time 0 (the open motion is 180 ms); the capture at 600 ms is settled.
const SCENES: &[UiScene] = &[UiScene {
    id: "shortcut-hints",
    scale: 2,
    tier: MotionTier::Full,
    state: menus,
    script: &[Step {
        at_ms: 200,
        events: &[],
    }],
    capture_ms: 600,
    expect: Expect::Settled,
}];

/// What the bench replays: the pointer over each menu.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(200.0, 200.0)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::PointerAt(480.0, 200.0)],
    },
];
