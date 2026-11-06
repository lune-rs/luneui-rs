//! The AccessKit bridge, behind the `accesskit` feature: the frame's semantic tree goes out as an
//! AccessKit tree, and the actions an assistive client sends come back as [`AccessAction`]s.
//!
//! [`tree_update`] and [`action_from`] are pure, so nearly all of the bridge tests with no window.
//! [`AccessBridge`] owns the OS adapter. AccessKit calls its handlers on a thread of its choosing,
//! so they touch only [`Shared`], never the `Ui`, the window or the [`Translator`]: the frame reads
//! `Shared` on the app's thread, in [`AccessBridge::drain`] and [`AccessBridge::update`]. A handler
//! holds the queue's lock only to push; `wake` runs with no lock held.
//!
//! Every value here comes from outside this module and is trusted in no way. The tree is a public
//! struct, so `tree_update` assumes nothing about its shape (AccessKit panics on a malformed
//! update, and an inconsistent text run can panic it later, when a client reads it). An action
//! comes from another process, so `action_from` refuses what it cannot map and the queue is capped.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use accesskit::{
    Action as AkAction, ActionData, ActionHandler, ActionRequest, ActivationHandler, Affine,
    AriaCurrent, DeactivationHandler, Invalid, Live as AkLive, Node as AkNode, NodeId,
    Rect as AkRect, Role as AkRole, SortDirection, TextPosition, TextSelection, Toggled, TreeId,
    TreeInfo, TreeUpdate,
};
use accesskit_winit::Adapter;
use lune_ui_core::access::{
    AccessAction, AccessValue, Action, Live, Node, Role, SemanticTree, Sort, State, TextValue,
    Value,
};
use lune_ui_core::input::FrameOutput;
use lune_ui_core::paint::Rect;
use lune_ui_core::ui::WidgetId;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use crate::clipboard::MAX_PASTE_BYTES;
use crate::input::Translator;

/// The synthetic window node. `WidgetId::ROOT` is 0, so this is not it; a tree node that has this
/// id anyway is left out.
const WINDOW_NODE: NodeId = NodeId(u64::MAX);

/// The most actions waiting for a frame; beyond it new ones drop.
const MAX_QUEUED: usize = 256;

/// What the tree's root says about the window.
#[derive(Clone, PartialEq, Debug)]
pub struct RootInfo {
    /// The window's title.
    pub title: String,
    /// The window's scale factor: the root's transform, so `bounds` stay logical.
    pub scale: f64,
}

/// What the last update sent, so the next sends only what changed. Empty before the first update
/// and after [`NodeCache::clear`]; an update from empty is a full tree.
#[derive(Clone, Debug, Default)]
pub struct NodeCache {
    nodes: HashMap<NodeId, AkNode>,
}

impl NodeCache {
    /// Forgets what was sent: the next update is a full tree.
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

/// The nodes of a tree that go out and how they connect, as indices into `tree.nodes`. Every id
/// appears once: the first node with an id is the node, as in `SemanticTree::get`.
struct Shape<'a> {
    tree: &'a SemanticTree,
    index: HashMap<WidgetId, usize>,
    claimed: Vec<bool>,
    /// The nodes under the window.
    top: Vec<usize>,
    /// The children of each node.
    kids: Vec<Vec<usize>>,
    /// Every claimed node, parents before children.
    order: Vec<usize>,
}

impl<'a> Shape<'a> {
    /// Walks `tree` from its parentless nodes, down each node's `children`. What that leaves out (a
    /// cycle, a node its parent does not list) goes under its parent if that was reached, else
    /// under the window. No recursion: a node's depth is not bounded.
    fn of(tree: &'a SemanticTree) -> Self {
        let mut index = HashMap::with_capacity(tree.nodes.len());
        for (i, n) in tree.nodes.iter().enumerate() {
            if n.id.value() != WINDOW_NODE.0 {
                index.entry(n.id).or_insert(i);
            }
        }
        let mut shape = Shape {
            tree,
            index,
            claimed: vec![false; tree.nodes.len()],
            top: Vec::new(),
            kids: vec![Vec::new(); tree.nodes.len()],
            order: Vec::with_capacity(tree.nodes.len()),
        };
        for (i, n) in tree.nodes.iter().enumerate() {
            let real = shape.index.get(&n.id) == Some(&i);
            let parent = n.parent.and_then(|p| shape.index.get(&p));
            if real && parent.is_none() && !shape.claimed[i] {
                shape.claim(None, i);
            }
        }
        for (i, n) in tree.nodes.iter().enumerate() {
            if shape.index.get(&n.id) == Some(&i) && !shape.claimed[i] {
                let parent = n.parent.and_then(|p| shape.index.get(&p)).copied();
                shape.claim(parent.filter(|&p| shape.claimed[p]), i);
            }
        }
        shape
    }

