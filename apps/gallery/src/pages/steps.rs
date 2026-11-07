//! The `steps` page: four steps with the second current, a lead line and a long label.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

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

/// The steps; the knobs set the current one, the lead line and the width.
struct States {
    current: usize,
    lead: bool,
    width: f32,
}

fn states() -> Box<dyn PageState> {
    Box::new(States {
        current: 1,
        lead: true,
        width: 220.0,
    })
}

impl PageState for States {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Steps");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "DONE, CURRENT, UPCOMING", x, y);
        ui.steps_with_lead(
            "steps",
            Rect::new(x, y + 24.0, self.width, 0.0),
            &LABELS,
            self.current,
            self.lead
                .then(|| format!("Step {} of {}", self.current + 1, LABELS.len()))
                .as_deref(),
        );
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let mut current = self.current as f32;
        if panel.number(
            ui,
            "Current step",
            &mut current,
            (0.0, (LABELS.len() - 1) as f32, 1.0),
        ) {
            self.current = current.round() as usize;
        }
        panel.toggle(ui, "Lead line", &mut self.lead);
        panel.number(ui, "Width", &mut self.width, (160.0, 400.0, 1.0));
    }
}

const SCENES: &[UiScene] = &[UiScene {
    id: "steps-states",
    scale: 2.0,
    tier: MotionTier::Full,
    state: states,
    script: &[],
    capture_ms: 0,
    expect: Expect::Settled,
}];
