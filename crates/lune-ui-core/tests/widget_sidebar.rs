//! The sidebar (`layout.sidebar` acceptance): the nav padding and pitch, the footer's padding,
//! hairline, dot and text at scale 2, the ok and error footers, a click navigating once, the
//! footer ignoring input, the wizard with no links, one tab stop, the nodes and the disabled walk.

use lune_ui_core::access::{Live, Role, SemanticTree, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, TextItem};
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style, TextKey, elide};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::nav_item::NavEntry;
use lune_ui_core::widgets::sidebar::{
    FOOTER_H, MARGIN_BOTTOM, Sidebar, SidebarBody, SidebarFooter, WIDTH, footer_rect,
};
use lune_ui_core::widgets::tone::Tone;

const WIN: (f32, f32) = (400.0, 600.0);
/// The column at (20, 30), 208 wide and 500 high.
const COL: Rect = Rect::new(20.0, 30.0, WIDTH, 500.0);
const NAMES: [&str; 4] = ["Overview", "Sessions", "Requests", "Settings"];
const STEPS: [&str; 3] = ["Welcome", "Connect", "Finish"];
const OK: &str = "Proxy on 127.0.0.1:4646";

fn sid() -> WidgetId {
    WidgetId::ROOT.with("side")
}

fn row_id(key: u64) -> WidgetId {
    sid().with("nav").with(key)
}

fn row_centre(i: usize) -> (f32, f32) {
    (COL.x + 8.0 + 92.0, COL.y + 8.0 + 34.0 * i as f32 + 16.0)
}

#[derive(Clone)]
struct Case {
    wizard: bool,
    selected: u64,
    current: usize,
    footer: Option<SidebarFooter<'static>>,
    name: &'static str,
    enabled: bool,
    rect: Rect,
}

fn ok_footer() -> Option<SidebarFooter<'static>> {
    Some(SidebarFooter {
        tone: Tone::Ok,
        text: OK,
        mono: true,
    })
}

fn case() -> Case {
    Case {
        wizard: false,
        selected: 1,
        current: 1,
        footer: ok_footer(),
        name: "Main navigation",
        enabled: true,
        rect: COL,
    }
}

struct Out {
    scene: Scene,
    activated: Option<u64>,
    focused: Option<WidgetId>,
    tree: SemanticTree,
    repaint: Repaint,
    fired: usize,
}

fn frame(h: &mut UiHarness, c: &mut Case) -> Out {
    let mut activated = None;
    let (scene, out) = h.frame(|ui| {
        let entries: Vec<NavEntry<'_>> = NAMES
            .iter()
            .enumerate()
            .map(|(i, label)| NavEntry {
                key: i as u64 + 1,
                label,
                icon: None,
                enabled: true,
            })
            .collect();
        let body = if c.wizard {
            SidebarBody::Steps {
                labels: &STEPS,
                current: c.current,
            }
        } else {
            SidebarBody::Nav {
                entries: &entries,
                selected: &mut c.selected,
            }
        };
        let side = Sidebar {
            body,
            footer: c.footer,
            name: c.name,
        };
        activated = ui.with_enabled(c.enabled, |ui| ui.sidebar("side", c.rect, side));
    });
    Out {
        scene,
        activated,
        focused: out.focused,
        tree: out.tree,
        repaint: out.repaint,
        fired: out.fired.len(),
    }
}

fn settled(h: &mut UiHarness, c: &mut Case) -> Out {
    frame(h, c);
    h.at_ms(10);
    frame(h, c);
    h.at_ms(5000);
    frame(h, c)
}

fn tab(h: &mut UiHarness, c: &mut Case) -> Out {
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(20);
    frame(h, c)
}

fn key_of(spec: &Spec, scale: f32) -> TextKey {
    TextKey(spec.cache_key(scale))
}

fn find_text(scene: &Scene, spec: &Spec, scale: f32) -> Option<TextItem> {
    scene.layers[0]
        .texts
        .iter()
        .find(|t| t.key == key_of(spec, scale))
        .cloned()
}

fn footer_style(mono: bool) -> Style {
    if mono {
        Style::mono(DARK.font_s, 16.0, DARK.fg_tertiary)
    } else {
        Style {
            line: 16.0,
            ..Style::ui(DARK.font_s, DARK.fg_tertiary)
        }
    }
}

