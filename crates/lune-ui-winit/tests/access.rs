//! The AccessKit bridge with no window: the mapping and the update protocol, each update replayed
//! into `accesskit_consumer`, the tree AccessKit's OS adapters run, so "a tree AccessKit accepts"
//! is AccessKit's own verdict (it panics on a malformed update) and not a copy of its rules. The
//! handler thread's protocol is tested beside `Shared`, in `src/access.rs`.

#![cfg(feature = "accesskit")]

use std::collections::HashSet;
use std::time::Duration;

use accesskit::{
    Action as AkAction, ActionData, ActionRequest, Affine, AriaCurrent, Invalid, NodeId, Point,
    Rect as AkRect, Role as AkRole, ScrollHint, ScrollUnit, SortDirection, TextPosition,
    TextSelection, Toggled, TreeId, TreeUpdate,
};
use accesskit_consumer::{NodeRef, Tree, TreeChangeHandler};
use lune_ui_core::access::{
    AccessAction, AccessValue, Action, Live, Node, Role, SemanticTree, Sort, State, TextValue,
    Value,
};
use lune_ui_core::input::{FrameOutput, Keymap};
use lune_ui_core::motion::MotionTier;
use lune_ui_core::paint::Rect;
use lune_ui_core::shortcuts::Shortcuts;
use lune_ui_core::strings::{DisplayFormat, StringTable};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{FrameConfig, Ui, UiStore, WidgetId};
use lune_ui_core::widgets::dialog::{Cancel, Dialog, DialogGlass};
use lune_ui_core::widgets::field::{FieldOptions, FieldState, Variant};
use lune_ui_core::widgets::slider::SliderOptions;
use lune_ui_winit::access::{NodeCache, RootInfo, action_from, tree_update};
use lune_ui_winit::clipboard::MAX_PASTE_BYTES;
use lune_ui_winit::input::Translator;
use winit::dpi::PhysicalSize;

const AT: Rect = Rect::new(10.0, 20.0, 30.0, 40.0);

fn root() -> RootInfo {
    RootInfo {
        title: "Window".to_owned(),
        scale: 2.0,
    }
}

fn wid(key: &str) -> WidgetId {
    WidgetId::ROOT.with(key)
}

fn nid(id: WidgetId) -> NodeId {
    NodeId(id.value())
}

fn node(key: &str, role: Role) -> Node {
    Node::new(wid(key), role, key, AT)
}

/// `child` under `parent`, as the `Ui` wires them.
fn under(parent: &mut Node, mut child: Node) -> Node {
    child.parent = Some(parent.id);
    parent.children.push(child.id);
    child
}

fn sent(update: &TreeUpdate, id: WidgetId) -> &accesskit::Node {
    let found = update.nodes.iter().find(|(n, _)| *n == nid(id));
    &found.expect("the node was sent").1
}

fn ids(update: &TreeUpdate) -> HashSet<NodeId> {
    update.nodes.iter().map(|(id, _)| *id).collect()
}

/// One node through `tree_update`, from nothing.
fn one(n: Node) -> accesskit::Node {
    let id = n.id;
    let tree = SemanticTree { nodes: vec![n] };
    sent(&full(&tree), id).clone()
}

fn full(tree: &SemanticTree) -> TreeUpdate {
    tree_update(tree, None, &root(), &mut NodeCache::default())
}

fn window_id(tree: &TreeUpdate) -> NodeId {
    tree.tree.as_ref().expect("a full tree").root
}

/// The side an OS adapter runs: takes updates, and panics on one it cannot accept.
#[derive(Default)]
struct Mirror {
    tree: Option<Tree>,
}

struct Quiet;

