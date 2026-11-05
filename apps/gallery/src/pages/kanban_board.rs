//! The `kanban-board` page: a live board of three columns on a model the page changes, its error,
//! empty and inactive states, a card being dragged and a card landing.

use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::{CARD, WINDOW};
use lune_ui::motion::MotionTier;
use lune_ui::ui::Ui;
use lune_ui::widgets::kanban_board::{
    BoardError, BoardOptions, CardMove, KanbanCard, KanbanColumn, landed_index,
};

use super::card;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "kanban-board",
    title: "Kanban board",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// One card of the page's model.
#[derive(Clone, Copy)]
struct Item {
    key: u64,
    title: &'static str,
    tags: &'static [&'static str],
    assignee: Option<&'static str>,
    note: bool,
}

struct Col {
    key: u64,
    title: &'static str,
    cards: Vec<Item>,
}

const fn item(
    key: u64,
    title: &'static str,
    tags: &'static [&'static str],
    assignee: Option<&'static str>,
    note: bool,
) -> Item {
    Item {
        key,
        title,
        tags,
        assignee,
        note,
    }
}

fn model() -> Vec<Col> {
    vec![
        Col {
            key: 1,
            title: "Backlog",
            cards: vec![
                item(
                    11,
                    "Sketch the release notes",
                    &["docs"],
                    Some("Mira"),
                    true,
                ),
                item(
                    12,
                    "Measure the cold start on the oldest supported laptop",
                    &["perf", "needs data"],
                    None,
                    false,
                ),
                item(13, "Tidy the settings page", &[], Some("Jo"), false),
            ],
        },
        Col {
            key: 2,
            title: "In progress",
            cards: vec![
                item(21, "Keyboard moves", &["input"], Some("Sam"), true),
                item(22, "Drop line motion", &["motion", "polish"], None, false),
            ],
        },
        Col {
            key: 3,
            title: "Done",
            cards: vec![item(31, "Board layout", &[], None, true)],
        },
    ]
}

const ERROR: BoardError<'static> = BoardError {
    title: "Roadmap",
    message: "This board was saved by a newer version and cannot be opened.",
    path: "/Users/demo/Boards/roadmap.board.json",
};

/// A board on a model the page applies its moves to, with a new card or column per add press.
struct Board {
    columns: Vec<Col>,
    error: bool,
    active: bool,
    added: u64,
}

fn board(columns: Vec<Col>, error: bool, active: bool) -> Box<dyn PageState> {
    Box::new(Board {
        columns,
        error,
        active,
        added: 100,
    })
}

fn live() -> Box<dyn PageState> {
    board(model(), false, true)
}

fn errored() -> Box<dyn PageState> {
    board(model(), true, true)
}

fn inactive() -> Box<dyn PageState> {
    board(model(), false, false)
}

fn empty() -> Box<dyn PageState> {
    let mut columns = model();
    columns.iter_mut().for_each(|c| c.cards.clear());
    board(columns, false, true)
}

impl Board {
    /// Applies `m` as the app would: the card leaves its list, then goes in at the landed index.
    fn apply(&mut self, m: CardMove) {
        let Some((from, at)) = self.columns.iter().enumerate().find_map(|(c, col)| {
            let i = col.cards.iter().position(|k| k.key == m.card)?;
            Some((c, i))
        }) else {
            return;
        };
        let Some(to) = self.columns.iter().position(|c| c.key == m.column) else {
            return;
        };
        let index = landed_index((from == to).then_some(at), m.index);
        let moved = self.columns[from].cards.remove(at);
        let cards = &mut self.columns[to].cards;
        cards.insert(index.min(cards.len()), moved);
    }
}

