//! The `steps` page: four steps with the second current, a lead line and a long label.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;

use super::{card, heading};
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "steps",
    title: "Steps",
    kind: PageKind::Ui(UiPage {
        state: states,
        scenes: SCENES,
        bench: &[],
    }),
};

const LABELS: [&str; 4] = [
    "Create an account",
    "Choose a storage location for the workspace",
    "Review the settings",
    "Finish",
];

struct States;

fn states() -> Box<dyn PageState> {
    Box::new(States)
}

impl PageState for States {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Steps");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "DONE, CURRENT, UPCOMING", x, y);
        ui.steps_with_lead(
            "steps",
            Rect::new(x, y + 24.0, 220.0, 0.0),
            &LABELS,
            1,
            Some("Step 2 of 4"),
        );
    }
}

const SCENES: &[UiScene] = &[UiScene {
    id: "steps-states",
    scale: 2,
    tier: MotionTier::Full,
    state: states,
    script: &[],
    capture_ms: 0,
    expect: Expect::Settled,
}];