#[test]
fn the_nav_is_padded_8_and_the_rows_are_34_apart_with_no_fill_of_the_column() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &mut case());
    let g = &out.scene.layers[0].geometry;
    // The selected row is the one fill under the text: inset 8, 32 high.
    let sel = g
        .under
        .iter()
        .find(|s| s.color == premul(DARK.surface_selected))
        .expect("the selected row's fill");
    assert_eq!(sel.rect, [28.0, 38.0, 192.0, 32.0]);
    assert!(g.acrylic.is_empty(), "the column has no glass of its own");
    // The label sits at the row's padding; the third row is two pitches down.
    let third = Spec::plain("Requests", Style::ui(DARK.font_base, DARK.fg_secondary));
    let t = find_text(&out.scene, &third, 1.0).expect("third label");
    assert_eq!(t.x, 28.0 + 8.0);
    let second = Spec::plain("Sessions", Style::ui(DARK.font_base, DARK.fg_secondary));
    let s = find_text(&out.scene, &second, 1.0).expect("second label");
    assert_eq!(t.y - s.y, 34.0);
    assert_eq!(out.repaint, Repaint::Idle);
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let out = settled(&mut h, &mut case());
    let g = &out.scene.layers[0].geometry;
    // Column 416 px wide; the nav padding is 16 px.
    let sel = g
        .under
        .iter()
        .find(|s| s.color == premul(DARK.surface_selected))
        .expect("selected");
    assert_eq!(sel.rect, [56.0, 76.0, 384.0, 64.0]);
    assert_eq!(COL.w * 2.0, 416.0);
    assert_eq!(sel.rect[0] - COL.x * 2.0, 16.0, "nav padding");
    assert_eq!(COL.x * 2.0 + 416.0 - (sel.rect[0] + sel.rect[2]), 16.0);
    // The footer: a 2 px hairline at its top over the full width, then 24 px of padding above the
    // text and 24 px below it, 32 px at the sides.
    let f = footer_rect(COL);
    assert_eq!(f.h, FOOTER_H);
    assert_eq!(f.bottom(), COL.bottom() - MARGIN_BOTTOM, "10 px gutter");
    let hair = g
        .under
        .iter()
        .find(|s| s.color == premul(DARK.border_subtle))
        .expect("hairline");
    assert_eq!(hair.rect, [f.x * 2.0, f.y * 2.0, 416.0, 2.0]);
    let dot = g
        .under
        .iter()
        .find(|s| s.color == premul(DARK.ok))
        .expect("dot");
    assert_eq!(dot.rect[0] - f.x * 2.0, 32.0, "side padding");
    assert_eq!(dot.rect[2], 16.0);
    assert_eq!(dot.radii, [8.0; 4]);
    let text = find_text(&out.scene, &Spec::plain(OK, footer_style(true)), 2.0).expect("text");
    assert_eq!(text.y * 2.0 - f.y * 2.0, 24.0, "padding above the text");
    // 40 high: 12 + 16 + 12 (24 + 32 + 24 at this scale).
    assert_eq!(FOOTER_H * 2.0, 80.0);
    // The text starts after the dot and its 8 px gap: 16 + 8 + 8 logical.
    assert_eq!(text.x - f.x, 32.0);
}

#[test]
fn the_footer_is_an_ok_dot_and_text_or_an_err_dot_and_a_stopped_line() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &mut case());
    let g = &out.scene.layers[0].geometry;
    assert!(g.under.iter().any(|s| s.color == premul(DARK.ok)));
    assert!(find_text(&out.scene, &Spec::plain(OK, footer_style(true)), 1.0).is_some());

    let mut h = UiHarness::new(WIN);
    let mut c = Case {
        footer: Some(SidebarFooter {
            tone: Tone::Err,
            text: "Proxy stopped",
            mono: false,
        }),
        ..case()
    };
    let out = settled(&mut h, &mut c);
    let g = &out.scene.layers[0].geometry;
    assert!(g.under.iter().any(|s| s.color == premul(DARK.err)));
    assert!(!g.under.iter().any(|s| s.color == premul(DARK.ok)));
    let stopped = Spec::plain("Proxy stopped", footer_style(false));
    let t = find_text(&out.scene, &stopped, 1.0).expect("stopped line");
    assert_eq!(t.x, footer_rect(COL).x + 32.0);
    // No footer: no hairline, no dot, no status node.
    let mut h = UiHarness::new(WIN);
    let out = settled(
        &mut h,
        &mut Case {
            footer: None,
            ..case()
        },
    );
    let g = &out.scene.layers[0].geometry;
    assert!(
        !g.under
            .iter()
            .any(|s| s.color == premul(DARK.border_subtle))
    );
    assert!(out.tree.nodes.iter().all(|n| n.role != Role::Status));
}

