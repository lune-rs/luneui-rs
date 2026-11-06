//! The `ime` page: input-method composition in a text field (`text.ime`). Scenes script the
//! platform's preedit and commit events into a real field, so the underline, the emphasised clause
//! and the shifted text after the caret come from the widget, not from a look.
//!
//! The preedit is the catalogue's Japanese "かな". The gallery bundles no CJK font, so it draws as
//! boxes (D90); the underline, the clause and the shift are still those of the real glyph run.

use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::field::FieldOptions;
use lune_ui::widgets::text_area::AreaOptions;

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

/// What the scene's widget is.
#[derive(Clone, Copy)]
enum Kind {
    Field,
    /// A multi-line area.
    Area,
    /// A field with the `ime_area` it reports outlined over it.
    Outlined,
}

/// The app's side of the page: the text the widget edits.
struct Ime {
    text: String,
    kind: Kind,
}

fn ime(text: &str, kind: Kind) -> Box<dyn PageState> {
    Box::new(Ime {
        text: text.to_owned(),
        kind,
    })
}

fn lune() -> Box<dyn PageState> {
    ime("lune", Kind::Field)
}

fn lines() -> Box<dyn PageState> {
    ime("one\ntwo\nthree", Kind::Area)
}

fn outlined() -> Box<dyn PageState> {
    ime("lune", Kind::Outlined)
}

impl PageState for Ime {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Input method");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "INPUT METHOD", x, y);
        let field = FieldOptions {
            placeholder: "Type here",
            ..FieldOptions::default()
        };
        let rect = Rect::new(x, y + 20.0, 260.0, 32.0);
        match self.kind {
            Kind::Field => {
                ui.text_field_with("field", rect, &mut self.text, &field);
            }
            Kind::Area => {
                let opts = AreaOptions {
                    placeholder: "Type here",
                    ..AreaOptions::default()
                };
                let area = Rect::new(x, y + 20.0, 300.0, 110.0);
                ui.text_area("field", area, &mut self.text, &opts);
            }
            Kind::Outlined => {
                let r = ui.text_field_with("field", rect, &mut self.text, &field);
                // What the window hands the platform as the candidate window's anchor.
                if let Some(area) = r.ime_area {
                    let ok = ui.theme().ok;
                    ui.painter().ring(area, 0.0, 1.0, ok, true);
                }
            }
        }
    }
}

/// Tab focuses the field (the ring has settled long before the capture).
const FOCUS: Step = Step {
    at_ms: 0,
    events: &[Ev::Tab],
};

const KANA: &str = "かな";

const SCENES: &[UiScene] = &[
    // Cursor (0, 3) is in bytes: the first character of "かな" (3 bytes) is the selected clause.
    // The gallery has no CJK font, so the preedit draws as boxes: the user's decision (D90).
    UiScene {
        id: "ime-preedit",
        scale: 2.0,
        tier: MotionTier::Full,
        state: lune,
        script: &[
            FOCUS,
            Step {
                at_ms: 50,
                events: &[Ev::ImePreedit(KANA, Some((0, 3)))],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // No cursor: no caret and no emphasised clause, only the underline.
    UiScene {
        id: "ime-preedit-no-cursor",
        scale: 2.0,
        tier: MotionTier::Full,
        state: lune,
        script: &[
            FOCUS,
            Step {
                at_ms: 50,
                events: &[Ev::ImePreedit(KANA, None)],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Home and two Rights put the caret between "lu" and "ne"; "ne" shifts right by the preedit.
    UiScene {
        id: "ime-preedit-middle",
        scale: 2.0,
        tier: MotionTier::Full,
        state: lune,
        script: &[
            FOCUS,
            Step {
                at_ms: 50,
                events: &[
                    Ev::Key(NamedKey::Home),
                    Ev::Key(NamedKey::ArrowRight),
                    Ev::Key(NamedKey::ArrowRight),
                ],
            },
            Step {
                at_ms: 100,
                events: &[Ev::ImePreedit(KANA, Some((0, 3)))],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // The commit turns the preedit into text and the caret goes after it.
    UiScene {
        id: "ime-committed",
        scale: 2.0,
        tier: MotionTier::Full,
        state: lune,
        script: &[
            FOCUS,
            Step {
                at_ms: 50,
                events: &[Ev::ImePreedit(KANA, Some((0, 3)))],
            },
            Step {
                at_ms: 100,
                events: &[Ev::ImeCommit(KANA)],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Up from the end of line 3 puts the caret on line 2, where the preedit goes.
    UiScene {
        id: "ime-multiline",
        scale: 2.0,
        tier: MotionTier::Full,
        state: lines,
        script: &[
            FOCUS,
            Step {
                at_ms: 50,
                events: &[Ev::Key(NamedKey::ArrowUp)],
            },
            Step {
                at_ms: 100,
                events: &[Ev::ImePreedit(KANA, Some((0, 3)))],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // The reported `ime_area` (here the caret's rect) outlined over a focused field.
    UiScene {
        id: "ime-area",
        scale: 2.0,
        tier: MotionTier::Full,
        state: outlined,
        script: &[FOCUS],
        capture_ms: 400,
        expect: Expect::Settled,
    },
];

/// What the bench replays: focus, a preedit, its commit.
const BENCH: &[Step] = &[
    FOCUS,
    Step {
        at_ms: 100,
        events: &[Ev::ImePreedit(KANA, Some((0, 3)))],
    },
    Step {
        at_ms: 200,
        events: &[Ev::ImeCommit(KANA)],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::TextValue;
    use lune_ui::input::FrameOutput;
    use lune_ui::testing::FakeText;
    use lune_ui::ui::WidgetId;

    fn run(id: &str) -> FrameOutput {
        let scene = SCENES.iter().find(|s| s.id == id).expect("scene");
        run_scene(PAGE.id, scene, &mut FakeText).expect("runs").1
    }

    fn value(out: &FrameOutput) -> TextValue {
        let node = out
            .tree
            .get(WidgetId::ROOT.with(PAGE.id).with("field"))
            .expect("node");
        node.text.clone().expect("text")
    }

    #[test]
    fn a_preedit_stays_out_of_the_value_and_the_window_gets_an_area() {
        for id in [
            "ime-preedit",
            "ime-preedit-no-cursor",
            "ime-preedit-middle",
            "ime-multiline",
        ] {
            let out = run(id);
            assert!(!value(&out).text.contains('か'), "{id}");
            assert!(
                out.ime_area.is_some(),
                "{id}: a composing field reports its area"
            );
        }
    }

    #[test]
    fn the_middle_scene_composes_at_char_2() {
        let v = value(&run("ime-preedit-middle"));
        assert_eq!((v.text.as_str(), v.selection), ("lune", (2, 2)));
    }

    #[test]
    fn the_commit_scene_puts_the_text_in_the_value_with_the_caret_after_it() {
        let v = value(&run("ime-committed"));
        assert_eq!((v.text.as_str(), v.selection), ("luneかな", (6, 6)));
    }

    #[test]
    fn the_multiline_scene_composes_on_line_2() {
        let v = value(&run("ime-multiline"));
        assert!((4..=7).contains(&v.selection.0), "{:?}", v.selection);
    }

    #[test]
    fn the_area_scene_reports_an_area_inside_the_field() {
        let out = run("ime-area");
        let area = out.ime_area.expect("focused");
        assert!(area.x > 34.0 && area.y >= 78.0 && area.bottom() <= 78.0 + 32.0);
    }
}
