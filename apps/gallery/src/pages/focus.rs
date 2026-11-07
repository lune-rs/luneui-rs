//! The `focus` page: focus-visible rings on every control, the ring appearing, the initial focus
//! of a dialog opened after a keyboard press, and pointer focus (no rings, the field's accent
//! border).

use std::time::Duration;

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, ButtonLook, ButtonState};
use lune_ui::widgets::checkbox::{CheckboxLook, CheckboxState};
use lune_ui::widgets::combo::{ComboLook, ComboState};
use lune_ui::widgets::dialog::{Cancel, Dialog};
use lune_ui::widgets::field::{FieldLook, FieldState};
use lune_ui::widgets::radio_group::{self, RadioLook, RadioState};
use lune_ui::widgets::stepper::{StepperLook, StepperState};
use lune_ui::widgets::toggle::{ToggleLook, ToggleState};
use lune_ui::widgets::{button, checkbox, combo, field, stepper, toggle};

use super::controls_ui::{Controls, glass};
use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "focus",
    title: "Focus",
    kind: PageKind::Ui(UiPage {
        state: controls,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The controls, and a confirm dialog from `open_at` on.
struct Focus {
    controls: Controls,
    open_at: Option<Duration>,
}

fn controls() -> Box<dyn PageState> {
    Box::new(Focus {
        controls: Controls::new(),
        open_at: None,
    })
}

fn dialog_after() -> Box<dyn PageState> {
    Box::new(Focus {
        controls: Controls::new(),
        open_at: Some(Duration::from_millis(100)),
    })
}

impl PageState for Focus {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "luneui gallery");
        self.controls.draw(ui, area);
        if self.open_at.is_some_and(|t| ui.time() >= t) {
            let d = Dialog {
                title: "Discard changes",
                message: "The document has unsaved changes. Closing it will discard them.",
                extra_h: 0.0,
                confirm: "Discard",
                cancel: Cancel::Default,
            };
            ui.dialog("dialog", &d, &glass(), |_, _, _, _| {});
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        // A dialog is modal: it is closed in the dialog, not here.
        let mut open = self.open_at.is_some();
        if panel.toggle(ui, "Dialog over the controls", &mut open) {
            self.open_at = open.then_some(Duration::ZERO);
        }
    }
}

/// Each kind of ring at full opacity, painted from the widgets' public looks (which draw it
/// through `focus_ring::paint`).
struct Kinds;

impl PageState for Kinds {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "Focus rings");
        let (x, y) = (area.x + 24.0, area.y + 24.0);
        heading(ui, "FOCUS-VISIBLE RINGS", x, y);
        let theme = *ui.theme();
        let focus_button = ButtonState {
            focus_visible: true,
            ..ButtonState::default()
        };
        let row = |ui: &mut Ui<'_>, i: usize, name: &str, draw: &dyn Fn(&mut Ui<'_>, Rect)| {
            let r = Rect::new(x, y + 28.0 + i as f32 * 44.0, 210.0, 32.0);
            draw(ui, r);
            caption(ui, name, r.right() + 24.0, r.y + 8.0);
        };
        row(ui, 0, "Button", &|ui, r| {
            let look = ButtonLook::of(&theme, ButtonKind::Secondary, focus_button);
            button::paint(ui.painter(), r, "Button", ButtonKind::Secondary, &look);
        });
        row(ui, 1, "Primary button", &|ui, r| {
            let look = ButtonLook::of(&theme, ButtonKind::Primary, focus_button);
            button::paint(ui.painter(), r, "Primary", ButtonKind::Primary, &look);
        });
        row(ui, 2, "Toggle", &|ui, r| {
            let state = ToggleState {
                focus_visible: true,
                ..ToggleState::default()
            };
            let r = Rect::new(r.x, r.y + 4.0, 40.0, 24.0);
            toggle::paint(ui.painter(), r, &ToggleLook::of(&theme, state));
        });
        row(ui, 3, "Checkbox", &|ui, r| {
            let state = CheckboxState {
                focus_visible: true,
                ..CheckboxState::default()
            };
            let r = Rect::new(r.x, r.y + 4.0, 160.0, 24.0);
            checkbox::paint(
                ui.painter(),
                r,
                "Checkbox",
                &CheckboxLook::of(&theme, state),
            );
        });
        row(ui, 4, "Text field", &|ui, r| {
            let look = FieldLook::of(&theme, false, true, FieldState::default());
            field::paint(ui.painter(), r, "lune", "Search", &look);
        });
        row(ui, 5, "Stepper", &|ui, r| {
            let state = StepperState {
                focus_within: true,
                ..StepperState::default()
            };
            let r = Rect::new(r.x, r.y, 140.0, 32.0);
            stepper::paint(ui.painter(), r, 12, &StepperLook::of(&theme, state));
        });
        row(ui, 6, "Combo", &|ui, r| {
            let state = ComboState {
                focus_visible: true,
                ..ComboState::default()
            };
            combo::paint(ui.painter(), r, "RznSans", &ComboLook::of(&theme, state));
        });
        row(ui, 7, "Radio", &|ui, r| {
            let state = RadioState {
                selected: true,
                focus_visible: true,
                ..RadioState::default()
            };
            let r = Rect::new(r.x, r.y + 2.0, 160.0, 28.0);
            radio_group::paint(ui.painter(), r, "Radio", &RadioLook::of(&theme, state));
        });
    }
}

const BUTTON: &[&str] = &["primary"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "focus-ring-kinds",
        scale: 2.0,
        tier: MotionTier::Full,
        state: || Box::new(Kinds),
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Tab focuses the first stop, the primary button; the ring is `fast`: half way at 90 ms.
    UiScene {
        id: "focus-ring-appearing",
        scale: 2.0,
        tier: MotionTier::Full,
        state: controls,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::Tab],
        }],
        capture_ms: 90,
        expect: Expect::Moving,
    },
    // Tab at 0, the dialog opens at 100 ms: its confirm button is focused, ring visible.
    UiScene {
        id: "focus-modal-initial",
        scale: 2.0,
        tier: MotionTier::Full,
        state: dialog_after,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::Tab],
            },
            Step {
                at_ms: 100,
                events: &[],
            },
            // Focus moves into the dialog when its first frame ends; the ring fades in from the
            // next frame.
            Step {
                at_ms: 150,
                events: &[],
            },
        ],
        capture_ms: 700,
        expect: Expect::Settled,
    },
    // Clicks focus the button, the toggle, the stepper and the field: no rings anywhere, the
    // field (focused last) shows its accent border.
    UiScene {
        id: "focus-pointer",
        scale: 2.0,
        tier: MotionTier::Full,
        state: controls,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(BUTTON), Ev::Click],
            },
            Step {
                at_ms: 100,
                events: &[Ev::PointerOver(&["toggle-off"]), Ev::Click],
            },
            Step {
                at_ms: 200,
                events: &[Ev::PointerOver(&["field-focused"]), Ev::Click],
            },
        ],
        capture_ms: 900,
        expect: Expect::Settled,
    },
];

/// What the bench replays: Tab through the controls and back.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 150,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 300,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 450,
        events: &[Ev::ShiftTab],
    },
];
