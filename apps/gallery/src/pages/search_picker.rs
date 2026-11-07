//! The `search-picker` page: both triggers over one list. The scenes show each trigger alone, then
//! the link's popup with a search typed into it.

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::search_picker::{PickerOptions, Trigger};

use super::{caption, card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "search-picker",
    title: "Search picker",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

const ITEMS: [&str; 12] = [
    "Alder", "Ash", "Aspen", "Beech", "Birch", "Cedar", "Elm", "Hazel", "Larch", "Maple", "Oak",
    "Willow",
];

fn state(field: bool, link: bool) -> Box<dyn PageState> {
    Box::new(Live {
        field: field.then_some(Some(4)),
        link: link.then_some(Some(8)),
        disabled: false,
        placeholder: "Find a tree".to_owned(),
        glyph: true,
    })
}

/// Both triggers, disabled.
fn disabled() -> Box<dyn PageState> {
    Box::new(Live {
        field: Some(Some(4)),
        link: Some(Some(8)),
        disabled: true,
        placeholder: "Find a tree".to_owned(),
        glyph: true,
    })
}

fn live() -> Box<dyn PageState> {
    state(true, true)
}

fn field_only() -> Box<dyn PageState> {
    state(true, false)
}

fn link_only() -> Box<dyn PageState> {
    state(false, true)
}

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
/// The label column's width; the triggers start after it.
const LABEL_W: f32 = 140.0;
const FIELD: Rect = Rect::new(X + LABEL_W, Y + 24.0, 320.0, 36.0);
const LINK: Rect = Rect::new(X + LABEL_W, Y + 76.0, 160.0, 24.0);

/// Each trigger's selection, for the triggers the page shows. The knobs set whether they are
/// disabled, the search placeholder and the link's glyph.
struct Live {
    field: Option<Option<usize>>,
    link: Option<Option<usize>>,
    disabled: bool,
    placeholder: String,
    glyph: bool,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Search picker");
        let (ox, oy) = (card.x + 24.0, card.y + 24.0);
        let (field, link) = (on_card(card, FIELD), on_card(card, LINK));
        heading(ui, "SEARCH PICKER", ox, oy);
        if let Some(selected) = &mut self.field {
            caption(ui, "Field", ox, field.y + 11.0);
            let opts = PickerOptions {
                trigger: Trigger::Field,
                icon: None,
                placeholder: &self.placeholder,
                name: "Tree",
            };
            ui.with_enabled(!self.disabled, |ui| {
                ui.search_picker("field", field, &ITEMS, selected, &opts)
            });
        }
        if let Some(selected) = &mut self.link {
            caption(ui, "Link", ox, link.y + 5.0);
            let opts = PickerOptions {
                trigger: Trigger::Link,
                icon: self.glyph.then_some(icon::GEAR),
                placeholder: &self.placeholder,
                name: "Other tree",
            };
            ui.with_enabled(!self.disabled, |ui| {
                ui.search_picker("link", link, &ITEMS, selected, &opts)
            });
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "Disabled", &mut self.disabled);
        panel.text(ui, "Placeholder", &mut self.placeholder);
        panel.toggle(ui, "Link glyph", &mut self.glyph);
    }
}

const fn scene(
    id: &'static str,
    state: fn() -> Box<dyn PageState>,
    script: &'static [Step],
    capture_ms: u32,
) -> UiScene {
    UiScene {
        id,
        scale: 2.0,
        tier: MotionTier::Full,
        state,
        script,
        capture_ms,
        expect: Expect::Settled,
    }
}

const SCENES: &[UiScene] = &[
    scene("search-picker-field", field_only, &[], 0),
    scene("search-picker-link", link_only, &[], 0),
    // Tab reaches the trigger and its ring settles: the field's edge ring, then the link's button
    // ring (D124).
    scene(
        "search-picker-focus-field",
        field_only,
        &[Step {
            at_ms: 0,
            events: &[Ev::Tab],
        }],
        400,
    ),
    scene(
        "search-picker-focus-link",
        link_only,
        &[Step {
            at_ms: 0,
            events: &[Ev::Tab],
        }],
        400,
    ),
    // Both disabled (0.4 opacity) with the pointer on the link: no hover, no popup on a click.
    scene(
        "search-picker-disabled",
        disabled,
        &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(&["link"])],
            },
            Step {
                at_ms: 50,
                events: &[Ev::Click],
            },
        ],
        400,
    ),
    // A click opens the link's popup (`fast`, 180 ms) and typing narrows the list; the pointer
    // stays on the trigger, under no row.
    scene(
        "search-picker-open",
        link_only,
        &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(&["link"])],
            },
            Step {
                at_ms: 50,
                events: &[Ev::Click],
            },
            Step {
                at_ms: 300,
                events: &[Ev::Type("a")],
            },
        ],
        800,
    ),
];

/// What the bench replays: the link's popup opens, a search narrows it, Enter picks and closes,
/// then the field's opens and Escape closes it.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(&["link"])],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Type("ce")],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Key(NamedKey::Enter)],
    },
    Step {
        at_ms: 900,
        events: &[Ev::PointerOver(&["field"])],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1300,
        events: &[Ev::Key(NamedKey::Escape)],
    },
];
