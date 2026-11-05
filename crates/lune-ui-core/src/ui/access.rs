//! Semantic nodes (emission into the frame's tree) and accessibility action routing (D14): each
//! action becomes the pointer or key input the widget already handles.

use std::collections::{BTreeMap, BTreeSet};

use crate::access::{AccessAction, AccessValue, Action, Actions, Node, SemanticTree, State};
use crate::input::NamedKey;

use super::keys::KeyPress;
use super::overlay::Dismissal;
use super::pointer::PointerFrame;
use super::{Ui, WidgetId};

/// Where an action lands, collected at begin.
#[derive(Debug, Default)]
pub(crate) struct Routed {
    pub(crate) keys: Vec<KeyPress>,
    pub(crate) set_values: BTreeMap<WidgetId, AccessValue>,
    pub(crate) scroll_into_view: BTreeSet<WidgetId>,
}

/// Routes one action whose target passed the modal and previous-node checks. `top` is the top
/// active overlay and whether it dismisses on Escape.
pub(crate) fn route(
    a: &AccessAction,
    node: &Node,
    top: Option<(WidgetId, bool)>,
    pointer: &mut PointerFrame,
    routed: &mut Routed,
    dismissed: &mut BTreeMap<WidgetId, Dismissal>,
) -> Option<WidgetId> {
    let t = a.target;
    let expanded = node.states.contains(State::Expanded);
    let key = |key| KeyPress::synthetic(key, t);
    match a.action {
        Action::Default => {
            pointer.clicked.insert(t);
        }
        Action::Focus => return Some(t),
        Action::Increment => routed.keys.push(key(NamedKey::ArrowUp)),
        Action::Decrement => routed.keys.push(key(NamedKey::ArrowDown)),
        Action::SetValue => {
            if let Some(v) = &a.value {
                routed.set_values.insert(t, v.clone());
            }
        }
        Action::Expand if !expanded => {
            pointer.clicked.insert(t);
        }
        Action::Collapse if expanded => {
            pointer.clicked.insert(t);
        }
        Action::Expand | Action::Collapse => {}
        Action::ScrollIntoView => {
            routed.scroll_into_view.insert(t);
        }
        Action::ShowMenu => {
            pointer.secondary_clicked.insert(t);
        }
        Action::Dismiss => {
            if let Some((id, true)) = top {
                dismissed.entry(id).or_insert(Dismissal::Access);
            }
        }
    }
    None
}

impl Ui<'_> {
    /// Appends `node` to the frame's tree: its parent is the open container, its bounds go
    /// through the transform stack, and the Ui owns its `focused` state. Ignores the node's own
    /// `parent` and `children`. Inside a disabled scope ([`Ui::with_enabled`]) the node is
    /// `Disabled`, not `Focusable`, and offers no action, whatever it asked for.
    pub fn node(&mut self, mut node: Node) {
        if !self.is_enabled() {
            node.states.insert(State::Disabled);
            node.states.remove(State::Focusable);
            node.actions = Actions::default();
        }
        node.bounds = self.painter.transformed(node.bounds);
        node.parent = self.containers.last().map(|&i| self.tree.nodes[i].id);
        node.children.clear();
        if let Some(&i) = self.containers.last() {
            self.tree.nodes[i].children.push(node.id);
        }
        let (layer, tag) = self.painter.hit_context();
        self.node_meta.insert(node.id, (layer, tag));
        self.tree.nodes.push(node);
    }

    /// Appends `node` and makes it the parent of the nodes `f` emits.
    pub fn node_container<R>(&mut self, node: Node, f: impl FnOnce(&mut Self) -> R) -> R {
        let index = self.tree.nodes.len();
        self.node(node);
        self.containers.push(index);
        let r = f(self);
        self.containers.pop();
        r
    }

    /// The previous frame's semantic tree.
    pub fn previous_tree(&self) -> &SemanticTree {
        &self.store.prev_tree
    }
}

/// Sets each node's `focused` state from the frame's final focus.
pub(crate) fn mark_focus(tree: &mut SemanticTree, focused: Option<WidgetId>) {
    for n in &mut tree.nodes {
        if Some(n.id) == focused {
            n.states.insert(State::Focused);
        } else {
            n.states.remove(State::Focused);
        }
    }
}
