//! The setting row and the item row (`widget.row` acceptance): sizes and radius, the title and the
//! `fg_muted` description, the item grid, hover read from the pointer so the control inside keeps
//! its input, the off colour at `fast`, the disabled row, and the `Group` node.

use lune_ui_core::access::{Role, SemanticTree, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape, TextItem};
use lune_ui_core::testing::{ScriptedInput, UiHarness};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::row::{HOVER, ItemRow, SELECTION, SettingRow};

const WIN: (f32, f32) = (600.0, 300.0);
const RECT: Rect = Rect::new(20.0, 30.0, 400.0, 0.0);
/// The toggle's track centre in a row of RECT with a 40 px control: x 368 + 20, y 30 + 28.
const TOGGLE: (f32, f32) = (388.0, 58.0);

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers.iter().flat_map(|l| l.texts.clone()).collect()
}

fn under(scene: &Scene) -> Vec<Shape> {
    scene.layers[0].geometry.under.clone()
}

fn plain(title: &str, description: Option<&str>) -> SettingRow<'static> {
    // The strings leak into 'static for the test's sake only.
    SettingRow {
        title: Box::leak(title.to_owned().into_boxed_str()),
        description: description.map(|d| &*Box::leak(d.to_owned().into_boxed_str())),
        off: false,
        control_w: 40.0,
    }
}

struct Drawn {
    scene: Scene,
    tree: SemanticTree,
    height: f32,
    toggle: Response,
    repaint: Repaint,
}

fn setting(h: &mut UiHarness, rect: Rect, row: &SettingRow<'_>, on: &mut bool) -> Drawn {
    let (mut height, mut toggle) = (0.0, None);
    let (scene, out) = h.frame(|ui| {
        height = ui.setting_row("row", rect, row, |ui, slot| {
            toggle = Some(ui.toggle("t", slot, on, "Switch"));
        });
    });
    Drawn {
        scene,
        tree: out.tree,
        height,
        toggle: toggle.expect("the control ran"),
        repaint: out.repaint,
    }
}

fn row_fill(scene: &Scene, bounds: [f32; 4]) -> Option<Shape> {
    under(scene).into_iter().find(|s| s.rect == bounds)
}

#[test]
fn a_setting_row_is_at_least_56_high_and_grows_with_its_description() {
    let mut h = UiHarness::new(WIN);
    let mut on = false;
    let d = setting(&mut h, RECT, &plain("Title", None), &mut on);
    assert_eq!(d.height, 56.0);
    // 12 + title 20 + 2 + one 16 px line + 12.
    let d = setting(&mut h, RECT, &plain("Title", Some("Short")), &mut on);
    assert_eq!(d.height, 62.0);
    // A description that wraps to three rows.
    let long = "x".repeat(200);
    let d = setting(&mut h, RECT, &plain("Title", Some(&long)), &mut on);
    let text_w = 400.0 - 24.0 - 40.0 - 16.0;
    let rows = (200.0_f32 * 5.5 / text_w).ceil();
    assert_eq!(d.height, 12.0 + 20.0 + 2.0 + 16.0 * rows + 12.0);
}

#[test]
fn at_scale_2_the_row_is_112_px_with_radius_12_and_the_title_24_px_in() {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let mut on = false;
    setting(&mut h, RECT, &plain("Title", None), &mut on);
    h.input.pointer_to(300.0, 50.0);
    h.at_ms(10);
    setting(&mut h, RECT, &plain("Title", None), &mut on);
    h.at_ms(1000);
    let d = setting(&mut h, RECT, &plain("Title", None), &mut on);
    let fill = row_fill(&d.scene, [40.0, 60.0, 800.0, 112.0]).expect("the hover fill, 112 px high");
    assert_eq!(fill.radii, [12.0; 4]);
    let title = &texts(&d.scene)[0];
    assert_eq!((title.x - RECT.x) * 2.0, 24.0, "the title starts 24 px in");
}

#[test]
fn the_title_is_primary_and_the_description_fg_muted() {
    let mut h = UiHarness::new(WIN);
    let mut on = false;
    let d = setting(&mut h, RECT, &plain("Title", Some("Details")), &mut on);
    let t = texts(&d.scene);
    let title = Spec::plain(
        "Title",
        Style {
            line: 20.0,
            ..Style::ui(13.0, DARK.fg_primary)
        },
    );
    let text_w = 400.0 - 24.0 - 40.0 - 16.0;
    let desc = Spec::plain(
        "Details",
        Style {
            line: 16.0,
            ..Style::ui(11.0, DARK.fg_muted)
        },
    )
    .wrapped(text_w);
    assert_eq!(t[0].key.0, title.cache_key(1.0));
    assert_eq!(t[1].key.0, desc.cache_key(1.0));
    // The block sits 12 px in from the top and the left; the description 2 px under the title.
    assert_eq!((t[0].x, t[0].y), (32.0, 42.0));
    assert_eq!((t[1].x, t[1].y), (32.0, 64.0));
}