#[test]
fn a_long_footer_is_elided_to_the_room_beside_the_dot() {
    let long = "Proxy listening on 127.0.0.1:4646 and a long path ~/.wane/state/somewhere";
    let mut h = UiHarness::new(WIN);
    let mut c = Case {
        footer: Some(SidebarFooter {
            tone: Tone::Ok,
            text: long,
            mono: true,
        }),
        ..case()
    };
    let out = settled(&mut h, &mut c);
    // Room: the width less the 16 px right padding and the 32 px before the text.
    let room = WIDTH - 16.0 - 32.0;
    let want = elide(
        &mut FakeText,
        &Spec::plain(long, footer_style(true)),
        1.0,
        room,
    );
    assert_ne!(want.runs[0].0, long);
    assert!(
        find_text(&out.scene, &want, 1.0).is_some(),
        "cut with an ellipsis"
    );
    // The node still carries the whole text.
    let status = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Status)
        .expect("status");
    assert_eq!(status.name, long);
}

#[test]
fn a_click_on_a_nav_row_navigates_once_and_the_footer_ignores_input() {
    let mut h = UiHarness::new(WIN);
    let mut c = case();
    settled(&mut h, &mut c);
    let (x, y) = row_centre(2);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    h.at_ms(5100);
    let out = frame(&mut h, &mut c);
    assert_eq!(out.activated, Some(3));
    assert_eq!(c.selected, 3);
    h.at_ms(5200);
    assert_eq!(frame(&mut h, &mut c).activated, None, "once");
    // A click on the current row reports it again, changing nothing.
    h.input.click(PointerButton::Primary);
    h.at_ms(5300);
    assert_eq!(frame(&mut h, &mut c).activated, Some(3));

    // A click on the footer navigates nothing and focuses nothing.
    let f = footer_rect(COL);
    h.input
        .pointer_to(f.x + f.w / 2.0, f.y + f.h / 2.0)
        .click(PointerButton::Primary);
    h.at_ms(5400);
    let out = frame(&mut h, &mut c);
    assert_eq!(out.activated, None);
    assert_eq!(c.selected, 3);
}

#[test]
fn the_list_is_one_tab_stop_and_the_footer_is_none() {
    let mut h = UiHarness::new(WIN);
    let mut c = case();
    settled(&mut h, &mut c);
    let first = tab(&mut h, &mut c).focused;
    assert_eq!(first, Some(row_id(1)), "the selected row is the stop");
    // Tab again leaves the only stop and comes back to it: there is no second.
    let second = tab(&mut h, &mut c).focused;
    assert_eq!(second, first);
    // Enter on it navigates (to the current row: reported, not changed).
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    h.at_ms(40);
    frame(&mut h, &mut c);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    h.at_ms(60);
    assert_eq!(frame(&mut h, &mut c).activated, Some(2));
    assert_eq!(c.selected, 2);
}

#[test]
fn the_wizard_has_steps_in_the_nav_slot_and_no_links() {
    let mut h = UiHarness::new(WIN);
    let mut c = Case {
        wizard: true,
        name: "Setup steps",
        ..case()
    };
    let out = settled(&mut h, &mut c);
    // Nothing focuses and nothing activates.
    assert!(
        out.tree
            .nodes
            .iter()
            .all(|n| !n.states.contains(State::Focusable))
    );
    assert_eq!(out.activated, None);
    assert_eq!(tab(&mut h, &mut c).focused, None);
    let (x, y) = row_centre(1);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    h.at_ms(6000);
    assert_eq!(frame(&mut h, &mut c).activated, None);
    // One done step (an accent circle), a current one (the selected fill).
    let g = &out.scene.layers[0].geometry;
    assert!(g.under.iter().any(|s| s.color == premul(DARK.accent)));
    assert!(
        g.under
            .iter()
            .any(|s| s.color == premul(DARK.surface_selected))
    );
}

