//! The `primitives` page: one scene per new painter primitive (D15): arc, sector, path fill, dash,
//! rounded clip and rotate. The catalogue's owning scenes come in later milestones; these show each
//! primitive on its own.

use std::f32::consts::{PI, TAU};

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, StrokeStyle};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "primitives",
    title: "Primitives",
    kind: PageKind::Ui(UiPage {
        state: sheet,
        scenes: SCENES,
        bench: BENCH,
    }),
};

#[derive(Debug, Clone, Copy)]
enum Shape {
    Arc,
    Sector,
    PathFill,
    Dash,
    ClipRounded,
    Rotate,
}

const SHAPES: [Shape; 6] = [
    Shape::Arc,
    Shape::Sector,
    Shape::PathFill,
    Shape::Dash,
    Shape::ClipRounded,
    Shape::Rotate,
];

fn name(which: Shape) -> &'static str {
    match which {
        Shape::Arc => "arc",
        Shape::Sector => "sector",
        Shape::PathFill => "path fill",
        Shape::Dash => "dash",
        Shape::ClipRounded => "clip rounded",
        Shape::Rotate => "rotate",
    }
}

/// One primitive in `cell`: its name at the top-left, the drawing below it.
fn shape(ui: &mut Ui<'_>, which: Shape, cell: Rect) {
    caption(ui, name(which), cell.x, cell.y);
    let c = [cell.x + 90.0, cell.y + 100.0];
    match which {
        Shape::Arc => {
            let p = ui.painter();
            p.arc(c, 50.0, 0.0, TAU, StrokeStyle::solid(THEME.border, 6.0));
            p.arc(
                c,
                50.0,
                -PI / 2.0,
                1.5 * PI,
                StrokeStyle::solid(THEME.accent, 6.0),
            );
        }
        Shape::Sector => {
            let p = ui.painter();
            p.sector(c, 50.0, -PI / 2.0, 0.6 * PI, THEME.accent);
            p.sector(c, 50.0, -PI / 2.0 + 0.6 * PI, 0.4 * PI, THEME.surface_hover);
        }
        Shape::PathFill => {
            // A five-point star: ten vertices alternating outer and inner radius.
            let star: Vec<[f32; 2]> = (0..10)
                .map(|i| {
                    let r = if i % 2 == 0 { 55.0 } else { 24.0 };
                    let a = -PI / 2.0 + i as f32 * PI / 5.0;
                    [c[0] + r * a.cos(), c[1] + r * a.sin()]
                })
                .collect();
            ui.painter().fill_path(&star, THEME.accent);
        }
        Shape::Dash => {
            let p = ui.painter();
            let line = [
                [cell.x + 10.0, cell.y + 40.0],
                [cell.x + 170.0, cell.y + 40.0],
            ];
            p.dash(&line, false, 2.0, 8.0, 6.0, THEME.fg_secondary);
            let frame = [
                [cell.x + 10.0, cell.y + 70.0],
                [cell.x + 170.0, cell.y + 70.0],
                [cell.x + 170.0, cell.y + 150.0],
                [cell.x + 10.0, cell.y + 150.0],
            ];
            p.dash(&frame, true, 2.0, 6.0, 4.0, THEME.accent);
        }
        Shape::ClipRounded => {
            let clip = Rect::new(cell.x + 20.0, cell.y + 30.0, 140.0, 120.0);
            let p = ui.painter();
            p.push_clip_rounded(clip, 24.0);
            p.fill(
                Rect::new(cell.x, cell.y + 20.0, 180.0, 140.0),
                0.0,
                THEME.accent,
            );
            p.pop_clip();
            p.ring(clip, 24.0, 1.0, THEME.border, false);
        }
        Shape::Rotate => {
            let spec = Spec::plain("Turned", Style::ui(THEME.font_base, THEME.fg_primary));
            let p = ui.painter();
            p.text(&spec, cell.x + 10.0, cell.y + 40.0);
            p.text_rotated(&spec, (c[0], c[1] + 20.0), PI / 6.0);
        }
    }
}

/// The card's heading, then the content area under it.
fn content(ui: &mut Ui<'_>) -> Rect {
    let card = card(ui, "Primitives");
    let (x, y) = (card.x + 24.0, card.y + 24.0);
    heading(ui, "PRIMITIVES", x, y);
    Rect::new(x, y + 24.0, card.w - 48.0, card.h - 48.0 - 24.0)
}

/// Every primitive in a grid of three columns.
struct Sheet;

impl PageState for Sheet {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let area = content(ui);
        for (i, which) in SHAPES.iter().enumerate() {
            let cell = Rect::new(
                area.x + (i % 3) as f32 * 300.0,
                area.y + (i / 3) as f32 * 170.0,
                300.0,
                170.0,
            );
            shape(ui, *which, cell);
        }
    }
}

/// One primitive filling the content area.
struct Show(Shape);

impl PageState for Show {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let area = content(ui);
        shape(ui, self.0, area);
    }
}

fn sheet() -> Box<dyn PageState> {
    Box::new(Sheet)
}

fn arc() -> Box<dyn PageState> {
    Box::new(Show(Shape::Arc))
}

fn sector() -> Box<dyn PageState> {
    Box::new(Show(Shape::Sector))
}

fn path_fill() -> Box<dyn PageState> {
    Box::new(Show(Shape::PathFill))
}

fn dash() -> Box<dyn PageState> {
    Box::new(Show(Shape::Dash))
}

fn clip_rounded() -> Box<dyn PageState> {
    Box::new(Show(Shape::ClipRounded))
}

fn rotate() -> Box<dyn PageState> {
    Box::new(Show(Shape::Rotate))
}

const fn still(id: &'static str, state: fn() -> Box<dyn PageState>) -> UiScene {
    UiScene {
        id,
        scale: 2,
        tier: MotionTier::Full,
        state,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    }
}

const SCENES: &[UiScene] = &[
    still("primitive-arc", arc),
    still("primitive-sector", sector),
    still("primitive-path-fill", path_fill),
    still("primitive-dash", dash),
    still("primitive-clip-rounded", clip_rounded),
    still("primitive-rotate", rotate),
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
