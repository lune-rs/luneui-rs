//! The `bar-chart` page: 24 hourly columns of two stacked series, the empty chart, a hovered
//! column with its tooltip and a change of the data (D164, D165). A button retargets the data; the
//! bench presses it.

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::bar_chart::{BarChartOptions, BarColumn, BarSeries};
use lune_ui::widgets::button::{ButtonKind, button_width};

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "bar-chart",
    title: "Bar chart",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

const HOURS: usize = 24;
const CHART_W: f32 = 760.0;
/// The chart's top left from the card's: under the heading and the button.
const CHART_AT: (f32, f32) = (24.0, 24.0 + 80.0);
/// The card's top left in the window (the gallery's page card).
const CARD_AT: (f32, f32) = (10.0, 34.0);

/// The hourly input tokens: cache reads and the rest, a day that peaks at `peak` o'clock.
fn hourly(peak: f64) -> Vec<[f64; 2]> {
    (0..HOURS)
        .map(|h| {
            let d = (h as f64 - peak) / 3.5;
            let bump = (-d * d).exp();
            let cache = 1_150_000.0 * bump + 6_000.0;
            [cache.round(), (cache * 0.12 + 3_000.0).round()]
        })
        .collect()
}

fn short(n: f64) -> String {
    if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.0}k", n / 1_000.0)
    } else {
        format!("{n:.0}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Data,
    Empty,
}

struct Live {
    shape: Shape,
    /// Which of the two days is shown; the button flips it, and so does the scenes' script via the
    /// frame count.
    second: bool,
    /// The scene that changes the data on its second frame.
    changes: bool,
    frames: u32,
}

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        shape: Shape::Data,
        second: false,
        changes: false,
        frames: 0,
    })
}

fn empty() -> Box<dyn PageState> {
    Box::new(Live {
        shape: Shape::Empty,
        second: false,
        changes: false,
        frames: 0,
    })
}

fn change() -> Box<dyn PageState> {
    Box::new(Live {
        shape: Shape::Data,
        second: false,
        changes: true,
        frames: 0,
    })
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Bar chart");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "HOURLY INPUT TOKENS, TWO SERIES", x, y);
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
        if self.changes {
            self.frames += 1;
            self.second = self.frames > 1;
        }
        let data = match self.shape {
            Shape::Data => hourly(if self.second { 17.0 } else { 11.0 }),
            Shape::Empty => vec![[0.0, 0.0]; HOURS],
        };
        let tips: Vec<String> = data
            .iter()
            .enumerate()
            .map(|(h, v)| {
                format!(
                    "{h:02}:00  cache read {}, uncached {}",
                    short(v[0]),
                    short(v[1])
                )
            })
            .collect();
        let columns: Vec<BarColumn<'_>> = data
            .iter()
            .zip(&tips)
            .map(|(v, tip)| BarColumn { values: v, tip })
            .collect();
        let series = [
            BarSeries {
                name: "Cache read",
                color: THEME.data,
            },
            BarSeries {
                name: "Uncached + cache write",
                color: THEME.accent,
            },
        ];
        let x_labels: Vec<(usize, &str)> = (0..HOURS)
            .step_by(3)
            .map(|h| (h, ["00", "03", "06", "09", "12", "15", "18", "21"][h / 3]))
            .collect();
        let max = if self.shape == Shape::Empty {
            0.0
        } else {
            1_400_000.0
        };
        ui.bar_chart(
            "chart",
            Rect::new(x, y + 80.0, CHART_W, 0.0),
            &BarChartOptions {
                series: &series,
                columns: &columns,
                max,
                y_labels: ["1.4M", "700k", "0"],
                x_labels: &x_labels,
                label: "Input tokens per hour",
                description: "Hourly input tokens, mostly cache reads, peaking late morning",
                live: false,
            },
        );
    }
}

/// The window point at the middle of column `c`, half way up the plot.
const fn column_point(c: usize) -> (f32, f32) {
    let slot = (CHART_W - 48.0) / HOURS as f32;
    (
        CARD_AT.0 + CHART_AT.0 + 48.0 + slot * (c as f32 + 0.5),
        CARD_AT.1 + CHART_AT.1 + 80.0,
    )
}

const HOVER: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(column_point(10).0, column_point(10).1)],
    },
    Step {
        at_ms: 800,
        events: &[],
    },
];

/// The second frame at time 0 carries the other day, so the capture at 90 ms is the middle of
/// the 180 ms tween.
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
    scene("bar-chart-data", live, &[], 0, Expect::Settled),
    scene("bar-chart-empty", empty, &[], 0, Expect::Settled),
    scene("bar-chart-hover", live, HOVER, 1200, Expect::Settled),
    scene("bar-chart-value-mid", change, &[CHANGE], 90, Expect::Moving),
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
