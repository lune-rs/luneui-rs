//! The `status-dot` page: the five tones, each beside its word (a dot never stands alone).

use lune_ui::motion::MotionTier;
use lune_ui::ui::Ui;
use lune_ui::widgets::tone::Tone;

use super::{card, heading};
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "status-dot",
    title: "Status dot",
    kind: PageKind::Ui(UiPage {
        state: tones,
        scenes: SCENES,
        bench: &[],
    }),
};

fn tones() -> Box<dyn PageState> {
    Box::new(Tones)
}

const CASES: [(&str, Tone); 5] = [
    ("Healthy", Tone::Ok),
    ("Degraded", Tone::Warn),
    ("Failed", Tone::Err),
    ("Syncing", Tone::Data),
    ("Paused", Tone::Paused),
];

struct Tones;

impl PageState for Tones {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Status dot");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "STATUS DOTS", x, y);
        for (i, (word, tone)) in CASES.into_iter().enumerate() {
            ui.status_dot(i, (x, y + 28.0 + i as f32 * 28.0), tone, word);
        }
    }
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "status-dot-tones",
        scale: 2,
        tier: MotionTier::Full,
        state: tones,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "status-dot-tones",
        scale: 1,
        tier: MotionTier::Full,
        state: tones,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];
