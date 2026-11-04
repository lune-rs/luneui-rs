//! The wizard steps (`widget.steps` acceptance): rows of 32 with radius 6, the 20 px number circle
//! and its states, the 148 px label column that elides and never wraps, the lead line, no input,
//! and the `Navigation` over a `List` of `ListItem`s with `Selected`, `Current` and `Checked`.

use lune_ui_core::access::{Role, SemanticTree, State};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{Key, NamedKey, Repaint};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape, TextItem};
use lune_ui_core::testing::{FakeText, ScriptedInput, UiHarness};
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK;

const WIN: (f32, f32) = (600.0, 400.0);
const RECT: Rect = Rect::new(20.0, 30.0, 240.0, 0.0);
const LABELS: [&str; 4] = ["Account", "Storage", "Review", "Finish"];

struct Drawn {
    scene: Scene,
    tree: SemanticTree,
    height: f32,
    repaint: Repaint,
}

fn draw(h: &mut UiHarness, labels: &[&str], current: usize, lead: Option<&str>) -> Drawn {
    let mut height = 0.0;
    let (scene, out) = h.frame(|ui| {
        height = match lead {
            Some(l) => ui.steps_with_lead("s", RECT, labels, current, Some(l)),
            None => ui.steps("s", RECT, labels, current),
        };
    });
    Drawn {
        scene,
        tree: out.tree,
        height,
        repaint: out.repaint,
    }
}

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers.iter().flat_map(|l| l.texts.clone()).collect()
}

fn under(scene: &Scene) -> Vec<Shape> {
    scene.layers[0].geometry.under.clone()
}

fn label_key(text: &str, style: Style) -> u64 {
    Spec::plain(
        text,
        Style {
            line: 20.0,
            ..style
        },
    )
    .cache_key(1.0)
}

fn number_key(n: usize, color: [u8; 4]) -> u64 {
    Spec::plain(
        n.to_string(),
        Style {
            line: 16.0,
            ..Style::ui(11.0, color).medium()
        },
    )
    .cache_key(1.0)
}

#[test]
fn four_rows_of_32_two_apart_with_the_current_one_filled_surface_selected() {
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &LABELS, 1, None);
    assert_eq!(d.height, 4.0 * 32.0 + 3.0 * 2.0);
    let fills: Vec<Shape> = under(&d.scene)
        .into_iter()
        .filter(|s| s.color == premul(DARK.surface_selected))
        .collect();
    assert_eq!(fills.len(), 1, "only the current row is filled");
    // The second row: 30 + 32 + 2.
    assert_eq!(fills[0].rect, [20.0, 64.0, 240.0, 32.0]);
    assert_eq!(fills[0].radii, [6.0; 4]);
    assert_eq!(d.repaint, Repaint::Idle, "no motion");
}

#[test]
fn at_scale_2_a_row_is_64_px_the_circle_40_and_the_label_column_296() {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let d = draw(&mut h, &LABELS, 0, None);
    let fill = under(&d.scene)
        .into_iter()
        .find(|s| s.color == premul(DARK.surface_selected))
        .expect("the current row");
    assert_eq!(fill.rect[3], 64.0);
    let ring = under(&d.scene)
        .into_iter()
        .find(|s| s.radii == [20.0; 4] && s.params[1] == 2.0)
        .expect("the circle's ring, 2 px wide");
    assert_eq!(ring.rect[2], 40.0);
    // A long label is cut at 148 logical px: the column is 296 device px.
    let long = "L".repeat(80);
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let d = draw(&mut h, &[&long], 0, None);
    let drawn = texts(&d.scene).into_iter().last().expect("the label");
    let whole = Spec::plain(
        &long,
        Style {
            line: 20.0,
            ..Style::ui(13.0, DARK.fg_bright).medium()
        },
    );
    let cut = lune_ui_core::text::elide(&mut FakeText, &whole, 2.0, 148.0);
    assert_eq!(drawn.key.0, cut.cache_key(2.0));
    assert!(FakeText.measure(&cut, 2.0).0 <= 148.0);
}

#[test]
fn an_upcoming_step_is_tertiary_over_a_ring_number() {
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &LABELS, 0, None);
    let t = texts(&d.scene);
    // Row 1 (index 1) is upcoming: its number then its label.
    assert!(t.iter().any(|i| i.key.0 == number_key(2, DARK.fg_tertiary)));
    assert!(
        t.iter()
            .any(|i| i.key.0 == label_key("Storage", Style::ui(13.0, DARK.fg_tertiary)))
    );
    let ring = under(&d.scene)
        .into_iter()
        .filter(|s| s.color == premul(DARK.border) && s.radii == [10.0; 4])
        .count();
    assert_eq!(ring, 4, "no step is done, so every row rings its number");
}