#[test]
fn the_nodes_are_a_group_over_a_list_and_a_polite_status() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &mut case());
    let group = out.tree.get(sid()).expect("group");
    assert_eq!(
        (group.role, group.name.as_str()),
        (Role::Group, "Main navigation")
    );
    let list = out.tree.get(sid().with("nav")).expect("list");
    assert_eq!(list.role, Role::List);
    assert_eq!(list.parent, Some(sid()));
    let items: Vec<_> = list
        .children
        .iter()
        .map(|id| out.tree.get(*id).expect("item"))
        .collect();
    assert_eq!(items.len(), 4);
    assert!(items.iter().all(|n| n.role == Role::ListItem));
    assert!(items[0].states.contains(State::Selected));
    assert!(!items[1].states.contains(State::Selected));
    // The footer is a polite status after the group, outside it.
    let status = out.tree.get(sid().with("footer")).expect("status");
    assert_eq!(status.role, Role::Status);
    assert_eq!(status.name, OK);
    assert_eq!(status.live, Some(Live::Polite));
    assert_eq!(status.value, Some(Value::Text(OK.to_owned())));
    assert!(!status.states.contains(State::Focusable));
    assert!(status.actions == Default::default());
    assert_eq!(status.parent, None);
    let i_group = out
        .tree
        .nodes
        .iter()
        .position(|n| n.id == sid())
        .expect("present");
    let i_status = out
        .tree
        .nodes
        .iter()
        .position(|n| n.id == status.id)
        .expect("present");
    assert!(i_group < i_status);
}

#[test]
fn the_steps_node_is_a_navigation_named_by_the_sidebar() {
    let mut h = UiHarness::new(WIN);
    let out = settled(
        &mut h,
        &mut Case {
            wizard: true,
            name: "Setup steps",
            ..case()
        },
    );
    let nav = out.tree.get(sid().with("steps")).expect("navigation");
    assert_eq!(
        (nav.role, nav.name.as_str()),
        (Role::Navigation, "Setup steps")
    );
    assert!(out.tree.get(sid()).is_none(), "no group of its own");
    let list = out
        .tree
        .get(sid().with("steps").with("list"))
        .expect("list");
    assert_eq!(list.children.len(), STEPS.len());
    assert!(out.tree.get(sid().with("footer")).is_some());
}

#[test]
fn a_disabled_sidebar_is_disabled_all_the_way_down() {
    for wizard in [false, true] {
        let mut h = UiHarness::new(WIN);
        let mut c = Case {
            wizard,
            enabled: false,
            ..case()
        };
        let out = settled(&mut h, &mut c);
        let root = if wizard { sid().with("steps") } else { sid() };
        for root in [root, sid().with("footer")] {
            let bad = disabled_violations(&out.tree, root);
            assert!(bad.is_empty(), "wizard {wizard}: {bad:?}");
        }
        assert_eq!(tab(&mut h, &mut c).focused, None);
    }
    // An enabled one is not marked.
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &mut case());
    assert!(
        !out.tree
            .get(sid())
            .expect("present")
            .states
            .contains(State::Disabled)
    );
}

#[test]
fn a_disabled_sidebar_consumes_no_key_a_binding_wants() {
    for k in [
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::Enter,
        NamedKey::Space,
    ] {
        let mut h = UiHarness::new(WIN);
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        let mut c = Case {
            enabled: false,
            ..case()
        };
        settled(&mut h, &mut c);
        h.input.key_down(Key::Named(k), None);
        h.at_ms(5100);
        assert_eq!(frame(&mut h, &mut c).fired, 1, "{k:?}");
    }
}

#[test]
fn the_sidebar_draws_only_on_the_layer_it_is_called_on() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &mut case());
    for (i, l) in out.scene.layers.iter().enumerate().skip(1) {
        assert!(
            l.texts.is_empty() && l.geometry.under.is_empty(),
            "layer {i}"
        );
    }
}
