//! The `filter-bar` page: four live bars: empty with its watermark, with text and the clear button,
//! one the scene focuses, and a disabled one.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::filter_bar::FilterOptions;

use super::{caption, card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "filter-bar",
    title: "Filter bar",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        empty: String::new(),
        text: "main.rs".to_owned(),
        focused: "readme".to_owned(),
        placeholder: "Filter files".to_owned(),
        label: String::new(),
        clear_unfocusable: false,
        enabled: true,
    })
}

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
/// The label column's width; the bars start after it.
const LABEL_W: f32 = 140.0;
const BAR_W: f32 = 320.0;
const BAR_H: f32 = 30.0;
const PITCH: f32 = 44.0;

/// Row `i`'s bar.
const fn row(i: usize) -> Rect {
    Rect::new(X + LABEL_W, Y + 24.0 + PITCH * i as f32, BAR_W, BAR_H)
}

/// Four bars. The knobs set the placeholder, the accessible label, whether the clear button is
/// out of the tab order and whether the first three take input.
struct Live {
    empty: String,
    text: String,
    focused: String,
    placeholder: String,
    label: String,
    clear_unfocusable: bool,
    enabled: bool,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Filter bar");
        let (ox, oy) = (card.x + 24.0, card.y + 24.0);
        let row = |i| on_card(card, row(i));
        heading(ui, "FILTER BAR", ox, oy);
        for (i, name) in ["Empty", "With text", "Focused", "Disabled"]
            .into_iter()
            .enumerate()
        {
            caption(ui, name, ox, row(i).y + 8.0);
        }
        let opts = FilterOptions {
            placeholder: &self.placeholder,
            label: (!self.label.is_empty()).then_some(self.label.as_str()),
            clear_unfocusable: self.clear_unfocusable,
        };
        let (empty, text, focused) = (&mut self.empty, &mut self.text, &mut self.focused);
        ui.with_enabled(self.enabled, |ui| {
            ui.filter_bar("empty", row(0), empty, &opts);
            ui.filter_bar("text", row(1), text, &opts);
            ui.filter_bar("focused", row(2), focused, &opts);
        });
        let mut held = "locked".to_owned();
        ui.with_enabled(false, |ui| {
            ui.filter_bar("disabled", row(3), &mut held, &opts);
        });
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.text(ui, "Placeholder", &mut self.placeholder);
        panel.text(ui, "Accessible label", &mut self.label);
        panel.toggle(ui, "Clear out of tab order", &mut self.clear_unfocusable);
        panel.toggle(ui, "Enabled", &mut self.enabled);
    }
}

const SCENES: &[UiScene] = &[
    // A click puts the caret in the third bar; the settled look is all the page has.
    UiScene {
        id: "filter-bar-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(&["focused"])],
            },
            Step {
                at_ms: 50,
                events: &[Ev::Click],
            },
        ],
        capture_ms: 500,
        expect: Expect::Settled,
    },
];

/// What the bench replays: a click into the first bar, typing, then the keyboard onto its clear
/// button and through it.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(&["empty"])],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 300,
        events: &[Ev::Type("lune")],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Key(lune_ui::input::NamedKey::Enter)],
    },
];
