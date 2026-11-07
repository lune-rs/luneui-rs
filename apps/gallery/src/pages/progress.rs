//! The `progress` page: a determinate bar with a head, the three tones, a pending row and the
//! indeterminate bar (D118) under Full and Reduced.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::progress::ProgressOptions;
use lune_ui::widgets::tone::Tone;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};
use crate::showcase::knobs::{Knobs, TONES, tone_names};

pub(crate) static PAGE: Page = Page {
    id: "progress",
    title: "Progress",
    kind: PageKind::Ui(UiPage {
        state: all,
        scenes: SCENES,
        bench: &[],
    }),
};

const BAR_W: f32 = 360.0;

/// A page state that draws its sections with one function.
/// A page state that draws its sections with one function, and a sample the knobs shape.
struct Sections {
    list: &'static [fn(&mut Ui<'_>, f32, f32) -> f32],
    /// Show a sample the knobs shape, under the sections.
    show: bool,
    value: f32,
    indeterminate: bool,
    /// An index into [`TONES`].
    tone: usize,
    head: String,
}

impl Sections {
    fn new(list: &'static [fn(&mut Ui<'_>, f32, f32) -> f32]) -> Self {
        Self {
            list,
            show: false,
            value: 0.4,
            indeterminate: false,
            tone: 0,
            head: "Sample".to_owned(),
        }
    }
}

impl PageState for Sections {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Progress");
        let (x, mut y) = (card.x + 24.0, card.y + 24.0);
        for section in self.list {
            y += section(ui, x, y) + 28.0;
        }
        if self.show {
            let opts = ProgressOptions {
                value: (!self.indeterminate).then_some(self.value),
                tone: TONES[self.tone.min(TONES.len() - 1)].1,
                head: (!self.head.is_empty()).then_some((self.head.as_str(), "")),
                name: "Sample",
            };
            ui.progress("sample", Rect::new(x, y, BAR_W, 0.0), &opts);
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "Show a sample", &mut self.show);
        panel.number(ui, "Value", &mut self.value, (0.0, 1.0, 0.0));
        panel.toggle(ui, "Indeterminate", &mut self.indeterminate);
        panel.choice(ui, "Tone", &tone_names(), &mut self.tone);
        panel.text(ui, "Head", &mut self.head);
    }
}

fn determinate_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "DETERMINATE, WITH A HEAD", x, y);
    let opts = ProgressOptions {
        value: Some(0.4),
        head: Some(("Indexing", "40%")),
        ..ProgressOptions::default()
    };
    16.0 + ui.progress("determinate", Rect::new(x, y + 20.0, BAR_W, 0.0), &opts)
}

fn variants_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "DEFAULT, WARN, ERR", x, y);
    let mut at = y + 20.0;
    for (i, (tone, value)) in [(Tone::Neutral, 0.6), (Tone::Warn, 0.8), (Tone::Err, 0.9)]
        .into_iter()
        .enumerate()
    {
        let opts = ProgressOptions {
            value: Some(value),
            tone,
            name: "Variant",
            ..ProgressOptions::default()
        };
        at += ui.progress(("variant", i), Rect::new(x, at, BAR_W, 0.0), &opts) + 12.0;
    }
    at - y - 12.0
}

fn pending_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "PENDING", x, y);
    let opts = ProgressOptions {
        value: Some(0.0),
        head: Some(("Waiting for the queue", "pending")),
        ..ProgressOptions::default()
    };
    16.0 + ui.progress("pending", Rect::new(x, y + 20.0, BAR_W, 0.0), &opts)
}

fn indeterminate_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "INDETERMINATE", x, y);
    let opts = ProgressOptions {
        value: None,
        name: "Loading",
        ..ProgressOptions::default()
    };
    16.0 + ui.progress("indeterminate", Rect::new(x, y + 20.0, BAR_W, 0.0), &opts)
}

fn all() -> Box<dyn PageState> {
    Box::new(Sections::new(&[
        determinate_section,
        variants_section,
        pending_section,
        indeterminate_section,
    ]))
}

fn determinate() -> Box<dyn PageState> {
    Box::new(Sections::new(&[determinate_section]))
}

fn variants() -> Box<dyn PageState> {
    Box::new(Sections::new(&[variants_section]))
}

fn pending() -> Box<dyn PageState> {
    Box::new(Sections::new(&[pending_section]))
}

fn indeterminate() -> Box<dyn PageState> {
    Box::new(Sections::new(&[indeterminate_section]))
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "progress-determinate",
        scale: 2.0,
        tier: MotionTier::Full,
        state: determinate,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "progress-variants",
        scale: 2.0,
        tier: MotionTier::Full,
        state: variants,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "progress-pending",
        scale: 2.0,
        tier: MotionTier::Full,
        state: pending,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The segment travels 2400 ms: a quarter of the way at 600 ms.
    UiScene {
        id: "progress-indeterminate",
        scale: 2.0,
        tier: MotionTier::Full,
        state: indeterminate,
        script: &[],
        capture_ms: 600,
        expect: Expect::Moving,
    },
    // The Reduced fill breathes over 1800 ms: fully up at 900 ms.
    UiScene {
        id: "progress-indeterminate-reduced",
        scale: 2.0,
        tier: MotionTier::Reduced,
        state: indeterminate,
        script: &[],
        capture_ms: 900,
        expect: Expect::Moving,
    },
];