    /// Puts node `i` under `parent` (the window for `None`), and everything it reaches.
    fn claim(&mut self, parent: Option<usize>, i: usize) {
        self.claimed[i] = true;
        match parent {
            Some(p) => self.kids[p].push(i),
            None => self.top.push(i),
        }
        let mut open = vec![i];
        while let Some(at) = open.pop() {
            self.order.push(at);
            for id in &self.tree.nodes[at].children {
                if let Some(&child) = self.index.get(id)
                    && !self.claimed[child]
                {
                    self.claimed[child] = true;
                    self.kids[at].push(child);
                    open.push(child);
                }
            }
        }
    }
}

fn node_id(id: WidgetId) -> NodeId {
    NodeId(id.value())
}

/// The update that takes AccessKit from what `prev` holds to this frame's tree. From an empty
/// `prev` it is a full tree; otherwise it carries the nodes that changed, and a child that went is
/// told by its parent's `children`. Focus falls back to the window for an id the tree lacks.
pub fn tree_update(
    tree: &SemanticTree,
    focused: Option<WidgetId>,
    root: &RootInfo,
    prev: &mut NodeCache,
) -> TreeUpdate {
    let shape = Shape::of(tree);
    let mut used: HashSet<NodeId> = shape.index.keys().map(|&id| node_id(id)).collect();
    used.insert(WINDOW_NODE);

    let mut built = Vec::with_capacity(tree.nodes.len() + 1);
    built.push((WINDOW_NODE, window_node(root, &shape)));
    for &i in &shape.order {
        let n = &tree.nodes[i];
        let mut node = mapped(n);
        let mut children: Vec<NodeId> = shape.kids[i]
            .iter()
            .map(|&k| node_id(tree.nodes[k].id))
            .collect();
        let mut runs = Vec::new();
        if let Some(text) = &n.text {
            let (nodes, selection) = text_runs(n, text, &mut used);
            children.extend(nodes.iter().map(|(id, _)| *id));
            runs = nodes;
            if let Some(selection) = selection {
                node.set_text_selection(selection);
            }
        }
        if !children.is_empty() {
            node.set_children(children);
        }
        built.push((node_id(n.id), node));
        built.append(&mut runs);
    }

    let first = prev.nodes.is_empty();
    let mut nodes = Vec::new();
    let mut sent = HashMap::with_capacity(built.len());
    for (id, node) in built {
        if prev.nodes.get(&id) != Some(&node) {
            nodes.push((id, node.clone()));
        }
        sent.insert(id, node);
    }
    prev.nodes = sent;

    let focus = focused
        .filter(|id| shape.index.contains_key(id))
        .map_or(WINDOW_NODE, node_id);
    TreeUpdate {
        nodes,
        tree: first.then(|| TreeInfo::new(WINDOW_NODE)),
        tree_id: TreeId::ROOT,
        focus,
    }
}

/// The synthetic root: its transform is the scale, and a scale of 1 or one that means nothing
/// leaves it out.
fn window_node(root: &RootInfo, shape: &Shape<'_>) -> AkNode {
    let mut node = AkNode::new(AkRole::Window);
    node.set_label(root.title.as_str());
    if root.scale.is_finite() && root.scale > 0.0 && root.scale != 1.0 {
        node.set_transform(Affine::scale(root.scale));
    }
    if !shape.top.is_empty() {
        let ids = shape.top.iter().map(|&i| node_id(shape.tree.nodes[i].id));
        node.set_children(ids.collect::<Vec<_>>());
    }
    node
}

