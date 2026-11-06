//! The `drag-and-drop` page: a strip of tabs reordered by the pointer and a kanban board of
//! cards moved between columns (`infra.drag-and-drop`), through [`ReorderDrag`], [`FreeDrag`]
//! and their `Ui` glue. The page plays the app: it owns the order and applies a `Commit` or a
//! `Drop` to it. Tabs are 100, 200 and 100 px wide with a 5 px gap, the numbers of the state
//! machine's own tests, so a scene's offsets can be read against them.
//!
//! Every scene stages the pointer the way a real gesture does: press, a 4 px move that crosses
//! the threshold, then the far move. `ReorderDrag` re-takes its grab offset while it is armed, so
//! a single big jump after the press would grab the tab at its far edge.

use lune_ui::access::{Node, Role};
use lune_ui::drag::{
    Column, DropResult, FreeDrag, FreeResult, Metrics, ReorderDrag, Strip, paint_drop_line,
};
use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::{Sense, Ui};

use super::{card, card_shift, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "drag-and-drop",
    title: "Drag and drop",
    kind: PageKind::Ui(UiPage {
        state: tabs,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The strip's left edge and top in window coordinates, inside the card under the heading.
const ORIGIN: f64 = 34.0;
const TOP: f32 = 84.0;
const HEIGHT: f32 = 32.0;
const SPACING: f64 = 5.0;
const VIEWPORT: f64 = 500.0;

/// The content x of each tab, unshifted.
fn layout_x(widths: &[f64]) -> Vec<f64> {
    let mut x = Vec::with_capacity(widths.len());
    let mut at = 0.0;
    for w in widths {
        x.push(at);
        at += w + SPACING;
    }
    x
}

/// The strip's page state: tab keys in order, each key's width, and the scroll offset.
struct Tabs {
    drag: ReorderDrag,
    keys: Vec<u32>,
    widths: Vec<f64>,
    offset: f64,
}

fn tabs() -> Box<dyn PageState> {
    Box::new(Tabs {
        drag: ReorderDrag::new(),
        keys: vec![0, 1, 2],
        widths: vec![100.0, 200.0, 100.0],
        offset: 0.0,
    })
}

/// Six wide tabs, so the strip is longer than its viewport and can scroll.
fn overflow() -> Box<dyn PageState> {
    Box::new(Tabs {
        drag: ReorderDrag::new(),
        keys: (0..6).collect(),
        widths: vec![160.0; 6],
        offset: 0.0,
    })
}

impl PageState for Tabs {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Drag and drop");
        heading(ui, "TABS", card.x + 24.0, card.y + 24.0);
        let (dx, dy) = card_shift(card);
        let (origin, top) = (ORIGIN + f64::from(dx), TOP + dy);
        let theme = *ui.theme();
        let widths: Vec<f64> = self.keys.iter().map(|k| self.widths[*k as usize]).collect();
        let x = layout_x(&widths);
        let extent = x.last().zip(widths.last()).map_or(0.0, |(x, w)| x + w);

        // The strip's track, then each tab's hit region and node.
        let track = Rect::new(
            origin as f32 - 4.0,
            top - 4.0,
            VIEWPORT as f32 + 8.0,
            HEIGHT + 8.0,
        );
        ui.painter().fill(track, 8.0, theme.surface_input);
        let mut resps = Vec::new();
        let mut rects = Vec::new();
        for (i, key) in self.keys.iter().enumerate() {
            let rect = Rect::new(
                (origin + x[i] - self.offset) as f32,
                top,
                widths[i] as f32,
                HEIGHT,
            );
            let id = ui.id(("tab", *key));
            let resp = ui.interact(id, rect, Sense::DRAG);
            ui.node(Node::new(id, Role::Tab, &format!("Tab {}", key + 1), rect));
            resps.push(resp);
            rects.push(rect);
        }

        // Every gesture before any offset, so the offsets are this frame's.
        let metrics = Metrics {
            widths: &widths,
            spacing: SPACING,
            viewport: VIEWPORT,
            offset: self.offset,
            extent,
        };
        let mut commit = None;
        let mut scroll = 0.0;
        for (i, key) in self.keys.iter().enumerate() {
            let strip = Strip {
                index: i,
                origin,
                metrics,
            };
            let id = ui.id(("tab", *key));
            let frame = ui.drag_gesture(id, &resps[i], &mut self.drag, &strip);
            if let DropResult::Commit { from, to } = frame.result {
                commit = Some((from, to));
            }
            scroll += frame.scroll_by;
        }
        let mut shifts = Vec::new();
        for (i, key) in self.keys.iter().enumerate() {
            let id = ui.id(("tab", *key));
            shifts.push(ui.drag_offset(id, &self.drag, i, x[i] as f32));
        }

        // Paint inside the viewport: the dragged tab last, so it rides over its neighbours.
        let dragged = self.drag.dragged_index();
        let order = (0..self.keys.len())
            .filter(|i| Some(*i) != dragged)
            .chain(dragged);
        let p = ui.painter();
        p.push_clip(Rect::new(
            origin as f32,
            top - 4.0,
            VIEWPORT as f32,
            HEIGHT + 8.0,
        ));
        for i in order {
            let rect = Rect {
                x: rects[i].x + shifts[i],
                ..rects[i]
            };
            let lifted = Some(i) == dragged;
            p.fill(
                rect,
                6.0,
                if lifted {
                    theme.surface_selected
                } else {
                    theme.surface_hover
                },
            );
            if lifted {
                p.ring(rect, 6.0, 1.0, theme.accent, false);
            }
            let spec = Spec::plain(
                format!("Tab {}", self.keys[i] + 1),
                Style::ui(theme.font_base, theme.fg_primary),
            );
            p.text_centered(&spec, rect);
        }
        p.pop_clip();

        if let Some((from, to)) = commit {
            let key = self.keys.remove(from);
            self.keys.insert(to, key);
        }
        if scroll != 0.0 {
            self.offset = (self.offset + scroll).clamp(0.0, (extent - VIEWPORT).max(0.0));
        }
    }
}

// The board: three columns of cards.

const COL_X: f32 = 34.0;
const COL_W: f32 = 230.0;
const COL_GAP: f32 = 20.0;
const COL_TOP: f32 = 84.0;
const COL_H: f32 = 300.0;
const CARD_TOP: f32 = 124.0;
const CARD_H: f32 = 40.0;
const CARD_STEP: f32 = 48.0;

fn col_x(dx: f32, c: usize) -> f32 {
    COL_X + dx + c as f32 * (COL_W + COL_GAP)
}

/// The board's page state: the card keys of each column, and the free drag.
struct Board {
    drag: FreeDrag,
    columns: Vec<Vec<u64>>,
}

fn board() -> Box<dyn PageState> {
    Box::new(Board {
        drag: FreeDrag::new(),
        columns: vec![vec![1, 2, 3], vec![4, 5], vec![6]],
    })
}

fn card_rect((dx, dy): (f32, f32), column: usize, index: usize) -> Rect {
    Rect::new(
        col_x(dx, column) + 10.0,
        CARD_TOP + dy + index as f32 * CARD_STEP,
        COL_W - 20.0,
        CARD_H,
    )
}

impl PageState for Board {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Drag and drop");
        heading(ui, "BOARD", card.x + 24.0, card.y + 24.0);
        let shift = card_shift(card);
        let theme = *ui.theme();

        // Columns in window coordinates, with each card's vertical midpoint.
        let mids: Vec<Vec<f64>> = self
            .columns
            .iter()
            .enumerate()
            .map(|(c, cards)| {
                (0..cards.len())
                    .map(|i| f64::from(card_rect(shift, c, i).y + CARD_H / 2.0))
                    .collect()
            })
            .collect();
        let columns: Vec<Column<'_>> = mids
            .iter()
            .enumerate()
            .map(|(c, m)| Column {
                x: f64::from(col_x(shift.0, c)),
                w: f64::from(COL_W),
                card_mids: m,
            })
            .collect();

        // Hit regions and nodes of every card, then every gesture.
        let mut cards = Vec::new();
        for (c, keys) in self.columns.iter().enumerate() {
            for (i, key) in keys.iter().enumerate() {
                let rect = card_rect(shift, c, i);
                let id = ui.id(("card", *key));
                let resp = ui.interact(id, rect, Sense::DRAG);
                ui.node(Node::new(id, Role::ListItem, &format!("Card {key}"), rect));
                cards.push((id, *key, rect, resp));
            }
        }
        let mut dropped = None;
        for (id, key, _, resp) in &cards {
            let frame = ui.free_drag_gesture(*id, resp, &mut self.drag, *key, &columns);
            if let FreeResult::Drop { source, target } = frame.result {
                dropped = Some((source, target));
            }
            if ui.consume_key_while_pressed(*id, NamedKey::Escape) {
                self.drag.cancel();
            }
        }

        // Column backgrounds; the column under the pointer takes the hover fill.
        let target = self.drag.target();
        for c in 0..self.columns.len() {
            let fill = if target.is_some_and(|t| t.column == c) {
                theme.surface_hover
            } else {
                theme.surface_input
            };
            let rect = Rect::new(col_x(shift.0, c), COL_TOP + shift.1, COL_W, COL_H);
            ui.painter().fill(rect, 8.0, fill);
        }
        // Cards; the source stays in place, dimmed.
        for (id, key, rect, _) in &cards {
            let alpha = ui.drag_dim(*id, self.drag.source() == Some(*key));
            let p = ui.painter();
            p.push_xform(Xform {
                origin: (0.0, 0.0),
                scale: 1.0,
                dx: 0.0,
                dy: 0.0,
                alpha,
            });
            p.fill(*rect, 6.0, theme.surface_selected);
            let spec = Spec::plain(
                format!("Card {key}"),
                Style::ui(theme.font_base, theme.fg_primary),
            );
            p.text_centered(&spec, *rect);
            p.pop_xform();
        }
        // The drop line sits in the gap before the card at the target index.
        if let Some(t) = target {
            let y = CARD_TOP + shift.1 + t.index as f32 * CARD_STEP - (CARD_STEP - CARD_H);
            let gap = Rect::new(
                col_x(shift.0, t.column) + 10.0,
                y,
                COL_W - 20.0,
                CARD_STEP - CARD_H,
            );
            paint_drop_line(ui.painter(), gap, &theme);
        }

        if let Some((source, t)) = dropped {
            self.apply(source, t.column, t.index);
        }
    }
}

impl Board {
    /// Moves card `source` to `index` of column `to`; the index counts the cards as they were
    /// before the move, so a card moved down its own column lands one place earlier.
    fn apply(&mut self, source: u64, to: usize, index: usize) {
        let Some((c, i)) = self
            .columns
            .iter()
            .enumerate()
            .find_map(|(c, cards)| cards.iter().position(|k| *k == source).map(|i| (c, i)))
        else {
            return;
        };
        self.columns[c].remove(i);
        let at = if c == to && i < index {
            index - 1
        } else {
            index
        };
        let at = at.min(self.columns[to].len());
        self.columns[to].insert(at, source);
    }
}

/// The window point `at` px into the strip's content and `grab_in` px into a tab starting there.
const fn grab(at: f32, grab_in: f32) -> (f32, f32) {
    (ORIGIN as f32 + at + grab_in, TOP + HEIGHT / 2.0)
}

/// The scripted pointer of the tab scenes: pressed 50 px into tab 1, a 4 px move that crosses the
/// threshold (the grab point is then 54 px), then the far move to the tab's left edge at `left`.
const PRESS: Step = Step {
    at_ms: 0,
    events: &[
        Ev::PointerAt(grab(0.0, 50.0).0, grab(0.0, 50.0).1),
        Ev::Press,
    ],
};
const CROSS: Step = Step {
    at_ms: 10,
    events: &[Ev::PointerAt(grab(0.0, 54.0).0, grab(0.0, 54.0).1)],
};
/// Left edge at content x 300: the target slot is 2 and the other tabs sit 105 px left.
const FAR: Step = Step {
    at_ms: 20,
    events: &[Ev::PointerAt(grab(300.0, 54.0).0, grab(300.0, 54.0).1)],
};
/// Left edge at content x 150: the nearest slot is 1.
const MID: Step = Step {
    at_ms: 20,
    events: &[Ev::PointerAt(grab(150.0, 54.0).0, grab(150.0, 54.0).1)],
};

/// The kanban pointer: pressed on card 1 (column 0, first card), a 4 px move, then the far move.
const fn card_point(x: f32, y: f32) -> Ev {
    Ev::PointerAt(x, y)
}
const CARD_PRESS: Step = Step {
    at_ms: 0,
    events: &[card_point(100.0, 144.0), Ev::Press],
};
const CARD_CROSS: Step = Step {
    at_ms: 10,
    events: &[card_point(104.0, 144.0)],
};
/// Over column 2, between its first and second cards: the drop index is 1.
const CARD_ONTO_COLUMN_2: Step = Step {
    at_ms: 20,
    events: &[card_point(400.0, 160.0)],
};
/// Between column 1 and column 2: over no column.
const CARD_BETWEEN: Step = Step {
    at_ms: 20,
    events: &[card_point(274.0, 160.0)],
};

/// Six wide tabs: the pointer 6 px from the viewport's right edge, with 485 px of strip to go.
const EDGE: Step = Step {
    at_ms: 20,
    events: &[Ev::PointerAt(
        ORIGIN as f32 + VIEWPORT as f32 - 6.0,
        TOP + HEIGHT / 2.0,
    )],
};

const SCENES: &[UiScene] = &[
    UiScene {
        id: "dnd-idle",
        scale: 2.0,
        tier: MotionTier::Full,
        state: tabs,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Pressed 3 px from the press point: under the 4 px threshold, still armed.
    UiScene {
        id: "dnd-armed",
        scale: 2.0,
        tier: MotionTier::Full,
        state: tabs,
        script: &[
            PRESS,
            Step {
                at_ms: 10,
                events: &[Ev::PointerAt(grab(0.0, 53.0).0, grab(0.0, 53.0).1)],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Tab 1 (100 wide) is dragged until its left edge is at content x 300: the target slot is 2
    // and the other tabs sit 105 px left of where they were. Captured after the neighbours' slide
    // (180 ms) has ended; the pointer stays down.
    UiScene {
        id: "dnd-dragging",
        scale: 2.0,
        tier: MotionTier::Full,
        state: tabs,
        script: &[PRESS, CROSS, FAR],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // The catalogue lists this scene as settled, but a pointer parked 6 px from the viewport edge
    // keeps the strip scrolling (9 px per 16 ms tick), so a frame is always due: `Moving`. The
    // capture is 60 ms after the far move, well before the strip reaches its end.
    UiScene {
        id: "dnd-autoscroll",
        scale: 2.0,
        tier: MotionTier::Full,
        state: overflow,
        script: &[PRESS, CROSS, EDGE],
        capture_ms: 80,
        expect: Expect::Moving,
    },
    // A card dragged over column 2 below its first card: dimmed source, the drop line in the gap,
    // the column's hover fill.
    UiScene {
        id: "dnd-kanban-target",
        scale: 2.0,
        tier: MotionTier::Full,
        state: board,
        script: &[CARD_PRESS, CARD_CROSS, CARD_ONTO_COLUMN_2],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // The pointer in the gap between two columns: still dragging, the source dimmed, but no drop
    // line and no hovered column.
    UiScene {
        id: "dnd-kanban-no-target",
        scale: 2.0,
        tier: MotionTier::Full,
        state: board,
        script: &[CARD_PRESS, CARD_CROSS, CARD_BETWEEN],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Escape ends the drag: the offsets are cleared and the tabs are back where they were.
    UiScene {
        id: "dnd-cancelled",
        scale: 2.0,
        tier: MotionTier::Full,
        state: tabs,
        script: &[
            PRESS,
            CROSS,
            FAR,
            Step {
                at_ms: 300,
                events: &[Ev::Key(NamedKey::Escape)],
            },
        ],
        capture_ms: 600,
        expect: Expect::Settled,
    },
    // The drag starts at 20 ms and the target changes at 30: the neighbours slide into their
    // slots over `fast` (180 ms); 90 ms later they are half way.
    UiScene {
        id: "dnd-slide",
        scale: 2.0,
        tier: MotionTier::Full,
        state: tabs,
        script: &[
            PRESS,
            CROSS,
            Step {
                at_ms: 30,
                events: &[Ev::PointerAt(grab(300.0, 54.0).0, grab(300.0, 54.0).1)],
            },
        ],
        capture_ms: 120,
        expect: Expect::Moving,
    },
    // Released at left edge 150 (target 1) at 300 ms; the tab settles from where it was drawn into
    // its slot, 90 ms into `settle` at the capture.
    UiScene {
        id: "dnd-settle",
        scale: 2.0,
        tier: MotionTier::Full,
        state: tabs,
        script: &[
            PRESS,
            CROSS,
            MID,
            Step {
                at_ms: 300,
                events: &[Ev::Release],
            },
        ],
        capture_ms: 390,
        expect: Expect::Moving,
    },
    // The source card dims over `selection` (`fast`, linear) once the drag starts at 10 ms.
    UiScene {
        id: "dnd-dim",
        scale: 2.0,
        tier: MotionTier::Full,
        state: board,
        script: &[CARD_PRESS, CARD_CROSS],
        capture_ms: 100,
        expect: Expect::Moving,
    },
];

/// What the bench replays: a tab dragged across the strip with the pointer parked at the edge,
/// then released.
const BENCH: &[Step] = &[
    PRESS,
    Step {
        at_ms: 50,
        events: &[Ev::PointerAt(grab(0.0, 54.0).0, grab(0.0, 54.0).1)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::PointerAt(grab(150.0, 54.0).0, grab(150.0, 54.0).1)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::PointerAt(grab(300.0, 54.0).0, grab(300.0, 54.0).1)],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Release],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::State;
    use lune_ui::input::FrameOutput;
    use lune_ui::testing::FakeText;
    use lune_ui::ui::WidgetId;

    fn run(id: &str) -> FrameOutput {
        let scene = SCENES.iter().find(|s| s.id == id).expect("scene");
        run_scene(PAGE.id, scene, &mut FakeText).expect("runs").1
    }

    fn pressed(out: &FrameOutput, key: impl std::hash::Hash) -> bool {
        let id = WidgetId::ROOT.with(PAGE.id).with(key);
        out.tree
            .get(id)
            .is_some_and(|n| n.states.contains(State::Pressed))
    }

    #[test]
    fn the_dragging_scene_drags_tab_1_and_marks_its_node_pressed() {
        assert!(pressed(&run("dnd-dragging"), ("tab", 0u32)));
    }

    #[test]
    fn the_armed_scene_has_a_pressed_tab() {
        assert!(pressed(&run("dnd-armed"), ("tab", 0u32)));
    }

    #[test]
    fn the_idle_and_cancelled_scenes_have_nothing_pressed() {
        assert!(!pressed(&run("dnd-idle"), ("tab", 0u32)));
        assert!(!pressed(&run("dnd-cancelled"), ("tab", 0u32)));
    }

    #[test]
    fn the_kanban_scenes_drag_card_1_whether_or_not_there_is_a_target() {
        assert!(pressed(&run("dnd-kanban-target"), ("card", 1u64)));
        assert!(pressed(&run("dnd-kanban-no-target"), ("card", 1u64)));
    }

    #[test]
    fn a_drop_moves_the_card_and_a_move_within_a_column_counts_the_old_order() {
        let mut b = Board {
            drag: FreeDrag::new(),
            columns: vec![vec![1, 2, 3], vec![4, 5], vec![6]],
        };
        b.apply(1, 1, 1);
        assert_eq!(b.columns, vec![vec![2, 3], vec![4, 1, 5], vec![6]]);
        // Index 2 of column 0 is "after card 3": card 2 lands last, one place earlier than 2.
        b.apply(2, 0, 2);
        assert_eq!(b.columns[0], vec![3, 2]);
    }
}
