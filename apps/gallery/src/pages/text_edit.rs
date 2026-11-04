//! The `text-edit` page: the editing engine behind the text field and the text area
//! (`text.text-edit`). Each scene scripts keys into a real field through the `Ui`, so what the
//! capture shows is what the engine did: the selection comes from key presses, not from a look.

use lune_ui::input::{Modifiers, NamedKey};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::field::{FieldOptions, Variant};
use lune_ui::widgets::text_area::AreaOptions;

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "text-edit",
    title: "Text edit",
    kind: PageKind::Ui(UiPage {
        state: term,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// What the scene's widget is.
#[derive(Clone, Copy)]
enum Kind {
    /// A plain one-line field.
    Plain,
    /// A one-line field in the error colour.
    Error,
    /// A disabled one-line field.
    Disabled,
    /// The password variant, with a second, longer value beneath to show the mask stays 16 dots.
    Password,
    /// A multi-line text area.
    Area,
    /// A read-only multi-line text area.
    ReadOnlyArea,
}

/// The app's side of the page: the text the widget edits.
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

fn lune() -> Box<dyn PageState> {
    edit("lune", Kind::Plain)
}

fn wide() -> Box<dyn PageState> {
    edit(
        "a line of text that is wider than the box it sits in",
        Kind::Plain,
    )
}

fn lines() -> Box<dyn PageState> {
    edit("one\ntwo\nthree", Kind::Area)
}

fn read_only() -> Box<dyn PageState> {
    edit(
        "A read-only note.\nIts text can be selected and copied,\nbut not edited.",
        Kind::ReadOnlyArea,
    )
}

fn disabled() -> Box<dyn PageState> {
    edit("lune", Kind::Disabled)
}

fn error() -> Box<dyn PageState> {
    edit("lune", Kind::Error)
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
        let field = FieldOptions {
            placeholder: "Type here",
            ..FieldOptions::default()
        };
        match self.kind {
            Kind::Plain => {
                ui.text_field_with("field", rect, &mut self.text, &field);
            }
            Kind::Error => {
                let opts = FieldOptions {
                    error: true,
                    ..field
                };
                ui.text_field_with("field", rect, &mut self.text, &opts);
            }
            Kind::Disabled => {
                ui.with_enabled(false, |ui| {
                    ui.text_field_with("field", rect, &mut self.text, &field);
                });
            }
            Kind::Password => {
                let opts = FieldOptions {
                    placeholder: "Password",
                    variant: Variant::Password,
                    ..field
                };
                ui.text_field_with("field", rect, &mut self.text, &opts);
                caption(ui, "6 chars", rect.right() + 12.0, rect.y + 8.0);
                let below = Rect::new(x, rect.bottom() + 12.0, 260.0, 32.0);
                ui.text_field_with("other", below, &mut self.other, &opts);
                caption(ui, "27 chars", below.right() + 12.0, below.y + 8.0);
            }
            Kind::Area | Kind::ReadOnlyArea => {
                let opts = AreaOptions {
                    placeholder: "Type here",
                    read_only: matches!(self.kind, Kind::ReadOnlyArea),
                    ..AreaOptions::default()
                };
                let area = Rect::new(x, y + 20.0, 300.0, 110.0);
                ui.text_area("field", area, &mut self.text, &opts);
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

/// The scenes run on the Mac keymap, where the primary modifier is Command.
const COMMAND: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: true,
};

/// Tab focuses the field (the ring has settled long before the capture).
const FOCUS: Step = Step {
    at_ms: 0,
    events: &[Ev::Tab],
};

const SCENES: &[UiScene] = &[
    // Focused, the caret after the text ("lune" is 4 chars, the caret at 4).
    UiScene {
        id: "text-edit-caret",
        scale: 2,
        tier: MotionTier::Full,
        state: lune,
        script: &[FOCUS],
        capture_ms: 400,
        expect: Expect::Settled,
    },
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
    // Wider than the box: the caret starts at the end, so the text scrolls to keep it in view.
    UiScene {
        id: "text-edit-scrolled",
        scale: 2,
        tier: MotionTier::Full,
        state: wide,
        script: &[FOCUS],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // The caret starts at the end of line 3; Up moves it to the end of line 2.
    UiScene {
        id: "text-edit-multiline",
        scale: 2,
        tier: MotionTier::Full,
        state: lines,
        script: &[
            FOCUS,
            Step {
                at_ms: 50,
                events: &[Ev::Key(NamedKey::ArrowUp)],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Command-A selects everything; a read-only area keeps the selection and refuses edits.
    UiScene {
        id: "text-edit-readonly",
        scale: 2,
        tier: MotionTier::Full,
        state: read_only,
        script: &[
            FOCUS,
            Step {
                at_ms: 50,
                events: &[Ev::KeyChar("a", COMMAND)],
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
    UiScene {
        id: "text-edit-disabled",
        scale: 2,
        tier: MotionTier::Full,
        state: disabled,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "text-edit-error",
        scale: 2,
        tier: MotionTier::Full,
        state: error,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Tab focuses the field; the ring is `fast` (180 ms): half way at 90 ms.
    UiScene {
        id: "text-edit-focus-ring",
        scale: 2,
        tier: MotionTier::Full,
        state: lune,
        script: &[FOCUS],
        capture_ms: 90,
        expect: Expect::Moving,
    },
    // The pointer enters the box; hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "text-edit-hover",
        scale: 2,
        tier: MotionTier::Full,
        state: lune,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(&["field"])],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
];

/// What the bench replays: a word typed, then a selection dragged across it.
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
    Step {
        at_ms: 300,
        events: &[Ev::PointerAt(44.0, 94.0), Ev::Press],
    },
    Step {
        at_ms: 340,
        events: &[Ev::PointerAt(90.0, 94.0)],
    },
    Step {
        at_ms: 380,
        events: &[Ev::PointerAt(130.0, 94.0)],
    },
    Step {
        at_ms: 420,
        events: &[Ev::Release],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::{Node, Role, State, TextValue};
    use lune_ui::testing::FakeText;
    use lune_ui::ui::WidgetId;

    fn node(id: &str) -> Node {
        let scene = SCENES.iter().find(|s| s.id == id).expect("scene");
        let (_, out) = run_scene(PAGE.id, scene, &mut FakeText).expect("runs");
        let wid = WidgetId::ROOT.with(PAGE.id).with("field");
        out.tree.get(wid).expect("node").clone()
    }

    fn value(id: &str) -> TextValue {
        node(id).text.expect("node text")
    }

    #[test]
    fn the_selection_scene_selects_ne_te() {
        let v = value("text-edit-selection");
        assert_eq!(v.text, "lune term");
        assert_eq!(v.selection, (2, 7));
    }

    #[test]
    fn the_caret_scene_has_the_caret_after_the_text() {
        let v = value("text-edit-caret");
        assert_eq!((v.text.as_str(), v.selection), ("lune", (4, 4)));
    }

    #[test]
    fn the_scrolled_scene_has_the_caret_at_the_end() {
        let v = value("text-edit-scrolled");
        let n = v.text.chars().count();
        assert_eq!(v.selection, (n, n));
    }

    #[test]
    fn the_multiline_scene_has_the_caret_on_line_2() {
        let v = value("text-edit-multiline");
        // "one\ntwo\nthree": line 2 spans chars 4 to 7.
        assert!((4..=7).contains(&v.selection.0), "{:?}", v.selection);
        assert_eq!(v.selection.0, v.selection.1);
        assert_eq!(node("text-edit-multiline").role, Role::MultilineText);
    }

    #[test]
    fn the_readonly_scene_selects_all_and_is_read_only() {
        let n = node("text-edit-readonly");
        let v = n.text.expect("text");
        assert_eq!(v.selection, (0, v.text.chars().count()));
        assert!(n.states.contains(State::ReadOnly));
    }

    #[test]
    fn the_password_scene_gives_the_tree_neither_text_nor_value() {
        let n = node("text-edit-password");
        assert!(n.text.is_none() && n.value.is_none());
    }

    #[test]
    fn the_disabled_and_error_scenes_carry_their_states() {
        assert!(node("text-edit-disabled").states.contains(State::Disabled));
        assert!(node("text-edit-error").states.contains(State::Invalid));
    }
}
