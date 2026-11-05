//! The `callout` page: the four tones, each with a figure and an action button.

use lune_ui::icon;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::callout::CalloutOptions;
use lune_ui::widgets::tone::Tone;

use super::{card, heading};
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "callout",
    title: "Callout",
    kind: PageKind::Ui(UiPage {
        state: tones,
        scenes: SCENES,
        bench: &[],
    }),
};

const WIDTH: f32 = 640.0;

/// A tone, its text, its figure and its action.
const CASES: [(Tone, &str, &str, &str); 4] = [
    (
        Tone::Neutral,
        "Today's spend is approaching the daily budget.",
        "$ 8.20",
        "Open budget",
    ),
    (
        Tone::Data,
        "A larger local model fits in memory and would answer these requests.",
        "7B",
        "Compare",
    ),
    (
        Tone::Warn,
        "The upstream is answering slowly; requests may queue.",
        "4.2 s",
        "Details",
    ),
    (
        Tone::Err,
        "The proxy is offline. No requests are being compressed.",
        "0 / 12",
        "Retry",
    ),
];

struct Tones;

impl PageState for Tones {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Callout");
        let (x, mut y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "NEUTRAL, DATA, WARN, ERR", x, y);
        y += 24.0;
        for (i, (tone, text, figure, action)) in CASES.into_iter().enumerate() {
            let opts = CalloutOptions {
                tone,
                icon: icon::INFO,
                text,
                figure: Some(figure),
                action: Some(action),
                announce: false,
            };
            y += ui
                .callout(("tone", i), Rect::new(x, y, WIDTH, 0.0), &opts)
                .height
                + 12.0;
        }
    }
}

fn tones() -> Box<dyn PageState> {
    Box::new(Tones)
}

const SCENES: &[UiScene] = &[UiScene {
    id: "callout-tones",
    scale: 2,
    tier: MotionTier::Full,
    state: tones,
    script: &[],
    capture_ms: 0,
    expect: Expect::Settled,
}];
