//! The `sidebar` page: the 208 px column on the window glass with a nav list or the wizard's steps
//! and the status footer, three variants side by side (main with the ok footer, main with the
//! error footer, the wizard) and one at a time in the scenes. The wizard has Back and Continue
//! under it; the nav rows navigate.

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, ButtonOptions};
use lune_ui::widgets::nav_item::NavEntry;
use lune_ui::widgets::sidebar::{Sidebar, SidebarBody, SidebarFooter, WIDTH};
use lune_ui::widgets::tone::Tone;

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "sidebar",
    title: "Sidebar",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
/// The columns' top and height, and the distance between two columns.
const TOP: f32 = Y + 36.0;
const HEIGHT: f32 = 420.0;
const STRIDE: f32 = WIDTH + 24.0;

/// Neutral stand-ins for an app's screens (the glyphs are the toolkit's).
const SCREENS: [(u64, &str, &str); 6] = [
    (1, "Overview", icon::INFO),
    (2, "Sessions", icon::SYNC),
    (3, "Requests", icon::SEARCH),
    (4, "Routes", icon::COPY),
    (5, "Upstreams", icon::EDIT),
    (6, "Settings", icon::GEAR),
];

const STEPS: [&str; 4] = ["Welcome", "Connect an upstream", "Choose a model", "Finish"];

const OK: SidebarFooter<'static> = SidebarFooter {
    tone: Tone::Ok,
    text: "Proxy on 127.0.0.1:4646",
    mono: true,
};
const STOPPED: SidebarFooter<'static> = SidebarFooter {
    tone: Tone::Err,
    text: "Proxy stopped",
    mono: false,
};

/// Which columns a state draws.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Show {
    All,
    Main,
    Error,
    Wizard,
}

fn live() -> Box<dyn PageState> {
    Box::new(Live::of(Show::All))
}

fn main_column() -> Box<dyn PageState> {
    Box::new(Live::of(Show::Main))
}

fn error_column() -> Box<dyn PageState> {
    Box::new(Live::of(Show::Error))
}

fn wizard_column() -> Box<dyn PageState> {
    Box::new(Live::of(Show::Wizard))
}

struct Live {
    show: Show,
    selected: u64,
    /// The nav list of the error column keeps its own selection.
    selected_error: u64,
    step: usize,
}

impl Live {
    fn of(show: Show) -> Live {
        Live {
            show,
            selected: 1,
            selected_error: 2,
            step: 1,
        }
    }
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Sidebar");
        let (ox, oy) = (card.x + 24.0, card.y + 24.0);
        let top = oy + 36.0;
        heading(ui, "SIDEBAR", ox, oy);
        let all = self.show == Show::All;
        let mut col = 0.0;
        let mut next = |label: &str, ui: &mut Ui<'_>| {
            let x = ox + col * STRIDE;
            col += 1.0;
            if all {
                caption(ui, label, x, top - 20.0);
            }
            Rect::new(x, top, WIDTH, HEIGHT)
        };
        let entries: Vec<NavEntry<'_>> = SCREENS
            .iter()
            .map(|(key, label, glyph)| NavEntry {
                key: *key,
                label,
                icon: Some(glyph),
                enabled: true,
            })
            .collect();
        if matches!(self.show, Show::All | Show::Main) {
            let r = next("Main", ui);
            ui.sidebar(
                "main",
                r,
                Sidebar {
                    body: SidebarBody::Nav {
                        entries: &entries,
                        selected: &mut self.selected,
                    },
                    footer: Some(OK),
                    name: "Main navigation",
                },
            );
        }
        if matches!(self.show, Show::All | Show::Error) {
            let r = next("Footer error", ui);
            ui.sidebar(
                "error",
                r,
                Sidebar {
                    body: SidebarBody::Nav {
                        entries: &entries,
                        selected: &mut self.selected_error,
                    },
                    footer: Some(STOPPED),
                    name: "Main navigation",
                },
            );
        }
        if matches!(self.show, Show::All | Show::Wizard) {
            let r = next("Wizard", ui);
            ui.sidebar(
                "wizard",
                r,
                Sidebar {
                    body: SidebarBody::Steps {
                        labels: &STEPS,
                        current: self.step,
                    },
                    footer: Some(OK),
                    name: "Setup steps",
                },
            );
            let small = ButtonOptions {
                kind: ButtonKind::Secondary,
                small: true,
                icon: None,
            };
            let y = r.bottom() + 12.0;
            if ui
                .button_with("back", Rect::new(r.x, y, 80.0, 24.0), "Back", &small)
                .clicked
            {
                self.step = self.step.saturating_sub(1);
            }
            let primary = ButtonOptions {
                kind: ButtonKind::Primary,
                ..small
            };
            if ui
                .button_with(
                    "continue",
                    Rect::new(r.x + 88.0, y, 96.0, 24.0),
                    "Continue",
                    &primary,
                )
                .clicked
            {
                self.step = (self.step + 1).min(STEPS.len() - 1);
            }
        }
    }
}

const SCENES: &[UiScene] = &[
    // Six nav items, one selected, the ok footer.
    UiScene {
        id: "sidebar-main",
        scale: 2.0,
        tier: MotionTier::Full,
        state: main_column,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The stopped footer: err dot, no address.
    UiScene {
        id: "sidebar-footer-error",
        scale: 2.0,
        tier: MotionTier::Full,
        state: error_column,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The wizard's steps in the nav slot: one done, one current, two upcoming.
    UiScene {
        id: "sidebar-wizard",
        scale: 2.0,
        tier: MotionTier::Full,
        state: wizard_column,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];

/// The centre of nav row `i` of the first column.
const fn row(i: usize) -> (f32, f32) {
    (X + 8.0 + 92.0, TOP + 8.0 + 34.0 * i as f32 + 16.0)
}

/// What the bench replays: hover and click a row, Tab into the list, step down and activate.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(row(2).0, row(2).1)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Key(NamedKey::Enter)],
    },
];