impl TreeChangeHandler for Quiet {
    fn node_added(&mut self, _: &NodeRef<'_>) {}
    fn node_updated(&mut self, _: &NodeRef<'_>, _: &NodeRef<'_>) {}
    fn focus_moved(&mut self, _: Option<&NodeRef<'_>>, _: Option<&NodeRef<'_>>) {}
    fn node_removed(&mut self, _: &NodeRef<'_>) {}
}

impl Mirror {
    fn apply(&mut self, update: TreeUpdate) {
        match &mut self.tree {
            Some(tree) => tree.update_and_process_changes(update, &mut Quiet),
            None => self.tree = Some(Tree::new(update, true)),
        }
    }

    /// Every id from the root down, checking each is reached once, and reading each text node as a
    /// client reads it, which panics on an inconsistent run.
    fn walk(&self) -> HashSet<NodeId> {
        let state = self.tree.as_ref().expect("an update came").state();
        let mut seen = HashSet::new();
        let mut open = vec![state.root()];
        while let Some(n) = open.pop() {
            assert!(seen.insert(n.locate().0), "reached twice: {:?}", n.locate());
            if n.supports_text_ranges() {
                let _ = n.document_range().text();
                if let Some(selection) = n.text_selection() {
                    let _ = selection.text();
                }
            }
            open.extend(n.children());
        }
        assert!(
            seen.contains(
                &state
                    .node_by_id(state.focus_id_in_tree())
                    .expect("focus")
                    .locate()
                    .0
            )
        );
        seen
    }
}

mod mapping {
    use super::*;

    #[test]
    fn every_role_maps_to_the_role_of_its_meaning() {
        let expected = [
            AkRole::Button,
            AkRole::CheckBox,
            AkRole::Switch,
            AkRole::RadioButton,
            AkRole::RadioGroup,
            AkRole::Slider,
            AkRole::SpinButton,
            AkRole::TextInput,
            AkRole::MultilineTextInput,
            AkRole::ComboBox,
            AkRole::List,
            AkRole::ListItem,
            AkRole::Tree,
            AkRole::TreeItem,
            AkRole::TabList,
            AkRole::Tab,
            AkRole::Menu,
            AkRole::MenuItem,
            AkRole::Dialog,
            AkRole::Alert,
            AkRole::Tooltip,
            AkRole::Table,
            AkRole::Row,
            AkRole::Cell,
            AkRole::ColumnHeader,
            AkRole::ProgressIndicator,
            AkRole::ScrollView,
            AkRole::Toolbar,
            AkRole::Group,
            AkRole::Image,
            AkRole::Link,
            AkRole::Label,
            AkRole::Status,
            AkRole::Navigation,
            AkRole::Note,
            AkRole::Splitter,
            AkRole::PasswordInput,
        ];
        assert_eq!(Role::ALL.len(), expected.len());
        for (role, ak) in Role::ALL.iter().zip(expected) {
            assert_eq!(one(node("n", *role)).role(), ak, "{role:?}");
        }
    }

    #[test]
    fn static_text_with_a_level_is_a_heading() {
        let mut n = node("n", Role::StaticText);
        n.level = Some(2);
        let got = one(n);
        assert_eq!(got.role(), AkRole::Heading);
        assert_eq!(got.level(), Some(2));
        assert_eq!(got.label(), Some("n"));
    }

    #[test]
    fn a_label_says_its_name_through_its_value() {
        let got = one(node("n", Role::StaticText));
        assert_eq!(got.value(), Some("n"));
        assert_eq!(got.label(), None);
    }

    #[test]
    fn name_description_bounds_live_sort_and_level_carry_over() {
        let mut n = node("n", Role::ColumnHeader);
        n.description = Some("about".to_owned());
        n.live = Some(Live::Assertive);
        n.sort = Some(Sort::Descending);
        n.level = Some(3);
        let got = one(n);
        assert_eq!(got.label(), Some("n"));
        assert_eq!(got.description(), Some("about"));
        assert_eq!(got.live(), Some(accesskit::Live::Assertive));
        assert_eq!(got.sort_direction(), Some(SortDirection::Descending));
        assert_eq!(got.level(), Some(3));
        let bounds = AkRect {
            x0: 10.0,
            y0: 20.0,
            x1: 40.0,
            y1: 60.0,
        };
        assert_eq!(got.bounds(), Some(bounds));

        for (ours, theirs) in [
            (Sort::Ascending, Some(SortDirection::Ascending)),
            (Sort::None, None),
        ] {
            let mut n = node("n", Role::ColumnHeader);
            n.sort = Some(ours);
            assert_eq!(one(n).sort_direction(), theirs);
        }
        let mut n = node("n", Role::Status);
        n.live = Some(Live::Polite);
        assert_eq!(one(n).live(), Some(accesskit::Live::Polite));
    }

    #[test]
    fn a_number_gives_value_min_max_and_step_and_a_bad_one_gives_none_of_itself() {
        let mut n = node("n", Role::Slider);
        n.value = Some(Value::Number {
            value: 30.0,
            min: 0.0,
            max: 100.0,
            step: 10.0,
        });
        let got = one(n);
        assert_eq!(got.numeric_value(), Some(30.0));
        assert_eq!(got.min_numeric_value(), Some(0.0));
        assert_eq!(got.max_numeric_value(), Some(100.0));
        assert_eq!(got.numeric_value_step(), Some(10.0));

        let mut n = node("n", Role::Slider);
        n.value = Some(Value::Number {
            value: f64::NAN,
            min: 0.0,
            max: f64::INFINITY,
            step: 10.0,
        });
        let got = one(n);
        assert_eq!(got.numeric_value(), None);
        assert_eq!(got.min_numeric_value(), Some(0.0));
        assert_eq!(got.max_numeric_value(), None);
    }

    #[test]
    fn each_state_has_its_property() {
        let with = |role, state| {
            let mut n = node("n", role);
            n.states.insert(state);
            one(n)
        };
        let g = Role::Group;
        assert!(with(g, State::Focusable).supports_action(AkAction::Focus));
        assert!(with(g, State::Disabled).is_disabled());
        assert_eq!(with(g, State::Checked).toggled(), Some(Toggled::True));
        assert_eq!(with(g, State::Pressed).toggled(), Some(Toggled::True));
        assert_eq!(with(g, State::Selected).is_selected(), Some(true));
        assert_eq!(with(g, State::Expanded).is_expanded(), Some(true));
        assert!(with(g, State::Modal).is_modal());
        assert!(with(g, State::ReadOnly).is_read_only());
        assert!(with(g, State::Required).is_required());
        assert_eq!(with(g, State::Invalid).invalid(), Some(Invalid::True));
        assert!(with(g, State::Busy).is_busy());
        assert!(with(g, State::Hidden).is_hidden());
        assert_eq!(
            with(g, State::Current).aria_current(),
            Some(AriaCurrent::True)
        );
        // No property of its own: `Focused` is the update's focus, `Dragging` has no counterpart.
        let plain = one(node("n", g));
        assert_eq!(with(g, State::Focused), plain);
        assert_eq!(with(g, State::Dragging), plain);
    }

    #[test]
    fn a_checkable_role_says_unchecked_and_another_role_says_nothing() {
        for role in [Role::Checkbox, Role::Switch, Role::Radio] {
            assert_eq!(
                one(node("n", role)).toggled(),
                Some(Toggled::False),
                "{role:?}"
            );
        }
        assert_eq!(one(node("n", Role::Button)).toggled(), None);
    }

    #[test]
    fn an_expandable_node_without_the_state_is_collapsed() {
        let mut n = node("n", Role::TreeItem);
        n.actions.insert(Action::Expand);
        assert_eq!(one(n).is_expanded(), Some(false));
        assert_eq!(one(node("n", Role::TreeItem)).is_expanded(), None);
    }

    #[test]
    fn each_action_has_its_action_and_dismiss_only_on_a_tooltip() {
        for (ours, theirs) in [
            (Action::Default, AkAction::Click),
            (Action::Focus, AkAction::Focus),
            (Action::Increment, AkAction::Increment),
            (Action::Decrement, AkAction::Decrement),
            (Action::SetValue, AkAction::SetValue),
            (Action::Expand, AkAction::Expand),
            (Action::Collapse, AkAction::Collapse),
            (Action::ScrollIntoView, AkAction::ScrollIntoView),
            (Action::ShowMenu, AkAction::ShowContextMenu),
        ] {
            let mut n = node("n", Role::Group);
            n.actions.insert(ours);
            assert!(one(n).supports_action(theirs), "{ours:?}");
        }
        let dismissable = |role| {
            let mut n = node("n", role);
            n.actions.insert(Action::Dismiss);
            one(n)
        };
        assert!(dismissable(Role::Tooltip).supports_action(AkAction::HideTooltip));
        assert_eq!(dismissable(Role::Dialog), one(node("n", Role::Dialog)));
    }

    #[test]
    fn the_root_is_a_window_with_the_title_and_the_scale_as_its_transform() {
        let mut child = node("top", Role::Button);
        child.parent = None;
        let update = full(&SemanticTree { nodes: vec![child] });
        let window = window_id(&update);
        assert_ne!(window, NodeId(WidgetId::ROOT.value()));
        let (_, w) = &update.nodes[0];
        assert_eq!(update.nodes[0].0, window);
        assert_eq!(w.role(), AkRole::Window);
        assert_eq!(w.label(), Some("Window"));
        assert_eq!(w.transform(), Some(&Affine::scale(2.0)));
        assert_eq!(w.children(), [nid(wid("top"))]);
        assert_eq!(update.tree_id, TreeId::ROOT);
    }

    #[test]
    fn a_scale_of_one_or_a_meaningless_one_has_no_transform() {
        for scale in [1.0, 0.0, -1.0, f64::NAN, f64::INFINITY] {
            let info = RootInfo {
                title: String::new(),
                scale,
            };
            let update = tree_update(
                &SemanticTree::default(),
                None,
                &info,
                &mut NodeCache::default(),
            );
            assert_eq!(update.nodes[0].1.transform(), None, "{scale}");
        }
    }
}

mod text {
    use super::*;

    fn with_text(text: &str, selection: (usize, usize)) -> (TreeUpdate, Vec<NodeId>) {
        let mut n = node("f", Role::MultilineText);
        n.text = Some(TextValue {
            text: text.to_owned(),
            selection,
        });
        let update = full(&SemanticTree { nodes: vec![n] });
        let runs = sent(&update, wid("f")).children().to_vec();
        (update, runs)
    }

    fn run(update: &TreeUpdate, id: NodeId) -> &accesskit::Node {
        &update.nodes.iter().find(|(n, _)| *n == id).expect("run").1
    }

    #[test]
    fn two_lines_give_two_runs_and_the_selection_a_run_and_an_index_on_each() {
        let (update, runs) = with_text("ab\ncd", (1, 4));
        assert_eq!(runs.len(), 2);
        let (a, b) = (run(&update, runs[0]), run(&update, runs[1]));
        assert_eq!(a.role(), AkRole::TextRun);
        // The line break is the run's last char, as AccessKit asks.
        assert_eq!(a.value(), Some("ab\n"));
        assert_eq!(a.character_lengths(), [1, 1, 1]);
        assert_eq!(b.value(), Some("cd"));
        assert_eq!(b.character_lengths(), [1, 1]);
        assert_eq!(
            sent(&update, wid("f")).text_selection(),
            Some(&TextSelection {
                anchor: TextPosition {
                    node: runs[0],
                    character_index: 1
                },
                focus: TextPosition {
                    node: runs[1],
                    character_index: 1
                },
            })
        );
    }

    fn selection(update: &TreeUpdate) -> TextSelection {
        *sent(update, wid("f")).text_selection().expect("selection")
    }

    #[test]
    fn a_reversed_selection_keeps_its_ends() {
        let (update, runs) = with_text("ab\ncd", (4, 1));
        let s = selection(&update);
        assert_eq!((s.anchor.node, s.anchor.character_index), (runs[1], 1));
        assert_eq!((s.focus.node, s.focus.character_index), (runs[0], 1));
    }

    #[test]
    fn an_out_of_range_selection_clamps_to_the_end() {
        let (update, runs) = with_text("ab\ncd", (5, usize::MAX));
        let s = selection(&update);
        assert_eq!((s.anchor.node, s.anchor.character_index), (runs[1], 2));
        assert_eq!(s.anchor, s.focus);
    }

    #[test]
    fn the_end_of_a_line_is_on_its_break_and_the_next_char_starts_the_next_run() {
        let (update, runs) = with_text("ab\ncd", (2, 3));
        let s = selection(&update);
        assert_eq!((s.anchor.node, s.anchor.character_index), (runs[0], 2));
        assert_eq!((s.focus.node, s.focus.character_index), (runs[1], 0));
    }

    #[test]
    fn empty_text_has_one_empty_run_and_a_final_break_has_an_empty_last_run() {
        let (update, runs) = with_text("", (0, 0));
        assert_eq!(runs.len(), 1);
        assert_eq!(run(&update, runs[0]).value(), Some(""));
        assert_eq!(selection(&update).focus.node, runs[0]);

        let (update, runs) = with_text("ab\n", (3, 3));
        assert_eq!(runs.len(), 2);
        assert_eq!(run(&update, runs[1]).value(), Some(""));
        assert_eq!(selection(&update).focus.node, runs[1]);
    }

    #[test]
    fn lengths_are_utf8_bytes_per_char_while_offsets_count_chars() {
        let (update, runs) = with_text("é漢😀\nz", (3, 4));
        assert_eq!(run(&update, runs[0]).character_lengths(), [2, 3, 4, 1]);
        let s = selection(&update);
        assert_eq!((s.anchor.node, s.anchor.character_index), (runs[0], 3));
        assert_eq!((s.focus.node, s.focus.character_index), (runs[1], 0));
    }

    #[test]
    fn a_client_reads_the_runs_back_as_the_text() {
        let mut mirror = Mirror::default();
        let (update, _) = with_text("one\ntwo\n\nfour", (2, 9));
        mirror.apply(update);
        mirror.walk();
        let state = mirror.tree.as_ref().expect("applied").state();
        let field = state.root().children().next().expect("the field");
        assert_eq!(field.document_range().text(), "one\ntwo\n\nfour");
        let selected = field.text_selection().expect("selection").text();
        assert_eq!(selected, "e\ntwo\n\n");
    }

    #[test]
    fn a_run_keeps_a_bounds_when_its_node_has_none() {
        let mut n = node("f", Role::TextInput);
        n.bounds = Rect::new(f32::NAN, 0.0, 1.0, 1.0);
        n.text = Some(TextValue {
            text: "a".to_owned(),
            selection: (0, 0),
        });
        let update = full(&SemanticTree { nodes: vec![n] });
        let f = sent(&update, wid("f"));
        assert_eq!(f.bounds(), None);
        assert_eq!(run(&update, f.children()[0]).bounds(), Some(AkRect::ZERO));
    }

    #[test]
    fn a_run_whose_id_a_widget_holds_is_left_out_and_the_selection_moves_past_it() {
        let (_, runs) = with_text("ab\ncd", (0, 0));
        let mut field = node("f", Role::MultilineText);
        field.text = Some(TextValue {
            text: "ab\ncd".to_owned(),
            selection: (1, 4),
        });
        let thief = Node::new(WidgetId::from_value(runs[0].0), Role::Button, "b", AT);
        let update = full(&SemanticTree {
            nodes: vec![field, thief],
        });
        let got = sent(&update, wid("f"));
        assert_eq!(got.children(), [runs[1]]);
        assert_eq!(
            sent(&update, WidgetId::from_value(runs[0].0)).role(),
            AkRole::Button
        );
        let s = selection(&update);
        assert_eq!((s.anchor.node, s.anchor.character_index), (runs[1], 0));
        assert_eq!((s.focus.node, s.focus.character_index), (runs[1], 1));
        let mut mirror = Mirror::default();
        mirror.apply(update);
        mirror.walk();
    }

    #[test]
    fn a_changed_line_sends_its_run_and_not_the_others() {
        let frame = |text: &str| {
            let mut n = node("f", Role::MultilineText);
            n.text = Some(TextValue {
                text: text.to_owned(),
                selection: (0, 0),
            });
            SemanticTree { nodes: vec![n] }
        };
        let mut prev = NodeCache::default();
        let first = tree_update(&frame("ab\ncd"), None, &root(), &mut prev);
        let runs = sent(&first, wid("f")).children().to_vec();
        let next = tree_update(&frame("ab\nce"), None, &root(), &mut prev);
        assert_eq!(ids(&next), HashSet::from([runs[1]]));
    }
}

mod shapes {
    use super::*;

    const WINDOW: NodeId = NodeId(u64::MAX);

    fn accepted(tree: &SemanticTree) -> HashSet<NodeId> {
        let mut mirror = Mirror::default();
        mirror.apply(full(tree));
        mirror.walk()
    }

    #[test]
    fn a_widget_on_the_reserved_id_is_left_out_and_its_children_stay() {
        let mut bad = Node::new(WidgetId::from_value(WINDOW.0), Role::Group, "x", AT);
        let kid = under(&mut bad, node("kid", Role::Button));
        let tree = SemanticTree {
            nodes: vec![bad, kid],
        };
        let update = tree_update(
            &tree,
            Some(WidgetId::from_value(WINDOW.0)),
            &root(),
            &mut NodeCache::default(),
        );
        assert_eq!(window_id(&update), WINDOW);
        assert_eq!(update.focus, WINDOW);
        assert_eq!(
            update.nodes.iter().filter(|(id, _)| *id == WINDOW).count(),
            1
        );
        assert_eq!(update.nodes[0].1.role(), AkRole::Window);
        assert_eq!(accepted(&tree), HashSet::from([WINDOW, nid(wid("kid"))]));
    }

    #[test]
    fn widget_id_root_is_an_ordinary_node() {
        let n = Node::new(WidgetId::ROOT, Role::Group, "zero", AT);
        let tree = SemanticTree { nodes: vec![n] };
        assert_eq!(accepted(&tree), HashSet::from([WINDOW, NodeId(0)]));
    }

    #[test]
    fn a_repeated_id_goes_out_once_and_the_first_node_is_it() {
        let mut parent = node("p", Role::Group);
        let first = under(&mut parent, node("x", Role::Button));
        let second = under(&mut parent, Node::new(wid("x"), Role::Link, "again", AT));
        let tree = SemanticTree {
            nodes: vec![parent, first, second],
        };
        let update = full(&tree);
        assert_eq!(sent(&update, wid("p")).children(), [nid(wid("x"))]);
        assert_eq!(sent(&update, wid("x")).role(), AkRole::Button);
        assert_eq!(accepted(&tree).len(), 3);
    }

    #[test]
    fn children_of_a_repeated_container_stay_in_the_tree() {
        let mut first = node("c", Role::Group);
        first.children.clear();
        let mut second = Node::new(wid("c"), Role::Group, "again", AT);
        let kid = under(&mut second, node("kid", Role::Button));
        let tree = SemanticTree {
            nodes: vec![first, second, kid],
        };
        let update = full(&tree);
        assert_eq!(sent(&update, wid("c")).children(), [nid(wid("kid"))]);
        assert_eq!(accepted(&tree).len(), 3);
    }

    #[test]
    fn a_child_listed_by_two_parents_stays_with_the_first() {
        let mut a = node("a", Role::Group);
        let mut b = node("b", Role::Group);
        let kid = under(&mut a, node("kid", Role::Button));
        b.children.push(kid.id);
        let tree = SemanticTree {
            nodes: vec![a, b, kid],
        };
        let update = full(&tree);
        assert_eq!(sent(&update, wid("a")).children(), [nid(wid("kid"))]);
        assert!(sent(&update, wid("b")).children().is_empty());
        assert_eq!(accepted(&tree).len(), 4);
    }

    #[test]
    fn a_child_named_twice_by_one_parent_appears_once() {
        let mut p = node("p", Role::Group);
        let kid = under(&mut p, node("kid", Role::Button));
        p.children.push(kid.id);
        let tree = SemanticTree {
            nodes: vec![p, kid],
        };
        assert_eq!(sent(&full(&tree), wid("p")).children().len(), 1);
        assert_eq!(accepted(&tree).len(), 3);
    }

    #[test]
    fn a_child_that_does_not_exist_is_dropped() {
        let mut p = node("p", Role::Group);
        p.children.push(wid("ghost"));
        let tree = SemanticTree { nodes: vec![p] };
        assert!(sent(&full(&tree), wid("p")).children().is_empty());
        assert_eq!(accepted(&tree).len(), 2);
    }

    #[test]
    fn a_parent_that_does_not_exist_leaves_the_node_under_the_window() {
        let mut n = node("n", Role::Button);
        n.parent = Some(wid("ghost"));
        let tree = SemanticTree { nodes: vec![n] };
        assert_eq!(full(&tree).nodes[0].1.children(), [nid(wid("n"))]);
    }

    #[test]
    fn a_node_its_parent_does_not_list_goes_under_that_parent() {
        let p = node("p", Role::Group);
        let mut kid = node("kid", Role::Button);
        kid.parent = Some(p.id);
        let tree = SemanticTree {
            nodes: vec![p, kid],
        };
        assert_eq!(sent(&full(&tree), wid("p")).children(), [nid(wid("kid"))]);
    }

    #[test]
    fn cycles_and_a_node_that_is_its_own_parent_reach_the_window() {
        let (mut a, mut b, mut c) = (
            node("a", Role::Group),
            node("b", Role::Group),
            node("c", Role::Group),
        );
        a.parent = Some(b.id);
        a.children.push(b.id);
        b.parent = Some(a.id);
        b.children.push(a.id);
        c.parent = Some(c.id);
        c.children.push(c.id);
        let tree = SemanticTree {
            nodes: vec![a, b, c],
        };
        assert_eq!(accepted(&tree).len(), 4);
    }

    #[test]
    fn a_chain_a_hundred_thousand_deep_needs_no_stack() {
        let mut nodes: Vec<Node> = Vec::new();
        for i in 0..100_000_u32 {
            let mut n = Node::new(WidgetId::ROOT.with(i), Role::Group, "", AT);
            if let Some(p) = nodes.last_mut() {
                n.parent = Some(p.id);
                p.children.push(n.id);
            }
            nodes.push(n);
        }
        let update = full(&SemanticTree { nodes });
        assert_eq!(update.nodes.len(), 100_001);
    }

    /// A small deterministic generator (xorshift), so a failure repeats.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: u64) -> usize {
            (self.next() % n) as usize
        }

        fn chance(&mut self, percent: u64) -> bool {
            self.next() % 100 < percent
        }
    }

