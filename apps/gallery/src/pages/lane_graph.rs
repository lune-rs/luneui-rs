//! The `lane-graph` page: rows of the graph a git client draws beside its commit list (D170). The
//! window and the bench show a 100,000-row graph in a virtualized list with a text column, its
//! lanes generated from the row's index; the scenes show a linear run, a merge, a branch, a hand-built
//! advanced graph (a fork to three lanes, an octopus merge, one lane forking twice, a lane reused)
//! and the list's own selected and hovered look. A knob swaps the list's graph between the
//! generated one and the advanced one. The row is decorative: the list owns hover, selection
//! and the accessible name. The app owns the lane layout (D16): the advanced graph is written out
//! by hand, there is no helper that assigns lanes.

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::lane_graph::{LaneEdge, LaneLine, LaneRow, ROW_H, width};
use lune_ui::widgets::virtual_list::{Heights, ListOptions, RowKey, SelectionMode};

use super::{card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "lane-graph",
    title: "Lane graph",
    kind: PageKind::Ui(UiPage {
        state: big,
        scenes: SCENES,
        bench: BENCH,
    }),
};

const LANE_COLORS: [[u8; 4]; 4] = [THEME.data, THEME.accent, THEME.warn, THEME.ok];
/// The last lane of the generated graph, and of the advanced one.
const MAX_LANE: u16 = 2;
const ADVANCED_MAX_LANE: u16 = 3;
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

/// A row of the advanced graph, written out by hand, newest at the top, four lanes. Every edge
/// runs from its row's dot down to its parent's, and carries the lane it ends on or starts from.
///
/// - Rows 0 to 3: a fork to three lanes. Three tips (lanes 1, 2, 3) end at one commit on the trunk
///   (row 3).
/// - Rows 4 to 8: an octopus merge. The commit at row 4 has three parents, on the trunk (row 5),
///   lane 1 (row 6) and lane 2 (row 7); the two branches join the trunk again at row 8.
/// - Rows 9 to 14: lane 1 forks off the trunk twice, at rows 9 and 12, one branch after the other.
/// - Rows 15 to 18: lane 2, free since row 7, is taken by a later branch.
fn advanced_rows() -> Vec<RowData> {
    vec![
        (1, vec![], vec![edge(1, 0, 3)]),
        (2, vec![], vec![edge(2, 0, 2)]),
        (3, vec![], vec![edge(3, 0, 1)]),
        (0, vec![line(0, false, true)], vec![]),
        (
            0,
            vec![line(0, false, true)],
            vec![edge(0, 0, 1), edge(0, 1, 2), edge(0, 2, 3)],
        ),
        (0, vec![line(0, true, true)], vec![]),
        (
            1,
            vec![line(0, true, true), line(1, true, false)],
            vec![edge(1, 0, 2)],
        ),
        (
            2,
            vec![line(0, true, true), line(2, true, false)],
            vec![edge(2, 0, 1)],
        ),
        (0, vec![line(0, true, true)], vec![]),
        (0, vec![line(0, true, true)], vec![edge(0, 1, 2)]),
        (0, vec![line(0, true, true)], vec![]),
        (
            1,
            vec![line(0, true, true), line(1, true, false)],
            vec![edge(1, 0, 1)],
        ),
        (0, vec![line(0, true, true)], vec![edge(0, 1, 2)]),
        (0, vec![line(0, true, true)], vec![]),
        (
            1,
            vec![line(0, true, true), line(1, true, false)],
            vec![edge(1, 0, 1)],
        ),
        (0, vec![line(0, true, true)], vec![edge(0, 2, 2)]),
        (0, vec![line(0, true, true)], vec![]),
        (
            2,
            vec![line(0, true, true), line(2, true, false)],
            vec![edge(2, 0, 1)],
        ),
        (0, vec![line(0, true, false)], vec![]),
    ]
}

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
    /// How many commits the list holds, and how rows are selected (an index into [`SELECTIONS`]).
    count: usize,
    selection: usize,
    /// Which graph the rows show: 0 the generated one, 1 the hand-built advanced one.
    graph: usize,
    /// The advanced graph's rows, built when it is first chosen.
    advanced: Vec<RowData>,
}

/// The graphs the knob offers, in the order shown.
const GRAPHS: [&str; 2] = ["Simple", "Advanced"];

/// The selection modes the knob offers, in the order shown.
const SELECTIONS: [(&str, SelectionMode); 3] = [
    ("None", SelectionMode::None),
    ("Single", SelectionMode::Single),
    ("Multi", SelectionMode::Multi),
];

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
        let advanced = self.graph == 1;
        if advanced && self.advanced.is_empty() {
            self.advanced = advanced_rows();
        }
        let (rows, max_lane) = if advanced {
            (self.advanced.len(), ADVANCED_MAX_LANE)
        } else {
            (self.count, MAX_LANE)
        };
        let opts = ListOptions {
            count: rows,
            heights: Heights::Fixed(ROW_H),
            selection: SELECTIONS[self.selection.min(SELECTIONS.len() - 1)].1,
            index_of: None,
            label: "Commits",
        };
        let advanced_rows = &self.advanced;
        ui.virtual_list("list", on_card(card, LIST), &opts, key, |ui, row| {
            let (dot, lanes, edges) = match advanced_rows.get(row.index) {
                Some(data) if advanced => data.clone(),
                _ => generated(row.index),
            };
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
                .text(&spec, row.rect.x + width(max_lane) + 8.0, row.rect.y + 12.0);
            ROW_H
        });
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let mut rows = self.count as f32;
        if panel.number(ui, "Commits", &mut rows, (0.0, COUNT as f32, 1.0)) {
            self.count = rows.round() as usize;
        }
        let names: Vec<&str> = SELECTIONS.iter().map(|(n, _)| *n).collect();
        panel.choice(ui, "Selection", &names, &mut self.selection);
        panel.choice(ui, "Graph", &GRAPHS, &mut self.graph);
    }
}

fn big() -> Box<dyn PageState> {
    Box::new(Listed {
        seed: &[],
        seeded: false,
        count: COUNT,
        selection: 1,
        graph: 0,
        advanced: Vec::new(),
    })
}

fn listed() -> Box<dyn PageState> {
    Box::new(Listed {
        seed: &[2],
        seeded: false,
        count: COUNT,
        selection: 1,
        graph: 0,
        advanced: Vec::new(),
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

/// The advanced graph in two columns, rows 0 to 8 and 9 to 18; the trunk runs off the first
/// column's last row and into the second's first.
struct Advanced;

impl PageState for Advanced {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Lane graph");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        let rows = advanced_rows();
        let (left, right) = rows.split_at(ADVANCED_SPLIT);
        let columns = [
            ("A FORK TO THREE LANES, AN OCTOPUS MERGE", left, x),
            ("LANE 1 FORKS TWICE, LANE 2 IS REUSED", right, x + 300.0),
        ];
        for (title, rows, x) in columns {
            heading(ui, title, x, y);
            for (i, (dot, lanes, edges)) in rows.iter().enumerate() {
                let at = Rect::new(x, y + 28.0 + i as f32 * ROW_H, 0.0, 0.0);
                graph_row(ui, at, *dot, lanes, edges);
            }
        }
    }
}

/// Where the advanced graph is cut into the scene's two columns: the first row of the second.
const ADVANCED_SPLIT: usize = 9;

fn advanced() -> Box<dyn PageState> {
    Box::new(Advanced)
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
    scene("lane-graph-advanced", advanced, &[], 0, Expect::Settled),
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
