//! A `Ui` test bench: one window's retained store, a fake text system, the default strings and a
//! scripted input, run one frame at a time.

use std::time::Duration;

use crate::access::{Actions, Role, SemanticTree, State};
use crate::input::{FrameOutput, Keymap};
use crate::motion::{Cadence, MotionTier};
use crate::paint::Rect;
use crate::scene::{LAYER_COUNT, LayerId, Scene};
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

/// One hit region of the last finished frame, as a test reads it: where it is, which layer and
/// overlay it was registered on, and whether it takes input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitRegion {
    /// The widget that registered the region.
    pub id: WidgetId,
    /// The region, in window coordinates.
    pub rect: Rect,
    /// The fixed layer the region was registered on.
    pub layer: LayerId,
    /// The overlay it belongs to (the tag [`Ui::overlay`] gave it); 0 for the page, below every
    /// overlay of its layer.
    pub tag: u32,
    /// The region takes input: false inside a disabled scope.
    pub enabled: bool,
}

/// The hit regions of the last frame `store` finished, in draw order. Regions of an overlay that is
/// only closing are not among them. A projection: the store keeps its own records.
pub fn hit_regions(store: &UiStore) -> Vec<HitRegion> {
    const LAYERS: [LayerId; LAYER_COUNT] = [
        LayerId::Base,
        LayerId::Page,
        LayerId::Popup,
        LayerId::Modal,
        LayerId::Raised,
        LayerId::Outside,
        LayerId::Toast,
    ];
    store
        .prev_hits
        .iter()
        .map(|h| HitRegion {
            id: h.id,
            rect: h.rect,
            layer: LAYERS.get(h.layer).copied().unwrap_or(LayerId::Toast),
            tag: h.tag,
            enabled: h.enabled,
        })
        .collect()
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
    /// How often a busy loop asks for its next frame; [`Cadence::BUSY`] by default.
    pub busy_cadence: Cadence,
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
            busy_cadence: Cadence::BUSY,
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
            busy_cadence: self.busy_cadence,
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

    #[test]
    fn hit_regions_lists_the_page_button_then_the_dialog_in_draw_order() {
        use crate::widgets::button::ButtonKind;
        use crate::widgets::dialog::{Cancel, Dialog, DialogGlass};
        let mut h = UiHarness::new((400.0, 300.0));
        let at = Rect::new(10.0, 10.0, 80.0, 30.0);
        let dialog = Dialog {
            title: "Title",
            message: "Message",
            extra_h: 0.0,
            confirm: "Yes",
            cancel: Cancel::Default,
        };
        h.frame(|ui| {
            ui.button("page", at, "page", ButtonKind::Secondary);
            ui.dialog("d", &dialog, &DialogGlass::of(&DARK), |_, _, _, _| {});
        });
        let regions = hit_regions(&h.store);
        // The button, then the dialog's own: its window-wide block, the card and its buttons.
        let first = regions[0];
        assert_eq!((first.rect, first.layer, first.tag), (at, LayerId::Base, 0));
        assert!(first.enabled);
        assert!(regions.len() >= 3, "{regions:?}");
        assert!(
            regions[1..]
                .iter()
                .all(|r| r.layer == LayerId::Modal && r.tag > 0 && r.enabled),
            "{regions:?}"
        );
        // The dialog's regions follow the page button, all on its overlay.
        assert!(regions[1].rect.w >= 400.0, "{:?}", regions[1]);
    }

    #[test]
    fn hit_regions_marks_a_disabled_scope_and_keeps_nothing_from_an_older_frame() {
        use crate::widgets::button::ButtonKind;
        let mut h = UiHarness::new((400.0, 300.0));
        let at = Rect::new(10.0, 10.0, 80.0, 30.0);
        h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                ui.button("off", at, "off", ButtonKind::Secondary);
            });
        });
        let regions = hit_regions(&h.store);
        assert_eq!(regions.len(), 1);
        assert!(!regions[0].enabled);
        h.frame(|_| {});
        assert!(hit_regions(&h.store).is_empty());
    }
}
