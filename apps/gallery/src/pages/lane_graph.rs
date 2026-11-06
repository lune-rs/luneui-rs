//! The `lane-graph` page: rows of the graph a git client draws beside its commit list (D170). The
//! window and the bench show a 100,000-row graph in a virtualized list with a text column, its
//! lanes generated from the row's index; the scenes show a linear run, a merge, a branch and the
//! list's own selected and hovered look. The row is decorative: the list owns hover, selection
//! and the accessible name.

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::lane_graph::{LaneEdge, LaneLine, LaneRow, ROW_H, width};
use lune_ui::widgets::virtual_list::{Heights, ListOptions, RowKey, SelectionMode};

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "lane-graph",
    title: "Lane graph",
    kind: PageKind::Ui(UiPage {
        state: big,
        scenes: SCENES,
        bench: BENCH,
    }),
};

const LANE_COLORS: [[u8; 4]; 3] = [THEME.data, THEME.accent, THEME.warn];
const MAX_LANE: u16 = 2;
const COUNT: usize = 100_000;
const LIST: Rect = Rect::new(34.0, 82.0, 520.0, 360.0);

fn color(lane: u16) -> [u8; 4] {
    LANE_COLORS[usize::from(lane) % LANE_COLORS.len()]
}

fn line(lane: u16, above: bool, below: bool) -> LaneLine {
    LaneLine {
        lane,
        color: color(lane),
        above,
        below,
    }
}

fn edge(child_lane: u16, parent_lane: u16, rows: i32) -> LaneEdge {
    LaneEdge {
        child_lane,
        child_row: 0,
        parent_lane,
        parent_row: rows,
        color: color(parent_lane.max(child_lane)),
    }
}

/// A row of the generated graph: a trunk on lane 0 and, every six rows, a branch to lane 1 that
/// lives for two rows and merges back.
fn generated(i: usize) -> (u16, Vec<LaneLine>, Vec<LaneEdge>) {
    match i % 6 {
        1 => (0, vec![line(0, true, true)], vec![edge(0, 1, 2)]),
        2 => (0, vec![line(0, true, true), line(1, false, true)], vec![]),
        3 => (
            1,
            vec![line(0, true, true), line(1, true, false)],
            vec![edge(1, 0, 1)],
        ),
        _ => (0, vec![line(0, true, true)], vec![]),
    }
}

/// A row of data: the dot lane, the lane lines and the edges.
type RowData = (u16, Vec<LaneLine>, Vec<LaneEdge>);

fn graph_row(ui: &mut Ui<'_>, rect: Rect, dot: u16, lanes: &[LaneLine], edges: &[LaneEdge]) {
    ui.lane_graph_row(
        rect,
        &LaneRow {
            dot_lane: dot,
            dot_color: color(dot),
            lanes,
            edges,
        },
    );
}

/// The list: the graph column, then the row's text.
struct Listed {
    seed: &'static [usize],
    seeded: bool,
}

fn key(i: usize) -> RowKey {
    RowKey(i as u64)
}

impl PageState for Listed {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Lane graph");
        heading(
            ui,
            "100,000 ROWS IN A VIRTUALIZED LIST",
            card.x + 24.0,
            card.y + 24.0,
        );
        if !self.seeded {
            self.seeded = true;
            let state = ui.list_state("list");
            state.selection.extend(self.seed.iter().map(|&i| key(i)));
        }
        let opts = ListOptions {
            count: COUNT,
            heights: Heights::Fixed(ROW_H),
            selection: SelectionMode::Single,
            index_of: None,
            label: "Commits",
        };
        ui.virtual_list("list", LIST, &opts, key, |ui, row| {
            let (dot, lanes, edges) = generated(row.index);
            graph_row(
                ui,
                Rect::new(row.rect.x, row.rect.y, 0.0, 0.0),
                dot,
                &lanes,
                &edges,
            );
            let spec = Spec::plain(
                format!("Commit message {}", row.index),
                Style::ui(THEME.font_base, THEME.fg_primary),
            );
            ui.painter()
                .text(&spec, row.rect.x + width(MAX_LANE) + 8.0, row.rect.y + 12.0);
            ROW_H
        });
    }
}

fn big() -> Box<dyn PageState> {
    Box::new(Listed {
        seed: &[],
        seeded: false,
    })
}

fn listed() -> Box<dyn PageState> {
    Box::new(Listed {
        seed: &[2],
        seeded: false,
    })
}

/// Rows drawn one under the other without a list: the shapes in isolation.
struct Strip {
    title: &'static str,
    rows: fn() -> Vec<RowData>,
}

impl PageState for Strip {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Lane graph");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, self.title, x, y);
        for (i, (dot, lanes, edges)) in (self.rows)().iter().enumerate() {
            let at = Rect::new(x, y + 28.0 + i as f32 * ROW_H, 0.0, 0.0);
            graph_row(ui, at, *dot, lanes, edges);
        }
    }
}

fn linear_rows() -> Vec<RowData> {
    vec![
        (0, vec![line(0, false, true)], vec![]),
        (0, vec![line(0, true, true)], vec![]),
        (0, vec![line(0, true, true)], vec![]),
        (0, vec![line(0, true, false)], vec![]),
    ]
}

/// Lane 1's node curves into the trunk two rows below it: the edge leaves the node straight down
/// and bends in at the end.
fn merge_rows() -> Vec<RowData> {
    vec![
        (1, vec![line(0, true, true)], vec![edge(1, 0, 2)]),
        (0, vec![line(0, true, true)], vec![]),
        (0, vec![line(0, true, true)], vec![]),
        (0, vec![line(0, true, false)], vec![]),
    ]
}

/// A branch leaves the first row's node for lane 1, where its node sits two rows below.
fn branch_rows() -> Vec<RowData> {
    vec![
        (0, vec![line(0, false, true)], vec![edge(0, 1, 2)]),
        (0, vec![line(0, true, true)], vec![]),
        (1, vec![line(0, true, true), line(1, false, true)], vec![]),
        (0, vec![line(0, true, false), line(1, true, false)], vec![]),
    ]
}

fn linear() -> Box<dyn PageState> {
    Box::new(Strip {
        title: "ONE LANE",
        rows: linear_rows,
    })
}

fn merge() -> Box<dyn PageState> {
    Box::new(Strip {
        title: "A MERGE",
        rows: merge_rows,
    })
}

fn branch() -> Box<dyn PageState> {
    Box::new(Strip {
        title: "A BRANCH",
        rows: branch_rows,
    })
}

/// The centre of row `i` of the list at offset 0, over its text.
const fn row_point(i: usize) -> (f32, f32) {
    (LIST.x + 300.0, LIST.y + (i as f32 + 0.5) * ROW_H)
}

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
    scene("lane-graph-linear", linear, &[], 0, Expect::Settled),
    scene("lane-graph-merge", merge, &[], 0, Expect::Settled),
    scene("lane-graph-branch", branch, &[], 0, Expect::Settled),
    // Row 2 selected, the pointer on row 4: the list's own fills under the graph.
    scene(
        "lane-graph-list",
        listed,
        &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(row_point(4).0, row_point(4).1)],
        }],
        600,
        Expect::Settled,
    ),
];

/// What the bench replays: the pointer, then scroll bursts through the graph.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(row_point(3).0, row_point(3).1)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -5.0,
            precise: false,
        }],
    },
    Step {
        at_ms: 160,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -500.0,
            precise: true,
        }],
    },
    Step {
        at_ms: 300,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -4000.0,
            precise: true,
        }],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -50.0,
            precise: false,
        }],
    },
];
