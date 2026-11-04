//! Semantic nodes and accessibility actions (C4, infra.accessibility acceptance): D14 routing,
//! modal drops, node bounds under transforms, tree order and parents, the focused node, and the
//! frame's string lookups.

use lune_ui_core::access::{
    AccessAction, AccessValue, Action, Actions, Node, Role, SemanticTree, State, States,
};
use lune_ui_core::input::{FrameOutput, NamedKey, PointerButton};
use lune_ui_core::motion::{Kind, Motion};
use lune_ui_core::paint::{LayerId, Rect, Xform};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::{Dismissal, OverlayOptions, Response, Sense, Ui, WidgetId};

fn harness() -> UiHarness {
    UiHarness::new((400.0, 300.0))
}

fn node(id: WidgetId, role: Role, bounds: Rect, states: States) -> Node {
    Node {
        id,
        role,
        name: "n".to_owned(),
        description: None,
        value: None,
        text: None,
        bounds,
        parent: None,
        children: Vec::new(),
        live: None,
        states,
        actions: Actions::default(),
        sort: None,
    }
}

const BOX: Rect = Rect::new(10.0, 10.0, 20.0, 20.0);

/// A test checkbox: click or Space toggles `on`; emits a checkbox node.
fn checkbox(ui: &mut Ui<'_>, key: &str, rect: Rect, on: &mut bool) -> Response {
    let id = ui.id(key);
    let mut r = ui.interact(id, rect, Sense::CLICK_FOCUS);
    if r.clicked || ui.consume_key(id, NamedKey::Space) {
        *on = !*on;
        r.mark_changed();
    }
    let mut states = States::default();
    if *on {
        states.insert(State::Checked);
    }
    ui.node(node(id, Role::Checkbox, rect, states));
    r
}

fn act(h: &mut UiHarness, key: &str, action: Action, value: Option<AccessValue>) {
    h.input.access(AccessAction {
        target: WidgetId::ROOT.with(key),
        action,
        value,
    });
}

#[test]
fn default_clicks_a_checkbox_and_sets_changed() {
    let mut h = harness();
    let mut on = false;
    let mut changed = false;
    h.frame(|ui| {
        checkbox(ui, "cb", BOX, &mut on);
    });
    act(&mut h, "cb", Action::Default, None);
    h.frame(|ui| changed = checkbox(ui, "cb", BOX, &mut on).changed);
    assert!(on && changed);
}

#[test]
fn focus_focuses_without_a_ring() {
    let mut h = harness();
    let mut on = false;
    h.frame(|ui| {
        checkbox(ui, "cb", BOX, &mut on);
    });
    act(&mut h, "cb", Action::Focus, None);
    let mut r = None;
    h.frame(|ui| r = Some(checkbox(ui, "cb", BOX, &mut on)));
    let r = r.expect("ran");
    assert!(r.focused && !r.focus_visible);
}

#[test]
fn increment_and_decrement_reach_the_target_as_up_and_down() {
    let mut h = harness();
    let step = |ui: &mut Ui<'_>, v: &mut i32| {
        let id = ui.id("stepper");
        ui.interact(id, BOX, Sense::CLICK_FOCUS);
        while ui.consume_key(id, NamedKey::ArrowUp) {
            *v += 1;
        }
        while ui.consume_key(id, NamedKey::ArrowDown) {
            *v -= 1;
        }
        ui.node(node(id, Role::SpinButton, BOX, States::default()));
    };
    let mut v = 0;
    h.frame(|ui| step(ui, &mut v));
    act(&mut h, "stepper", Action::Increment, None);
    act(&mut h, "stepper", Action::Increment, None);
    act(&mut h, "stepper", Action::Decrement, None);
    let mut focused = true;
    h.frame(|ui| {
        step(ui, &mut v);
        focused = ui
            .previous_tree()
            .get(WidgetId::ROOT.with("stepper"))
            .is_some();
    });
    assert_eq!(v, 1, "consumable without focus");
    assert!(focused);
}

#[test]
fn set_value_arrives_in_the_response() {
    let mut h = harness();
    let mut on = false;
    h.frame(|ui| {
        checkbox(ui, "cb", BOX, &mut on);
    });
    act(
        &mut h,
        "cb",
        Action::SetValue,
        Some(AccessValue::Number(2.5)),
    );
    let mut r = None;
    h.frame(|ui| r = Some(checkbox(ui, "cb", BOX, &mut on)));
    assert_eq!(r.expect("ran").set_value, Some(AccessValue::Number(2.5)));
}