#[test]
fn a_long_title_is_elided_to_the_text_column() {
    let mut h = UiHarness::new(WIN);
    let mut on = false;
    let long = "A very long setting title that cannot fit in the narrow text column at all";
    let d = setting(&mut h, RECT, &plain(long, None), &mut on);
    let title = &texts(&d.scene)[0];
    let whole = Spec::plain(
        long,
        Style {
            line: 20.0,
            ..Style::ui(13.0, DARK.fg_primary)
        },
    );
    assert_ne!(
        title.key.0,
        whole.cache_key(1.0),
        "the drawn title is the cut one"
    );
}

#[test]
fn the_pointer_on_the_control_keeps_the_row_lit_and_the_control_takes_the_click() {
    let mut h = UiHarness::new(WIN);
    let mut on = false;
    setting(&mut h, RECT, &plain("Title", None), &mut on);
    h.input.pointer_to(TOGGLE.0, TOGGLE.1);
    h.at_ms(10);
    let d = setting(&mut h, RECT, &plain("Title", None), &mut on);
    assert!(d.toggle.hovered, "the toggle is hovered");
    h.at_ms(500);
    let d = setting(&mut h, RECT, &plain("Title", None), &mut on);
    let fill = row_fill(&d.scene, [20.0, 30.0, 400.0, 56.0]).expect("the row is lit");
    assert_eq!(fill.color, premul(DARK.surface_hover));
    assert_eq!(fill.radii, [6.0; 4]);
    assert!(!on);
    h.input.click(PointerButton::Primary);
    h.at_ms(510);
    let d = setting(&mut h, RECT, &plain("Title", None), &mut on);
    assert!(d.toggle.clicked && on, "one click toggles once");
    h.at_ms(520);
    setting(&mut h, RECT, &plain("Title", None), &mut on);
    assert!(on, "and no more");
}

#[test]
fn a_row_registers_no_region_so_what_is_under_it_stays_hoverable() {
    let mut h = UiHarness::new(WIN);
    let frame = |h: &mut UiHarness| {
        let mut under = None;
        h.frame(|ui| {
            let mut on = false;
            under = Some(ui.toggle(
                "below",
                Rect::new(368.0, 30.0, 40.0, 56.0),
                &mut on,
                "Below",
            ));
            let row = SettingRow {
                title: "Title",
                ..SettingRow::default()
            };
            ui.setting_row("row", RECT, &row, |_, _| {});
        });
        under.expect("drawn")
    };
    frame(&mut h);
    h.input.pointer_to(TOGGLE.0, TOGGLE.1);
    h.at_ms(10);
    assert!(frame(&mut h).hovered);
}

#[test]
fn the_hover_fill_is_monotonic_ends_exact_and_settles_within_120_ms_and_a_frame() {
    let mut h = UiHarness::new(WIN);
    let mut on = false;
    setting(&mut h, RECT, &plain("Title", None), &mut on);
    h.input.pointer_to(300.0, 50.0);
    let (mut last, mut started, mut settled_at) = (0.0_f32, None, None);
    for k in 1..=40_u64 {
        let ms = k * 1000 / 60;
        h.at_ms(ms);
        let d = setting(&mut h, RECT, &plain("Title", None), &mut on);
        let a = row_fill(&d.scene, [20.0, 30.0, 400.0, 56.0]).map_or(0.0, |s| s.color[3]);
        assert!(a >= last, "monotonic at {ms} ms: {a} after {last}");
        last = a;
        if d.repaint == Repaint::Now && started.is_none() {
            started = Some(ms);
        }
        if d.repaint == Repaint::Idle && started.is_some() && settled_at.is_none() {
            settled_at = Some(ms);
            assert_eq!(a, premul(DARK.surface_hover)[3], "ends exact");
        }
    }
    let (from, at) = (started.expect("starts"), settled_at.expect("settles"));
    assert!(
        at - from <= 120 + 17,
        "started at {from} ms, settled at {at} ms"
    );
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        let (a, b) = (
            DARK.fg_primary.map(f32::from),
            DARK.fg_secondary.map(f32::from),
        );
        conformance(&SELECTION, &DARK.motion, tier, a, b)
            .unwrap_or_else(|e| panic!("{tier:?}: {e}"));
        let (a, b) = ([0.0_f32, 0.0, 0.0, 0.0], DARK.surface_hover.map(f32::from));
        conformance(&HOVER, &DARK.motion, tier, a, b).unwrap_or_else(|e| panic!("{tier:?}: {e}"));
    }
}

