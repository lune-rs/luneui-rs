//! The `ime` page: input-method composition in a text field (`text.ime`). Scenes script the
//! platform's preedit and commit events into a real field, so the underline, the emphasised clause
//! and the shifted text after the caret come from the widget, not from a look.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::field::FieldOptions;

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "ime",
    title: "Input method",
    kind: PageKind::Ui(UiPage {
        state: lune,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The app's side of the page: the text the field edits.
struct Ime {
    text: String,
}

fn lune() -> Box<dyn PageState> {
    Box::new(Ime {
        text: "lune".to_owned(),
    })
}

impl PageState for Ime {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Input method");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "INPUT METHOD", x, y);
        let opts = FieldOptions {
            placeholder: "Type here",
            ..FieldOptions::default()
        };
        let rect = Rect::new(x, y + 20.0, 260.0, 32.0);
        ui.text_field_with("field", rect, &mut self.text, &opts);
    }
}

/// Tab focuses the field (the ring has settled long before the capture).
const FOCUS: Step = Step {
    at_ms: 0,
    events: &[Ev::Tab],
};

const SCENES: &[UiScene] = &[
    // Cursor (0, 3) is in bytes: the first character of "かな" (3 bytes) is the selected clause.
    UiScene {
        id: "ime-preedit",
        scale: 2,
        tier: MotionTier::Full,
        state: lune,
        script: &[
            FOCUS,
            Step {
                at_ms: 50,
                events: &[Ev::ImePreedit("かな", Some((0, 3)))],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
];

/// What the bench replays: focus, a preedit, its commit.
const BENCH: &[Step] = &[
    FOCUS,
    Step {
        at_ms: 100,
        events: &[Ev::ImePreedit("かな", Some((0, 3)))],
    },
    Step {
        at_ms: 200,
        events: &[Ev::ImeCommit("かな")],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::testing::FakeText;
    use lune_ui::ui::WidgetId;

    #[test]
    fn the_preedit_scene_keeps_the_preedit_out_of_the_value() {
        let scene = SCENES
            .iter()
            .find(|s| s.id == "ime-preedit")
            .expect("scene");
        let (_, out) = run_scene(PAGE.id, scene, &mut FakeText).expect("runs");
        let node = out
            .tree
            .get(WidgetId::ROOT.with(PAGE.id).with("field"))
            .expect("node");
        let text = node.text.as_ref().expect("text");
        assert_eq!(text.text, "lune");
        assert!(out.ime_area.is_some(), "a composing field reports its area");
    }
}