    /// One of ten ids, the reserved one and `WidgetId::ROOT` among them.
    fn any_id(r: &mut Rng) -> WidgetId {
        match r.below(10) {
            0 => WidgetId::from_value(WINDOW.0),
            1 => WidgetId::ROOT,
            k => WidgetId::ROOT.with(k),
        }
    }

    fn any_node(r: &mut Rng) -> Node {
        let role = Role::ALL[r.below(Role::ALL.len() as u64)];
        let mut n = Node::new(any_id(r), role, "n", AT);
        if r.chance(40) {
            n.parent = Some(any_id(r));
        }
        for _ in 0..r.below(4) {
            n.children.push(any_id(r));
        }
        if r.chance(30) {
            let texts = ["", "a", "a\n", "\n\n", "é\n漢😀\r\nz", "ab\ncd"];
            n.text = Some(TextValue {
                text: texts[r.below(6)].to_owned(),
                selection: (r.below(12), r.below(12)),
            });
        }
        if r.chance(10) {
            n.bounds = Rect::new(f32::NAN, f32::INFINITY, -1.0, 0.0);
        }
        if r.chance(20) {
            n.states.insert(State::Hidden);
        }
        if r.chance(20) {
            n.value = Some(Value::Number {
                value: f64::NAN,
                min: 0.0,
                max: 1.0,
                step: 0.0,
            });
        }
        n
    }

