//! Accessibility data: the semantic tree a frame returns, its node fields, and the actions an
//! assistive technology sends back. Plain data; the `Ui` fills the tree and routes the actions.
//! Roles, states and actions are the closed sets of the accessibility component's anatomy.

use crate::paint::Rect;
use crate::ui::WidgetId;

/// A widget's role, one of the 37 the accessibility component lists, in its order (the last, a
/// password field, was added by M10's review: assistive technology needs to know a field is one
/// to stop echoing what is typed).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Role {
    Button,
    Checkbox,
    Switch,
    Radio,
    RadioGroup,
    Slider,
    SpinButton,
    TextInput,
    MultilineText,
    ComboBox,
    List,
    ListItem,
    Tree,
    TreeItem,
    TabList,
    Tab,
    Menu,
    MenuItem,
    Dialog,
    Alert,
    Tooltip,
    Table,
    Row,
    Cell,
    ColumnHeader,
    Progress,
    ScrollArea,
    Toolbar,
    Group,
    Image,
    Link,
    StaticText,
    Status,
    Navigation,
    Note,
    Separator,
    /// A text field whose text is a secret: no value or text is ever exposed, and the role is
    /// what tells a screen reader not to speak the typed characters.
    PasswordInput,
}

impl Role {
    /// Every role, in declaration order.
    pub const ALL: [Role; 37] = [
        Role::Button,
        Role::Checkbox,
        Role::Switch,
        Role::Radio,
        Role::RadioGroup,
        Role::Slider,
        Role::SpinButton,
        Role::TextInput,
        Role::MultilineText,
        Role::ComboBox,
        Role::List,
        Role::ListItem,
        Role::Tree,
        Role::TreeItem,
        Role::TabList,
        Role::Tab,
        Role::Menu,
        Role::MenuItem,
        Role::Dialog,
        Role::Alert,
        Role::Tooltip,
        Role::Table,
        Role::Row,
        Role::Cell,
        Role::ColumnHeader,
        Role::Progress,
        Role::ScrollArea,
        Role::Toolbar,
        Role::Group,
        Role::Image,
        Role::Link,
        Role::StaticText,
        Role::Status,
        Role::Navigation,
        Role::Note,
        Role::Separator,
        Role::PasswordInput,
    ];
}

/// A state a node can be in, one of the 15 the accessibility component lists, in its order.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum State {
    Focusable,
    Focused,
    Disabled,
    /// Checked, including mixed.
    Checked,
    Selected,
    Expanded,
    Pressed,
    Modal,
    ReadOnly,
    Required,
    Invalid,
    Busy,
    /// Content the app keeps but does not show.
    Hidden,
    /// The active page, step or location.
    Current,
    /// The item under a drag.
    Dragging,
}

/// A set of [`State`]s.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct States(u16);

impl States {
    pub fn contains(self, state: State) -> bool {
        self.0 & (1 << state as u16) != 0
    }

    pub fn insert(&mut self, state: State) {
        self.0 |= 1 << state as u16;
    }

    pub fn remove(&mut self, state: State) {
        self.0 &= !(1 << state as u16);
    }
}

/// An action an assistive technology may request, one of the 10 the accessibility component
/// lists, in its order.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// Click.
    Default,
    Focus,
    Increment,
    Decrement,
    SetValue,
    Expand,
    Collapse,
    ScrollIntoView,
    ShowMenu,
    Dismiss,
}

/// A set of [`Action`]s.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Actions(u16);

impl Actions {
    pub fn contains(self, action: Action) -> bool {
        self.0 & (1 << action as u16) != 0
    }

    pub fn insert(&mut self, action: Action) {
        self.0 |= 1 << action as u16;
    }

    pub fn remove(&mut self, action: Action) {
        self.0 &= !(1 << action as u16);
    }
}

/// A node's value: text, or a number with its range and step.
#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Text(String),
    Number {
        value: f64,
        min: f64,
        max: f64,
        step: f64,
    },
}

/// The text of a text input and its selection, as char offsets.
#[derive(Clone, PartialEq, Debug)]
pub struct TextValue {
    pub text: String,
    pub selection: (usize, usize),
}

/// The politeness of a live region.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Live {
    Polite,
    Assertive,
}

/// The sort order of a column header.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sort {
    Ascending,
    Descending,
    None,
}

