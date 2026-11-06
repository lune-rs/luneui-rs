//! The `badge` page: the five tones, plain and with a leading dot (D117 tone recipe).

use lune_ui::motion::MotionTier;
use lune_ui::ui::Ui;
use lune_ui::widgets::tone::Tone;

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "badge",
    title: "Badge",
    kind: PageKind::Ui(UiPage {
        state: tones,
        scenes: SCENES,
        bench: &[],
    }),
};

fn tones() -> Box<dyn PageState> {
    Box::new(Tones)
}

/// The tones and the word each one shows here; the toolkit maps no domain status to a tone.
const CASES: [(&str, Tone); 5] = [
    ("Neutral", Tone::Neutral),
    ("Healthy", Tone::Ok),
    ("Pending", Tone::Warn),
    ("Failed", Tone::Err),
    ("Info", Tone::Data),
];

/// Every tone in a row, then the same row with the leading dot.
struct Tones;

impl PageState for Tones {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Badge");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        for (row, (title, dot)) in [("BADGES", false), ("BADGES WITH A DOT", true)]
            .into_iter()
            .enumerate()
        {
            let ry = y + row as f32 * 72.0;
            heading(ui, title, x, ry);
            let mut bx = x;
            for (i, (text, tone)) in CASES.into_iter().enumerate() {
                let r = ui.badge((row, i), (bx, ry + 24.0), text, tone, dot);
                caption(ui, &format!("{tone:?}"), bx, ry + 24.0 + 26.0);
                bx += r.w.max(64.0) + 24.0;
            }
        }
    }
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "badge-tones",
        scale: 2.0,
        tier: MotionTier::Full,
        state: tones,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "badge-tones",
        scale: 1.0,
        tier: MotionTier::Full,
        state: tones,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];
