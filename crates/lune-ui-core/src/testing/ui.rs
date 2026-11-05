//! A `Ui` test bench: one window's retained store, a fake text system, the default strings and a
//! scripted input, run one frame at a time.

use std::time::Duration;

use crate::access::{Actions, Role, SemanticTree, State};
use crate::input::{FrameOutput, Keymap};
use crate::motion::MotionTier;
use crate::scene::Scene;
use crate::shortcuts::Shortcuts;
use crate::strings::{DisplayFormat, StringTable};
use crate::theme::{DARK, Theme};
use crate::ui::{FrameConfig, Ui, UiStore, WidgetId};

use super::{FakeText, ScriptedInput};

/// Every node at `root` or below it in `tree` that a disabled scope should have marked but did not:
/// one that is not `Disabled`, is `Focusable`, or offers an action.
pub fn disabled_violations(tree: &SemanticTree, root: WidgetId) -> Vec<(WidgetId, Role)> {
    let mut bad = Vec::new();
    let mut open = vec![root];
    while let Some(id) = open.pop() {
        let Some(n) = tree.get(id) else { continue };
        let good = n.states.contains(State::Disabled)
            && !n.states.contains(State::Focusable)
            && n.actions == Actions::default();
        if !good {
            bad.push((n.id, n.role));
        }
        open.extend(n.children.iter().rev());
    }
    bad
}

/// One scripted window for `Ui` tests. Script input on [`UiHarness::input`], then run
/// [`UiHarness::frame`]; the frame's input is everything scripted since the previous frame.
#[derive(Debug)]
pub struct UiHarness {
    pub store: UiStore,
    pub text: FakeText,
    pub strings: StringTable,
    pub input: ScriptedInput,
    pub theme: Theme,
    pub tier: MotionTier,
    /// The keyboard conventions the frames run with; macOS by default.
    pub keymap: Keymap,
    /// The shortcut table the frames run with; empty by default.
    pub shortcuts: Shortcuts,
}

impl UiHarness {
    /// A focused window of logical `size` at scale 1, the dark preset, the Full tier and an empty
    /// string table.
    pub fn new(size: (f32, f32)) -> Self {
        let mut input = ScriptedInput::new(size, 1.0);
        input.focus(true);
        Self {
            store: UiStore::default(),
            text: FakeText,
            strings: StringTable::default(),
            input,
            theme: DARK,
            tier: MotionTier::Full,
            keymap: Keymap::MAC,
            shortcuts: Shortcuts::default(),
        }
    }

    /// Sets the time of the following frames, in milliseconds of app time.
    pub fn at_ms(&mut self, ms: u64) -> &mut Self {
        self.input.set_time(Duration::from_millis(ms));
        self
    }

    /// Runs one frame: `f` builds it, the result is what `Ui::finish` returns.
    pub fn frame(&mut self, f: impl FnOnce(&mut Ui<'_>)) -> (Scene, FrameOutput) {
        let raw = self.input.end_frame();
        let config = FrameConfig {
            theme: self.theme,
            tier: self.tier,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: self.keymap,
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &raw, config, &mut self.text);
        f(&mut ui);
        ui.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::{Action, Node};
    use crate::paint::Rect;

    #[test]
    fn disabled_violations_names_the_bad_nodes_under_the_root_only() {
        let mut h = UiHarness::new((100.0, 100.0));
        let (root, good, bad, acts, outside) = (
            WidgetId::ROOT.with("root"),
            WidgetId::ROOT.with("good"),
            WidgetId::ROOT.with("bad"),
            WidgetId::ROOT.with("acts"),
            WidgetId::ROOT.with("outside"),
        );
        let at = Rect::new(0.0, 0.0, 10.0, 10.0);
        let (_, out) = h.frame(|ui| {
            let disabled = |id, role| {
                let mut n = Node::new(id, role, "n", at);
                n.states.insert(State::Disabled);
                n
            };
            ui.node_container(disabled(root, Role::Group), |ui| {
                ui.node(disabled(good, Role::Button));
                ui.node(Node::new(bad, Role::Checkbox, "n", at));
                let mut n = disabled(acts, Role::Switch);
                n.actions.insert(Action::Default);
                ui.node(n);
            });
            ui.node(Node::new(outside, Role::Button, "n", at));
        });
        assert_eq!(
            disabled_violations(&out.tree, root),
            [(bad, Role::Checkbox), (acts, Role::Switch)]
        );
    }
}