/// The AccessKit role of `n`. A `StaticText` with a level is a heading.
fn role_of(n: &Node) -> AkRole {
    match n.role {
        Role::Button => AkRole::Button,
        Role::Checkbox => AkRole::CheckBox,
        Role::Switch => AkRole::Switch,
        Role::Radio => AkRole::RadioButton,
        Role::RadioGroup => AkRole::RadioGroup,
        Role::Slider => AkRole::Slider,
        Role::SpinButton => AkRole::SpinButton,
        Role::TextInput => AkRole::TextInput,
        Role::MultilineText => AkRole::MultilineTextInput,
        Role::ComboBox => AkRole::ComboBox,
        Role::List => AkRole::List,
        Role::ListItem => AkRole::ListItem,
        Role::Tree => AkRole::Tree,
        Role::TreeItem => AkRole::TreeItem,
        Role::TabList => AkRole::TabList,
        Role::Tab => AkRole::Tab,
        Role::Menu => AkRole::Menu,
        Role::MenuItem => AkRole::MenuItem,
        Role::Dialog => AkRole::Dialog,
        Role::Alert => AkRole::Alert,
        Role::Tooltip => AkRole::Tooltip,
        Role::Table => AkRole::Table,
        Role::Row => AkRole::Row,
        Role::Cell => AkRole::Cell,
        Role::ColumnHeader => AkRole::ColumnHeader,
        Role::Progress => AkRole::ProgressIndicator,
        Role::ScrollArea => AkRole::ScrollView,
        Role::Toolbar => AkRole::Toolbar,
        Role::Group => AkRole::Group,
        Role::Image => AkRole::Image,
        Role::Link => AkRole::Link,
        Role::StaticText if n.level.is_some() => AkRole::Heading,
        Role::StaticText => AkRole::Label,
        Role::Status => AkRole::Status,
        Role::Navigation => AkRole::Navigation,
        Role::Note => AkRole::Note,
        Role::Separator => AkRole::Splitter,
        Role::PasswordInput => AkRole::PasswordInput,
    }
}

/// `r` as an AccessKit rect, or `None` when a field is not finite: a client has no use for it.
fn rect(r: Rect) -> Option<AkRect> {
    let [x, y, w, h] = [r.x, r.y, r.w, r.h].map(f64::from);
    [x, y, w, h]
        .iter()
        .all(|v| v.is_finite())
        .then_some(AkRect {
            x0: x,
            y0: y,
            x1: x + w,
            y1: y + h,
        })
}

/// `n` as an AccessKit node, with neither children nor text runs.
fn mapped(n: &Node) -> AkNode {
    let role = role_of(n);
    let mut node = AkNode::new(role);
    let text_value = match &n.value {
        Some(Value::Text(t)) => Some(t.as_str()),
        _ => None,
    };
    // A label's text is its value: AccessKit reads no name from it.
    match text_value {
        None if role == AkRole::Label => node.set_value(n.name.as_str()),
        _ if n.name.is_empty() => {}
        _ => node.set_label(n.name.as_str()),
    }
    if let Some(t) = text_value {
        node.set_value(t);
    }
    if let Some(Value::Number {
        value,
        min,
        max,
        step,
    }) = n.value
    {
        let finite = |v: f64| Some(v).filter(|v| v.is_finite());
        if let Some(v) = finite(value) {
            node.set_numeric_value(v);
        }
        if let Some(v) = finite(min) {
            node.set_min_numeric_value(v);
        }
        if let Some(v) = finite(max) {
            node.set_max_numeric_value(v);
        }
        if let Some(v) = finite(step) {
            node.set_numeric_value_step(v);
        }
    }
    if let Some(d) = &n.description {
        node.set_description(d.as_str());
    }
    if let Some(r) = rect(n.bounds) {
        node.set_bounds(r);
    }
    match n.live {
        Some(Live::Polite) => node.set_live(AkLive::Polite),
        Some(Live::Assertive) => node.set_live(AkLive::Assertive),
        None => {}
    }
    match n.sort {
        Some(Sort::Ascending) => node.set_sort_direction(SortDirection::Ascending),
        Some(Sort::Descending) => node.set_sort_direction(SortDirection::Descending),
        Some(Sort::None) | None => {}
    }
    if let Some(level) = n.level {
        node.set_level(level as usize);
    }
    map_states(n, &mut node);
    map_actions(n, &mut node);
    node
}

