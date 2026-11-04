//! The `text-field` page: a live field (window mode, the bench and the mid-transition scenes) and a
//! specimen sheet of every state painted through the public look (D16). M4 has the focus, node and
//! transitions only; variants, search clear and composing come with M5.

use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::ui::Ui;
use lune_ui::widgets::field::{FieldLook, FieldState, paint};

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "text-field",
    title: "Text field",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        text: String::new(),
    })
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

/// One real field through the `Ui`, empty at first.
struct Live {
    text: String,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Text field");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "TEXT FIELD", x, y);
        ui.text_field(
            "field",
            Rect::new(x, y + 20.0, 210.0, 32.0),
            &mut self.text,
            "Search",
            FieldState::default(),
        );
    }
}

/// Every state, each showing "lune", drawn from a [`FieldLook`] through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Text field states");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "TEXT FIELD STATES", x, y);
        let theme = *ui.theme();
        // (name, hovered, focused, error, disabled)
        let cases = [
            ("Rest", false, false, false, false),
            ("Hover", true, false, false, false),
            ("Focus-visible", false, true, false, false),
            ("Disabled", false, false, false, true),
            ("Error", false, false, true, false),
        ];
        for (i, (name, hovered, focused, error, disabled)) in cases.into_iter().enumerate() {
            let r = Rect::new(x, y + 24.0 + i as f32 * 44.0, 210.0, 32.0);
            let look = FieldLook::of(&theme, hovered, focused, FieldState { error });
            let p = ui.painter();
            if disabled {
                p.push_xform(Xform {
                    origin: (0.0, 0.0),
                    scale: 1.0,
                    dy: 0.0,
                    alpha: 0.4,
                });
            }
            paint(p, r, "lune", "Search", &look);
            if disabled {
                p.pop_xform();
            }
            caption(ui, name, r.right() + 12.0, r.y + 8.0);
        }
    }
}

/// The live field, by its path.
const FIELD: &[&str] = &["field"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "text-field-states",
        scale: 2,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "text-field-empty",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "text-field-hover-fade",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(FIELD)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // Tab focuses the field; the ring is `fast` (180 ms): half way at 90 ms.
    UiScene {
        id: "text-field-focus-fade",
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

/// What the bench replays: hover, click, Tab.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(FIELD)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Tab],
    },
];