    /// Frames of every odd shape, in a row, into one consumer: incremental updates across reparents,
    /// removals, re-additions and resends, each accepted and each leaving the walk consistent.
    #[test]
    fn thirty_thousand_random_frames_are_all_accepted() {
        let mut r = Rng(0x9E37_79B9_7F4A_7C15);
        for _ in 0..300 {
            let mut mirror = Mirror::default();
            let mut prev = NodeCache::default();
            for _ in 0..100 {
                let count = r.below(12);
                let tree = SemanticTree {
                    nodes: (0..count).map(|_| any_node(&mut r)).collect(),
                };
                let focus = r.chance(60).then(|| any_id(&mut r));
                if r.chance(5) {
                    prev.clear();
                }
                mirror.apply(tree_update(&tree, focus, &root(), &mut prev));
                let reached = mirror.walk();
                for n in &tree.nodes {
                    assert!(n.id.value() == WINDOW.0 || reached.contains(&nid(n.id)));
                }
            }
        }
    }
}

mod frames {
    use super::*;

    const SLIDER: SliderOptions<'static> = SliderOptions {
        min: 0.0,
        max: 100.0,
        step: 10.0,
        name: "Volume",
    };

    #[derive(Debug)]
    struct App {
        checked: bool,
        volume: f32,
        text: String,
        rows: usize,
        dialog: bool,
    }

    impl Default for App {
        fn default() -> Self {
            App {
                checked: false,
                volume: 30.0,
                text: "lune".to_owned(),
                rows: 2,
                dialog: true,
            }
        }
    }

    fn page(ui: &mut Ui<'_>, app: &mut App) {
        ui.checkbox(
            "check",
            Rect::new(20.0, 20.0, 120.0, 24.0),
            &mut app.checked,
            "Remember",
        );
        ui.slider(
            "slider",
            Rect::new(20.0, 60.0, 200.0, 24.0),
            &mut app.volume,
            &SLIDER,
        );
        ui.text_field(
            "field",
            Rect::new(20.0, 100.0, 200.0, 28.0),
            &mut app.text,
            "Search",
            FieldState::default(),
        );
        ui.node_container(node("list", Role::List), |ui| {
            for i in 0..app.rows {
                ui.node(Node::new(row(i), Role::ListItem, "row", AT));
            }
        });
        if app.dialog {
            let d = Dialog {
                title: "Discard",
                message: "Sure?",
                extra_h: 0.0,
                confirm: "Discard",
                cancel: Cancel::Default,
            };
            let glass = DialogGlass {
                panel: [0.1, 0.1, 0.12, 1.0],
                fill: [0.2, 0.2, 0.2, 0.62],
                noise: true,
            };
            ui.dialog("dialog", &d, &glass, |_, _, _, _| {});
        }
    }

    fn row(i: usize) -> WidgetId {
        wid("row").with(i)
    }

    fn id_of(out: &FrameOutput, role: Role) -> WidgetId {
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == role)
            .expect("node")
            .id
    }

    /// A settled frame: past every opening animation.
    fn settled(h: &mut UiHarness, app: &mut App, ms: &mut u64) -> FrameOutput {
        *ms += 2000;
        h.at_ms(*ms);
        h.frame(|ui| page(ui, app)).1
    }

    /// The bench, the first full tree, and the app.
    fn start() -> (UiHarness, App, NodeCache, Mirror, u64, FrameOutput) {
        let (mut h, mut app, mut ms) = (UiHarness::new((800.0, 600.0)), App::default(), 0);
        settled(&mut h, &mut app, &mut ms);
        let out = settled(&mut h, &mut app, &mut ms);
        let (mut prev, mut mirror) = (NodeCache::default(), Mirror::default());
        mirror.apply(tree_update(&out.tree, out.focused, &root(), &mut prev));
        (h, app, prev, mirror, ms, out)
    }

    #[test]
    fn a_password_field_reaches_the_client_as_a_password_input_with_no_value() {
        // The role is the only thing that tells a screen reader to stop speaking the typed
        // characters: AccessKit derives every platform's password signal from it alone.
        const SECRET: &str = "hunter2-secret";
        let mut text = SECRET.to_owned();
        let mut h = UiHarness::new((300.0, 100.0));
        let opts = FieldOptions {
            variant: Variant::Password,
            ..FieldOptions::default()
        };
        let mut out = None;
        for ms in [0, 1000] {
            h.at_ms(ms);
            out = Some(
                h.frame(|ui| {
                    ui.text_field_with("pw", Rect::new(20.0, 20.0, 200.0, 28.0), &mut text, &opts);
                })
                .1,
            );
        }
        let out = out.expect("ran");
        let update = tree_update(&out.tree, out.focused, &root(), &mut NodeCache::default());
        let node = sent(&update, id_of(&out, Role::PasswordInput));
        assert_eq!(node.role(), AkRole::PasswordInput);
        assert_eq!(node.value(), None);
        assert!(node.children().is_empty() && node.text_selection().is_none());
        assert!(!format!("{update:?}").contains(SECRET));
    }

    #[test]
    fn the_frame_gives_the_expected_nodes() {
        let (_, _, _, mirror, _, out) = start();
        mirror.walk();
        let update = tree_update(&out.tree, out.focused, &root(), &mut NodeCache::default());
        assert!(update.tree.is_some());

        let check = sent(&update, id_of(&out, Role::Checkbox));
        assert_eq!(check.role(), AkRole::CheckBox);
        assert_eq!(check.label(), Some("Remember"));
        assert_eq!(check.toggled(), Some(Toggled::False));
        assert!(check.supports_action(AkAction::Click));

        let slider = sent(&update, id_of(&out, Role::Slider));
        assert_eq!(slider.numeric_value(), Some(30.0));
        assert_eq!(slider.max_numeric_value(), Some(100.0));
        assert!(slider.supports_action(AkAction::Increment));

        let field = sent(&update, id_of(&out, Role::TextInput));
        assert_eq!(field.role(), AkRole::TextInput);
        assert_eq!(field.value(), Some("lune"));
        assert_eq!(field.children().len(), 1);
        assert!(field.text_selection().is_some());

        let list = sent(&update, wid("list"));
        assert_eq!(list.children(), [nid(row(0)), nid(row(1))]);
        assert_eq!(sent(&update, row(0)).role(), AkRole::ListItem);

        let dialog = sent(&update, id_of(&out, Role::Dialog));
        assert_eq!(dialog.role(), AkRole::Dialog);
        assert!(dialog.is_modal());
        assert!(!dialog.supports_action(AkAction::HideTooltip));
    }

    #[test]
    fn a_second_identical_frame_sends_no_nodes() {
        let (mut h, mut app, mut prev, mut mirror, mut ms, _) = start();
        let out = settled(&mut h, &mut app, &mut ms);
        let update = tree_update(&out.tree, out.focused, &root(), &mut prev);
        assert!(update.nodes.is_empty(), "{:?}", ids(&update));
        assert_eq!(update.tree, None);
        mirror.apply(update);
        mirror.walk();
    }

    #[test]
    fn a_toggled_checkbox_sends_just_it() {
        let (mut h, mut app, mut prev, mut mirror, mut ms, _) = start();
        app.checked = true;
        let out = settled(&mut h, &mut app, &mut ms);
        let update = tree_update(&out.tree, out.focused, &root(), &mut prev);
        let check = id_of(&out, Role::Checkbox);
        assert_eq!(ids(&update), HashSet::from([nid(check)]));
        assert_eq!(sent(&update, check).toggled(), Some(Toggled::True));
        mirror.apply(update);
        mirror.walk();
    }

    #[test]
    fn a_removed_child_changes_the_parent_and_sends_nothing_of_itself() {
        let (mut h, mut app, mut prev, mut mirror, mut ms, _) = start();
        app.rows = 1;
        let out = settled(&mut h, &mut app, &mut ms);
        let update = tree_update(&out.tree, out.focused, &root(), &mut prev);
        assert_eq!(ids(&update), HashSet::from([nid(wid("list"))]));
        assert_eq!(sent(&update, wid("list")).children(), [nid(row(0))]);
        mirror.apply(update);
        assert!(!mirror.walk().contains(&nid(row(1))));
    }

    #[test]
    fn an_added_child_sends_it_and_the_parent() {
        let (mut h, mut app, mut prev, mut mirror, mut ms, _) = start();
        app.rows = 3;
        let out = settled(&mut h, &mut app, &mut ms);
        let update = tree_update(&out.tree, out.focused, &root(), &mut prev);
        assert_eq!(ids(&update), HashSet::from([nid(wid("list")), nid(row(2))]));
        mirror.apply(update);
        assert!(mirror.walk().contains(&nid(row(2))));
    }

    #[test]
    fn a_cleared_cache_resends_everything_with_the_tree_info() {
        let (mut h, mut app, mut prev, mut mirror, mut ms, first) = start();
        let out = settled(&mut h, &mut app, &mut ms);
        prev.clear();
        let update = tree_update(&out.tree, out.focused, &root(), &mut prev);
        let every = tree_update(
            &first.tree,
            first.focused,
            &root(),
            &mut NodeCache::default(),
        );
        assert_eq!(ids(&update), ids(&every));
        assert!(update.tree.is_some());
        mirror.apply(update);
        mirror.walk();
    }

    #[test]
    fn focus_names_the_node_and_falls_back_to_the_window_on_a_stale_id() {
        let (mut h, mut app, mut prev, mut mirror, mut ms, _) = start();
        let out = settled(&mut h, &mut app, &mut ms);
        let window = NodeId(u64::MAX);
        let on = id_of(&out, Role::Checkbox);
        let update = tree_update(&out.tree, Some(on), &root(), &mut prev);
        assert_eq!(update.focus, nid(on));
        mirror.apply(update);
        let update = tree_update(&out.tree, Some(wid("gone")), &root(), &mut prev);
        assert_eq!(update.focus, window);
        mirror.apply(update);
        let update = tree_update(&out.tree, None, &root(), &mut prev);
        assert_eq!(update.focus, window);
        mirror.apply(update);
        mirror.walk();
    }

    fn real_frame(
        store: &mut UiStore,
        translator: &mut Translator,
        ms: u64,
        checked: &mut bool,
    ) -> FrameOutput {
        let raw = translator.take(Duration::from_millis(ms));
        let strings = StringTable::default();
        let shortcuts = Shortcuts::default();
        let config = FrameConfig {
            theme: DARK,
            tier: MotionTier::Full,
            strings: &strings,
            arg_format: &DisplayFormat,
            keymap: Keymap::MAC,
            shortcuts: &shortcuts,
        };
        let mut text = FakeText;
        let mut ui = Ui::begin(store, &raw, config, &mut text);
        ui.checkbox(
            "check",
            Rect::new(20.0, 20.0, 120.0, 24.0),
            checked,
            "Remember",
        );
        ui.finish().1
    }

    #[test]
    fn a_click_from_a_client_toggles_a_checkbox_end_to_end() {
        let mut store = UiStore::default();
        let mut translator = Translator::new(PhysicalSize::new(800, 600), 1.0);
        let mut checked = false;
        let out = real_frame(&mut store, &mut translator, 16, &mut checked);
        let id = id_of(&out, Role::Checkbox);

        let action = action_from(&ActionRequest {
            action: AkAction::Click,
            target_tree: TreeId::ROOT,
            target_node: nid(id),
            data: None,
        });
        translator.access(action.expect("mapped"));
        real_frame(&mut store, &mut translator, 32, &mut checked);
        assert!(checked);
    }

    #[test]
    fn core_ignores_every_action_on_an_id_it_cannot_find() {
        let (mut h, mut app, _, _, mut ms, before) = start();
        let values = [
            None,
            Some(AccessValue::Text("x".to_owned())),
            Some(AccessValue::Number(1.0)),
        ];
        for action in [
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
        ] {
            for value in &values {
                h.input.access(AccessAction {
                    target: WidgetId::from_value(0xDEAD),
                    action,
                    value: value.clone(),
                });
            }
        }
        let out = settled(&mut h, &mut app, &mut ms);
        assert_eq!(out.tree, before.tree);
        assert_eq!(out.focused, before.focused);
        assert!(app.dialog && !app.checked);
    }
}

