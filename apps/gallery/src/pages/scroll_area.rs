//! The `scroll-area` page: a clipped viewport over content three times its height
//! (`layout.scroll-area`). The window shows the live area; the scenes script the wheel and the
//! pointer. [`AREA`] and [`lines`] are shared with the `scrollbar` page, which draws the same area.

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::scroll_area::{Bars, Extent, ScrollOptions, ScrollView};

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "scroll-area",
    title: "Scroll area",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The area, in window coordinates: inside the card, under the heading.
pub(super) const AREA: Rect = Rect::new(34.0, 82.0, 520.0, 360.0);
/// Height of one content line.
pub(super) const LINE_H: f32 = 45.0;
/// Lines that make the content three times the viewport.
pub(super) const TALL: u32 = 24;

/// `count` lines of `LINE_H` in content coordinates (origin at the viewport's top-left at offset
/// 0), alternate lines tinted, each numbered. The extent is the lines' stack.
pub(super) fn lines(ui: &mut Ui<'_>, view: &ScrollView, count: u32) -> Extent {
    let vp = view.viewport;
    for i in 0..count {
        let r = Rect::new(vp.x, vp.y + i as f32 * LINE_H, vp.w, LINE_H);
        let p = ui.painter();
        if i % 2 == 0 {
            p.fill(r, 0.0, THEME.surface_hover);
        }
        let spec = Spec::plain(
            format!("Line {}", i + 1),
            Style::ui(THEME.font_base, THEME.fg_primary),
        );
        p.text(&spec, r.x + 16.0, r.y + 12.0);
    }
    Extent::new(f64::from(vp.w), f64::from(count) * f64::from(LINE_H))
}

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        lines: TALL,
        bars: (Bars::Auto, Bars::Auto),
    })
}

/// The page plays the app: it owns the content and the bar settings.
struct Live {
    lines: u32,
    bars: (Bars, Bars),
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Scroll area");
        heading(ui, "SCROLL AREA", card.x + 24.0, card.y + 24.0);
        let opts = ScrollOptions {
            bars: self.bars,
            ..ScrollOptions::default()
        };
        let count = self.lines;
        ui.scroll_area("area", AREA, &opts, |ui, view| ((), lines(ui, view, count)));
    }
}

const AREA_PATH: &[&str] = &["area"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "scroll-area-rest",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // One wheel line (48 px) is the `scroll` kind at `fast` (180 ms): half way at 90 ms. The
    // pointer entered the area 100 ms before, so the thumb has finished its own reveal.
    UiScene {
        id: "scroll-area-wheel",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(AREA_PATH)],
            },
            Step {
                at_ms: 300,
                events: &[Ev::Scroll {
                    x: 0.0,
                    y: -1.0,
                    precise: false,
                }],
            },
        ],
        capture_ms: 390,
        expect: Expect::Moving,
    },
];

/// What the bench replays: the pointer enters, a burst of wheel lines, one in the other direction.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(AREA_PATH)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -3.0,
            precise: false,
        }],
    },
    Step {
        at_ms: 260,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -3.0,
            precise: false,
        }],
    },
    Step {
        at_ms: 320,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -3.0,
            precise: false,
        }],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Scroll {
            x: 0.0,
            y: 4.0,
            precise: false,
        }],
    },
];
