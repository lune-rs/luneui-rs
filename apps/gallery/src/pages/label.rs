//! The `label` page: every text role of `label.md` drawn through the `Ui`, settled.

use lune_ui::motion::MotionTier;
use lune_ui::ui::Ui;
use lune_ui::widgets::label::LabelRole;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "label",
    title: "Label",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Roles::default()),
        scenes: SCENES,
        bench: &[],
    }),
};

/// The roles, in the order of the page and of the knob.
const ROLES: [(&str, LabelRole); 6] = [
    ("Base", LabelRole::Base),
    ("Small", LabelRole::Small),
    ("Section", LabelRole::Section),
    ("Title", LabelRole::Title),
    ("Mono", LabelRole::Mono),
    ("Strong", LabelRole::Strong),
];

/// One line per role, and under them a sample line when the knobs give it text.
#[derive(Default)]
struct Roles {
    sample: String,
    /// An index into [`ROLES`].
    role: usize,
}

impl PageState for Roles {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Labels");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "LABEL ROLES", x, y);
        for (i, (text, role)) in ROLES.into_iter().enumerate() {
            ui.label(text, (x, y + 28.0 + i as f32 * 32.0), text, role);
        }
        if !self.sample.is_empty() {
            let role = ROLES[self.role.min(ROLES.len() - 1)].1;
            let at = (x, y + 28.0 + ROLES.len() as f32 * 32.0);
            ui.label("sample", at, &self.sample, role);
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.text(ui, "Sample text", &mut self.sample);
        let names: Vec<&str> = ROLES.iter().map(|(n, _)| *n).collect();
        panel.choice(ui, "Sample role", &names, &mut self.role);
    }
}

const SCENES: &[UiScene] = &[UiScene {
    id: "label-roles",
    scale: 2.0,
    tier: MotionTier::Full,
    state: || Box::new(Roles::default()),
    script: &[],
    capture_ms: 0,
    expect: Expect::Settled,
}];
