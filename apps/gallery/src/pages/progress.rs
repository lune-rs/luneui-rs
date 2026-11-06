//! The `progress` page: a determinate bar with a head, the three tones, a pending row and the
//! indeterminate bar (D118) under Full and Reduced.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::progress::ProgressOptions;
use lune_ui::widgets::tone::Tone;

use super::{card, heading};
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};

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
struct Sections(&'static [fn(&mut Ui<'_>, f32, f32) -> f32]);

impl PageState for Sections {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Progress");
        let (x, mut y) = (card.x + 24.0, card.y + 24.0);
        for section in self.0 {
            y += section(ui, x, y) + 28.0;
        }
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
    Box::new(Sections(&[
        determinate_section,
        variants_section,
        pending_section,
        indeterminate_section,
    ]))
}

fn determinate() -> Box<dyn PageState> {
    Box::new(Sections(&[determinate_section]))
}

fn variants() -> Box<dyn PageState> {
    Box::new(Sections(&[variants_section]))
}

fn pending() -> Box<dyn PageState> {
    Box::new(Sections(&[pending_section]))
}

fn indeterminate() -> Box<dyn PageState> {
    Box::new(Sections(&[indeterminate_section]))
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