/// The 15 states. `Focused` is the update's `focus`, `Focusable` the focus action. `Dragging` has
/// no AccessKit property. Nothing makes `Toggled::Mixed`: core's `Checked` is one bit.
fn map_states(n: &Node, node: &mut AkNode) {
    let has = |s| n.states.contains(s);
    if has(State::Focusable) {
        node.add_action(AkAction::Focus);
    }
    if has(State::Disabled) {
        node.set_disabled();
    }
    if has(State::Checked) || has(State::Pressed) {
        node.set_toggled(Toggled::True);
    } else if matches!(n.role, Role::Checkbox | Role::Switch | Role::Radio) {
        node.set_toggled(Toggled::False);
    }
    if has(State::Selected) {
        node.set_selected(true);
    }
    if has(State::Expanded) {
        node.set_expanded(true);
    } else if n.actions.contains(Action::Expand) || n.actions.contains(Action::Collapse) {
        node.set_expanded(false);
    }
    if has(State::Modal) {
        node.set_modal();
    }
    if has(State::ReadOnly) {
        node.set_read_only();
    }
    if has(State::Required) {
        node.set_required();
    }
    if has(State::Invalid) {
        node.set_invalid(Invalid::True);
    }
    if has(State::Busy) {
        node.set_busy();
    }
    if has(State::Hidden) {
        node.set_hidden();
    }
    if has(State::Current) {
        node.set_aria_current(AriaCurrent::True);
    }
}

/// The 10 actions. `Dismiss` has a counterpart on a tooltip only.
fn map_actions(n: &Node, node: &mut AkNode) {
    const PAIRS: [(Action, AkAction); 9] = [
        (Action::Default, AkAction::Click),
        (Action::Focus, AkAction::Focus),
        (Action::Increment, AkAction::Increment),
        (Action::Decrement, AkAction::Decrement),
        (Action::SetValue, AkAction::SetValue),
        (Action::Expand, AkAction::Expand),
        (Action::Collapse, AkAction::Collapse),
        (Action::ScrollIntoView, AkAction::ScrollIntoView),
        (Action::ShowMenu, AkAction::ShowContextMenu),
    ];
    for (ours, theirs) in PAIRS {
        if n.actions.contains(ours) {
            node.add_action(theirs);
        }
    }
    if n.role == Role::Tooltip && n.actions.contains(Action::Dismiss) {
        node.add_action(AkAction::HideTooltip);
    }
}

/// One line of a text node: its run's id, its first char and its length in chars.
struct Span {
    id: NodeId,
    start: usize,
    len: usize,
}

/// The `TextRun` children of `n`, one per `\n`-separated line, and its selection. A run's value
/// holds its line break, as AccessKit asks, and its `character_lengths` the UTF-8 length of each
/// char, whose sum is the value's length. Text with no lines still gets one empty run, for the
/// caret. A run has its node's bounds: the tree holds no per-line geometry.
///
/// A run's id is derived, so it is checked against `used`, and a line that collides has no run
/// rather than a corrupt tree.
fn text_runs(
    n: &Node,
    text: &TextValue,
    used: &mut HashSet<NodeId>,
) -> (Vec<(NodeId, AkNode)>, Option<TextSelection>) {
    let t = text.text.as_str();
    let tail = (t.is_empty() || t.ends_with('\n')).then_some("");
    let bounds = rect(n.bounds).unwrap_or(AkRect::ZERO);
    let base = n.id.with("text-run");
    let mut runs = Vec::new();
    let mut spans = Vec::new();
    let mut start = 0;
    for (line_no, line) in t.split_inclusive('\n').chain(tail).enumerate() {
        let lengths: Vec<u8> = line.chars().map(|c| c.len_utf8() as u8).collect();
        let len = lengths.len();
        let id = node_id(base.with(line_no as u64));
        if used.insert(id) {
            let mut run = AkNode::new(AkRole::TextRun);
            run.set_value(line);
            run.set_character_lengths(lengths);
            run.set_bounds(bounds);
            runs.push((id, run));
            spans.push(Span { id, start, len });
        }
        start += len;
    }
    let (anchor, focus) = text.selection;
    let selection = position(&spans, anchor)
        .zip(position(&spans, focus))
        .map(|(anchor, focus)| TextSelection { anchor, focus });
    (runs, selection)
}

