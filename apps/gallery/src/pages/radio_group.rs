//! The `radio-group` page: a live group (window mode, the bench and the mid-transition scenes) and
//! a specimen sheet of every state painted through the public look (D16).

use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::ui::Ui;
use lune_ui::widgets::radio_group::{RadioLook, RadioOption, RadioState, paint};

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "radio-group",
    title: "Radio group",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live { selected: 1 })
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
/// The live group's top left and width; rows are 28 high.
const GROUP: Rect = Rect::new(X, Y + 24.0, 260.0, 0.0);

/// The centre of live row `i`.
const fn row(i: usize) -> (f32, f32) {
    (GROUP.x + 130.0, GROUP.y + 28.0 * i as f32 + 14.0)
}

/// A group of four: the last option is disabled.
struct Live {
    selected: u64,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        card(ui, stage, "Radio group");
        heading(ui, "RADIO GROUP", X, Y);
        let options = [
            RadioOption {
                key: 1,
                label: "Compact",
                enabled: true,
            },
            RadioOption {
                key: 2,
                label: "Comfortable",
                enabled: true,
            },
            RadioOption {
                key: 3,
                label: "Spacious",
                enabled: true,
            },
            RadioOption {
                key: 4,
                label: "Custom (unavailable)",
                enabled: false,
            },
        ];
        ui.radio_group("group", GROUP, &options, &mut self.selected, "Density");
    }
}

/// Every state, each drawn from a [`RadioState`] through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        card(ui, stage, "Radio group states");
        heading(ui, "RADIO STATES", X, Y);
        let theme = *ui.theme();
        let off = RadioState::default();
        let on = RadioState {
            selected: true,
            ..off
        };
        let cases = [
            ("Unselected", off, false),
            ("Selected", on, false),
            (
                "Hover",
                RadioState {
                    hovered: true,
                    ..off
                },
                false,
            ),
            (
                "Selected, focus-visible",
                RadioState {
                    focus_visible: true,
                    ..on
                },
                false,
            ),
            ("Disabled", off, true),
            ("Disabled, selected", on, true),
        ];
        for (i, (name, state, disabled)) in cases.into_iter().enumerate() {
            let r = Rect::new(X, Y + 24.0 + i as f32 * 32.0, 260.0, 28.0);
            let look = RadioLook::of(&theme, state);
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
            paint(p, r, name, &look);
            if disabled {
                p.pop_xform();
            }
        }
    }
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "radio-group-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The pointer enters row 2, then clicks at 200 ms: the fill is `fast` (180 ms), half way at
    // 90 ms.
    UiScene {
        id: "radio-select-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(row(2).0, row(2).1)],
            },
            Step {
                at_ms: 200,
                events: &[Ev::Click],
            },
        ],
        capture_ms: 290,
        expect: Expect::Moving,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "radio-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(row(2).0, row(2).1)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // Tab focuses the group's stop, the selected row; the ring is `fast`: half way at 90 ms.
    UiScene {
        id: "radio-focus-mid",
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

/// What the bench replays: hover, click, Tab, the arrows and Space.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(row(1).0, row(1).1)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::ArrowUp)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Tab],
    },
];
