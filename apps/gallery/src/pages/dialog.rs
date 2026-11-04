//! The `dialog` page: a row of buttons over the gallery background, one of which opens a modal
//! dialog through `Ui::dialog`. The scenes freeze it closed, half way through the open, and half
//! way through the close (the app stops calling the dialog, the Ui keeps drawing it).

use std::time::Duration;

use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::dialog::{Cancel, Dialog, DialogGlass};

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "dialog",
    title: "Dialog",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// When the closing scene's app stops calling the dialog (after the 250 ms open).
const STOP_AT: Duration = Duration::from_millis(300);

/// The page with a dialog that is open, or opens from its button.
struct Dialogs {
    open: bool,
    /// The app stops calling the dialog from this time on.
    stop_at: Option<Duration>,
}

fn live() -> Box<dyn PageState> {
    Box::new(Dialogs {
        open: false,
        stop_at: None,
    })
}

fn opening() -> Box<dyn PageState> {
    Box::new(Dialogs {
        open: true,
        stop_at: None,
    })
}

fn closing() -> Box<dyn PageState> {
    Box::new(Dialogs {
        open: true,
        stop_at: Some(STOP_AT),
    })
}

fn glass() -> DialogGlass {
    DialogGlass {
        panel: lunegit_dark::PANEL.premultiplied(),
        fill: lunegit_dark::WINDOW.premultiplied(),
        noise: true,
    }
}

impl PageState for Dialogs {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Dialog");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "A DIALOG OVER THE PAGE", x, y);
        let by = y + 20.0;
        let mut bx = x;
        for (key, text, kind) in [
            ("open", "Open dialog", ButtonKind::Primary),
            ("other", "Page action", ButtonKind::Secondary),
            ("more", "Another", ButtonKind::Secondary),
        ] {
            let w = button_width(ui.painter(), text);
            let r = ui.button(key, Rect::new(bx, by, w, 32.0), text, kind);
            if key == "open" && r.clicked {
                self.open = true;
            }
            bx += w + 8.0;
        }
        caption(
            ui,
            "Enter confirms, Escape cancels, Tab stays inside.",
            x,
            by + 48.0,
        );

        if self.open && self.stop_at.is_none_or(|t| ui.time() < t) {
            let d = Dialog {
                title: "Discard changes",
                message: "The document has unsaved changes. Closing it will discard them.",
                extra_h: 28.0,
                confirm: "Discard",
                cancel: Cancel::Default,
            };
            let r = ui.dialog("dialog", &d, &glass(), |ui, bx, by, _| {
                caption(ui, "3 files changed since the last save.", bx, by + 12.0);
            });
            if r.confirmed || r.cancelled {
                self.open = false;
            }
        }
    }
}

const OPEN: &[&str] = &["open"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "dialog-closed",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The open is `pop`, base (250 ms): half way at 125 ms.
    UiScene {
        id: "dialog-opening",
        scale: 2,
        tier: MotionTier::Full,
        state: opening,
        script: &[],
        capture_ms: 125,
        expect: Expect::Moving,
    },
    // Open by 260 ms; the app stops calling at 300 ms; the close is `dismiss`, fast (180 ms):
    // half way at 390 ms.
    UiScene {
        id: "dialog-closing",
        scale: 2,
        tier: MotionTier::Full,
        state: closing,
        script: &[
            Step {
                at_ms: 260,
                events: &[],
            },
            Step {
                at_ms: 300,
                events: &[],
            },
        ],
        capture_ms: 390,
        expect: Expect::Moving,
    },
];

/// What the bench replays: open from the button, Tab, Enter, reopen, Escape.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(OPEN)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Key(NamedKey::Enter)],
    },
    Step {
        at_ms: 1200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1700,
        events: &[Ev::Key(NamedKey::Escape)],
    },
];
