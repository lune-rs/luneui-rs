//! The dropdown and popup panel (C4 retrofit, `overlay.dropdown` and `overlay.popup-panel`
//! acceptance): rest identity with the Painter-API `dropdown`, keyboard highlight, disabled rows,
//! dismissal, focus restore to the combo, the semantic tree, the panel probe.

use lune_ui_core::access::{Action, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::dropdown::{DropdownResponse, dropdown, panel_rect, row_rect};
use lune_ui_core::widgets::popup_panel;

const WIN: (f32, f32) = (1040.0, 660.0);
const ANCHOR: Rect = Rect::new(34.0, 100.0, 210.0, 32.0);
/// Near the bottom edge: the list opens above it.
const FLIPPED: Rect = Rect::new(34.0, 560.0, 210.0, 32.0);
const P: PointerButton = PointerButton::Primary;
const OPTIONS: [(&str, bool); 4] = [
    ("RznSans", true),
    ("Sans Serif", true),
    ("Monospace", true),
    ("Coming soon", false),
];

fn win() -> Rect {
    Rect::new(0.0, 0.0, WIN.0, WIN.1)
}

fn old_scene(anchor: Rect, selected: usize, hover: Option<u8>) -> Scene {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, hover, DARK);
    p.on(LayerId::Popup);
    dropdown(&mut p, anchor, &OPTIONS, selected, |i| i as u8);
    p.scene
}

fn row_centre(anchor: Rect, i: usize) -> (f32, f32) {
    let mut text = FakeText;
    let p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let r = row_rect(panel_rect(&p, anchor, OPTIONS.len()), i);
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

fn anchor_id() -> WidgetId {
    WidgetId::ROOT.with("combo")
}

fn frame(
    h: &mut UiHarness,
    ms: u64,
    anchor: Rect,
    open: bool,
    selected: usize,
) -> (Scene, DropdownResponse) {
    let mut resp = DropdownResponse::default();
    let (scene, _) = h.at_ms(ms).frame(|ui| {
        if open {
            resp = ui.dropdown("dd", anchor, anchor_id(), &OPTIONS, selected);
        }
    });
    (scene, resp)
}

#[test]
fn at_rest_the_lists_are_the_old_lists() {
    for anchor in [ANCHOR, FLIPPED] {
        let mut h = UiHarness::new(WIN);
        frame(&mut h, 0, anchor, true, 0);
        let (scene, _) = frame(&mut h, 1000, anchor, true, 0);
        assert!(scene == old_scene(anchor, 0, None), "{anchor:?}");
    }
    // The flipped list really is above its anchor.
    let mut text = FakeText;
    let p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    assert!(panel_rect(&p, FLIPPED, 4).bottom() <= FLIPPED.y);
    assert!(panel_rect(&p, ANCHOR, 4).y > ANCHOR.bottom());
}

#[test]
fn a_hovered_enabled_row_gets_the_hover_fill_and_a_disabled_one_does_not() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    let (x, y) = row_centre(ANCHOR, 1);
    h.input.pointer_to(x, y);
    frame(&mut h, 10, ANCHOR, true, 0);
    let (scene, _) = frame(&mut h, 1000, ANCHOR, true, 0);
    assert!(scene == old_scene(ANCHOR, 0, Some(1)));
    // Over the disabled row: no hover fill, same as the old list with that hit absent.
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    let (x, y) = row_centre(ANCHOR, 3);
    h.input.pointer_to(x, y);
    frame(&mut h, 10, ANCHOR, true, 0);
    let (scene, _) = frame(&mut h, 1000, ANCHOR, true, 0);
    assert!(scene == old_scene(ANCHOR, 0, Some(3)));
}

#[test]
fn a_click_picks_an_enabled_row_once_and_never_a_disabled_one() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    let (x, y) = row_centre(ANCHOR, 2);
    h.input.pointer_to(x, y).click(P);
    let (_, r) = frame(&mut h, 10, ANCHOR, true, 0);
    assert_eq!(r.picked, Some(2));
    let (_, r) = frame(&mut h, 20, ANCHOR, true, 0);
    assert_eq!(r.picked, None);
    let (x, y) = row_centre(ANCHOR, 3);
    h.input.pointer_to(x, y).click(P);
    let (_, r) = frame(&mut h, 30, ANCHOR, true, 0);
    assert!(
        r.picked.is_none() && !r.dismissed,
        "inside the panel, nothing fires"
    );
}