/// Char offset `at` as a run and an index. An offset past the end is the end of the last run.
fn position(spans: &[Span], at: usize) -> Option<TextPosition> {
    let span = spans
        .iter()
        .find(|s| at < s.start + s.len)
        .or(spans.last())?;
    Some(TextPosition {
        node: span.id,
        character_index: at.saturating_sub(span.start).min(span.len),
    })
}

/// The action an assistive client asked for, or `None` for anything with no meaning here: an
/// action core does not have, another tree, a `SetValue` with no usable value. A node id passes
/// unchecked, as core ignores a target it cannot find.
pub fn action_from(req: &ActionRequest) -> Option<AccessAction> {
    if req.target_tree != TreeId::ROOT {
        return None;
    }
    let (action, value) = match req.action {
        AkAction::Click => (Action::Default, None),
        AkAction::Focus => (Action::Focus, None),
        AkAction::Increment => (Action::Increment, None),
        AkAction::Decrement => (Action::Decrement, None),
        AkAction::Expand => (Action::Expand, None),
        AkAction::Collapse => (Action::Collapse, None),
        AkAction::ScrollIntoView => (Action::ScrollIntoView, None),
        AkAction::ShowContextMenu => (Action::ShowMenu, None),
        AkAction::HideTooltip => (Action::Dismiss, None),
        AkAction::SetValue => (Action::SetValue, Some(set_value(req.data.as_ref())?)),
        AkAction::Blur
        | AkAction::CustomAction
        | AkAction::ReplaceSelectedText
        | AkAction::ScrollDown
        | AkAction::ScrollLeft
        | AkAction::ScrollRight
        | AkAction::ScrollUp
        | AkAction::ScrollToPoint
        | AkAction::SetScrollOffset
        | AkAction::SetTextSelection
        | AkAction::SetSequentialFocusNavigationStartingPoint
        | AkAction::ShowTooltip => return None,
    };
    Some(AccessAction {
        target: WidgetId::from_value(req.target_node.0),
        action,
        value,
    })
}

/// The value of a `SetValue`: text within the paste limit (it is outside text, as a paste is), or a
/// finite number.
fn set_value(data: Option<&ActionData>) -> Option<AccessValue> {
    match data? {
        ActionData::Value(text) if text.len() <= MAX_PASTE_BYTES => {
            Some(AccessValue::Text(text.to_string()))
        }
        ActionData::NumericValue(v) if v.is_finite() => Some(AccessValue::Number(*v)),
        _ => None,
    }
}

/// What the handlers share with the frame.
struct Shared {
    /// The next update must be a full tree.
    full: AtomicBool,
    queue: Mutex<VecDeque<AccessAction>>,
    wake: Box<dyn Fn() + Send + Sync>,
}

impl Shared {
    fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            full: AtomicBool::new(false),
            queue: Mutex::new(VecDeque::new()),
            wake: Box::new(wake),
        }
    }

    /// The queue. A panic elsewhere cannot leave it half written: only `push_back` and `take` run
    /// under the lock, so a poisoned one is still good.
    fn queue(&self) -> MutexGuard<'_, VecDeque<AccessAction>> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// An assistive client wants the whole tree. AccessKit may ask again with no deactivation, and
    /// each time needs a full tree; it must come even on an idle window, hence the wake.
    fn activated(&self) {
        self.full.store(true, Ordering::Release);
        (self.wake)();
    }

    /// The client left; what it was sent is gone. The next tree is for no one yet, so no wake.
    fn deactivated(&self) {
        self.full.store(true, Ordering::Release);
    }

    /// Queues `action` and wakes the app, unless the queue is full: then a wake is already owed.
    fn push(&self, action: AccessAction) {
        let pushed = {
            let mut queue = self.queue();
            let room = queue.len() < MAX_QUEUED;
            if room {
                queue.push_back(action);
            }
            room
        };
        if pushed {
            (self.wake)();
        }
    }

    /// The actions waiting, oldest first; the queue is empty after.
    fn take(&self) -> VecDeque<AccessAction> {
        std::mem::take(&mut *self.queue())
    }

    /// The update for this frame. `full` is read here, inside the adapter's closure: a request that
    /// lands after the swap stays set for the next update, which is a full tree again.
    fn next_update(&self, out: &FrameOutput, root: &RootInfo, prev: &mut NodeCache) -> TreeUpdate {
        if self.full.swap(false, Ordering::AcqRel) {
            prev.clear();
        }
        tree_update(&out.tree, out.focused, root, prev)
    }
}

