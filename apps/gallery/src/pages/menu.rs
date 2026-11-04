//! The `menu` page: an "Open menu" button and the glass menu it opens, through the `Ui` (window
//! mode, the bench and the closing scenes). The opening and settled looks are the legacy `menu` and
//! `menu-opening` scenes; this page adds the exit frames.

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::menu::MenuRow;

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "menu",
    title: "Menu",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

const ROWS: [MenuRow<'static>; 6] = [
    MenuRow::Item {
        glyph: Some(icon::ADD),
        text: "New Window",
        shortcut: "Cmd+N",
    },
    MenuRow::Item {
        glyph: Some(icon::EDIT),
        text: "Rename...",
        shortcut: "",
    },
    MenuRow::Item {
        glyph: Some(icon::COPY),
        text: "Copy",
        shortcut: "Cmd+C",
    },
    MenuRow::Separator,
    MenuRow::Item {
        glyph: Some(icon::GEAR),
        text: "Settings...",
        shortcut: "Cmd+,",
    },
    MenuRow::Item {
        glyph: Some(icon::INFO),
        text: "About",
        shortcut: "",
    },
];

fn live() -> Box<dyn PageState> {
    Box::new(Live { open: false })
}

/// A page whose menu stops being called at 300 ms of app time (closing starts there).
fn closing() -> Box<dyn PageState> {
    Box::new(Closing)
}

/// The page plays the app: the button opens the menu, a pick or a dismissal closes it.
struct Live {
    open: bool,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Menu");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "MENU", x, y);
        let w = button_width(ui.painter(), "Open menu");
        let r = ui.button(
            "open",
            Rect::new(x, y + 20.0, w, 32.0),
            "Open menu",
            ButtonKind::Secondary,
        );
        if r.clicked {
            self.open = true;
        }
        if self.open {
            let m = ui.menu("menu", &ROWS, (x, y + 60.0));
            if m.picked.is_some() || m.dismissed {
                self.open = false;
            }
        }
    }
}

/// The menu is open until 300 ms, then the app stops calling it.
struct Closing;

impl PageState for Closing {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Menu");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "MENU", x, y);
        if ui.time().as_millis() < 300 {
            ui.menu("menu", &ROWS, (x, y + 60.0));
        }
    }
}

const OPEN: &[&str] = &["open"];

const SCENES: &[UiScene] = &[
    // MENU_OUT is 120 ms: half way at 60 ms. The frame at 300 ms is the first without the call.
    UiScene {
        id: "menu-closing",
        scale: 2,
        tier: MotionTier::Full,
        state: closing,
        script: &[Step {
            at_ms: 300,
            events: &[],
        }],
        capture_ms: 360,
        expect: Expect::Moving,
    },
    // The same picture under the motion.curves id.
    UiScene {
        id: "curves-menu-closing",
        scale: 2,
        tier: MotionTier::Full,
        state: closing,
        script: &[Step {
            at_ms: 300,
            events: &[],
        }],
        capture_ms: 360,
        expect: Expect::Moving,
    },
];

/// What the bench replays: hover and click the button (opens), Escape, open again, Enter.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(OPEN)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Key(NamedKey::Escape)],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1100,
        events: &[Ev::Key(NamedKey::Enter)],
    },
];