impl PageState for Board {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let rect = card(ui, "Kanban board").inset(12.0, 12.0);
        let cards: Vec<Vec<KanbanCard<'_>>> = self
            .columns
            .iter()
            .map(|c| {
                c.cards
                    .iter()
                    .map(|k| KanbanCard {
                        key: k.key,
                        title: k.title,
                        tags: k.tags,
                        assignee: k.assignee,
                        note: k.note,
                    })
                    .collect()
            })
            .collect();
        let columns: Vec<KanbanColumn<'_>> = self
            .columns
            .iter()
            .zip(&cards)
            .map(|(c, cards)| KanbanColumn {
                key: c.key,
                title: c.title,
                cards,
            })
            .collect();
        let opts = BoardOptions {
            title: "Roadmap",
            columns: &columns,
            error: self.error.then_some(&ERROR),
            active: self.active,
            glass: WINDOW,
            under: CARD.premultiplied(),
            name: "",
        };
        let r = ui.kanban_board("board", rect, &opts);
        if let Some(m) = r.moved {
            self.apply(m);
        }
        if let Some(column) = r.add_card {
            self.added += 1;
            let key = self.added;
            if let Some(c) = self.columns.iter_mut().find(|c| c.key == column) {
                c.cards.push(item(key, "New card", &[], None, false));
            }
        }
        if r.add_column {
            self.added += 1;
            self.columns.push(Col {
                key: self.added,
                title: "New column",
                cards: Vec::new(),
            });
        }
    }
}

/// The first card of the first column, near its top, in any text system.
const FIRST: (f32, f32) = (120.0, 150.0);
/// Under the middle of the second column's first card: the slot between its two cards.
const SECOND: (f32, f32) = (450.0, 205.0);

/// Over the first card, the pointer pressed, then dragged into the second column.
const DRAG: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(FIRST.0, FIRST.1)],
    },
    Step {
        at_ms: 20,
        events: &[Ev::Press],
    },
    Step {
        at_ms: 40,
        events: &[Ev::PointerAt(SECOND.0 - 40.0, SECOND.1 - 10.0)],
    },
    Step {
        at_ms: 60,
        events: &[Ev::PointerAt(SECOND.0, SECOND.1)],
    },
];

const DROP: &[Step] = &[
    DRAG[0],
    DRAG[1],
    DRAG[2],
    DRAG[3],
    Step {
        at_ms: 400,
        events: &[Ev::Release],
    },
];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "kanban-board-columns",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "kanban-board-dragging",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: DRAG,
        capture_ms: 500,
        expect: Expect::Settled,
    },
    UiScene {
        id: "kanban-board-error",
        scale: 2,
        tier: MotionTier::Full,
        state: errored,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Four stops in: the add-column button, the first column's two buttons, then the cards.
    UiScene {
        id: "kanban-board-focus",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::Tab, Ev::Tab, Ev::Tab, Ev::Tab],
        }],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    UiScene {
        id: "kanban-board-inactive",
        scale: 2,
        tier: MotionTier::Full,
        state: inactive,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "kanban-board-empty",
        scale: 2,
        tier: MotionTier::Full,
        state: empty,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The landing is `fast` (180 ms): half way at 90 ms after the release.
    UiScene {
        id: "kanban-board-settle-mid",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: DROP,
        capture_ms: 490,
        expect: Expect::Moving,
    },
];