mod actions {
    use super::*;

    fn req(action: AkAction, data: Option<ActionData>) -> ActionRequest {
        ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node: NodeId(77),
            data,
        }
    }

    fn data_forms() -> Vec<ActionData> {
        let at = Point::new(1.0, 2.0);
        let position = TextPosition {
            node: NodeId(1),
            character_index: 0,
        };
        vec![
            ActionData::CustomAction(1),
            ActionData::Value("v".into()),
            ActionData::NumericValue(1.5),
            ActionData::ScrollUnit(ScrollUnit::Page),
            ActionData::ScrollHint(ScrollHint::TopEdge),
            ActionData::ScrollToPoint(at),
            ActionData::SetScrollOffset(at),
            ActionData::SetTextSelection(TextSelection {
                anchor: position,
                focus: position,
            }),
        ]
    }

    #[test]
    fn each_mapped_action_maps_with_any_data() {
        for (theirs, ours) in [
            (AkAction::Click, Action::Default),
            (AkAction::Focus, Action::Focus),
            (AkAction::Increment, Action::Increment),
            (AkAction::Decrement, Action::Decrement),
            (AkAction::Expand, Action::Expand),
            (AkAction::Collapse, Action::Collapse),
            (AkAction::ScrollIntoView, Action::ScrollIntoView),
            (AkAction::ShowContextMenu, Action::ShowMenu),
            (AkAction::HideTooltip, Action::Dismiss),
        ] {
            let want = AccessAction {
                target: WidgetId::from_value(77),
                action: ours,
                value: None,
            };
            assert_eq!(action_from(&req(theirs, None)).as_ref(), Some(&want));
            for data in data_forms() {
                assert_eq!(action_from(&req(theirs, Some(data))).as_ref(), Some(&want));
            }
        }
    }

    #[test]
    fn every_other_action_is_refused_with_any_data() {
        for action in [
            AkAction::Blur,
            AkAction::CustomAction,
            AkAction::ReplaceSelectedText,
            AkAction::ScrollDown,
            AkAction::ScrollLeft,
            AkAction::ScrollRight,
            AkAction::ScrollUp,
            AkAction::ScrollToPoint,
            AkAction::SetScrollOffset,
            AkAction::SetTextSelection,
            AkAction::SetSequentialFocusNavigationStartingPoint,
            AkAction::ShowTooltip,
        ] {
            assert_eq!(action_from(&req(action, None)), None, "{action:?}");
            for data in data_forms() {
                assert_eq!(action_from(&req(action, Some(data))), None, "{action:?}");
            }
        }
    }

    #[test]
    fn set_value_takes_text_or_a_finite_number_and_nothing_else() {
        let set = |data| action_from(&req(AkAction::SetValue, data)).and_then(|a| a.value);
        assert_eq!(
            set(Some(ActionData::Value("hi".into()))),
            Some(AccessValue::Text("hi".to_owned()))
        );
        assert_eq!(
            set(Some(ActionData::NumericValue(2.5))),
            Some(AccessValue::Number(2.5))
        );
        assert_eq!(action_from(&req(AkAction::SetValue, None)), None);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let data = Some(ActionData::NumericValue(bad));
            assert_eq!(action_from(&req(AkAction::SetValue, data)), None, "{bad}");
        }
        for data in data_forms() {
            if !matches!(data, ActionData::Value(_) | ActionData::NumericValue(_)) {
                assert_eq!(action_from(&req(AkAction::SetValue, Some(data))), None);
            }
        }
    }

    #[test]
    fn set_value_text_over_the_paste_limit_is_refused_whole() {
        let at_limit = "a".repeat(MAX_PASTE_BYTES);
        let data = Some(ActionData::Value(at_limit.as_str().into()));
        assert!(action_from(&req(AkAction::SetValue, data)).is_some());
        let over = "a".repeat(MAX_PASTE_BYTES + 1);
        let data = Some(ActionData::Value(over.as_str().into()));
        assert_eq!(action_from(&req(AkAction::SetValue, data)), None);
    }

    #[test]
    fn a_request_for_another_tree_is_refused() {
        let mut r = req(AkAction::Click, None);
        r.target_tree = TreeId(accesskit::Uuid::from_u128(1));
        assert_eq!(action_from(&r), None);
    }

    #[test]
    fn any_node_id_passes() {
        for raw in [0, 1, u64::MAX] {
            let mut r = req(AkAction::Click, None);
            r.target_node = NodeId(raw);
            assert_eq!(action_from(&r).map(|a| a.target.value()), Some(raw));
        }
    }
}
