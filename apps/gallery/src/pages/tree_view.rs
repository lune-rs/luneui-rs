//! The `tree-view` page: a small file tree with guides, a size in the right-hand cell of each file,
//! the keys and the pointer live (`Ui::tree_view`). What was last activated is named under it.
//!
//! The scenes freeze three depths with their guides (at both scales), one row hovered, one folder
//! open, a selected row, the cursor's focus ring, and the expand half way through.

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::tree_view::{ROW, TreeOptions, TreeRow, TreeRowCtx};

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "tree-view",
    title: "Tree view",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

const fn dir(key: u64, depth: u16, text: &'static str) -> TreeRow<'static> {
    TreeRow {
        key,
        depth,
        text,
        glyph: None,
        has_children: true,
    }
}

const fn file(key: u64, depth: u16, text: &'static str) -> TreeRow<'static> {
    TreeRow {
        key,
        depth,
        text,
        glyph: None,
        has_children: false,
    }
}

const SRC: u64 = 1;
const GPU: u64 = 2;
const SHADERS: u64 = 4;
const WIDGETS: u64 = 8;
const DOCS: u64 = 11;
const ASSETS: u64 = 13;

const ROWS: [TreeRow<'static>; 15] = [
    dir(SRC, 0, "src"),
    dir(GPU, 1, "gpu"),
    file(3, 2, "render.rs"),
    dir(SHADERS, 2, "shaders"),
    file(5, 3, "blur.wgsl"),
    file(6, 3, "glass.wgsl"),
    file(7, 2, "text.rs"),
    dir(WIDGETS, 1, "widgets"),
    file(9, 2, "tree.rs"),
    file(10, 1, "main.rs"),
    dir(DOCS, 0, "docs"),
    TreeRow {
        glyph: Some(icon::EDIT),
        ..file(12, 1, "notes.md")
    },
    dir(ASSETS, 0, "assets"),
    file(14, 1, "logo.png"),
    TreeRow {
        glyph: Some(icon::INFO),
        ..file(15, 0, "README.md")
    },
];

/// The tree's rect in the 1040 x 660 window: the card's content, under the heading.
const TREE: Rect = Rect::new(34.0, 78.0, 460.0, 16.0 * ROW);

/// The page: which rows it opens and selects on its first frame, then what the tree reports.
struct Live {
    seed: Option<(&'static [u64], Option<u64>)>,
    /// The text of the last row activated.
    activated: Option<&'static str>,
    /// Draw the tree inside `with_enabled(false, ..)`.
    disabled: bool,
}

fn state(open: &'static [u64], selected: Option<u64>) -> Box<dyn PageState> {
    Box::new(Live {
        seed: Some((open, selected)),
        activated: None,
        disabled: false,
    })
}

/// The selected tree, disabled.
fn disabled() -> Box<dyn PageState> {
    Box::new(Live {
        seed: Some((&[SRC, GPU], Some(GPU))),
        activated: None,
        disabled: true,
    })
}

fn live() -> Box<dyn PageState> {
    state(&[SRC, GPU, DOCS], None)
}

fn all_open() -> Box<dyn PageState> {
    state(&[SRC, GPU, SHADERS, WIDGETS, DOCS], None)
}

fn closed() -> Box<dyn PageState> {
    state(&[], None)
}

fn one_open() -> Box<dyn PageState> {
    state(&[SRC], None)
}

fn selected() -> Box<dyn PageState> {
    state(&[SRC, GPU], Some(GPU))
}

/// A file's right-hand cell: its size, right aligned.
fn size_cell(ui: &mut Ui<'_>, ctx: &TreeRowCtx) {
    if ROWS[ctx.row].has_children {
        return;
    }
    let text = format!("{}.{} KB", ctx.key + 1, ctx.key % 10);
    let spec = Spec::plain(text, Style::mono(11.0, 14.0, THEME.fg_tertiary));
    let w = ui.painter().measure(&spec).0;
    let r = Rect::new(ctx.rect.right() - 12.0 - w, ctx.rect.y, w, ctx.rect.h);
    ui.painter().text_in(&spec, r, 0.0);
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let c = card(ui, "Tree view");
        let (x, y) = (c.x + 24.0, c.y + 24.0);
        heading(ui, "A FILE TREE", x, y);
        if let Some((open, selected)) = self.seed.take() {
            let st = ui.tree_state("tree");
            st.expanded = open.iter().copied().collect();
            st.selected = selected.into_iter().collect();
        }
        let opts = TreeOptions {
            rows: &ROWS,
            label: "Files",
            guides: true,
            toggle_on_row: false,
        };
        let out = ui.with_enabled(!self.disabled, |ui| {
            ui.tree_view("tree", TREE, &opts, size_cell)
        });
        if let Some(key) = out.activated {
            self.activated = ROWS.iter().find(|r| r.key == key).map(|r| r.text);
        }
        let said = match self.activated {
            Some(text) => format!("Opened: {text}"),
            None => "Enter or a double click opens a row.".to_owned(),
        };
        caption(ui, &said, TREE.right() + 24.0, y + 20.0);
        caption(
            ui,
            "Arrows move and fold, Space selects.",
            TREE.right() + 24.0,
            y + 40.0,
        );
    }
}

const fn settled(id: &'static str, scale: u32, state: fn() -> Box<dyn PageState>) -> UiScene {
    UiScene {
        id,
        scale: scale as f32,
        tier: MotionTier::Full,
        state,
        script: &[],
        capture_ms: 100,
        expect: Expect::Settled,
    }
}

/// A point on the text of the tree's second row.
const SECOND_ROW: (f32, f32) = (TREE.x + 200.0, TREE.y + ROW + 14.0);

const SCENES: &[UiScene] = &[
    settled("tree-view-depth", 2, all_open),
    settled("tree-view-depth", 1, all_open),
    UiScene {
        id: "tree-view-hover",
        scale: 2.0,
        tier: MotionTier::Full,
        state: one_open,
        script: &[Step {
            at_ms: 100,
            events: &[Ev::PointerAt(SECOND_ROW.0, SECOND_ROW.1)],
        }],
        capture_ms: 300,
        expect: Expect::Settled,
    },
    settled("tree-view-expanded", 2, one_open),
    // The expand is base (250 ms): half way 125 ms after the key.
    UiScene {
        id: "tree-view-expand-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: closed,
        script: &[
            Step {
                at_ms: 100,
                events: &[Ev::Tab],
            },
            Step {
                at_ms: 200,
                events: &[Ev::Key(NamedKey::ArrowRight)],
            },
        ],
        capture_ms: 325,
        expect: Expect::Moving,
    },
    settled("tree-view-selected", 2, selected),
    // The scope is disabled (0.4 opacity): the pointer on a row draws no hover, Tab and Down
    // find no stop, and the selected row keeps its fill.
    UiScene {
        id: "tree-view-disabled",
        scale: 2.0,
        tier: MotionTier::Full,
        state: disabled,
        script: &[
            Step {
                at_ms: 100,
                events: &[Ev::PointerAt(SECOND_ROW.0, SECOND_ROW.1), Ev::Tab],
            },
            Step {
                at_ms: 200,
                events: &[Ev::Key(NamedKey::ArrowDown)],
            },
        ],
        capture_ms: 500,
        expect: Expect::Settled,
    },
    // Tab enters on the first row, Down moves the cursor and its ring.
    UiScene {
        id: "tree-view-focus",
        scale: 2.0,
        tier: MotionTier::Full,
        state: one_open,
        script: &[
            Step {
                at_ms: 100,
                events: &[Ev::Tab],
            },
            Step {
                at_ms: 200,
                events: &[Ev::Key(NamedKey::ArrowDown)],
            },
        ],
        capture_ms: 500,
        expect: Expect::Settled,
    },
];

/// What the bench replays: enter, walk down, fold and open a row in motion, select, activate.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Key(NamedKey::ArrowLeft)],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Key(NamedKey::ArrowRight)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Key(NamedKey::End)],
    },
    Step {
        at_ms: 1100,
        events: &[Ev::Key(NamedKey::Space)],
    },
    Step {
        at_ms: 1200,
        events: &[Ev::Key(NamedKey::Enter)],
    },
    Step {
        at_ms: 1300,
        events: &[Ev::Key(NamedKey::Home)],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::{Role, State};
    use lune_ui::input::FrameOutput;
    use lune_ui::scene::Scene;
    use lune_ui::testing::FakeText;

    fn run(id: &str, scale: u32) -> (Scene, FrameOutput) {
        let scene = SCENES
            .iter()
            .find(|s| s.id == id && s.scale == scale as f32)
            .expect("a scene");
        run_scene(PAGE.id, scene, &mut FakeText).expect("runs")
    }

    fn rows(out: &FrameOutput) -> Vec<&str> {
        out.tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::TreeItem)
            .map(|n| n.name.as_str())
            .collect()
    }

    #[test]
    fn the_rows_are_pre_order_and_their_keys_are_their_places() {
        for (i, r) in ROWS.iter().enumerate() {
            assert_eq!(r.key, i as u64 + 1);
        }
        assert!(ROWS.windows(2).all(|w| w[1].depth <= w[0].depth + 1));
    }

    #[test]
    fn the_scenes_show_what_they_are_named_for() {
        let (_, out) = run("tree-view-depth", 2);
        let deep = out.tree.nodes.iter().filter_map(|n| n.level).max();
        assert_eq!(deep, Some(4), "four levels, three depths under the top");
        assert_eq!(rows(&out).len(), 15 - 1, "the assets folder stays shut");
        let (_, one) = run("tree-view-depth", 1);
        assert_eq!(rows(&one), rows(&out));

        let (scene, _) = run("tree-view-hover", 2);
        let hover = lune_ui::color::premul(THEME.surface_hover);
        let filled = |s: &Scene| {
            s.layers
                .iter()
                .flat_map(|l| l.geometry.under.iter())
                .any(|f| f.color == hover)
        };
        assert!(filled(&scene));

        let (_, out) = run("tree-view-expanded", 2);
        assert_eq!(
            rows(&out),
            [
                "src",
                "gpu",
                "widgets",
                "main.rs",
                "docs",
                "assets",
                "README.md"
            ]
        );

        let (_, out) = run("tree-view-expand-mid", 2);
        assert_eq!(
            rows(&out),
            ["src", "docs", "assets", "README.md"],
            "no block nodes"
        );

        let (_, out) = run("tree-view-selected", 2);
        let on: Vec<_> = out
            .tree
            .nodes
            .iter()
            .filter(|n| n.states.contains(State::Selected))
            .map(|n| n.name.as_str())
            .collect();
        assert_eq!(on, ["gpu"]);

        let (_, out) = run("tree-view-focus", 2);
        assert!(out.focused.is_some(), "the tree holds the focus");
    }
}