/// AccessKit's three handlers. They run on AccessKit's thread: see the module note.
#[derive(Clone)]
struct Handlers(Arc<Shared>);

impl ActivationHandler for Handlers {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.0.activated();
        None
    }
}

impl ActionHandler for Handlers {
    fn do_action(&mut self, request: ActionRequest) {
        if let Some(action) = action_from(&request) {
            self.0.push(action);
        }
    }
}

impl DeactivationHandler for Handlers {
    fn deactivate_accessibility(&mut self) {
        self.0.deactivated();
    }
}

/// Why a bridge could not start.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum BridgeError {
    /// The window is already visible: create it hidden, build the bridge, then show it.
    WindowVisible,
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WindowVisible => write!(f, "the window was shown before its bridge was built"),
        }
    }
}

impl std::error::Error for BridgeError {}

/// One window's bridge to the OS accessibility API. The app calls [`AccessBridge::window_event`]
/// with every window event before it handles it, [`AccessBridge::drain`] before a frame and
/// [`AccessBridge::update`] after one. Drop it before the window.
pub struct AccessBridge {
    adapter: Adapter,
    shared: Arc<Shared>,
    prev: NodeCache,
}

impl std::fmt::Debug for AccessBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessBridge").finish_non_exhaustive()
    }
}

impl AccessBridge {
    /// A bridge for `window`, which must still be hidden. `wake` must bring a frame even on an idle
    /// window, and may run on any thread, so it should only post to the event loop.
    pub fn new(
        event_loop: &ActiveEventLoop,
        window: &Window,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, BridgeError> {
        if window.is_visible() == Some(true) {
            return Err(BridgeError::WindowVisible);
        }
        let shared = Arc::new(Shared::new(wake));
        let handlers = Handlers(shared.clone());
        let adapter = Adapter::with_direct_handlers(
            event_loop,
            window,
            handlers.clone(),
            handlers.clone(),
            handlers,
        );
        Ok(Self {
            adapter,
            shared,
            prev: NodeCache::default(),
        })
    }

    /// Lets the adapter see a window event: focus, for one.
    pub fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        self.adapter.process_event(window, event);
    }

    /// Sends the frame's tree when a client is active. With none, this costs nothing.
    pub fn update(&mut self, out: &FrameOutput, root: &RootInfo) {
        let Self {
            adapter,
            shared,
            prev,
        } = self;
        adapter.update_if_active(|| shared.next_update(out, root, prev));
    }

