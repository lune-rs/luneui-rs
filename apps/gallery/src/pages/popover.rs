//! The `popover` page: a button that opens a placed popover (`overlay.popover`). The scenes move
//! the anchor to the places the placement rules differ (below, shifted by the right edge, flipped
//! and clamped), freeze the open and the close, and run once at scale 1. The `dropdown`, `menu`
//! and `dropdown-flipped` scenes of the catalogue entry exist on their own pages.

use std::time::Duration;

use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::ButtonKind;
use lune_ui::widgets::popover::{PopoverOptions, Surface};

use super::{caption, card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "popover",
    title: "Popover",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The button, in window coordinates: inside the card, under the heading.
const BUTTON: Rect = Rect::new(34.0, 82.0, 120.0, 32.0);
/// Four px from the right window edge (width 1040).
const RIGHT: Rect = Rect::new(916.0, 82.0, 120.0, 32.0);
/// Low enough that a tall popover fits neither below (336 + 450 > 652) nor above (300 - 4 < 450).
const MIDDLE: Rect = Rect::new(34.0, 300.0, 120.0, 32.0);
const SIZE: (f32, f32) = (220.0, 120.0);
const TALL: (f32, f32) = (220.0, 450.0);
/// When the closing scene's app stops calling the popover (after the 180 ms open).
const STOP_AT: Duration = Duration::from_millis(300);

fn base() -> Live {
    Live {
        anchor: BUTTON,
        size: SIZE,
        open: true,
        stop_at: None,
        surface: Surface::Panel,
    }
}

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        open: false,
        ..base()
    })
}

fn below() -> Box<dyn PageState> {
    Box::new(base())
}

fn shifted() -> Box<dyn PageState> {
    Box::new(Live {
        anchor: RIGHT,
        ..base()
    })
}

fn clamped() -> Box<dyn PageState> {
    Box::new(Live {
        anchor: MIDDLE,
        size: TALL,
        ..base()
    })
}

fn glass() -> Box<dyn PageState> {
    Box::new(Live {
        surface: Surface::Glass,
        ..base()
    })
}

fn closing() -> Box<dyn PageState> {
    Box::new(Live {
        stop_at: Some(STOP_AT),
        ..base()
    })
}

/// The page plays the app: it owns `open` and the anchor.
struct Live {
    anchor: Rect,
    size: (f32, f32),
    open: bool,
    /// The app stops calling the popover from this time on.
    stop_at: Option<Duration>,
    surface: Surface,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Popover");
        heading(ui, "POPOVER", card.x + 24.0, card.y + 24.0);
        let anchor = on_card(card, self.anchor);
        let button = ui.button("anchor", anchor, "Open popover", ButtonKind::Secondary);
        if button.clicked {
            self.open = true;
        }
        if self.open && self.stop_at.is_none_or(|t| ui.time() < t) {
            let options = PopoverOptions {
                anchor_id: Some(button.id),
                surface: self.surface,
                ..PopoverOptions::new(anchor, self.size)
            };
            let (frame, _placement, ()) = ui.popover("popover", &options, |ui, at| {
                caption(ui, "A popover", at.x + 12.0, at.y + 12.0);
                caption(ui, "Escape or a press outside", at.x + 12.0, at.y + 36.0);
                caption(ui, "closes it.", at.x + 12.0, at.y + 52.0);
            });
            if frame.dismissed.is_some() {
                self.open = false;
            }
        }
    }
}

const ANCHOR: &[&str] = &["anchor"];

const fn scene(
    id: &'static str,
    scale: u32,
    state: fn() -> Box<dyn PageState>,
    script: &'static [Step],
    capture_ms: u32,
    expect: Expect,
) -> UiScene {
    UiScene {
        id,
        scale: scale as f32,
        tier: MotionTier::Full,
        state,
        script,
        capture_ms,
        expect,
    }
}

const SCENES: &[UiScene] = &[
    // The open is `fast` (180 ms): settled by 500 ms.
    scene("popover-below", 2, below, &[], 500, Expect::Settled),
    scene("popover-shifted", 2, shifted, &[], 500, Expect::Settled),
    scene(
        "popover-flipped-clamped",
        2,
        clamped,
        &[],
        500,
        Expect::Settled,
    ),
    scene("popover-scale-1", 1, below, &[], 500, Expect::Settled),
    // Open by 200 ms; the app stops calling at 300 ms; the dismiss is `instant` (120 ms): half
    // way at 360 ms.
    scene(
        "popover-closing",
        2,
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
    // The open is `fast` (180 ms): half way at 90 ms.
    scene("popover-opening", 2, below, &[], 90, Expect::Moving),
    // The glass surface of the menu: the blurred page under a translucent fill.
    scene("popover-glass", 2, glass, &[], 500, Expect::Settled),
];

/// What the bench replays: open from the button, Escape, reopen, press outside.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(ANCHOR)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Key(NamedKey::Escape)],
    },
    Step {
        at_ms: 900,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1400,
        events: &[Ev::PointerAt(600.0, 500.0)],
    },
    Step {
        at_ms: 1500,
        events: &[Ev::Click],
    },
];