#[test]
fn the_current_step_is_bright_and_medium() {
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &LABELS, 1, None);
    let t = texts(&d.scene);
    assert!(t.iter().any(|i| i.key.0 == number_key(2, DARK.fg_bright)));
    assert!(
        t.iter()
            .any(|i| i.key.0 == label_key("Storage", Style::ui(13.0, DARK.fg_bright).medium()))
    );
}

#[test]
fn a_done_step_is_an_accent_circle_with_a_12_px_check_and_a_secondary_label() {
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &LABELS, 2, None);
    let accent: Vec<Shape> = under(&d.scene)
        .into_iter()
        .filter(|s| s.color == premul(DARK.accent))
        .collect();
    assert_eq!(accent.len(), 2, "rows 0 and 1 are done");
    assert_eq!(accent[0].rect, [28.0, 36.0, 20.0, 20.0]);
    assert_eq!(accent[0].radii, [10.0; 4]);
    let t = texts(&d.scene);
    let check = icon::glyph(icon::CHECK, 12.0, DARK.on_accent);
    assert_eq!(
        t.iter().filter(|i| i.key.0 == check.cache_key(1.0)).count(),
        2
    );
    assert!(
        t.iter()
            .any(|i| i.key.0 == label_key("Account", Style::ui(13.0, DARK.fg_secondary)))
    );
    // A done row shows no number.
    assert!(!t.iter().any(|i| i.key.0 == number_key(1, DARK.fg_tertiary)));
}

#[test]
fn a_label_never_wraps_and_never_leaves_its_148_px_column() {
    let long = "A very long step label that does not fit";
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &[long], 0, None);
    assert_eq!(d.height, 32.0, "one row, not wrapped");
    let drawn = texts(&d.scene).into_iter().last().expect("the label");
    let style = Style {
        line: 20.0,
        ..Style::ui(13.0, DARK.fg_bright).medium()
    };
    let whole = Spec::plain(long, style);
    assert_ne!(
        drawn.key.0,
        whole.cache_key(1.0),
        "the drawn label is the cut one"
    );
    // The cut label fits the column on the fake text.
    let cut = lune_ui_core::text::elide(&mut FakeText, &whole, 1.0, 148.0);
    assert!(FakeText.measure(&cut, 1.0).0 <= 148.0);
    assert_eq!(drawn.key.0, cut.cache_key(1.0));
}

#[test]
fn the_lead_line_sits_12_px_below_the_last_row_and_adds_to_the_height() {
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &LABELS, 0, Some("Set up"));
    assert_eq!(d.height, 4.0 * 32.0 + 6.0 + 12.0 + 20.0);
    let lead = texts(&d.scene).into_iter().last().expect("the lead");
    assert_eq!((lead.x, lead.y), (20.0, 30.0 + 134.0 + 12.0));
    let spec = Spec::plain(
        "Set up",
        Style {
            line: 20.0,
            ..Style::ui(13.0, DARK.fg_tertiary)
        },
    )
    .wrapped(240.0);
    assert_eq!(lead.key.0, spec.cache_key(1.0));
}

#[test]
fn steps_take_no_input_and_no_tab_stop() {
    let mut h = UiHarness::new(WIN);
    draw(&mut h, &LABELS, 1, None);
    h.input.pointer_to(100.0, 80.0);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    let d = draw(&mut h, &LABELS, 1, None);
    assert!(
        d.tree
            .nodes
            .iter()
            .all(|n| !n.states.contains(State::Focusable))
    );
    assert!(
        d.tree
            .nodes
            .iter()
            .all(|n| !n.states.contains(State::Focused))
    );
    assert_eq!(d.repaint, Repaint::Idle);
}

#[test]
fn the_node_is_a_navigation_over_a_list_of_items_with_selected_current_and_checked() {
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &LABELS, 2, None);
    let nav = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Navigation)
        .expect("navigation");
    let list = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::List)
        .expect("list");
    assert_eq!(list.parent, Some(nav.id));
    let items: Vec<_> = d
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::ListItem)
        .collect();
    assert_eq!(items.len(), 4);
    assert!(items.iter().all(|n| n.parent == Some(list.id)));
    let names: Vec<&str> = items.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, LABELS);
    let flags = |n: &&lune_ui_core::access::Node| {
        (
            n.states.contains(State::Selected),
            n.states.contains(State::Current),
            n.states.contains(State::Checked),
        )
    };
    let got: Vec<_> = items.iter().map(flags).collect();
    assert_eq!(
        got,
        [
            (false, false, true),
            (false, false, true),
            (true, true, false),
            (false, false, false)
        ]
    );
}

#[test]
fn a_long_label_is_named_in_full_on_its_node() {
    let long = "A very long step label that does not fit";
    let mut h = UiHarness::new(WIN);
    let d = draw(&mut h, &[long], 0, None);
    let item = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::ListItem)
        .expect("item");
    assert_eq!(item.name, long);
}