    /// Moves the waiting actions into `translator`, for the next frame's input.
    pub fn drain(&self, translator: &mut Translator) {
        for action in self.shared.take() {
            translator.access(action);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::sync::{Arc, OnceLock};

    use super::*;

    fn click(raw: u64) -> AccessAction {
        AccessAction {
            target: WidgetId::from_value(raw),
            action: Action::Default,
            value: None,
        }
    }

    fn counted() -> (Arc<Shared>, Arc<AtomicUsize>) {
        let wakes = Arc::new(AtomicUsize::new(0));
        let w = wakes.clone();
        let shared = Shared::new(move || {
            w.fetch_add(1, Ordering::SeqCst);
        });
        (Arc::new(shared), wakes)
    }

    fn request(action: AkAction, raw: u64) -> ActionRequest {
        ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node: NodeId(raw),
            data: None,
        }
    }

    #[test]
    fn the_queue_stops_at_the_cap_and_keeps_the_oldest() {
        let (shared, wakes) = counted();
        let mut handlers = Handlers(shared.clone());
        for raw in 0..MAX_QUEUED as u64 + 10 {
            handlers.do_action(request(AkAction::Click, raw));
        }
        let got: Vec<u64> = shared.take().iter().map(|a| a.target.value()).collect();
        assert_eq!(got, (0..MAX_QUEUED as u64).collect::<Vec<_>>());
        assert_eq!(wakes.load(Ordering::SeqCst), MAX_QUEUED);
        assert!(shared.take().is_empty());
    }

    #[test]
    fn an_action_with_no_meaning_neither_queues_nor_wakes() {
        let (shared, wakes) = counted();
        Handlers(shared.clone()).do_action(request(AkAction::Blur, 1));
        assert!(shared.take().is_empty());
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn wake_runs_with_the_queue_unlocked() {
        let cell: Arc<OnceLock<Arc<Shared>>> = Arc::new(OnceLock::new());
        let seen = Arc::new(AtomicUsize::new(0));
        let (c, s) = (cell.clone(), seen.clone());
        let shared = Arc::new(Shared::new(move || {
            let free = c.get().is_some_and(|sh| sh.queue.try_lock().is_ok());
            assert!(free, "wake ran with the queue locked");
            s.fetch_add(1, Ordering::SeqCst);
        }));
        assert!(cell.set(shared.clone()).is_ok());
        shared.push(click(1));
        shared.activated();
        assert_eq!(seen.load(Ordering::SeqCst), 2);
    }

    fn frame() -> (FrameOutput, RootInfo) {
        let mut out = FrameOutput::default();
        let id = WidgetId::ROOT.with("button");
        let bounds = Rect::new(0.0, 0.0, 10.0, 10.0);
        out.tree
            .nodes
            .push(Node::new(id, Role::Button, "b", bounds));
        let root = RootInfo {
            title: "t".to_owned(),
            scale: 1.0,
        };
        (out, root)
    }

    #[test]
    fn activation_sets_the_flag_and_wakes_and_the_next_update_is_full() {
        let (shared, wakes) = counted();
        let (out, root) = frame();
        let mut prev = NodeCache::default();
        assert_eq!(shared.next_update(&out, &root, &mut prev).nodes.len(), 2);
        assert!(shared.next_update(&out, &root, &mut prev).nodes.is_empty());

        assert_eq!(Handlers(shared.clone()).request_initial_tree(), None);
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        let full = shared.next_update(&out, &root, &mut prev);
        assert_eq!(full.nodes.len(), 2);
        assert!(full.tree.is_some());
        assert!(shared.next_update(&out, &root, &mut prev).nodes.is_empty());

        // Asked again with no deactivation between.
        assert_eq!(Handlers(shared.clone()).request_initial_tree(), None);
        assert_eq!(shared.next_update(&out, &root, &mut prev).nodes.len(), 2);
    }

    #[test]
    fn deactivation_sets_the_flag_without_a_wake() {
        let (shared, wakes) = counted();
        let (out, root) = frame();
        let mut prev = NodeCache::default();
        shared.next_update(&out, &root, &mut prev);
        Handlers(shared.clone()).deactivate_accessibility();
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
        assert_eq!(shared.next_update(&out, &root, &mut prev).nodes.len(), 2);
    }

    /// Handlers on other threads while the frame drains: nothing lost, nothing doubled, in order
    /// per thread, and no deadlock.
    #[test]
    fn four_threads_push_while_the_frame_drains() {
        const PER_THREAD: u64 = 5_000;
        let (shared, _) = counted();
        let threads: Vec<_> = (0..4_u64)
            .map(|t| {
                let shared = shared.clone();
                std::thread::spawn(move || {
                    for i in 0..PER_THREAD {
                        // Backs off short of the cap by one push per thread, the most that can
                        // land after this check, so a loss here would be the queue's own.
                        while shared.queue().len() >= MAX_QUEUED - 4 {
                            std::thread::yield_now();
                        }
                        shared.push(click(t << 32 | i));
                    }
                })
            })
            .collect();
        let mut next = [0_u64; 4];
        let mut seen = 0;
        while seen < 4 * PER_THREAD {
            for a in shared.take() {
                let (t, i) = (a.target.value() >> 32, a.target.value() & 0xFFFF_FFFF);
                assert_eq!(next[t as usize], i, "thread {t}");
                next[t as usize] += 1;
                seen += 1;
            }
            if threads.iter().all(|t| t.is_finished()) && shared.queue().is_empty() {
                break;
            }
        }
        for t in threads {
            assert!(t.join().is_ok());
        }
        assert_eq!(next, [PER_THREAD; 4]);
    }
}
