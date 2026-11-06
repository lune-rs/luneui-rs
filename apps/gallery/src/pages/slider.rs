//! The `slider` page: live sliders (window mode, the bench and the mid-transition scenes) and a
//! specimen sheet of every state painted through the public look (D16).

use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::slider::{SliderLook, SliderOptions, SliderState, paint};

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "slider",
    title: "Slider",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        stepped: 40.0,
        uneven: 9.0,
        continuous: 0.25,
    })
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
/// The label column's width; the sliders start after it.
const LABEL_W: f32 = 180.0;
/// The sliders' left edge and width, and the row pitch.
const TRACK_X: f32 = X + LABEL_W;
const TRACK_W: f32 = 320.0;
const PITCH: f32 = 36.0;

/// Row `i`'s slider: 24 high, the track centred in it.
const fn row(i: usize) -> Rect {
    Rect::new(TRACK_X, Y + 24.0 + PITCH * i as f32, TRACK_W, 24.0)
}

/// The point at `fraction` of row `i`'s track.
const fn on_track(i: usize, fraction: f32) -> (f32, f32) {
    (
        TRACK_X + TRACK_W * fraction,
        Y + 24.0 + PITCH * i as f32 + 12.0,
    )
}

fn label(ui: &mut Ui<'_>, text: &str, y: f32) {
    let spec = Spec::plain(text, Style::ui(THEME.font_base, THEME.fg_secondary));
    ui.painter().text(&spec, X, y + 3.0);
}

/// Four sliders: stepped, a step that does not divide the range, continuous and disabled.
struct Live {
    stepped: f32,
    uneven: f32,
    continuous: f32,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        card(ui, "Slider");
        heading(ui, "SLIDERS", X, Y);
        let stepped = SliderOptions {
            min: 0.0,
            max: 100.0,
            step: 10.0,
            name: "Stepped",
        };
        label(ui, "Step 10", row(0).y);
        ui.slider("stepped", row(0), &mut self.stepped, &stepped);
        let uneven = SliderOptions {
            min: 0.0,
            max: 10.0,
            step: 3.0,
            name: "Uneven",
        };
        label(ui, "Step 3 of 10", row(1).y);
        ui.slider("uneven", row(1), &mut self.uneven, &uneven);
        let continuous = SliderOptions {
            min: 0.0,
            max: 1.0,
            step: 0.0,
            name: "Continuous",
        };
        label(ui, "Continuous", row(2).y);
        ui.slider("continuous", row(2), &mut self.continuous, &continuous);
        label(ui, "Disabled", row(3).y);
        let mut held = 60.0;
        ui.with_enabled(false, |ui| {
            let disabled = SliderOptions {
                name: "Disabled",
                ..stepped
            };
            ui.slider("disabled", row(3), &mut held, &disabled);
        });
    }
}

/// Every state, each drawn from a [`SliderState`] through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        card(ui, "Slider states");
        heading(ui, "SLIDER STATES", X, Y);
        let theme = *ui.theme();
        let rest = SliderState {
            fraction: 0.25,
            ..SliderState::default()
        };
        let cases = [
            ("Rest", rest, false),
            (
                "Hover",
                SliderState {
                    hovered: true,
                    ..rest
                },
                false,
            ),
            (
                "Focus-visible",
                SliderState {
                    focus_visible: true,
                    ..rest
                },
                false,
            ),
            ("Disabled", rest, true),
            (
                "Value mid-track",
                SliderState {
                    fraction: 0.5,
                    ..rest
                },
                false,
            ),
            (
                "Start",
                SliderState {
                    fraction: 0.0,
                    ..rest
                },
                false,
            ),
            (
                "End",
                SliderState {
                    fraction: 1.0,
                    ..rest
                },
                false,
            ),
        ];
        for (i, (name, state, disabled)) in cases.into_iter().enumerate() {
            let r = row(i);
            label(ui, name, r.y);
            let look = SliderLook::of(&theme, state);
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
            paint(p, r, &look);
            if disabled {
                p.pop_xform();
            }
        }
    }
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "slider-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Tab focuses the first slider and Right steps it at 200 ms: the knob is `fast` (180 ms), half
    // way at 90 ms.
    UiScene {
        id: "slider-value-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::Tab],
            },
            Step {
                at_ms: 200,
                events: &[Ev::Key(NamedKey::ArrowRight)],
            },
        ],
        capture_ms: 290,
        expect: Expect::Moving,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "slider-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(on_track(0, 0.4).0, on_track(0, 0.4).1)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // Tab focuses the first slider; the ring is `fast`: half way at 90 ms.
    UiScene {
        id: "slider-focus-mid",
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

/// What the bench replays: hover, a press and a drag along the track, Tab and the arrows.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(on_track(2, 0.2).0, on_track(2, 0.2).1)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Press],
    },
    Step {
        at_ms: 300,
        events: &[Ev::PointerAt(on_track(2, 0.5).0, on_track(2, 0.5).1)],
    },
    Step {
        at_ms: 400,
        events: &[Ev::PointerAt(on_track(2, 0.8).0, on_track(2, 0.8).1)],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Release],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 900,
        events: &[Ev::Key(NamedKey::ArrowRight)],
    },
    Step {
        at_ms: 1100,
        events: &[Ev::Key(NamedKey::ArrowLeft)],
    },
];