#[test]
fn scroll_into_view_and_show_menu_are_routed() {
    let mut h = harness();
    let mut on = false;
    h.frame(|ui| {
        checkbox(ui, "cb", BOX, &mut on);
    });
    act(&mut h, "cb", Action::ScrollIntoView, None);
    act(&mut h, "cb", Action::ShowMenu, None);
    let mut r = None;
    h.frame(|ui| r = Some(checkbox(ui, "cb", BOX, &mut on)));
    let r = r.expect("ran");
    assert!(r.scroll_into_view && r.secondary_clicked && !r.clicked);
}

/// A disclosure: a click toggles `open`; its node is expanded when open.
fn disclosure(ui: &mut Ui<'_>, open: &mut bool) {
    let id = ui.id("more");
    if ui.interact(id, BOX, Sense::CLICK_FOCUS).clicked {
        *open = !*open;
    }
    let mut s = States::default();
    if *open {
        s.insert(State::Expanded);
    }
    ui.node(node(id, Role::Button, BOX, s));
}

#[test]
fn expand_clicks_only_when_collapsed_and_collapse_only_when_expanded() {
    let mut h = harness();
    let mut open = false;
    h.frame(|ui| disclosure(ui, &mut open));
    act(&mut h, "more", Action::Collapse, None);
    h.frame(|ui| disclosure(ui, &mut open));
    assert!(!open, "collapse on a collapsed node does nothing");
    act(&mut h, "more", Action::Expand, None);
    h.frame(|ui| disclosure(ui, &mut open));
    assert!(open);
    act(&mut h, "more", Action::Expand, None);
    h.frame(|ui| disclosure(ui, &mut open));
    assert!(open, "expand on an expanded node does nothing");
    act(&mut h, "more", Action::Collapse, None);
    h.frame(|ui| disclosure(ui, &mut open));
    assert!(!open);
}

fn modal() -> OverlayOptions {
    OverlayOptions {
        layer: LayerId::Modal,
        modal: true,
        dismiss_on_escape: true,
        dismiss_on_outside_press: false,
        dismiss_on_focus_loss: false,
        anchor: None,
        initial_focus: None,
        open: Motion::of(Kind::Appear),
        close: Motion::of(Kind::Dismiss),
    }
}

#[test]
fn dismiss_dismisses_the_top_overlay_and_actions_under_a_modal_are_dropped() {
    let mut h = harness();
    let mut on = false;
    let run = |h: &mut UiHarness, on: &mut bool| {
        let mut d = None;
        let mut page = None;
        h.frame(|ui| {
            ui.painter().on(LayerId::Page);
            page = Some(checkbox(ui, "cb", BOX, on));
            let (f, ()) = ui.overlay(
                "dialog",
                &modal(),
                |_, _, _| {},
                |ui, _| {
                    let id = ui.id("card");
                    ui.interact(id, Rect::new(100.0, 100.0, 200.0, 100.0), Sense::HOVER);
                    ui.node(node(
                        id,
                        Role::Dialog,
                        Rect::new(100.0, 100.0, 200.0, 100.0),
                        States::default(),
                    ));
                },
            );
            d = f.dismissed;
        });
        (d, page.expect("ran"))
    };
    run(&mut h, &mut on);
    act(&mut h, "cb", Action::Default, None);
    act(&mut h, "cb", Action::Focus, None);
    let (d, page) = run(&mut h, &mut on);
    assert!(
        !on && !page.changed && !page.focused,
        "dropped under the modal"
    );
    assert_eq!(d, None);
    act(&mut h, "card", Action::Dismiss, None);
    let (d, _) = run(&mut h, &mut on);
    assert_eq!(d, Some(Dismissal::Access));
}

#[test]
fn an_action_for_a_node_missing_last_frame_is_dropped() {
    let mut h = harness();
    let mut on = false;
    act(&mut h, "cb", Action::Default, None);
    let mut r = None;
    h.frame(|ui| r = Some(checkbox(ui, "cb", BOX, &mut on)));
    assert!(!on && !r.expect("ran").clicked);
}

