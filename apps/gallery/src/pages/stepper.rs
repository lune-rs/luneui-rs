//! The `stepper` page: a live stepper (window mode, the bench and the mid-transition scenes) and a
//! specimen sheet of every state painted through the public look (D16).

use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::ui::Ui;
use lune_ui::widgets::stepper::{StepRange, StepperLook, StepperState, paint};

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "stepper",
    title: "Stepper",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live { value: 12 })
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

const RANGE: StepRange = StepRange {
    min: 0.0,
    max: 99.0,
    step: 1.0,
};

/// One real stepper through the `Ui`; the app applies each reported step.
struct Live {
    value: u32,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Stepper");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "STEPPER", x, y);
        let resp = ui.stepper(
            "stepper",
            Rect::new(x, y + 20.0, 140.0, 32.0),
            self.value,
            RANGE,
            "Count",
        );
        if let Some(step) = resp.step {
            let next = i64::from(self.value) + i64::from(step);
            self.value = next.clamp(0, 99) as u32;
        }
        if let Some(set) = resp.set {
            self.value = set as u32;
        }
    }
}

/// Every state, each drawn from a [`StepperState`] through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Stepper states");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "STEPPER STATES", x, y);
        let theme = *ui.theme();
        let off = StepperState::default();
        let cases = [
            ("Rest", off, false),
            (
                "Minus hovered",
                StepperState {
                    minus_hovered: true,
                    ..off
                },
                false,
            ),
            (
                "Plus hovered",
                StepperState {
                    plus_hovered: true,
                    ..off
                },
                false,
            ),
            (
                "Focus-within",
                StepperState {
                    focus_within: true,
                    ..off
                },
                false,
            ),
            ("Disabled", off, true),
        ];
        for (i, (name, state, disabled)) in cases.into_iter().enumerate() {
            let r = Rect::new(x, y + 24.0 + i as f32 * 44.0, 140.0, 32.0);
            let look = StepperLook::of(&theme, state);
            let p = ui.painter();
            if disabled {
                p.push_xform(Xform {
                    origin: (0.0, 0.0),
                    scale: 1.0,
                    dx: 0.0,
                    dy: 0.0,
                    alpha: 0.4,
                });
            }
            paint(p, r, 12, &look);
            if disabled {
                p.pop_xform();
            }
            caption(ui, name, r.right() + 12.0, r.y + 8.0);
        }
    }
}

/// The live stepper's plus button, by its path.
const PLUS: &[&str] = &["stepper", "plus"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "stepper-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "stepper-button-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(PLUS)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // Tab focuses the stepper; the ring is `fast` (180 ms): half way at 90 ms.
    UiScene {
        id: "stepper-focus-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::Tab],
        }],
        capture_ms: 90,
        expect: Expect::Moving,
    },
];

/// What the bench replays: hover, click, Tab, Up.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(PLUS)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Key(NamedKey::ArrowUp)],
    },
];
