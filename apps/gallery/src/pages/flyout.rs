//! The `flyout` page: a stand-in for the tray icon that opens the glass flyout above it
//! (`overlay.flyout`): the header with the state, a strip of compact stats, the routes and the
//! action rows. The scenes show the three tray states, the closed page, and the open and the
//! close in flight.

use std::time::Duration;

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::tray::TrayState as Tray;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::ButtonKind;
use lune_ui::widgets::flyout::Flyout;
use lune_ui::widgets::menu::{MenuItem, MenuRow};
use lune_ui::widgets::popover::{PlaceAlign, Side};
use lune_ui::widgets::stat::StatOptions;
use lune_ui::widgets::tone::Tone;

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "flyout",
    title: "Tray flyout",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The state's text in the flyout's header.
pub(super) fn state_text(tray: Tray) -> &'static str {
    match tray {
        Tray::Serving => "Serving",
        Tray::Degraded => "degraded",
        Tray::Paused => "paused",
    }
}

/// The tone of the dot beside the state's text.
pub(super) fn state_tone(tray: Tray) -> Tone {
    match tray {
        Tray::Serving => Tone::Ok,
        Tray::Degraded => Tone::Warn,
        Tray::Paused => Tone::Paused,
    }
}

pub(super) const ROUTES: [(&str, &str); 3] = [
    ("Opus", "corp-claude-opus"),
    ("Sonnet", "corp-gpt-5"),
    ("Haiku", "qwen3-coder-30b-a3b"),
];

pub(super) fn stats() -> [StatOptions<'static>; 3] {
    let cell = |label, value, gold| StatOptions {
        label,
        value,
        gold,
        ..StatOptions::default()
    };
    [
        cell("Today", "RM 48.20", false),
        cell("Saved", "14.6%", true),
        cell("Sessions", "3 live", false),
    ]
}

/// The action rows; the second follows the state.
pub(super) fn rows(tray: Tray) -> [MenuRow<'static>; 5] {
    let row = |text, shortcut, chevron, danger| {
        MenuRow::Entry(MenuItem {
            glyph: None,
            text,
            shortcut,
            disabled: false,
            danger,
            chevron,
        })
    };
    [
        row("Open Wane", "", false, false),
        row("Profile", "Work", true, false),
        row(
            if tray == Tray::Paused {
                "Resume compression"
            } else {
                "Pause compression"
            },
            "",
            false,
            false,
        ),
        row("Unload local model", "frees 19.3 GB", false, false),
        row("Quit Wane (stops proxy)", "", false, true),
    ]
}

/// The stand-in for the tray icon, near the window's bottom right.
fn tray_button(ui: &mut Ui<'_>) -> Rect {
    let win = ui.painter().window;
    Rect::new(win.right() - 96.0, win.bottom() - 56.0, 64.0, 28.0)
}

fn pinned(tray: Tray) -> Live {
    Live {
        open: true,
        tray,
        stop_at: None,
    }
}

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        open: false,
        ..pinned(Tray::Serving)
    })
}

fn serving() -> Box<dyn PageState> {
    Box::new(pinned(Tray::Serving))
}

fn degraded() -> Box<dyn PageState> {
    Box::new(pinned(Tray::Degraded))
}

fn paused() -> Box<dyn PageState> {
    Box::new(pinned(Tray::Paused))
}

fn closed() -> Box<dyn PageState> {
    Box::new(Live {
        open: false,
        ..pinned(Tray::Serving)
    })
}

fn closing() -> Box<dyn PageState> {
    Box::new(Live {
        stop_at: Some(STOP_AT),
        ..pinned(Tray::Serving)
    })
}

/// When the closing scene's app stops calling the flyout (after the 180 ms open).
const STOP_AT: Duration = Duration::from_millis(300);

/// The page plays the app: it owns `open` and the tray state.
struct Live {
    open: bool,
    tray: Tray,
    /// The app stops calling the flyout from this time on.
    stop_at: Option<Duration>,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Tray flyout");
        heading(ui, "TRAY FLYOUT", card.x + 24.0, card.y + 24.0);
        caption(
            ui,
            "The tray icon opens the flyout above it; Escape, a press outside or leaving the window closes it.",
            card.x + 24.0,
            card.y + 44.0,
        );
        let button = tray_button(ui);
        let tray = ui.button("tray", button, "Tray icon", ButtonKind::Secondary);
        if tray.clicked {
            self.open = !self.open;
        }
        if !self.open || self.stop_at.is_some_and(|t| ui.time() >= t) {
            return;
        }
        let stats = stats();
        let rows = rows(self.tray);
        let flyout = Flyout {
            anchor: tray.rect,
            anchor_id: Some(tray.id),
            side: Side::Top,
            align: PlaceAlign::End,
            mark: Some(icon::INFO),
            wordmark: "Wane",
            state: state_text(self.tray),
            tone: state_tone(self.tray),
            stats: &stats,
            routes: &ROUTES,
            rows: &rows,
            name: "Wane tray menu",
        };
        let r = ui.flyout("flyout", &flyout);
        if r.dismissed {
            self.open = false;
        }
        match r.picked {
            // Pause or resume: the header follows, the flyout stays.
            Some(2) => {
                self.tray = if self.tray == Tray::Paused {
                    Tray::Serving
                } else {
                    Tray::Paused
                };
            }
            Some(_) => self.open = false,
            None => {}
        }
    }
}

const TRAY: &[&str] = &["tray"];

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

const SCENES: &[UiScene] = &[
    // The open is `fast` (180 ms): settled by 500 ms.
    scene("flyout-open-serving", serving, &[], 500, Expect::Settled),
    scene("flyout-open-degraded", degraded, &[], 500, Expect::Settled),
    scene("flyout-open-paused", paused, &[], 500, Expect::Settled),
    scene("flyout-closed", closed, &[], 300, Expect::Settled),
    // Half way through the open at 90 ms.
    scene("flyout-opening", serving, &[], 90, Expect::Moving),
    // Open by 200 ms; the app stops calling at 300 ms; the dismiss is `instant` (120 ms): half
    // way at 360 ms.
    scene(
        "flyout-closing",
        closing,
        &[
            Step {
                at_ms: 200,
                events: &[],
            },
            Step {
                at_ms: 300,
                events: &[],
            },
        ],
        360,
        Expect::Moving,
    ),
];

/// What the bench replays: open from the tray icon, Down twice, pick, reopen, Escape, reopen,
/// press outside.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(TRAY)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Key(NamedKey::Enter)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1400,
        events: &[Ev::Key(NamedKey::Escape)],
    },
    Step {
        at_ms: 1700,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 2200,
        events: &[Ev::PointerAt(200.0, 200.0)],
    },
    Step {
        at_ms: 2300,
        events: &[Ev::Click],
    },
];
