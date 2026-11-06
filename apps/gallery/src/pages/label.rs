//! The `label` page: every text role of `label.md` drawn through the `Ui`, settled.

use lune_ui::motion::MotionTier;
use lune_ui::ui::Ui;
use lune_ui::widgets::label::LabelRole;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "label",
    title: "Label",
    kind: PageKind::Ui(UiPage {
        state: || Box::new(Roles),
        scenes: SCENES,
        bench: &[],
    }),
};

/// One line per role.
struct Roles;

impl PageState for Roles {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Labels");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "LABEL ROLES", x, y);
        let roles = [
            ("Base", LabelRole::Base),
            ("Small", LabelRole::Small),
            ("Section", LabelRole::Section),
            ("Title", LabelRole::Title),
            ("Mono", LabelRole::Mono),
            ("Strong", LabelRole::Strong),
        ];
        for (i, (text, role)) in roles.into_iter().enumerate() {
            ui.label(text, (x, y + 28.0 + i as f32 * 32.0), text, role);
        }
    }
}

const SCENES: &[UiScene] = &[UiScene {
    id: "label-roles",
    scale: 2.0,
    tier: MotionTier::Full,
    state: || Box::new(Roles),
    script: &[],
    capture_ms: 0,
    expect: Expect::Settled,
}];
