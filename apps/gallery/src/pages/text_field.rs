//! The `text-field` page: a live field (window mode, the bench and the mid-transition scenes) and a
//! specimen sheet of every state painted through the public look (D16), the variants side by side,
//! the search field with its clear button, and a field composing with an input method.

use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::ui::Ui;
use lune_ui::widgets::field::{FieldLook, FieldOptions, FieldState, Variant, paint};

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

/// The live field holding "lune": the composing scene's start.
fn lune() -> Box<dyn PageState> {
    Box::new(Live {
        text: "lune".to_owned(),
    })
}

fn search() -> Box<dyn PageState> {
    Box::new(Search {
        text: "lune".to_owned(),
    })
}

/// The search variant with text in it, so the clear button shows.
struct Search {
    text: String,
}

impl PageState for Search {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Search field");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "SEARCH WITH CLEAR", x, y);
        let opts = FieldOptions {
            variant: Variant::Search,
            placeholder: "Search",
            ..FieldOptions::default()
        };
        ui.text_field_with(
            "field",
            Rect::new(x, y + 20.0, 260.0, 32.0),
            &mut self.text,
            &opts,
        );
    }
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
                    dx: 0.0,
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

fn variants() -> Box<dyn PageState> {
    Box::new(Variants {
        texts: ["lune", "lune", "lune", "sample", "lune", "2026-11-04"].map(str::to_owned),
    })
}

/// The six variants side by side through the real widget, each with its own text.
struct Variants {
    texts: [String; 6],
}

impl PageState for Variants {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Text field variants");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "TEXT FIELD VARIANTS", x, y);
        let plain = FieldOptions::default();
        let cases = [
            ("Fill", plain),
            (
                "Small",
                FieldOptions {
                    small: true,
                    ..plain
                },
            ),
            (
                "Mono",
                FieldOptions {
                    variant: Variant::Mono,
                    ..plain
                },
            ),
            (
                "Password",
                FieldOptions {
                    variant: Variant::Password,
                    ..plain
                },
            ),
            (
                "Search",
                FieldOptions {
                    variant: Variant::Search,
                    placeholder: "Search",
                    ..plain
                },
            ),
            (
                "Date",
                FieldOptions {
                    variant: Variant::Date,
                    ..plain
                },
            ),
        ];
        for (i, ((name, opts), text)) in cases.into_iter().zip(&mut self.texts).enumerate() {
            let (col, row) = ((i % 3) as f32, (i / 3) as f32);
            let (fx, fy) = (x + col * 300.0, y + 28.0 + row * 80.0);
            caption(ui, name, fx, fy);
            ui.text_field_with(name, Rect::new(fx, fy + 20.0, 260.0, 32.0), text, &opts);
        }
    }
}

/// The live field, by its path.
const FIELD: &[&str] = &["field"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "text-field-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "text-field-empty",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Settled: nothing is focused or hovered, so the page reports `Idle`.
    UiScene {
        id: "text-field-variants",
        scale: 2.0,
        tier: MotionTier::Full,
        state: variants,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Not focused, so the clear button is the only extra: a press on it empties the field.
    UiScene {
        id: "text-field-search-clear",
        scale: 2.0,
        tier: MotionTier::Full,
        state: search,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The same preedit as `ime-preedit` (cursor (0, 3) in bytes), here as a field's own state. The
    // gallery has no CJK font, so "かな" draws as boxes (D90).
    UiScene {
        id: "text-field-composing",
        scale: 2.0,
        tier: MotionTier::Full,
        state: lune,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::Tab],
            },
            Step {
                at_ms: 50,
                events: &[Ev::ImePreedit("かな", Some((0, 3)))],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "text-field-hover-fade",
        scale: 2.0,
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
        scale: 2.0,
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

/// What the bench replays: hover, click, a word typed, Tab.
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
        at_ms: 300,
        events: &[Ev::Type("lune")],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Tab],
    },
];
