//! The `verdict-row` page: OK, WARN and NO in the three column layout, then the same three
//! stacked in a narrow column.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::verdict_row::Verdict;

use super::{card, heading};
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "verdict-row",
    title: "Verdict row",
    kind: PageKind::Ui(UiPage {
        state: states,
        scenes: SCENES,
        bench: &[],
    }),
};

fn states() -> Box<dyn PageState> {
    Box::new(States)
}

/// The three verdicts with a name and a text each; the second text is long enough to wrap in
/// the narrow column.
const CASES: [(Verdict, &str, &str); 3] = [
    (
        Verdict::Ok,
        "Quant",
        "Fits in memory with room for the context",
    ),
    (
        Verdict::Warn,
        "Speed",
        "About 6 tokens per second on this machine",
    ),
    (
        Verdict::No,
        "Disk",
        "Not enough free space for the download",
    ),
];

struct States;

impl PageState for States {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Verdict row");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "THREE COLUMNS", x, y);
        let mut at = y + 24.0;
        for (i, (verdict, name, text)) in CASES.into_iter().enumerate() {
            at += ui.verdict_row(
                ("wide", i),
                Rect::new(x, at, 560.0, 0.0),
                verdict,
                name,
                text,
                false,
            );
        }
        let sy = at + 32.0;
        heading(ui, "STACKED", x, sy);
        let mut at = sy + 24.0;
        for (i, (verdict, name, text)) in CASES.into_iter().enumerate() {
            at += ui.verdict_row(
                ("stacked", i),
                Rect::new(x, at, 214.0, 0.0),
                verdict,
                name,
                text,
                true,
            );
        }
    }
}

const SCENES: &[UiScene] = &[UiScene {
    id: "verdict-row-states",
    scale: 2.0,
    tier: MotionTier::Full,
    state: states,
    script: &[],
    capture_ms: 0,
    expect: Expect::Settled,
}];