/// One node of the semantic tree.
#[derive(Clone, PartialEq, Debug)]
pub struct Node {
    pub id: WidgetId,
    pub role: Role,
    /// The accessible name.
    pub name: String,
    pub description: Option<String>,
    pub value: Option<Value>,
    pub text: Option<TextValue>,
    /// Logical pixels, after the transform stack.
    pub bounds: Rect,
    pub parent: Option<WidgetId>,
    pub children: Vec<WidgetId>,
    pub live: Option<Live>,
    pub states: States,
    pub actions: Actions,
    pub sort: Option<Sort>,
    /// The depth in a hierarchy, 1 at its top (a tree item's level); `None` outside one.
    pub level: Option<u32>,
}

impl Node {
    /// A node with no description, value, text, live region, sort, states or actions. The `Ui`
    /// sets `parent` and `children` when it appends the node.
    pub fn new(id: WidgetId, role: Role, name: &str, bounds: Rect) -> Node {
        Node {
            id,
            role,
            name: name.to_owned(),
            description: None,
            value: None,
            text: None,
            bounds,
            parent: None,
            children: Vec::new(),
            live: None,
            states: States::default(),
            actions: Actions::default(),
            sort: None,
            level: None,
        }
    }
}

/// The semantic tree of one frame: nodes in call order, parents before children.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct SemanticTree {
    pub nodes: Vec<Node>,
}

impl SemanticTree {
    /// The node with `id`, if the frame emitted one.
    pub fn get(&self, id: WidgetId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
}

/// The value an assistive technology sends with a `set-value` request.
#[derive(Clone, PartialEq, Debug)]
pub enum AccessValue {
    Text(String),
    Number(f64),
}

/// An assistive technology's request: an action on a node.
#[derive(Clone, PartialEq, Debug)]
pub struct AccessAction {
    pub target: WidgetId,
    pub action: Action,
    pub value: Option<AccessValue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATES: [State; 15] = [
        State::Focusable,
        State::Focused,
        State::Disabled,
        State::Checked,
        State::Selected,
        State::Expanded,
        State::Pressed,
        State::Modal,
        State::ReadOnly,
        State::Required,
        State::Invalid,
        State::Busy,
        State::Hidden,
        State::Current,
        State::Dragging,
    ];

    const ACTIONS: [Action; 10] = [
        Action::Default,
        Action::Focus,
        Action::Increment,
        Action::Decrement,
        Action::SetValue,
        Action::Expand,
        Action::Collapse,
        Action::ScrollIntoView,
        Action::ShowMenu,
        Action::Dismiss,
    ];

    fn node(id: WidgetId, role: Role, states: States, actions: Actions) -> Node {
        Node {
            id,
            role,
            name: "n".to_owned(),
            description: None,
            value: None,
            text: None,
            bounds: Rect::new(0.0, 0.0, 1.0, 1.0),
            parent: None,
            children: Vec::new(),
            live: None,
            states,
            actions,
            sort: None,
            level: None,
        }
    }

    #[test]
    fn the_closed_sets_have_the_documented_counts() {
        assert_eq!(Role::ALL.len(), 37);
        assert_eq!(STATES.len(), 15);
        assert_eq!(ACTIONS.len(), 10);
        for (i, role) in Role::ALL.iter().enumerate() {
            assert_eq!(*role as usize, i, "{role:?}");
        }
    }

    #[test]
    fn every_role_state_and_action_round_trips_through_a_node() {
        let mut tree = SemanticTree::default();
        let mut all_states = States::default();
        let mut all_actions = Actions::default();
        for s in STATES {
            all_states.insert(s);
        }
        for a in ACTIONS {
            all_actions.insert(a);
        }
        for (i, role) in Role::ALL.iter().enumerate() {
            let id = WidgetId::ROOT.with(i as u32);
            let mut states = States::default();
            let mut actions = Actions::default();
            states.insert(STATES[i % 15]);
            actions.insert(ACTIONS[i % 10]);
            tree.nodes.push(node(id, *role, states, actions));
        }
        for (i, role) in Role::ALL.iter().enumerate() {
            let id = WidgetId::ROOT.with(i as u32);
            let found = tree.get(id).expect("node emitted");
            assert_eq!(found.role, *role);
            assert!(found.states.contains(STATES[i % 15]));
            assert!(found.actions.contains(ACTIONS[i % 10]));
        }
        let full = node(WidgetId::ROOT, Role::Group, all_states, all_actions);
        for s in STATES {
            assert!(full.states.contains(s), "{s:?}");
        }
        for a in ACTIONS {
            assert!(full.actions.contains(a), "{a:?}");
        }
        let mut cleared = all_states;
        for s in STATES {
            cleared.remove(s);
        }
        assert_eq!(cleared, States::default());
    }
}