#[test]
fn off_fades_the_title_to_secondary_at_fast_and_ends_exact() {
    let mut h = UiHarness::new(WIN);
    let mut on = false;
    let mut row = plain("Title", None);
    setting(&mut h, RECT, &row, &mut on);
    row.off = true;
    h.at_ms(100);
    setting(&mut h, RECT, &row, &mut on);
    h.at_ms(190);
    let key = |c| {
        Spec::plain(
            "Title",
            Style {
                line: 20.0,
                ..Style::ui(13.0, c)
            },
        )
        .cache_key(1.0)
    };
    let mid = setting(&mut h, RECT, &row, &mut on);
    assert_eq!(mid.repaint, Repaint::Now, "the colour is in flight");
    let k = texts(&mid.scene)[0].key.0;
    assert!(
        k != key(DARK.fg_primary) && k != key(DARK.fg_secondary),
        "between the two"
    );
    h.at_ms(1000);
    let end = setting(&mut h, RECT, &row, &mut on);
    assert_eq!(texts(&end.scene)[0].key.0, key(DARK.fg_secondary));
    assert_eq!(end.repaint, Repaint::Idle);
}

#[test]
fn a_disabled_row_is_inert_dimmed_and_marked() {
    let mut h = UiHarness::new(WIN);
    let (mut on, mut toggle) = (false, None);
    let mut frame = |h: &mut UiHarness| {
        let (scene, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                ui.setting_row("row", RECT, &plain("Title", None), |ui, slot| {
                    toggle = Some(ui.toggle("t", slot, &mut on, "Switch"));
                });
            });
        });
        (scene, out.tree)
    };
    frame(&mut h);
    h.input.pointer_to(TOGGLE.0, TOGGLE.1);
    h.input.click(PointerButton::Primary);
    h.at_ms(500);
    let (scene, tree) = frame(&mut h);
    let t = toggle.expect("ran");
    assert!(!t.hovered && !t.clicked, "the control reports nothing");
    assert!(
        row_fill(&scene, [20.0, 30.0, 400.0, 56.0]).is_none(),
        "no hover fill"
    );
    let title = tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Group)
        .expect("the row");
    assert!(title.states.contains(State::Disabled));
    assert!(!on);
}

#[test]
fn the_node_is_a_group_named_by_the_title_over_the_controls_node() {
    let mut h = UiHarness::new(WIN);
    let mut on = false;
    let d = setting(&mut h, RECT, &plain("Title", Some("Details")), &mut on);
    let group = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Group)
        .expect("a group");
    assert_eq!(group.name, "Title");
    assert_eq!(group.description.as_deref(), Some("Details"));
    assert!(!group.states.contains(State::Disabled));
    let switch = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Switch)
        .expect("the control");
    assert_eq!(switch.parent, Some(group.id));
    assert!(group.children.contains(&switch.id));
    assert_eq!(group.id, WidgetId::ROOT.with("row"));
}

#[test]
fn an_item_row_is_32_high_over_a_40_1fr_88_176_grid_with_8_px_gaps() {
    let mut h = UiHarness::new(WIN);
    let rect = Rect::new(20.0, 30.0, 500.0, 0.0);
    let mut cells = None;
    let mut height = 0.0;
    let (_, out) = h.frame(|ui| {
        height = ui.item_row("item", rect, &ItemRow { name: "Item" }, |_, c| {
            cells = Some(c)
        });
    });
    let c = cells.expect("cols ran");
    assert_eq!(height, 32.0);
    let flex = 500.0 - 24.0 - 40.0 - 88.0 - 176.0 - 24.0;
    assert_eq!(c[0], Rect::new(32.0, 30.0, 40.0, 32.0));
    assert_eq!(c[1], Rect::new(80.0, 30.0, flex, 32.0));
    assert_eq!(c[2], Rect::new(80.0 + flex + 8.0, 30.0, 88.0, 32.0));
    assert_eq!(
        c[3],
        Rect::new(80.0 + flex + 8.0 + 88.0 + 8.0, 30.0, 176.0, 32.0)
    );
    assert_eq!(c[3].right(), 500.0 + 20.0 - 12.0, "12 px of side padding");
    let group = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Group)
        .expect("a group");
    assert_eq!(group.name, "Item");
}

#[test]
fn an_item_row_lights_on_hover_and_the_cells_keep_their_input() {
    let mut h = UiHarness::new(WIN);
    let rect = Rect::new(20.0, 30.0, 500.0, 0.0);
    let frame = |h: &mut UiHarness| {
        let mut on = false;
        let mut toggle = None;
        let (scene, _) = h.frame(|ui| {
            ui.item_row("item", rect, &ItemRow { name: "Item" }, |ui, c| {
                toggle = Some(ui.toggle("t", c[0], &mut on, "Lead"));
            });
        });
        (scene, toggle.expect("ran"))
    };
    frame(&mut h);
    // The toggle's track is centred in the first cell: x 32 + 20, y 30 + 16.
    h.input.pointer_to(52.0, 46.0);
    h.at_ms(10);
    frame(&mut h);
    h.at_ms(500);
    let (scene, toggle) = frame(&mut h);
    assert!(toggle.hovered);
    let fill = row_fill(&scene, [20.0, 30.0, 500.0, 32.0]).expect("lit");
    assert_eq!(fill.color, premul(DARK.surface_hover));
}
