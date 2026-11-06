//! The `scroll-area` page: a clipped viewport over content three times its height
//! (`layout.scroll-area`). The window shows the live area; the scenes script the wheel and the
//! pointer. [`AREA`] and [`lines`] are shared with the `scrollbar` page, which draws the same area.

use std::time::Duration;

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::scroll_area::{Bars, Extent, Reveal, ScrollOptions, ScrollView, Target};

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

/// Lines that fit inside the viewport (5 x 45 px < 360 px).
const FEW: u32 = 5;
/// The log of the follow-tail scene starts at this many lines and gets one more at [`GROW_AT`].
const LOG: u32 = 20;
const GROW_AT: Duration = Duration::from_millis(400);
/// The scroll-into-view scene asks at this time for the line one viewport down.
const REVEAL_AT: Duration = Duration::from_millis(300);
/// The line that starts one viewport (8 lines of 45 px) below the top.
const FAR_LINE: u32 = 8;

fn base() -> Live {
    Live {
        lines: TALL,
        bars: (Bars::Auto, Bars::Auto),
        follow_tail: false,
        grow_at: None,
        reveal_at: None,
        revealed: false,
    }
}

fn live() -> Box<dyn PageState> {
    Box::new(base())
}

fn fits() -> Box<dyn PageState> {
    Box::new(Live {
        lines: FEW,
        ..base()
    })
}

fn disabled() -> Box<dyn PageState> {
    Box::new(Live {
        bars: (Bars::Disabled, Bars::Disabled),
        ..base()
    })
}

fn log() -> Box<dyn PageState> {
    Box::new(Live {
        lines: LOG,
        follow_tail: true,
        grow_at: Some(GROW_AT),
        ..base()
    })
}

fn into_view() -> Box<dyn PageState> {
    Box::new(Live {
        reveal_at: Some(REVEAL_AT),
        ..base()
    })
}

/// The page plays the app: it owns the content and the bar settings.
struct Live {
    lines: u32,
    bars: (Bars, Bars),
    follow_tail: bool,
    /// A line is added from this time on (a log growing).
    grow_at: Option<Duration>,
    /// The app asks once, from this time on, to bring [`FAR_LINE`] into view.
    reveal_at: Option<Duration>,
    revealed: bool,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Scroll area");
        heading(ui, "SCROLL AREA", card.x + 24.0, card.y + 24.0);
        let opts = ScrollOptions {
            bars: self.bars,
            follow_tail: self.follow_tail,
            ..ScrollOptions::default()
        };
        let count = self.lines + u32::from(self.grow_at.is_some_and(|t| ui.time() >= t));
        if !self.revealed && self.reveal_at.is_some_and(|t| ui.time() >= t) {
            self.revealed = true;
            let top = f64::from(FAR_LINE) * f64::from(LINE_H);
            ui.scroll_to(
                "area",
                Target::span_y(top, f64::from(LINE_H)),
                Reveal::Center,
            );
        }
        ui.scroll_area("area", AREA, &opts, |ui, view| ((), lines(ui, view, count)));
    }
}

const AREA_PATH: &[&str] = &["area"];

/// A wheel-free scroll to 40 percent of the range (720 px): 288 px, applied 1:1.
const TO_40: Ev = Ev::Scroll {
    x: 0.0,
    y: -288.0,
    precise: true,
};

const SCENES: &[UiScene] = &[
    UiScene {
        id: "scroll-area-rest",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The pointer is over the area, the content is at 40 percent: the thumb shows part way down.
    UiScene {
        id: "scroll-area-hover",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(AREA_PATH)],
            },
            Step {
                at_ms: 100,
                events: &[TO_40],
            },
        ],
        capture_ms: 800,
        expect: Expect::Settled,
    },
    UiScene {
        id: "scroll-area-fits",
        scale: 2.0,
        tier: MotionTier::Full,
        state: fits,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(AREA_PATH)],
        }],
        capture_ms: 600,
        expect: Expect::Settled,
    },
    UiScene {
        id: "scroll-area-disabled",
        scale: 2.0,
        tier: MotionTier::Full,
        state: disabled,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(AREA_PATH)],
        }],
        capture_ms: 600,
        expect: Expect::Settled,
    },
    // One line is added at 400 ms: follow-tail takes the area to the new end.
    UiScene {
        id: "scroll-area-follow-tail",
        scale: 2.0,
        tier: MotionTier::Full,
        state: log,
        script: &[
            Step {
                at_ms: 200,
                events: &[],
            },
            Step {
                at_ms: 500,
                events: &[],
            },
        ],
        capture_ms: 1000,
        expect: Expect::Settled,
    },
    // One wheel line (48 px) is the `scroll` kind at `fast` (180 ms): half way at 90 ms. The
    // pointer entered the area 300 ms before, so the thumb has finished its own reveal.
    UiScene {
        id: "scroll-area-wheel",
        scale: 2.0,
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
    // The app asks at 300 ms for the line one viewport down: `scroll` kind, `fast`, sine-in-out.
    UiScene {
        id: "scroll-area-into-view",
        scale: 2.0,
        tier: MotionTier::Full,
        state: into_view,
        script: &[Step {
            at_ms: 300,
            events: &[],
        }],
        capture_ms: 390,
        expect: Expect::Moving,
    },
    // The pointer enters the area at 300 ms: the thumb fades in over `fast`.
    UiScene {
        id: "scroll-area-thumb-reveal",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 300,
            events: &[Ev::PointerOver(AREA_PATH)],
        }],
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
