//! The `stack` page: the layout helpers' gallery scenes (`layout.stack`). Each scene lays boxes out
//! with `row`, `column` or `grid` and draws them with theme tokens.

use lune_ui::layout::{self, Align, Child, Insets, Size, Track};
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "stack",
    title: "Stack",
    kind: PageKind::Ui(UiPage {
        state: sheet,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The five sample groups, in the order the window shows them.
#[derive(Debug, Clone, Copy)]
enum Sample {
    Gaps,
    Sizing,
    Alignment,
    Grid,
    Insets,
}

const SAMPLES: [Sample; 5] = [
    Sample::Gaps,
    Sample::Sizing,
    Sample::Alignment,
    Sample::Grid,
    Sample::Insets,
];

/// The card's heading, then the content area under it.
fn content(ui: &mut Ui<'_>, stage: &Stage) -> Rect {
    let card = card(ui, stage, "Stack");
    let (x, y) = (card.x + 24.0, card.y + 24.0);
    heading(ui, "STACK", x, y);
    Rect::new(x, y + 24.0, card.w - 48.0, card.h - 48.0 - 24.0)
}

/// The sample groups in the window's own layout, or the one the knob picks, filling the area.
#[derive(Default)]
struct Sheet {
    /// 0 shows every group; n shows `SAMPLES[n - 1]` alone.
    pick: usize,
}

impl PageState for Sheet {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = content(ui, stage);
        if let Some(which) = self.pick.checked_sub(1).and_then(|i| SAMPLES.get(i)) {
            sample(ui, *which, area);
            return;
        }
        let band = area.h / SAMPLES.len() as f32;
        for (i, which) in SAMPLES.iter().enumerate() {
            let at = Rect::new(area.x, area.y + i as f32 * band, area.w, band);
            sample(ui, *which, at);
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let names = ["All", "Gaps", "Sizing", "Alignment", "Grid", "Insets"];
        panel.choice(ui, "Group", &names, &mut self.pick);
    }
}

/// One sample group filling the content area.
struct Show(Sample);

impl PageState for Show {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = content(ui, stage);
        sample(ui, self.0, area);
    }
}

fn sample(ui: &mut Ui<'_>, which: Sample, band: Rect) {
    let name = match which {
        Sample::Gaps => "gaps 4, 8, 12",
        Sample::Sizing => "hug, fixed, fill 1 and 2",
        Sample::Alignment => "start, center, end, stretch",
        Sample::Grid => "grid Auto, *, Auto",
        Sample::Insets => "insets 8, 12, 8, 12",
    };
    caption(ui, name, band.x, band.y);
    let area = Rect::new(band.x, band.y + 22.0, band.w, (band.h - 22.0).max(0.0));
    match which {
        Sample::Gaps => gaps(ui, area),
        Sample::Sizing => sizing(ui, area),
        Sample::Alignment => alignment(ui, area),
        Sample::Grid => grid(ui, area),
        Sample::Insets => insets(ui, area),
    }
}

/// A filled, bordered box per rect.
fn boxes(ui: &mut Ui<'_>, rects: &[Rect]) {
    let p = ui.painter();
    for r in rects {
        p.fill(*r, 3.0, THEME.surface_hover);
        p.ring(*r, 3.0, 1.0, THEME.border, false);
    }
}

/// Three fixed children of `sizes` along the main axis, all `cross` across it.
fn fixed(sizes: &[f32], cross: f32) -> Vec<Child> {
    sizes
        .iter()
        .map(|&m| Child {
            main: Size::Fixed(m),
            cross,
        })
        .collect()
}

/// A row at gap 4, a column at gap 8 and a row at gap 12, three 20 px boxes each.
fn gaps(ui: &mut Ui<'_>, area: Rect) {
    let a = layout::row(
        Rect::new(area.x, area.y, 150.0, 20.0),
        &fixed(&[20.0; 3], 20.0),
        4.0,
        Align::Start,
        Insets::ZERO,
    );
    let b = layout::column(
        Rect::new(area.x + 170.0, area.y, 20.0, 76.0),
        &fixed(&[20.0; 3], 20.0),
        8.0,
        Align::Start,
        Insets::ZERO,
    );
    let c = layout::row(
        Rect::new(area.x + 220.0, area.y, 150.0, 20.0),
        &fixed(&[20.0; 3], 20.0),
        12.0,
        Align::Start,
        Insets::ZERO,
    );
    boxes(ui, &a);
    boxes(ui, &b);
    boxes(ui, &c);
    caption(ui, "gap 4", area.x, area.y + 26.0);
    caption(ui, "gap 8", area.x + 196.0, area.y + 28.0);
    caption(ui, "gap 12", area.x + 220.0, area.y + 26.0);
}

/// Hug 60 (measured), fixed 80, then fill weights 1 and 2 share the leftover.
fn sizing(ui: &mut Ui<'_>, area: Rect) {
    let kids = [
        Child {
            main: Size::Hug(60.0),
            cross: 20.0,
        },
        Child {
            main: Size::Fixed(80.0),
            cross: 20.0,
        },
        Child {
            main: Size::Fill(1.0),
            cross: 20.0,
        },
        Child {
            main: Size::Fill(2.0),
            cross: 20.0,
        },
    ];
    let parent = Rect::new(area.x, area.y, area.w.min(480.0), 20.0);
    let rects = layout::row(parent, &kids, 8.0, Align::Start, Insets::ZERO);
    boxes(ui, &rects);
    for (r, name) in rects.iter().zip(["hug", "fixed", "fill 1", "fill 2"]) {
        caption(ui, name, r.x, r.y + 26.0);
    }
}

/// Four cells side by side; each holds a 20 px and a 40 px box cross-aligned in its mode.
fn alignment(ui: &mut Ui<'_>, area: Rect) {
    let modes = [
        ("start", Align::Start),
        ("center", Align::Center),
        ("end", Align::End),
        ("stretch", Align::Stretch),
    ];
    for (i, (name, mode)) in modes.into_iter().enumerate() {
        let cell = Rect::new(area.x + i as f32 * 120.0, area.y, 100.0, 56.0);
        let kids = [
            Child {
                main: Size::Fixed(30.0),
                cross: 20.0,
            },
            Child {
                main: Size::Fixed(30.0),
                cross: 40.0,
            },
        ];
        let rects = layout::row(cell, &kids, 4.0, mode, Insets::ZERO);
        boxes(ui, &rects);
        caption(ui, name, cell.x, cell.y + 60.0);
    }
}

/// Columns Auto 60, star, Auto 60 with a gap of 8; two Auto 32 rows with a row gap of 12.
fn grid(ui: &mut Ui<'_>, area: Rect) {
    let g = layout::grid(
        Rect::new(area.x, area.y, 360.0, 76.0),
        &[Track::Auto(60.0), Track::Star(1.0), Track::Auto(60.0)],
        &[Track::Auto(32.0), Track::Auto(32.0)],
        8.0,
        12.0,
        Insets::ZERO,
    );
    for row in 0..g.rows() {
        for col in 0..g.columns() {
            boxes(ui, &[g.cell(col, row)]);
        }
    }
}

/// A filled parent with insets 8, 12, 8, 12; the child fills what is left.
fn insets(ui: &mut Ui<'_>, area: Rect) {
    let parent = Rect::new(area.x, area.y, area.w.min(300.0), 72.0);
    boxes(ui, &[parent]);
    let inner = layout::row(
        parent,
        &[Child {
            main: Size::Fill(1.0),
            cross: 0.0,
        }],
        0.0,
        Align::Stretch,
        Insets::new(8.0, 12.0, 8.0, 12.0),
    );
    let p = ui.painter();
    for r in inner {
        p.fill(r, 3.0, THEME.accent);
    }
}

fn sheet() -> Box<dyn PageState> {
    Box::new(Sheet::default())
}

fn gaps_state() -> Box<dyn PageState> {
    Box::new(Show(Sample::Gaps))
}

fn sizing_state() -> Box<dyn PageState> {
    Box::new(Show(Sample::Sizing))
}

fn align_state() -> Box<dyn PageState> {
    Box::new(Show(Sample::Alignment))
}

fn grid_state() -> Box<dyn PageState> {
    Box::new(Show(Sample::Grid))
}

fn insets_state() -> Box<dyn PageState> {
    Box::new(Show(Sample::Insets))
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "stack-gaps",
        scale: 2.0,
        tier: MotionTier::Full,
        state: gaps_state,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "stack-sizing",
        scale: 2.0,
        tier: MotionTier::Full,
        state: sizing_state,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "stack-align",
        scale: 2.0,
        tier: MotionTier::Full,
        state: align_state,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "stack-grid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: grid_state,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The pair: the same insets at scale 1 and scale 2.
    UiScene {
        id: "stack-insets",
        scale: 1.0,
        tier: MotionTier::Full,
        state: insets_state,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "stack-insets",
        scale: 2.0,
        tier: MotionTier::Full,
        state: insets_state,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];

/// A static page: the bench moves the pointer across it.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(120.0, 120.0)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::PointerAt(480.0, 360.0)],
    },
];
