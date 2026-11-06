//! The `menu` page: buttons and the glass menus they open, through the `Ui` (window mode, the bench
//! and the scenes). The opening and settled looks are the legacy `menu` and `menu-opening` scenes;
//! this page adds the exit frames, the keys, the row variants, the pressed fill and anchored menus.

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::{Ui, WidgetId};
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::menu::{MenuItem, MenuOptions, MenuPlace, MenuRow};
use lune_ui::widgets::popover::{PlaceAlign, Side};
use lune_ui::widgets::tone::Tone;

use super::{card, heading};
use crate::page::Stage;
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

/// A header, a plain row, a disabled one, chevron rows and a danger row.
const VARIANTS: [MenuRow<'static>; 8] = [
    MenuRow::Header {
        name: "Serving on :4646",
        status: "RM 48.20 today, 14.6% saved",
        tone: Tone::Ok,
    },
    MenuRow::Separator,
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::ADD),
        text: "Open",
        shortcut: "Cmd+O",
        disabled: false,
        danger: false,
        chevron: false,
    }),
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::EDIT),
        text: "Pause compression",
        shortcut: "Cmd+P",
        disabled: true,
        danger: false,
        chevron: false,
    }),
    MenuRow::Entry(MenuItem {
        glyph: None,
        text: "Profile",
        shortcut: "Work",
        disabled: false,
        danger: false,
        chevron: true,
    }),
    MenuRow::Entry(MenuItem {
        glyph: None,
        text: "Live sessions",
        shortcut: "3",
        disabled: true,
        danger: false,
        chevron: true,
    }),
    MenuRow::Separator,
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::CLOSE),
        text: "Quit",
        shortcut: "Cmd+Q",
        disabled: false,
        danger: true,
        chevron: false,
    }),
];

fn live() -> Box<dyn PageState> {
    Box::new(Live { open: None })
}

fn variants() -> Box<dyn PageState> {
    Box::new(Pinned { rows: &VARIANTS })
}

fn pinned() -> Box<dyn PageState> {
    Box::new(Pinned { rows: &ROWS })
}

fn anchored() -> Box<dyn PageState> {
    Box::new(Anchored { low: false })
}

fn anchored_low() -> Box<dyn PageState> {
    Box::new(Anchored { low: true })
}

/// A page whose menu stops being called at 300 ms of app time (closing starts there).
fn closing() -> Box<dyn PageState> {
    Box::new(Closing)
}

/// Which of the page's menus is open.
#[derive(Clone, Copy, PartialEq)]
enum Shown {
    Plain,
    Variants,
    Anchored,
}

/// The page plays the app: a button opens its menu, a pick or a dismissal closes it.
struct Live {
    open: Option<Shown>,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Menu");
        let (mut x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "MENU", x, y);
        let at = (x, y + 60.0);
        let mut anchor = None;
        for (key, label, shown) in [
            ("open", "Open menu", Shown::Plain),
            ("variants", "Variants", Shown::Variants),
            ("anchored", "Anchored", Shown::Anchored),
        ] {
            let w = button_width(ui.painter(), label);
            let rect = Rect::new(x, y + 20.0, w, 32.0);
            let r = ui.button(key, rect, label, ButtonKind::Secondary);
            if r.clicked {
                self.open = Some(shown);
            }
            if shown == Shown::Anchored {
                anchor = Some(r.rect);
            }
            x += w + 8.0;
        }
        let m = match (self.open, anchor) {
            (Some(Shown::Plain), _) => ui.menu("menu", &ROWS, at),
            (Some(Shown::Variants), _) => ui.menu("variants-menu", &VARIANTS, at),
            (Some(Shown::Anchored), Some(rect)) => {
                let anchor_id = Some(ui.id("anchored"));
                let options = options_below(rect, anchor_id);
                ui.menu_with("anchored-menu", &ROWS, &options)
            }
            _ => return,
        };
        if m.picked.is_some() || m.dismissed {
            self.open = None;
        }
    }
}

