//! The `button` page: a live row of buttons (window mode, the bench and the mid-transition
//! scenes) and a specimen sheet of every state painted through the public look (D16).

use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, ButtonLook, ButtonState, button_width, paint};

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "button",
    title: "Button",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live)
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

/// Real buttons through the `Ui`.
struct Live;

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Button");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "BUTTONS", x, y);
        let by = y + 20.0;
        let pw = button_width(ui.painter(), "Primary");
        ui.button(
            "primary",
            Rect::new(x, by, pw, 32.0),
            "Primary",
            ButtonKind::Primary,
        );
        let sw = button_width(ui.painter(), "Secondary");
        ui.button(
            "secondary",
            Rect::new(x + pw + 8.0, by, sw, 32.0),
            "Secondary",
            ButtonKind::Secondary,
        );
        let dw = button_width(ui.painter(), "Disabled");
        ui.with_enabled(false, |ui| {
            ui.button(
                "disabled",
                Rect::new(x + pw + sw + 16.0, by, dw, 32.0),
                "Disabled",
                ButtonKind::Secondary,
            );
        });
    }
}

/// Every state, each drawn from a [`ButtonState`] through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Button states");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "BUTTON STATES", x, y);
        let theme = *ui.theme();
        let rest = ButtonState::default();
        let cases = [
            ("Primary", ButtonKind::Primary, rest, false),
            ("Secondary", ButtonKind::Secondary, rest, false),
            (
                "Hover",
                ButtonKind::Secondary,
                ButtonState {
                    hovered: true,
                    ..rest
                },
                false,
            ),
            (
                "Pressed",
                ButtonKind::Secondary,
                ButtonState {
                    hovered: true,
                    pressed: true,
                    ..rest
                },
                false,
            ),
            (
                "Focus-visible",
                ButtonKind::Secondary,
                ButtonState {
                    focus_visible: true,
                    ..rest
                },
                false,
            ),
            ("Disabled", ButtonKind::Secondary, rest, true),
        ];
        for (i, (name, kind, state, disabled)) in cases.into_iter().enumerate() {
            let r = Rect::new(x + i as f32 * 136.0, y + 20.0, 120.0, 32.0);
            let look = ButtonLook::of(&theme, kind, state);
            let p = ui.painter();
            if disabled {
                p.push_xform(Xform {
                    origin: (0.0, 0.0),
                    scale: 1.0,
                    dy: 0.0,
                    alpha: 0.4,
                });
            }
            paint(p, r, "Button", kind, &look);
            if disabled {
                p.pop_xform();
            }
            caption(ui, name, r.x, r.y + 44.0);
        }
    }
}

const SECONDARY: &[&str] = &["secondary"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "button-states",
        scale: 2,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "button-hover-mid",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(SECONDARY)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // Press is `fast` (180 ms): half way at 90 ms.
    UiScene {
        id: "button-press-mid",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(SECONDARY), Ev::Press],
        }],
        capture_ms: 90,
        expect: Expect::Moving,
    },
    // Tab focuses the first stop, the primary button; the ring is `fast`.
    UiScene {
        id: "button-focus-mid",
        scale: 2,
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

/// What the bench replays: hover, click, Tab, Enter, Shift+Tab.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(SECONDARY)],
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
        events: &[Ev::Key(NamedKey::Enter)],
    },
    Step {
        at_ms: 800,
        events: &[Ev::ShiftTab],
    },
];
