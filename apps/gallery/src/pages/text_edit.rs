//! The `text-edit` page: the editing engine behind the text field and the text area
//! (`text.text-edit`). Each scene scripts keys into a real field through the `Ui`, so what the
//! capture shows is what the engine did: the selection comes from key presses, not from a look.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::field::{FieldOptions, Variant};

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use lune_ui::input::{Modifiers, NamedKey};

pub(crate) static PAGE: Page = Page {
    id: "text-edit",
    title: "Text edit",
    kind: PageKind::Ui(UiPage {
        state: term,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// What the scene's field shows.
#[derive(Clone, Copy)]
enum Kind {
    /// A plain one-line field.
    Plain,
    /// The password variant, with a second, longer value beneath to show the mask stays 16 dots.
    Password,
}

/// The app's side of the page: the text the field edits.
struct Edit {
    text: String,
    kind: Kind,
    /// The second field of the password scene.
    other: String,
}

fn edit(text: &str, kind: Kind) -> Box<dyn PageState> {
    Box::new(Edit {
        text: text.to_owned(),
        kind,
        other: "a much longer sample phrase".to_owned(),
    })
}

fn term() -> Box<dyn PageState> {
    edit("lune term", Kind::Plain)
}

fn password() -> Box<dyn PageState> {
    edit("sample", Kind::Password)
}

impl PageState for Edit {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Text edit");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "TEXT EDIT", x, y);
        let rect = Rect::new(x, y + 20.0, 260.0, 32.0);
        match self.kind {
            Kind::Plain => {
                let opts = FieldOptions {
                    placeholder: "Type here",
                    ..FieldOptions::default()
                };
                ui.text_field_with("field", rect, &mut self.text, &opts);
            }
            Kind::Password => {
                let opts = FieldOptions {
                    placeholder: "Password",
                    variant: Variant::Password,
                    ..FieldOptions::default()
                };
                ui.text_field_with("field", rect, &mut self.text, &opts);
                caption(ui, "6 chars", rect.right() + 12.0, rect.y + 8.0);
                let below = Rect::new(x, rect.bottom() + 12.0, 260.0, 32.0);
                ui.text_field_with("other", below, &mut self.other, &opts);
                caption(ui, "27 chars", below.right() + 12.0, below.y + 8.0);
            }
        }
    }
}

const SHIFT: Modifiers = Modifiers {
    shift: true,
    control: false,
    alt: false,
    logo: false,
};

/// Tab focuses the field (the ring has settled long before the capture).
const FOCUS: Step = Step {
    at_ms: 0,
    events: &[Ev::Tab],
};

const SCENES: &[UiScene] = &[
    // Home, two Rights, then Shift-Right five times: chars 2 to 7 of "lune term" are "ne te".
    UiScene {
        id: "text-edit-selection",
        scale: 2,
        tier: MotionTier::Full,
        state: term,
        script: &[
            FOCUS,
            Step {
                at_ms: 50,
                events: &[
                    Ev::Key(NamedKey::Home),
                    Ev::Key(NamedKey::ArrowRight),
                    Ev::Key(NamedKey::ArrowRight),
                    Ev::KeyMod(NamedKey::ArrowRight, SHIFT),
                    Ev::KeyMod(NamedKey::ArrowRight, SHIFT),
                    Ev::KeyMod(NamedKey::ArrowRight, SHIFT),
                    Ev::KeyMod(NamedKey::ArrowRight, SHIFT),
                    Ev::KeyMod(NamedKey::ArrowRight, SHIFT),
                ],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // The mask is 16 dots whatever the text: "sample" above and a 27-char phrase below match.
    UiScene {
        id: "text-edit-password",
        scale: 2,
        tier: MotionTier::Full,
        state: password,
        script: &[FOCUS],
        capture_ms: 400,
        expect: Expect::Settled,
    },
];

/// What the bench replays: a click into the field, then a word typed.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(&["field"])],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Type("lune term")],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::TextValue;
    use lune_ui::testing::FakeText;
    use lune_ui::ui::WidgetId;

    fn node_text(id: &str) -> Option<TextValue> {
        let scene = SCENES.iter().find(|s| s.id == id)?;
        let (_, out) = run_scene(PAGE.id, scene, &mut FakeText).ok()?;
        let wid = WidgetId::ROOT.with(PAGE.id).with("field");
        out.tree.get(wid)?.text.clone()
    }

    #[test]
    fn the_selection_scene_selects_ne_te() {
        let v = node_text("text-edit-selection").expect("node text");
        assert_eq!(v.text, "lune term");
        assert_eq!(v.selection, (2, 7));
    }

    #[test]
    fn the_password_scene_gives_the_tree_neither_text_nor_value() {
        let scene = SCENES
            .iter()
            .find(|s| s.id == "text-edit-password")
            .expect("scene");
        let (_, out) = run_scene(PAGE.id, scene, &mut FakeText).expect("runs");
        let wid = WidgetId::ROOT.with(PAGE.id).with("field");
        let node = out.tree.get(wid).expect("node");
        assert!(node.text.is_none() && node.value.is_none());
    }
}