#[test]
fn the_keyboard_highlight_starts_at_the_selected_row_and_skips_disabled_ones() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 1);
    frame(&mut h, 10, ANCHOR, true, 1);
    // Enter at once picks the selected row.
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, r) = frame(&mut h, 20, ANCHOR, true, 1);
    assert_eq!(r.picked, Some(1));
    // Down to row 2 (drawn with the hover fill), Down again stays (row 3 is disabled).
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    let (scene, _) = frame(&mut h, 30, ANCHOR, true, 0);
    assert!(scene == old_scene(ANCHOR, 0, Some(2)), "row 2 highlighted");
    // One press moves one row per frame; Down again stays (row 3 is disabled).
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    frame(&mut h, 40, ANCHOR, true, 0);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, r) = frame(&mut h, 50, ANCHOR, true, 0);
    assert_eq!(r.picked, Some(2), "the disabled row 3 is skipped");
    for ms in [60, 70, 80] {
        h.input.key_down(Key::Named(NamedKey::ArrowUp), None);
        frame(&mut h, ms, ANCHOR, true, 0);
    }
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, r) = frame(&mut h, 90, ANCHOR, true, 0);
    assert_eq!(r.picked, Some(0), "Up stops at the first row");
}

#[test]
fn escape_and_an_outside_press_dismiss() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (_, r) = frame(&mut h, 10, ANCHOR, true, 0);
    assert!(r.dismissed);
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    h.input.pointer_to(700.0, 400.0).click(P);
    let (_, r) = frame(&mut h, 10, ANCHOR, true, 0);
    assert!(r.dismissed);
}

#[test]
fn it_opens_and_closes_at_once_and_focus_returns_to_the_combo() {
    let mut h = UiHarness::new(WIN);
    let mut open = false;
    let run = |h: &mut UiHarness, ms: u64, open: &mut bool| {
        let mut out = DropdownResponse::default();
        let mut focused = false;
        let (scene, _) = h.at_ms(ms).frame(|ui| {
            let c = ui.combo("combo", ANCHOR, "RznSans", open, "Font");
            focused = c.focused;
            if *open {
                out = ui.dropdown("dd", ANCHOR, c.id, &OPTIONS, 0);
                if out.dismissed || out.picked.is_some() {
                    *open = false;
                }
            }
        });
        (out, focused, scene)
    };
    run(&mut h, 0, &mut open);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    run(&mut h, 10, &mut open);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, focused, _) = run(&mut h, 20, &mut open);
    assert!(open && focused);
    // The list takes focus on the next frame; keys go to it, not to the combo.
    let (_, focused, _) = run(&mut h, 30, &mut open);
    assert!(!focused, "focus moved into the list");
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    let (out, ..) = run(&mut h, 40, &mut open);
    assert!(
        open && out.picked.is_none(),
        "Down moves the highlight, it does not reopen"
    );
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (out, ..) = run(&mut h, 50, &mut open);
    assert!(out.dismissed && !open);
    // No exit frame: the first frame without the app's call draws nothing on the popup layer.
    let (_, _, scene) = run(&mut h, 60, &mut open);
    assert!(
        scene.layers[LayerId::Popup as usize]
            .geometry
            .under
            .is_empty()
    );
    // Focus returns when the close settles, one frame later.
    let (_, focused, _) = run(&mut h, 70, &mut open);
    assert!(focused, "focus is back on the combo");
}

#[test]
fn the_semantic_tree_is_a_list_of_list_items() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 2);
    let (_, out) = h.at_ms(10).frame(|ui| {
        ui.dropdown("dd", ANCHOR, anchor_id(), &OPTIONS, 2);
    });
    let list = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::List)
        .expect("list");
    let items: Vec<_> = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::ListItem)
        .collect();
    assert_eq!(items.len(), 4);
    assert_eq!(list.children.len(), 4);
    assert_eq!(items[2].name, "Monospace");
    for (i, n) in items.iter().enumerate() {
        assert_eq!(n.states.contains(State::Selected), i == 2, "{i}");
        assert_eq!(n.states.contains(State::Disabled), i == 3, "{i}");
        assert_eq!(n.actions.contains(Action::Default), i != 3, "{i}");
    }
}

#[test]
fn the_popup_panel_is_16_px_radius_with_a_2_px_ring_in_the_callers_fill() {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(2.0, win(), &mut text, None, DARK);
    let (bg, border) = ([10, 20, 30, 200], [255, 255, 255, 48]);
    popup_panel(&mut p, Rect::new(10.0, 10.0, 100.0, 60.0), bg, border);
    let g = &p.scene.layers[0].geometry;
    let (fill, ring) = (g.under[0], g.under[1]);
    assert_eq!(fill.radii, [16.0; 4]);
    assert_eq!(fill.color, premul(bg));
    assert_eq!(ring.radii, [16.0; 4]);
    assert_eq!(ring.params[1], 2.0);
    assert_eq!(ring.color, premul(border));
}
