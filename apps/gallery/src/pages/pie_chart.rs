//! The `pie-chart` page: three slices, none, one past half, one of the whole and a change of the
//! data (D164, D166). A button retargets the data; the bench presses it.

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::pie_chart::{PieOptions, PieSlice};

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "pie-chart",
    title: "Pie chart",
    kind: PageKind::Ui(UiPage {
        state: three,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// Storage in GB: objects, packs, large files.
const FIRST: [f64; 3] = [5.2, 3.1, 1.4];
const SECOND: [f64; 3] = [2.0, 6.5, 2.5];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Three,
    Empty,
    Half,
    Full,
    Change,
}

struct Live {
    shape: Shape,
    second: bool,
    frames: u32,
}

fn state(shape: Shape) -> Box<dyn PageState> {
    Box::new(Live {
        shape,
        second: false,
        frames: 0,
    })
}

fn three() -> Box<dyn PageState> {
    state(Shape::Three)
}
fn empty() -> Box<dyn PageState> {
    state(Shape::Empty)
}
fn half() -> Box<dyn PageState> {
    state(Shape::Half)
}
fn full() -> Box<dyn PageState> {
    state(Shape::Full)
}
fn change() -> Box<dyn PageState> {
    state(Shape::Change)
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Pie chart");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "STORAGE BY KIND", x, y);
        let label = "Retarget the data";
        let w = button_width(ui.painter(), label);
        if ui
            .button(
                "retarget",
                Rect::new(x, y + 20.0, w, 32.0),
                label,
                ButtonKind::Secondary,
            )
            .clicked
        {
            self.second = !self.second;
        }
        if self.shape == Shape::Change {
            self.frames += 1;
            self.second = self.frames > 1;
        }
        let sizes: Vec<f64> = match self.shape {
            Shape::Three | Shape::Change => if self.second { SECOND } else { FIRST }.to_vec(),
            Shape::Empty => Vec::new(),
            Shape::Half => vec![7.0, 2.0, 1.0],
            Shape::Full => vec![9.0],
        };
        let colors = [THEME.data, THEME.accent, THEME.warn];
        let slices: Vec<PieSlice> = sizes
            .iter()
            .enumerate()
            .map(|(i, &value)| PieSlice {
                key: i as u64,
                value,
                color: colors[i % colors.len()],
            })
            .collect();
        ui.pie_chart(
            "chart",
            Rect::new(x, y + 72.0, 160.0, 160.0),
            &PieOptions {
                slices: &slices,
                label: "Repository storage",
                description: "Objects, packs and large files by size",
                live: false,
            },
        );
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let shapes = [
            ("Three slices", Shape::Three),
            ("Empty", Shape::Empty),
            ("Half", Shape::Half),
            ("Full", Shape::Full),
        ];
        let names: Vec<&str> = shapes.iter().map(|(n, _)| *n).collect();
        let mut at = shapes
            .iter()
            .position(|(_, s)| *s == self.shape)
            .unwrap_or(0);
        if panel.choice(ui, "Slices", &names, &mut at) {
            self.shape = shapes[at].1;
        }
        panel.toggle(ui, "Second data", &mut self.second);
    }
}

/// The second frame at time 0 carries the other data, so the capture at 90 ms is the middle of the
/// 180 ms tween.
const CHANGE: Step = Step {
    at_ms: 0,
    events: &[],
};

const fn scene(
    id: &'static str,
    state: fn() -> Box<dyn PageState>,
    script: &'static [Step],
    capture_ms: u32,
    expect: Expect,
) -> UiScene {
    UiScene {
        id,
        scale: 2.0,
        tier: MotionTier::Full,
        state,
        script,
        capture_ms,
        expect,
    }
}

const SCENES: &[UiScene] = &[
    scene("pie-chart-three", three, &[], 0, Expect::Settled),
    scene("pie-chart-empty", empty, &[], 0, Expect::Settled),
    scene("pie-chart-half", half, &[], 0, Expect::Settled),
    scene("pie-chart-full", full, &[], 0, Expect::Settled),
    scene("pie-chart-value-mid", change, &[CHANGE], 90, Expect::Moving),
];

const RETARGET: &[&str] = &["retarget"];

/// What the bench replays: the data retargets twice.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(RETARGET), Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Click],
    },
];
