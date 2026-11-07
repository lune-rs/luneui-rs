//! The `icon-rail` page: the rail with five buttons at the top and five docked at the bottom, an
//! item marked active (the open panel), the dot on one top item and the count badge on one bottom
//! item. A click moves the active mark; the arrows rove through the two groups and wrap. The
//! glyphs are the page's own stand-ins for an app's panels.

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::icon_rail::{PITCH, RailItem};

use super::{caption, card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "icon-rail",
    title: "Icon rail",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
/// The rail's rect: 48 wide (4 + 40 + 4), tall enough for both groups with room between.
const RAIL: Rect = Rect::new(X, Y + 24.0, 48.0, 520.0);

// Stand-in glyphs, from the gallery's font (codicons).
const GLYPH_A: &str = "\u{ea62}";
const GLYPH_B: &str = "\u{ea68}";
const GLYPH_C: &str = "\u{eafc}";
const GLYPH_D: &str = "\u{ebdf}";
const GLYPH_E: &str = "\u{eb26}";
const GLYPH_F: &str = "\u{ec10}";
const GLYPH_G: &str = "\u{ea85}";
const GLYPH_H: &str = "\u{eaa2}";
const GLYPH_I: &str = "\u{eb32}";

/// The top group's keys and the item that carries the dot; the bottom group's and the badge's.
const TOP: [(u64, &str, &str); 5] = [
    (1, GLYPH_A, "Repository"),
    (2, GLYPH_B, "Branches"),
    (3, GLYPH_C, "Changes"),
    (4, GLYPH_D, "Files"),
    (5, GLYPH_E, "Notes"),
];
const BOTTOM: [(u64, &str, &str); 5] = [
    (6, GLYPH_F, "Assistant"),
    (7, GLYPH_G, "Console"),
    (8, GLYPH_H, "Alerts"),
    (9, GLYPH_I, "Help"),
    (10, icon::GEAR, "Settings"),
];
const DOT_KEY: u64 = 3;
const BADGE_KEY: u64 = 8;

fn live() -> Box<dyn PageState> {
    Box::new(Live::new(true, Some(3)))
}

fn plain() -> Box<dyn PageState> {
    Box::new(Live::new(false, None))
}

fn dotted() -> Box<dyn PageState> {
    Box::new(Live::new(true, None))
}

fn counted() -> Box<dyn PageState> {
    Box::new(Live::new(false, Some(3)))
}

struct Live {
    /// The open panel's key.
    active: u64,
    dot: bool,
    badge: Option<u32>,
    last: Option<u64>,
}

impl Live {
    fn new(dot: bool, badge: Option<u32>) -> Live {
        Live {
            active: 1,
            dot,
            badge,
            last: None,
        }
    }
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Icon rail");
        let (ox, oy) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "ICON RAIL", ox, oy);
        let item = |(key, glyph, name): (u64, &'static str, &'static str)| RailItem {
            key,
            glyph,
            name,
            active: key == self.active,
            enabled: true,
            dot: self.dot && key == DOT_KEY,
            badge: self.badge.filter(|_| key == BADGE_KEY),
            note: match key {
                DOT_KEY if self.dot => Some("Uncommitted changes"),
                BADGE_KEY if self.badge.is_some() => Some("Unread notifications"),
                _ => None,
            },
        };
        let top: Vec<RailItem<'_>> = TOP.iter().map(|i| item(*i)).collect();
        let bottom: Vec<RailItem<'_>> = BOTTOM.iter().map(|i| item(*i)).collect();
        if let Some(key) = ui.icon_rail("rail", on_card(card, RAIL), &top, &bottom, "Panels") {
            self.active = key;
            self.last = Some(key);
        }
        let shown = self.last.map_or("none".to_owned(), |k| k.to_string());
        caption(
            ui,
            &format!("Last activated: {shown}"),
            ox + 72.0,
            oy + 34.0,
        );
    }
}

/// The centre of the top item `i`.
const fn top_at(i: usize) -> (f32, f32) {
    (RAIL.x + 24.0, RAIL.y + 10.0 + PITCH * i as f32 + 20.0)
}

const HOVER: (f32, f32) = top_at(1);
const PRESS: (f32, f32) = top_at(2);

const SCENES: &[UiScene] = &[
    // Five buttons at the top; the first panel is open.
    UiScene {
        id: "icon-rail-top",
        scale: 2.0,
        tier: MotionTier::Full,
        state: plain,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Five docked at the bottom, the count badge on the third.
    UiScene {
        id: "icon-rail-bottom",
        scale: 2.0,
        tier: MotionTier::Full,
        state: counted,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The pointer rests on the second button: its fill has settled (`instant`, 120 ms); the
    // tooltip is still pending.
    UiScene {
        id: "icon-rail-hover",
        scale: 2.0,
        tier: MotionTier::Full,
        state: plain,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(HOVER.0, HOVER.1)],
        }],
        capture_ms: 400,
        expect: Expect::Waiting,
    },
    // Held down on the third button, settled at scale 0.98.
    UiScene {
        id: "icon-rail-pressed",
        scale: 2.0,
        tier: MotionTier::Full,
        state: plain,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(PRESS.0, PRESS.1), Ev::Press],
        }],
        capture_ms: 600,
        expect: Expect::Settled,
    },
    // Press is `fast` (180 ms): half way at 90 ms.
    UiScene {
        id: "icon-rail-press-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: plain,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(PRESS.0, PRESS.1), Ev::Press],
        }],
        capture_ms: 90,
        expect: Expect::Moving,
    },
    // The dot on the third top button (uncommitted changes).
    UiScene {
        id: "icon-rail-commit-dot",
        scale: 2.0,
        tier: MotionTier::Full,
        state: dotted,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];

/// What the bench replays: hover and click a top button, Tab into the rail, arrow through both
/// groups and wrap, Enter.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(HOVER.0, HOVER.1)],
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
        at_ms: 700,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::ArrowUp)],
    },
    Step {
        at_ms: 900,
        events: &[Ev::Key(NamedKey::ArrowUp)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Key(NamedKey::Enter)],
    },
];