/// What the bench replays: a drag across two columns, a drop, a keyboard move and a walk.
const BENCH: &[Step] = &[
    DRAG[0],
    DRAG[1],
    DRAG[2],
    DRAG[3],
    Step {
        at_ms: 300,
        events: &[Ev::PointerAt(SECOND.0 + 280.0, SECOND.1)],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Release],
    },
    Step {
        at_ms: 900,
        events: &[Ev::Tab, Ev::Tab, Ev::Tab, Ev::Tab],
    },
    Step {
        at_ms: 1100,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 1300,
        events: &[Ev::Key(NamedKey::ArrowRight)],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::{Role, State};
    use lune_ui::color::premul;
    use lune_ui::input::FrameOutput;
    use lune_ui::lunegit_dark::THEME;
    use lune_ui::scene::{LayerId, Scene};
    use lune_ui::testing::FakeText;

    fn run(id: &str) -> (Scene, FrameOutput) {
        let scene = SCENES.iter().find(|s| s.id == id).expect("a scene");
        run_scene(PAGE.id, scene, &mut FakeText).expect("runs")
    }

    fn card_x(out: &FrameOutput, name: &str) -> f32 {
        let node = out.tree.nodes.iter().find(|n| n.name == name);
        node.expect("a card").bounds.x
    }

    #[test]
    fn the_scenes_show_what_they_are_named_for() {
        let items = |out: &FrameOutput| {
            let n = out.tree.nodes.iter();
            n.filter(|n| n.role == Role::ListItem).count()
        };
        let (_, out) = run("kanban-board-columns");
        assert_eq!(items(&out), 6);
        let groups = out.tree.nodes.iter().filter(|n| n.role == Role::Group);
        assert_eq!(groups.count(), 3);

        let (scene, out) = run("kanban-board-dragging");
        let source = out
            .tree
            .nodes
            .iter()
            .find(|n| n.name == "Sketch the release notes");
        assert!(source.expect("a card").states.contains(State::Dragging));
        assert_eq!(items(&out), 6);
        let ghost = &scene.layers[LayerId::Popup as usize].geometry.under;
        assert!(!ghost.is_empty(), "the ghost is above the board");
        let line = scene.layers[LayerId::Base as usize].geometry.under.iter();
        let accent = premul(THEME.accent);
        assert_eq!(
            line.filter(|s| s.color == accent).count(),
            1,
            "the drop line"
        );

        let (_, out) = run("kanban-board-error");
        assert!(out.tree.nodes.iter().any(|n| n.role == Role::Alert));
        assert_eq!(items(&out), 0);

        let (_, out) = run("kanban-board-focus");
        let focused = out.focused.and_then(|f| out.tree.get(f)).expect("a focus");
        assert_eq!(focused.name, "Sketch the release notes");

        let (scene, _) = run("kanban-board-inactive");
        let shapes = scene.layers[LayerId::Base as usize].geometry.under.iter();
        let fallback = premul(THEME.window_fallback);
        assert_eq!(shapes.filter(|s| s.color == fallback).count(), 1);

        let (_, out) = run("kanban-board-empty");
        assert_eq!(items(&out), 0);
        let hints = out.tree.nodes.iter().filter(|n| n.name == "No cards yet.");
        assert_eq!(hints.count(), 3);

        let (scene, out) = run("kanban-board-settle-mid");
        let second = card_x(&out, "Keyboard moves");
        assert_eq!(
            card_x(&out, "Sketch the release notes"),
            second,
            "the model moved"
        );
        // Device px at scale 2; it left at the pointer's place, the grab point behind it.
        let ghost = scene.layers[LayerId::Popup as usize].geometry.under[0].rect[0] / 2.0;
        let left_at = SECOND.0 - (FIRST.0 - 45.0);
        assert!(ghost > second && ghost < left_at, "half way: {ghost}");
    }

    #[test]
    fn the_board_applies_a_move_as_the_widget_counts_it() {
        let mut b = Board {
            columns: model(),
            error: false,
            active: true,
            added: 100,
        };
        let order = |b: &Board, c: usize| -> Vec<u64> {
            b.columns[c].cards.iter().map(|k| k.key).collect()
        };
        b.apply(CardMove {
            card: 11,
            column: 2,
            index: 2,
        });
        assert_eq!(order(&b, 0), [12, 13]);
        assert_eq!(order(&b, 1), [21, 22, 11]);
        // Down its own column: index 2 as it stands, past the card's old place.
        b.apply(CardMove {
            card: 21,
            column: 2,
            index: 2,
        });
        assert_eq!(order(&b, 1), [22, 21, 11]);
    }
}