#[test]
fn node_bounds_follow_the_transform_stack() {
    let mut h = harness();
    let x = Xform {
        origin: (200.0, 150.0),
        scale: 0.97,
        dx: 0.0,
        dy: 4.0,
        alpha: 0.5,
    };
    let mut want = None;
    let (_, out) = h.frame(|ui| {
        ui.painter().push_xform(x);
        want = Some(ui.painter().transformed(BOX));
        let id = ui.id("b");
        ui.node(node(id, Role::Button, BOX, States::default()));
        ui.painter().pop_xform();
    });
    let got = out
        .tree
        .get(WidgetId::ROOT.with("b"))
        .expect("emitted")
        .bounds;
    assert_eq!(Some(got), want);
    assert_ne!(got, BOX);
}

fn tree_frame(h: &mut UiHarness) -> FrameOutput {
    h.frame(|ui| {
        let list = ui.id("list");
        ui.node_container(node(list, Role::List, BOX, States::default()), |ui| {
            for k in ["r0", "r1"] {
                let id = ui.id(k);
                ui.interact(id, BOX, Sense::CLICK_FOCUS);
                ui.node(node(id, Role::ListItem, BOX, States::default()));
            }
            let group = ui.id("g");
            ui.node_container(node(group, Role::Group, BOX, States::default()), |ui| {
                let id = ui.id("inner");
                ui.node(node(id, Role::StaticText, BOX, States::default()));
            });
        });
        let id = ui.id("after");
        ui.node(node(id, Role::Button, BOX, States::default()));
    })
    .1
}

#[test]
fn tree_order_and_parent_links() {
    let mut h = harness();
    let out = tree_frame(&mut h);
    let id = |k: &str| WidgetId::ROOT.with(k);
    let ids: Vec<WidgetId> = out.tree.nodes.iter().map(|n| n.id).collect();
    assert_eq!(
        ids,
        [
            id("list"),
            id("r0"),
            id("r1"),
            id("g"),
            id("inner"),
            id("after")
        ]
    );
    let t: &SemanticTree = &out.tree;
    let get = |k: &str| t.get(id(k)).expect("node");
    assert_eq!(get("list").parent, None);
    assert_eq!(get("list").children, [id("r0"), id("r1"), id("g")]);
    assert_eq!(get("r1").parent, Some(id("list")));
    assert_eq!(get("g").children, [id("inner")]);
    assert_eq!(get("inner").parent, Some(id("g")));
    assert_eq!(get("after").parent, None);
}

#[test]
fn the_focused_output_is_the_node_with_the_focused_state() {
    let mut h = harness();
    let out = tree_frame(&mut h);
    assert_eq!(out.focused, None);
    h.input.pointer_to(15.0, 15.0).click(PointerButton::Primary);
    let out = tree_frame(&mut h);
    let r1 = WidgetId::ROOT.with("r1");
    assert_eq!(out.focused, Some(r1), "the press focused the topmost stop");
    let marked: Vec<WidgetId> = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.states.contains(State::Focused))
        .map(|n| n.id)
        .collect();
    assert_eq!(marked, [r1]);
    // Focus on an id without a node: no focused output.
    let (_, out) = h.frame(|ui| {
        let id = ui.id("r1");
        ui.interact(id, BOX, Sense::CLICK_FOCUS);
    });
    assert_eq!(out.focused, None);
}

#[test]
fn tr_falls_back_from_the_table_to_the_default_to_the_key() {
    let mut h = harness();
    h.strings
        .insert("app.greeting", "Hello {0}")
        .expect("app key");
    h.frame(|ui| {
        let (a, b, c) = (
            ui.tr("app.greeting"),
            ui.tr("luneui.dialog.cancel"),
            ui.tr("app.missing"),
        );
        // `tr` borrows the table and the key, not the frame.
        ui.painter();
        assert_eq!((a, b, c), ("Hello {0}", "Cancel", "app.missing"));
        assert_eq!(ui.tr_fmt("app.greeting", &[&"Ada"]), "Hello Ada");
    });
    h.strings
        .insert("luneui.dialog.cancel", "Abbrechen")
        .expect("a default key");
    h.frame(|ui| assert_eq!(ui.tr("luneui.dialog.cancel"), "Abbrechen"));
}