/// A menu under `rect`, its left edge on the anchor's.
fn options_below(rect: Rect, anchor_id: Option<WidgetId>) -> MenuOptions<'static> {
    MenuOptions {
        place: MenuPlace::Anchor {
            rect,
            side: Side::Bottom,
            align: PlaceAlign::Start,
            gap: 4.0,
        },
        anchor_id,
        name: "Anchored actions",
    }
}

/// `rows` in a menu that stays open.
struct Pinned {
    rows: &'static [MenuRow<'static>],
}

impl PageState for Pinned {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Menu");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "MENU", x, y);
        ui.menu("menu", self.rows, (x, y + 60.0));
    }
}

/// A menu that stays open on a button, near the window's bottom edge when `low`, where it has no
/// room below and flips above.
struct Anchored {
    low: bool,
}

impl PageState for Anchored {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Menu");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "MENU", x, y);
        let top = if self.low {
            stage.area(ui).bottom() - 72.0
        } else {
            y + 20.0
        };
        let w = button_width(ui.painter(), "Actions");
        let rect = Rect::new(x, top, w, 32.0);
        let r = ui.button("actions", rect, "Actions", ButtonKind::Secondary);
        let anchor_id = Some(ui.id("actions"));
        ui.menu_with("menu", &ROWS, &options_below(r.rect, anchor_id));
    }
}

/// The menu is open until 300 ms, then the app stops calling it.
struct Closing;

impl PageState for Closing {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Menu");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "MENU", x, y);
        if ui.time().as_millis() < 300 {
            ui.menu("menu", &ROWS, (x, y + 60.0));
        }
    }
}

const OPEN: &[&str] = &["open"];
/// The panel's centre is on the third row of `ROWS`.
const PANEL: &[&str] = &["menu", "panel"];

const SCENES: &[UiScene] = &[
    // MENU_OUT is 120 ms: half way at 60 ms. The frame at 300 ms is the first without the call.
    UiScene {
        id: "menu-closing",
        scale: 2.0,
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
        scale: 2.0,
        tier: MotionTier::Full,
        state: closing,
        script: &[Step {
            at_ms: 300,
            events: &[],
        }],
        capture_ms: 360,
        expect: Expect::Moving,
    },
    // Tab to the button, Enter opens with the first row lit, Down twice lights the third.
    UiScene {
        id: "menu-keyboard",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 100,
                events: &[Ev::Tab],
            },
            Step {
                at_ms: 200,
                events: &[Ev::Key(NamedKey::Enter)],
            },
            Step {
                at_ms: 500,
                events: &[Ev::Key(NamedKey::ArrowDown)],
            },
            Step {
                at_ms: 600,
                events: &[Ev::Key(NamedKey::ArrowDown)],
            },
        ],
        capture_ms: 1000,
        expect: Expect::Settled,
    },
    UiScene {
        id: "menu-variants",
        scale: 2.0,
        tier: MotionTier::Full,
        state: variants,
        script: &[],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Held on the third row: the pressed fill.
    UiScene {
        id: "menu-pressed",
        scale: 2.0,
        tier: MotionTier::Full,
        state: pinned,
        script: &[
            Step {
                at_ms: 300,
                events: &[Ev::PointerOver(PANEL)],
            },
            Step {
                at_ms: 600,
                events: &[Ev::Press],
            },
        ],
        capture_ms: 900,
        expect: Expect::Settled,
    },
    UiScene {
        id: "menu-anchored",
        scale: 2.0,
        tier: MotionTier::Full,
        state: anchored,
        script: &[],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // No room below the button: the menu sits above it.
    UiScene {
        id: "menu-anchored-flipped",
        scale: 2.0,
        tier: MotionTier::Full,
        state: anchored_low,
        script: &[],
        capture_ms: 400,
        expect: Expect::Settled,
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
