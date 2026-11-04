//! The `tooltip` page: an icon button with a tooltip (`overlay.tooltip`). The tooltip shows after
//! 800 ms of hover; the scenes script the pointer and the time. The button is drawn here from the
//! public pieces (a region, a node, a fill and a glyph) because the toolkit has no icon button.

use lune_ui::access::{Node, Role, State};
use lune_ui::icon;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::{Sense, Ui};

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "tooltip",
    title: "Tooltip",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The icon button, in window coordinates: inside the card, under the heading.
const ANCHOR: Rect = Rect::new(34.0, 82.0, 32.0, 32.0);
const TEXT: &str = "Rename";

fn live() -> Box<dyn PageState> {
    Box::new(Live)
}

/// One icon button with a one-line tooltip.
struct Live;

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Tooltip");
        heading(ui, "TOOLTIP", card.x + 24.0, card.y + 24.0);
        let id = ui.id("icon");
        let r = ui.interact(id, ANCHOR, Sense::CLICK_FOCUS);
        let mut node = Node::new(id, Role::Button, "", ANCHOR);
        node.states.insert(State::Focusable);
        ui.node(node);
        let p = ui.painter();
        if r.hovered {
            p.fill(ANCHOR, 4.0, THEME.surface_hover);
        }
        p.text_centered(&icon::glyph(icon::EDIT, 16.0, THEME.fg_primary), ANCHOR);
        ui.tooltip(&r, TEXT);
    }
}

const ICON: &[&str] = &["icon"];

const SCENES: &[UiScene] = &[
    // The pointer arrives at 0; the tooltip is due at 800 ms and its appear is `fast` (180 ms):
    // half way at 900 ms.
    UiScene {
        id: "tooltip-appearing",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(ICON)],
        }],
        capture_ms: 900,
        expect: Expect::Moving,
    },
];

/// What the bench replays: the pointer arrives, the tooltip shows, the pointer leaves.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(ICON)],
    },
    Step {
        at_ms: 1600,
        events: &[Ev::PointerAt(500.0, 400.0)],
    },
];
