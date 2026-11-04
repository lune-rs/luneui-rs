//! The `scrollbar` page: the overlay thumb of the scroll area (`widget.scrollbar`), on the same
//! area and content the `scroll-area` page draws. The scenes put the pointer on the thumb and
//! drag it.

use lune_ui::motion::MotionTier;
use lune_ui::ui::Ui;
use lune_ui::widgets::scroll_area::{Bars, ScrollOptions};

use super::scroll_area::{AREA, TALL, lines};
use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "scrollbar",
    title: "Scrollbar",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        bars: (Bars::Auto, Bars::Auto),
    })
}

/// A scroll area whose only point is its thumb.
struct Live {
    bars: (Bars, Bars),
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Scrollbar");
        heading(ui, "SCROLLBAR", card.x + 24.0, card.y + 24.0);
        let opts = ScrollOptions {
            bars: self.bars,
            ..ScrollOptions::default()
        };
        ui.scroll_area("area", AREA, &opts, |ui, view| ((), lines(ui, view, TALL)));
    }
}

/// The vertical thumb: the area's key, its bar, the thumb.
const THUMB: &[&str] = &["area", "bar-y", "thumb"];
/// Where the pointer is dragged to: right of the area (so outside it), 60 px down from the thumb's
/// first place. The thumb at offset 0 spans `AREA.y` to `AREA.y + 120`; 60 px of travel is 180 px
/// of offset (travel 240, range 720).
const OUTSIDE: (f32, f32) = (AREA.x + AREA.w + 46.0, AREA.y + 60.0 + 60.0);

const SCENES: &[UiScene] = &[
    // The pointer settles on the thumb, the press grabs it, the pointer is dragged out of the area:
    // the thumb stays shown, in its hover colour, and the content has followed.
    UiScene {
        id: "scrollbar-dragging",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(THUMB)],
            },
            Step {
                at_ms: 300,
                events: &[Ev::Press],
            },
            Step {
                at_ms: 340,
                events: &[Ev::PointerAt(OUTSIDE.0, OUTSIDE.1)],
            },
        ],
        capture_ms: 900,
        expect: Expect::Settled,
    },
];

/// What the bench replays: the pointer finds the thumb, drags it down in steps and lets go.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(THUMB)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Press],
    },
    Step {
        at_ms: 230,
        events: &[Ev::PointerAt(OUTSIDE.0, AREA.y + 90.0)],
    },
    Step {
        at_ms: 260,
        events: &[Ev::PointerAt(OUTSIDE.0, AREA.y + 150.0)],
    },
    Step {
        at_ms: 290,
        events: &[Ev::PointerAt(OUTSIDE.0, AREA.y + 210.0)],
    },
    Step {
        at_ms: 320,
        events: &[Ev::PointerAt(OUTSIDE.0, AREA.y + 120.0)],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Release],
    },
];
