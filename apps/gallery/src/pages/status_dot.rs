//! The `status-dot` page: the five tones, each beside its word (a dot never stands alone).

use lune_ui::motion::MotionTier;
use lune_ui::ui::Ui;
use lune_ui::widgets::tone::Tone;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};
use crate::showcase::knobs::{Knobs, TONES, tone_names};

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
    Box::new(Tones::default())
}

const CASES: [(&str, Tone); 5] = [
    ("Healthy", Tone::Ok),
    ("Degraded", Tone::Warn),
    ("Failed", Tone::Err),
    ("Syncing", Tone::Data),
    ("Paused", Tone::Paused),
];

/// The five dots, and a sample when the knobs ask for it.
struct Tones {
    show: bool,
    text: String,
    /// An index into [`TONES`].
    tone: usize,
}

impl Default for Tones {
    fn default() -> Self {
        Self {
            show: false,
            text: "Sample".to_owned(),
            tone: 0,
        }
    }
}

impl PageState for Tones {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Status dot");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "STATUS DOTS", x, y);
        for (i, (word, tone)) in CASES.into_iter().enumerate() {
            ui.status_dot(i, (x, y + 28.0 + i as f32 * 28.0), tone, word);
        }
        if self.show {
            let tone = TONES[self.tone.min(TONES.len() - 1)].1;
            let at = (x, y + 28.0 + CASES.len() as f32 * 28.0 + 12.0);
            ui.status_dot("sample", at, tone, &self.text);
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "Show a sample", &mut self.show);
        panel.text(ui, "Text", &mut self.text);
        panel.choice(ui, "Tone", &tone_names(), &mut self.tone);
    }
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "status-dot-tones",
        scale: 2.0,
        tier: MotionTier::Full,
        state: tones,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "status-dot-tones",
        scale: 1.0,
        tier: MotionTier::Full,
        state: tones,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];
