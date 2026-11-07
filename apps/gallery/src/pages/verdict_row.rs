//! The `verdict-row` page: OK, WARN and NO in the three column layout, then the same three
//! stacked in a narrow column.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::verdict_row::Verdict;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

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
    Box::new(States::default())
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

/// The three verdicts in two layouts, and a sample row beside them when the knobs ask for it.
struct States {
    show: bool,
    name: String,
    text: String,
    /// An index into [`VERDICTS`].
    verdict: usize,
    stacked: bool,
}

impl Default for States {
    fn default() -> Self {
        Self {
            show: false,
            name: "Sample".to_owned(),
            text: "A verdict with a name and a line of text".to_owned(),
            verdict: 0,
            stacked: false,
        }
    }
}

/// The verdicts, in the order the knob lists them.
const VERDICTS: [(&str, Verdict); 3] = [
    ("Ok", Verdict::Ok),
    ("Warn", Verdict::Warn),
    ("No", Verdict::No),
];

impl PageState for States {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Verdict row");
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
        if self.show {
            heading(ui, "SAMPLE", x + 600.0, y);
            let verdict = VERDICTS[self.verdict.min(VERDICTS.len() - 1)].1;
            let width = if self.stacked { 214.0 } else { 380.0 };
            ui.verdict_row(
                "sample",
                Rect::new(x + 600.0, y + 24.0, width, 0.0),
                verdict,
                &self.name,
                &self.text,
                self.stacked,
            );
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "Show a sample", &mut self.show);
        panel.text(ui, "Name", &mut self.name);
        panel.text(ui, "Text", &mut self.text);
        let names: Vec<&str> = VERDICTS.iter().map(|(n, _)| *n).collect();
        panel.choice(ui, "Verdict", &names, &mut self.verdict);
        panel.toggle(ui, "Stacked", &mut self.stacked);
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
