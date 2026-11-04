//! The `kv-list` page: the default list, a mono list, the stacked and wrap modes, and a value
//! selected for copy (D125).

use lune_ui::input::Modifiers;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::kv_list::{KvMode, KvOptions};

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "kv-list",
    title: "Key-value list",
    kind: PageKind::Ui(UiPage {
        state: all,
        scenes: SCENES,
        bench: &[],
    }),
};

const PAIRS: [(&str, &str); 6] = [
    ("Repository", "luneui-rs"),
    ("Branch", "orch/toolkit-v1"),
    ("Commit", "3f9a2c1d8e7b4a6059c2d1e8f7a3b6c4d5e60718"),
    ("Remote", "git@example.com:lune/luneui-rs.git"),
    (
        "Working tree",
        "/Users/lune/code/luneui-rs/.claude/worktrees/orch-toolkit-v1-m6a/crates",
    ),
    ("Status", "Clean"),
];

const SHORT: [(&str, &str); 4] = [
    ("Model", "qwen2.5-coder"),
    ("Id", "b7e1-44f0-9a12"),
    ("Context", "32,768 tokens"),
    ("Quantisation", "Q4_K_M"),
];

const NOTES: [(&str, &str); 2] = [
    (
        "Probe note",
        "The probe ran against the local model with a 4k prompt and a 512 token reply; the speed is the median of five runs on this machine.",
    ),
    ("Verdict", "Fits"),
];

/// A page state that draws its sections with one function.
struct Sections(&'static [fn(&mut Ui<'_>, f32, f32) -> f32]);

impl PageState for Sections {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Key-value list");
        let (x, mut y) = (card.x + 24.0, card.y + 24.0);
        for section in self.0 {
            y += section(ui, x, y) + 28.0;
        }
    }
}

fn default_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "DEFAULT: THE LONGEST KEY SIZES THE COLUMN", x, y);
    20.0 + ui.kv_list(
        "default",
        Rect::new(x, y + 20.0, 560.0, 0.0),
        &PAIRS,
        &KvOptions::default(),
    )
}

fn mono_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "MONO VALUES", x, y);
    let opts = KvOptions {
        mono: true,
        ..KvOptions::default()
    };
    20.0 + ui.kv_list("mono", Rect::new(x, y + 20.0, 560.0, 0.0), &SHORT, &opts)
}

fn stacked_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "STACKED", x, y);
    let opts = KvOptions {
        mode: KvMode::Stacked,
        ..KvOptions::default()
    };
    20.0 + ui.kv_list("stacked", Rect::new(x, y + 20.0, 560.0, 0.0), &SHORT, &opts)
}

fn wrap_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "WRAP", x, y);
    let opts = KvOptions {
        mode: KvMode::Wrap,
        ..KvOptions::default()
    };
    20.0 + ui.kv_list("wrap", Rect::new(x, y + 20.0, 440.0, 0.0), &NOTES, &opts)
}

fn all() -> Box<dyn PageState> {
    Box::new(Sections(&[
        default_section,
        mono_section,
        stacked_section,
        wrap_section,
    ]))
}

fn default() -> Box<dyn PageState> {
    Box::new(Sections(&[default_section]))
}

fn stacked() -> Box<dyn PageState> {
    Box::new(Sections(&[stacked_section]))
}

fn wrap() -> Box<dyn PageState> {
    Box::new(Sections(&[wrap_section]))
}

/// The scenes run on the Mac keymap, where the primary modifier is Command.
const COMMAND: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: true,
};

const SCENES: &[UiScene] = &[
    UiScene {
        id: "kv-list-default",
        scale: 2,
        tier: MotionTier::Full,
        state: default,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "kv-list-stacked",
        scale: 2,
        tier: MotionTier::Full,
        state: stacked,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "kv-list-wrap",
        scale: 2,
        tier: MotionTier::Full,
        state: wrap,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // A press on the commit focuses it (a value is no tab stop, so Tab cannot); Command-A then
    // selects the whole value, which is elided: the fill covers what is drawn.
    UiScene {
        id: "kv-list-selected",
        scale: 2,
        tier: MotionTier::Full,
        state: default,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(&["default", "2", "value"]), Ev::Click],
            },
            Step {
                at_ms: 50,
                events: &[Ev::KeyChar("a", COMMAND)],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::testing::FakeText;
    use lune_ui::ui::WidgetId;

    #[test]
    fn the_selected_scene_selects_the_whole_commit_and_it_is_focused() {
        let scene = SCENES
            .iter()
            .find(|s| s.id == "kv-list-selected")
            .expect("scene");
        let (_, out) = run_scene(PAGE.id, scene, &mut FakeText).expect("runs");
        let id = WidgetId::ROOT
            .with(PAGE.id)
            .with("default")
            .with("2")
            .with("value");
        let node = out.tree.get(id).expect("the commit's value");
        let text = node.text.as_ref().expect("text");
        assert_eq!(text.text, PAIRS[2].1);
        assert_eq!(text.selection, (0, PAIRS[2].1.chars().count()));
        assert!(
            node.states.contains(lune_ui::access::State::Focused),
            "a press focused it"
        );
    }
}
