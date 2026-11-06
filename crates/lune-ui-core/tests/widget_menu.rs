//! The menu (C4 retrofit, `overlay.menu` acceptance): bit-exact rest and motion against the
//! Painter-API `menu` (D7), exit retention per tier, curve conformance, picking, dismissal and the
//! semantic tree.

use lune_ui_core::access::{AccessAction, Action, Role, State};
use lune_ui_core::color::Rgba;
use lune_ui_core::icon;
use lune_ui_core::input::{FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::shortcuts::{Binding, Chord, Mods, Scope};
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style, TextKey};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Sense, Ui, WidgetId};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::menu::{
    CLOSE, HEADER, HOVER, MenuEntry, MenuItem, MenuLayout, MenuOptions, MenuPlace, MenuResponse,
    MenuRow, OPEN, menu, menu_close_anim, menu_layout, menu_layout_at, menu_open_anim,
};
use lune_ui_core::widgets::popover::{PlaceAlign, Side};
use lune_ui_core::widgets::tone::Tone;

const WIN: (f32, f32) = (600.0, 400.0);
const AT: (f32, f32) = (100.0, 40.0);
const P: PointerButton = PointerButton::Primary;

const ROWS: [MenuRow<'static>; 6] = [
    MenuRow::Item {
        glyph: Some(icon::ADD),
        text: "New Window",
        shortcut: "Cmd+N",
    },
    MenuRow::Item {
        glyph: Some(icon::EDIT),
        text: "Rename...",
        shortcut: "",
    },
    MenuRow::Item {
        glyph: Some(icon::COPY),
        text: "Copy",
        shortcut: "Cmd+C",
    },
    MenuRow::Separator,
    MenuRow::Item {
        glyph: Some(icon::GEAR),
        text: "Settings...",
        shortcut: "Cmd+,",
    },
    MenuRow::Item {
        glyph: Some(icon::INFO),
        text: "About",
        shortcut: "",
    },
];

fn win() -> Rect {
    Rect::new(0.0, 0.0, WIN.0, WIN.1)
}

fn entries() -> Vec<MenuEntry<'static, u8>> {
    ROWS.iter()
        .enumerate()
        .map(|(i, r)| match r {
            MenuRow::Item {
                glyph,
                text,
                shortcut,
            } => MenuEntry::Item {
                glyph: *glyph,
                text,
                shortcut,
                hit: i as u8,
            },
            MenuRow::Separator => MenuEntry::Separator,
            _ => unreachable!("ROWS holds items and separators"),
        })
        .collect()
}

/// What the Painter-API `menu` draws on the popup layer.
fn old_scene(anim: (f32, f32, f32), interactive: bool, hover: Option<u8>) -> Scene {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, hover, DARK);
    p.on(LayerId::Popup);
    menu(&mut p, &entries(), AT, anim, interactive);
    p.scene
}

fn item_centre(i: usize) -> (f32, f32) {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let r = menu_layout(&mut p, &ROWS, AT).rows[i];
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

fn ui_frame(h: &mut UiHarness, ms: u64, open: bool) -> (Scene, MenuResponse, Repaint) {
    let mut resp = MenuResponse::default();
    let (scene, out) = h.at_ms(ms).frame(|ui| {
        if open {
            resp = ui.menu("m", &ROWS, AT);
        }
    });
    (scene, resp, out.repaint)
}

fn menu_id() -> WidgetId {
    WidgetId::ROOT.with("m")
}

#[test]
fn settled_with_the_pointer_over_item_one_is_the_old_menu_at_rest() {
    let mut h = UiHarness::new(WIN);
    ui_frame(&mut h, 0, true);
    let (x, y) = item_centre(1);
    h.input.pointer_to(x, y);
    ui_frame(&mut h, 10, true);
    let (scene, _, repaint) = ui_frame(&mut h, 1000, true);
    assert_eq!(repaint, Repaint::Idle);
    assert_eq!(scene, old_scene(menu_open_anim(1.0), true, Some(1)));
}

#[test]
fn opened_at_zero_and_drawn_at_90_ms_is_the_old_menu_at_half_progress() {
    let mut h = UiHarness::new(WIN);
    ui_frame(&mut h, 0, true);
    let (scene, _, repaint) = ui_frame(&mut h, 90, true);
    assert_eq!(repaint, Repaint::Now);
    assert_eq!(scene, old_scene(menu_open_anim(0.09 / 0.18), true, None));
    assert_eq!(0.09_f32 / 0.18, 0.5);
}

#[test]
fn closing_drawn_60_ms_after_the_app_stopped_is_the_old_menu_at_half() {
    let mut h = UiHarness::new(WIN);
    ui_frame(&mut h, 0, true);
    ui_frame(&mut h, 300, true);
    ui_frame(&mut h, 1000, false);
    let (scene, _, repaint) = ui_frame(&mut h, 1060, false);
    assert_eq!(repaint, Repaint::Now);
    let anim = menu_close_anim(0.06 / 0.12).expect("closing");
    assert_eq!(scene, old_scene(anim, false, None));
}

/// Opens, settles, stops calling at 1000 ms and steps 10 ms frames; returns the ms until idle
/// and the opacity (fill alpha) of each closing frame.
fn close_run(tier: MotionTier) -> (u64, Vec<f32>) {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    ui_frame(&mut h, 0, true);
    ui_frame(&mut h, 300, true);
    let mut alphas = Vec::new();
    let mut t = 1000;
    loop {
        let (scene, _, repaint) = ui_frame(&mut h, t, false);
        let g = &scene.layers[LayerId::Popup as usize].geometry;
        if let Some(s) = g.under.first() {
            alphas.push(s.color[3]);
        }
        if repaint == Repaint::Idle {
            return (t - 1000, alphas);
        }
        t += 10;
        assert!(t < 2000, "never settled");
    }
}

#[test]
fn exit_retention_is_120_ms_full_120_reduced_and_none_off() {
    for (tier, ms) in [
        (MotionTier::Full, 120),
        (MotionTier::Reduced, 120),
        (MotionTier::Off, 0),
    ] {
        let (idle_after, alphas) = close_run(tier);
        // Off removes it at once; its close frame still asks for one more frame because focus
        // returned after the page drew (D81), so the first idle frame is the next one.
        let first_idle = if ms == 0 { 10 } else { ms };
        assert_eq!(idle_after, first_idle, "{tier:?}");
        assert_eq!(alphas.len() as u64, ms / 10, "{tier:?}: a frame per 10 ms");
        assert!(
            alphas.windows(2).all(|w| w[1] <= w[0]),
            "{tier:?}: {alphas:?}"
        );
        assert!(alphas.iter().all(|a| a.is_finite()));
    }
}

#[test]
fn reduced_fades_opacity_only() {
    let mut h = UiHarness::new(WIN);
    h.tier = MotionTier::Reduced;
    ui_frame(&mut h, 0, true);
    let (scene, _, _) = ui_frame(&mut h, 40, true);
    let panel = {
        let mut text = FakeText;
        let mut p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
        menu_layout(&mut p, &ROWS, AT).panel
    };
    let fill = scene.layers[LayerId::Popup as usize].geometry.under[0];
    // Opacity moves, the rect does not (no scale, no offset).
    assert_eq!(fill.rect, [panel.x, panel.y, panel.w, panel.h]);
    assert!(fill.color[3] < 1.0);
}

#[test]
fn the_curves_conform_under_every_tier_and_opacity_is_monotone_through_the_ui() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, a, b) in [(OPEN, 0.0_f32, 1.0), (CLOSE, 1.0, 0.0)] {
            conformance(&row, &DARK.motion, tier, a, b)
                .unwrap_or_else(|e| panic!("{tier:?} {row:?}: {e}"));
        }
    }
    let mut h = UiHarness::new(WIN);
    ui_frame(&mut h, 0, true);
    let rest = old_scene(menu_open_anim(1.0), true, None).layers[LayerId::Popup as usize]
        .geometry
        .under[0]
        .color[3];
    let mut last = 0.0;
    for k in 1..=15_u64 {
        let dt = (k * 1000 + 30) / 60;
        let (scene, _, repaint) = ui_frame(&mut h, dt, true);
        let a = scene.layers[LayerId::Popup as usize].geometry.under[0].color[3];
        assert!(a >= last && a.is_finite(), "{dt}: {a} after {last}");
        last = a;
        if dt >= 180 + 17 {
            assert_eq!(a, rest, "settled at {dt} ms");
            assert_eq!(repaint, Repaint::Idle);
        }
    }
}

#[test]
fn a_click_picks_the_item_once_and_a_separator_cannot_be_picked() {
    let mut h = UiHarness::new(WIN);
    ui_frame(&mut h, 0, true);
    let (x, y) = item_centre(2);
    h.input.pointer_to(x, y).click(P);
    let (_, r, _) = ui_frame(&mut h, 10, true);
    assert_eq!(r.picked, Some(2));
    let (_, r, _) = ui_frame(&mut h, 20, true);
    assert_eq!(r.picked, None, "once per click");
    // The separator's strip sits between items 2 and 4.
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let sep = menu_layout(&mut p, &ROWS, AT).rows[3];
    h.input.pointer_to(sep.x + 5.0, sep.y + 5.0).click(P);
    let (_, r, _) = ui_frame(&mut h, 30, true);
    assert_eq!(r.picked, None);
    assert!(!r.dismissed, "a press inside the panel is not outside");
}

#[test]
fn enter_picks_the_hovered_item_else_the_focused_one() {
    let mut h = UiHarness::new(WIN);
    ui_frame(&mut h, 0, true);
    let (x, y) = item_centre(4);
    h.input.pointer_to(x, y);
    ui_frame(&mut h, 10, true);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, r, _) = ui_frame(&mut h, 20, true);
    assert_eq!(r.picked, Some(4));
    // No pointer: the item focused at open takes Enter.
    let mut h = UiHarness::new(WIN);
    ui_frame(&mut h, 0, true);
    ui_frame(&mut h, 10, true);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, r, _) = ui_frame(&mut h, 20, true);
    assert_eq!(r.picked, Some(0));
}

#[test]
fn escape_and_an_outside_press_dismiss_and_the_press_reaches_nothing() {
    let mut h = UiHarness::new(WIN);
    let run = |h: &mut UiHarness, ms: u64| {
        let mut out = (MenuResponse::default(), false);
        h.at_ms(ms).frame(|ui| {
            ui.painter().on(LayerId::Page);
            let id = ui.id("page-button");
            let b = ui.interact(id, Rect::new(400.0, 300.0, 80.0, 30.0), Sense::CLICK_FOCUS);
            out = (ui.menu("m", &ROWS, AT), b.clicked);
        });
        out
    };
    run(&mut h, 0);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    assert!(run(&mut h, 10).0.dismissed);
    h.input.pointer_to(440.0, 315.0).click(P);
    let (r, page_clicked) = run(&mut h, 20);
    assert!(
        r.dismissed && !page_clicked,
        "D22: the dismissing press is consumed"
    );
}

#[test]
fn a_closing_menu_hits_nothing() {
    let mut h = UiHarness::new(WIN);
    let (x, y) = item_centre(1);
    let run = |h: &mut UiHarness, ms: u64, open: bool| {
        let mut clicked = false;
        h.at_ms(ms).frame(|ui| {
            ui.painter().on(LayerId::Page);
            let id = ui.id("under");
            clicked = ui.interact(id, win(), Sense::CLICK).clicked;
            if open {
                ui.menu("m", &ROWS, AT);
            }
        });
        clicked
    };
    run(&mut h, 0, true);
    run(&mut h, 300, true);
    run(&mut h, 1000, false);
    h.input.pointer_to(x, y);
    run(&mut h, 1020, false);
    h.input.click(P);
    assert!(
        run(&mut h, 1040, false),
        "the page under a closing menu gets the click"
    );
}

#[test]
fn the_semantic_tree_has_a_menu_of_menu_items() {
    let mut h = UiHarness::new(WIN);
    ui_frame(&mut h, 0, true);
    let (_, out) = h.at_ms(10).frame(|ui| {
        ui.menu("m", &ROWS, AT);
    });
    let menu_node = out.tree.get(menu_id()).expect("menu node");
    assert_eq!(menu_node.role, Role::Menu);
    let items: Vec<_> = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::MenuItem)
        .collect();
    assert_eq!(items.len(), 5, "separators are not nodes");
    assert_eq!(items[0].name, "New Window");
    assert_eq!(items[0].description.as_deref(), Some("Cmd+N"));
    assert_eq!(items[1].description, None);
    assert!(items.iter().all(|n| n.actions.contains(Action::Default)));
    assert!(items.iter().all(|n| n.states.contains(State::Focusable)));
    assert_eq!(menu_node.children.len(), 5);
}

/// Rows 0 and 6 are not items; 2 is disabled; a Down from 1 lands on 4.
const VARIANTS: [MenuRow<'static>; 7] = [
    MenuRow::Header {
        name: "Serving on :4646",
        status: "3 sessions",
        tone: Tone::Ok,
    },
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::ADD),
        text: "Plain",
        shortcut: "Cmd+P",
        disabled: false,
        danger: false,
        chevron: false,
    }),
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::EDIT),
        text: "Disabled",
        shortcut: "Cmd+D",
        disabled: true,
        danger: false,
        chevron: false,
    }),
    MenuRow::Separator,
    MenuRow::Entry(MenuItem {
        glyph: None,
        text: "Profile",
        shortcut: "beside the chevron",
        disabled: false,
        danger: false,
        chevron: true,
    }),
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::CLOSE),
        text: "Quit",
        shortcut: "",
        disabled: false,
        danger: true,
        chevron: false,
    }),
    MenuRow::Separator,
];

fn entry(text: &'static str, disabled: bool) -> MenuRow<'static> {
    MenuRow::Entry(MenuItem {
        text,
        disabled,
        ..MenuItem::default()
    })
}

fn layout_of(rows: &[MenuRow<'_>]) -> MenuLayout {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    menu_layout(&mut p, rows, AT)
}

fn centre_of(rows: &[MenuRow<'_>], i: usize) -> (f32, f32) {
    let r = layout_of(rows).rows[i];
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

fn draw(h: &mut UiHarness, ms: u64, rows: &[MenuRow<'_>]) -> (Scene, MenuResponse, FrameOutput) {
    let mut resp = MenuResponse::default();
    let (scene, out) = h.at_ms(ms).frame(|ui| resp = ui.menu("m", rows, AT));
    (scene, resp, out)
}

/// The shapes filling row rects on the popup layer, as `(row, shape)`.
fn row_fills(scene: &Scene, rows: &[MenuRow<'_>]) -> Vec<(usize, Shape)> {
    let rects = layout_of(rows).rows;
    let under = &scene.layers[LayerId::Popup as usize].geometry.under;
    let mut fills = Vec::new();
    for (i, r) in rects.iter().enumerate() {
        for s in under {
            if s.rect == [r.x, r.y, r.w, r.h] && s.color[3] > 0.0 {
                fills.push((i, *s));
            }
        }
    }
    fills
}

fn lit(scene: &Scene, rows: &[MenuRow<'_>]) -> Vec<usize> {
    row_fills(scene, rows).iter().map(|f| f.0).collect()
}

fn key(h: &mut UiHarness, k: NamedKey) {
    h.input.key_down(Key::Named(k), None);
}

fn rgba(c: Rgba) -> [f32; 4] {
    let a = f32::from(c[3]) / 255.0;
    [
        f32::from(c[0]) / 255.0 * a,
        f32::from(c[1]) / 255.0 * a,
        f32::from(c[2]) / 255.0 * a,
        a,
    ]
}

fn assert_fill(shape: &Shape, color: Rgba) {
    for (a, b) in shape.color.iter().zip(rgba(color)) {
        assert!((a - b).abs() < 1e-5, "{:?} against {color:?}", shape.color);
    }
}

/// A menu of `rows` opened by a pointer: focus on its first enabled row, nothing lit.
fn pointer_open(rows: &[MenuRow<'_>]) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    draw(&mut h, 0, rows);
    draw(&mut h, 300, rows);
    h
}

/// A menu of `rows` opened from the keyboard: its first enabled row is lit.
fn keyboard_open(rows: &[MenuRow<'_>]) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    key(&mut h, NamedKey::Tab);
    draw(&mut h, 0, rows);
    draw(&mut h, 300, rows);
    h
}

fn on_key(h: &mut UiHarness, ms: u64, k: NamedKey, rows: &[MenuRow<'_>]) -> Vec<usize> {
    key(h, k);
    lit(&draw(h, ms, rows).0, rows)
}

#[test]
fn a_keyboard_open_lights_the_first_enabled_row_and_a_pointer_open_none() {
    let mut h = keyboard_open(&VARIANTS);
    let (scene, _, out) = draw(&mut h, 310, &VARIANTS);
    assert_eq!(lit(&scene, &VARIANTS), [1]);
    assert_eq!(out.focused, Some(menu_id().with(1_usize)));
    let mut h = pointer_open(&VARIANTS);
    let (scene, _, out) = draw(&mut h, 310, &VARIANTS);
    assert_eq!(lit(&scene, &VARIANTS), [] as [usize; 0]);
    assert_eq!(out.focused, Some(menu_id().with(1_usize)));
}

#[test]
fn a_keyboard_open_under_a_resting_pointer_lights_the_first_row_not_the_pointers() {
    let mut h = UiHarness::new(WIN);
    let (x, y) = centre_of(&ROWS, 2);
    h.input.pointer_to(x, y);
    h.at_ms(0).frame(|_| {});
    key(&mut h, NamedKey::Tab);
    draw(&mut h, 10, &ROWS);
    draw(&mut h, 300, &ROWS);
    assert_eq!(lit(&draw(&mut h, 600, &ROWS).0, &ROWS), [0]);
}

#[test]
fn down_and_up_wrap_and_skip_disabled_rows_header_and_separators() {
    let mut h = keyboard_open(&VARIANTS);
    let down = NamedKey::ArrowDown;
    assert_eq!(on_key(&mut h, 310, down, &VARIANTS), [4], "over 2 and 3");
    assert_eq!(on_key(&mut h, 320, down, &VARIANTS), [5]);
    assert_eq!(on_key(&mut h, 330, down, &VARIANTS), [1], "wraps over 6, 0");
    let up = NamedKey::ArrowUp;
    assert_eq!(on_key(&mut h, 340, up, &VARIANTS), [5], "wraps back");
    assert_eq!(on_key(&mut h, 350, up, &VARIANTS), [4]);
    assert_eq!(on_key(&mut h, 360, up, &VARIANTS), [1]);
}

#[test]
fn home_and_end_reach_the_enabled_ends() {
    let mut h = keyboard_open(&VARIANTS);
    assert_eq!(on_key(&mut h, 310, NamedKey::End, &VARIANTS), [5]);
    assert_eq!(on_key(&mut h, 320, NamedKey::Home, &VARIANTS), [1]);
}

#[test]
fn a_first_key_after_a_pointer_open_lights_an_end_and_moves_nothing_else() {
    for (k, want) in [
        (NamedKey::ArrowDown, 1),
        (NamedKey::Home, 1),
        (NamedKey::ArrowUp, 5),
        (NamedKey::End, 5),
    ] {
        let mut h = pointer_open(&VARIANTS);
        assert_eq!(on_key(&mut h, 310, k, &VARIANTS), [want], "{k:?}");
        let (scene, _, out) = draw(&mut h, 320, &VARIANTS);
        assert_eq!(lit(&scene, &VARIANTS), [want], "{k:?}");
        assert_eq!(out.focused, Some(menu_id().with(want)), "{k:?}");
    }
}

#[test]
fn a_row_is_lit_the_frame_the_key_arrives_and_enter_then_picks_it() {
    let mut h = keyboard_open(&ROWS);
    key(&mut h, NamedKey::ArrowDown);
    key(&mut h, NamedKey::ArrowDown);
    let (scene, _, _) = draw(&mut h, 310, &ROWS);
    let fills = row_fills(&scene, &ROWS);
    assert_eq!(fills.len(), 1, "one highlight, the old row out at once");
    assert_eq!(fills[0].0, 2, "two presses in one frame");
    assert_fill(&fills[0].1, DARK.menu_item_hover);
    key(&mut h, NamedKey::Enter);
    let (_, r, _) = draw(&mut h, 320, &ROWS);
    assert_eq!(r.picked, Some(2));
}

#[test]
fn a_pointer_resting_on_a_row_does_not_relight_it_after_an_arrow() {
    let mut h = pointer_open(&ROWS);
    let (x, y) = centre_of(&ROWS, 1);
    h.input.pointer_to(x, y);
    assert_eq!(lit(&draw(&mut h, 310, &ROWS).0, &ROWS), [] as [usize; 0]);
    assert_eq!(lit(&draw(&mut h, 500, &ROWS).0, &ROWS), [1], "faded in");
    assert_eq!(on_key(&mut h, 510, NamedKey::ArrowDown, &ROWS), [2]);
    assert_eq!(lit(&draw(&mut h, 800, &ROWS).0, &ROWS), [2], "still");
    key(&mut h, NamedKey::Enter);
    assert_eq!(draw(&mut h, 910, &ROWS).1.picked, Some(2), "the lit row");
    // Moving inside the same row is no input either; entering another one is.
    h.input.pointer_to(x + 3.0, y + 1.0);
    assert_eq!(lit(&draw(&mut h, 1100, &ROWS).0, &ROWS), [2]);
    let (x, y) = centre_of(&ROWS, 4);
    h.input.pointer_to(x, y);
    draw(&mut h, 1110, &ROWS);
    assert_eq!(lit(&draw(&mut h, 1400, &ROWS).0, &ROWS), [4]);
}

#[test]
fn the_pointer_leaving_puts_out_its_own_row_and_not_a_keys() {
    let (x, y) = centre_of(&ROWS, 1);
    // Its own: nothing is lit, Enter falls to the focused row, and the next Down starts over.
    let mut h = pointer_open(&ROWS);
    h.input.pointer_to(x, y);
    draw(&mut h, 310, &ROWS);
    draw(&mut h, 500, &ROWS);
    h.input.pointer_to(5.0, 390.0);
    draw(&mut h, 510, &ROWS);
    assert_eq!(lit(&draw(&mut h, 800, &ROWS).0, &ROWS), [] as [usize; 0]);
    key(&mut h, NamedKey::Enter);
    assert_eq!(draw(&mut h, 810, &ROWS).1.picked, Some(0));
    // A key's: the row the arrow lit stays.
    let mut h = pointer_open(&ROWS);
    h.input.pointer_to(x, y);
    draw(&mut h, 310, &ROWS);
    key(&mut h, NamedKey::ArrowDown);
    draw(&mut h, 500, &ROWS);
    h.input.pointer_to(5.0, 390.0);
    draw(&mut h, 510, &ROWS);
    assert_eq!(lit(&draw(&mut h, 800, &ROWS).0, &ROWS), [2]);
}

#[test]
fn a_press_takes_the_highlight_from_the_keys_and_shows_the_pressed_fill_while_held() {
    let mut h = pointer_open(&ROWS);
    let (x, y) = centre_of(&ROWS, 4);
    h.input.pointer_to(x, y);
    draw(&mut h, 310, &ROWS);
    key(&mut h, NamedKey::Home);
    draw(&mut h, 500, &ROWS);
    assert_eq!(lit(&draw(&mut h, 800, &ROWS).0, &ROWS), [0]);
    h.input.press(P);
    let (scene, r, _) = draw(&mut h, 810, &ROWS);
    assert_eq!(r.picked, None, "held");
    let fills = row_fills(&scene, &ROWS);
    let pressed: Vec<_> = fills.iter().filter(|f| f.0 == 4).collect();
    assert_eq!(pressed.len(), 1);
    assert_fill(&pressed[0].1, [0xFF, 0xFF, 0xFF, 0x23]);
    let (scene, _, _) = draw(&mut h, 1100, &ROWS);
    assert_eq!(lit(&scene, &ROWS), [4], "one row, the press's");
    // Dragged off the row, the press still shows and nothing else lights.
    h.input.pointer_to(5.0, 390.0);
    draw(&mut h, 1110, &ROWS);
    let (scene, _, _) = draw(&mut h, 1400, &ROWS);
    assert_eq!(lit(&scene, &ROWS), [4]);
    assert_fill(&row_fills(&scene, &ROWS)[0].1, DARK.menu_item_pressed);
    // An arrow while held moves the highlight, and the held row's fill gives way to it.
    key(&mut h, NamedKey::ArrowDown);
    draw(&mut h, 1410, &ROWS);
    assert_eq!(lit(&draw(&mut h, 1700, &ROWS).0, &ROWS), [0]);
}

#[test]
fn the_pressed_fill_ends_with_the_click_that_picks() {
    let mut h = pointer_open(&ROWS);
    let (x, y) = centre_of(&ROWS, 1);
    h.input.pointer_to(x, y);
    draw(&mut h, 310, &ROWS);
    h.input.press(P);
    draw(&mut h, 500, &ROWS);
    h.input.release(P);
    let (scene, r, _) = draw(&mut h, 510, &ROWS);
    assert_eq!(r.picked, Some(1));
    assert_fill(&row_fills(&scene, &ROWS)[0].1, DARK.menu_item_hover);
}

#[test]
fn a_row_disabled_while_lit_goes_out_and_enter_picks_nothing() {
    let on = [entry("a", false), entry("b", false), entry("c", false)];
    let off = [entry("a", false), entry("b", true), entry("c", false)];
    let mut h = keyboard_open(&on);
    assert_eq!(on_key(&mut h, 310, NamedKey::ArrowDown, &on), [1]);
    draw(&mut h, 320, &on);
    assert_eq!(lit(&draw(&mut h, 330, &off).0, &off), [] as [usize; 0]);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::Enter)),
        "go",
    ));
    key(&mut h, NamedKey::Enter);
    let (_, r, out) = draw(&mut h, 340, &off);
    assert_eq!(r.picked, None, "the focused row is the disabled one");
    assert!(out.fired.is_empty(), "the key is still taken");
    // The keys go on from there.
    assert_eq!(on_key(&mut h, 350, NamedKey::ArrowDown, &off), [0]);
}

#[test]
fn a_menu_of_disabled_rows_moves_nothing_and_escape_still_closes() {
    let rows = [entry("a", true), MenuRow::Separator, entry("b", true)];
    let mut h = pointer_open(&rows);
    key(&mut h, NamedKey::ArrowDown);
    let (scene, _, out) = draw(&mut h, 310, &rows);
    assert_eq!(lit(&scene, &rows), [] as [usize; 0]);
    assert_eq!(out.focused, None);
    key(&mut h, NamedKey::Escape);
    assert!(draw(&mut h, 320, &rows).1.dismissed);
}

#[test]
fn a_disabled_row_takes_no_pointer_no_action_and_no_fill_but_blocks() {
    let mut h = pointer_open(&VARIANTS);
    let (x, y) = centre_of(&VARIANTS, 2);
    h.input.pointer_to(x, y);
    draw(&mut h, 310, &VARIANTS);
    let (scene, _, _) = draw(&mut h, 600, &VARIANTS);
    assert_eq!(lit(&scene, &VARIANTS), [] as [usize; 0], "no hover");
    h.input.click(P);
    let (_, r, out) = draw(&mut h, 610, &VARIANTS);
    assert_eq!(
        (r.picked, r.dismissed),
        (None, false),
        "blocks, takes nothing"
    );
    assert_eq!(out.focused, Some(menu_id().with(1_usize)), "focus stays");
    h.input.access(AccessAction {
        target: menu_id().with(2_usize),
        action: Action::Default,
        value: None,
    });
    assert_eq!(draw(&mut h, 620, &VARIANTS).1.picked, None);
    let (_, _, out) = draw(&mut h, 630, &VARIANTS);
    let node = out.tree.get(menu_id().with(2_usize)).expect("its node");
    assert_eq!(node.role, Role::MenuItem);
    assert!(node.states.contains(State::Disabled));
    assert!(!node.states.contains(State::Focusable));
    assert!(!node.actions.contains(Action::Default));
}

#[test]
fn space_picks_as_enter_does() {
    let mut h = keyboard_open(&VARIANTS);
    assert_eq!(on_key(&mut h, 310, NamedKey::End, &VARIANTS), [5]);
    key(&mut h, NamedKey::Space);
    assert_eq!(draw(&mut h, 320, &VARIANTS).1.picked, Some(5));
}

#[test]
fn a_global_binding_on_a_key_the_menu_takes_does_not_fire() {
    for k in [
        NamedKey::Enter,
        NamedKey::Space,
        NamedKey::ArrowDown,
        NamedKey::ArrowUp,
        NamedKey::Home,
        NamedKey::End,
    ] {
        let mut h = keyboard_open(&ROWS);
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        key(&mut h, k);
        let (_, _, out) = draw(&mut h, 310, &ROWS);
        assert!(out.fired.is_empty(), "{k:?}");
    }
}

#[test]
fn a_chord_on_an_arrow_stays_a_shortcut() {
    let mut h = keyboard_open(&ROWS);
    let chord = Chord {
        mods: Mods {
            logo: true,
            ..Mods::default()
        },
        ..Chord::key(Key::Named(NamedKey::ArrowDown))
    };
    h.shortcuts.add(Binding::new(Scope::Global, chord, "go"));
    h.input.set_modifiers(Modifiers {
        logo: true,
        ..Modifiers::default()
    });
    key(&mut h, NamedKey::ArrowDown);
    h.input.set_modifiers(Modifiers::default());
    let (scene, _, out) = draw(&mut h, 310, &ROWS);
    assert_eq!(out.fired.len(), 1);
    assert_eq!(lit(&scene, &ROWS), [0], "the highlight stays");
}

#[test]
fn a_disabled_scope_takes_the_keys_picks_nothing_and_marks_every_node() {
    let keys = [
        NamedKey::Enter,
        NamedKey::Space,
        NamedKey::ArrowDown,
        NamedKey::ArrowUp,
        NamedKey::Home,
        NamedKey::End,
    ];
    let mut h = keyboard_open(&VARIANTS);
    for k in keys {
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
    }
    let mut r = MenuResponse::default();
    let mut frame = |h: &mut UiHarness, ms: u64| {
        h.at_ms(ms).frame(|ui| {
            ui.with_enabled(false, |ui| r = ui.menu("m", &VARIANTS, AT));
        })
    };
    frame(&mut h, 310);
    for (i, k) in keys.iter().enumerate() {
        key(&mut h, *k);
        let (_, out) = frame(&mut h, 320 + 10 * i as u64);
        assert!(out.fired.is_empty(), "{k:?}");
    }
    let (_, out) = frame(&mut h, 400);
    assert_eq!(r.picked, None);
    assert_eq!(disabled_violations(&out.tree, menu_id()), []);
    assert!(out.tree.get(menu_id()).is_some());
}

#[test]
fn tab_stays_on_the_rows_one_stop() {
    let mut h = keyboard_open(&ROWS);
    on_key(&mut h, 310, NamedKey::ArrowDown, &ROWS);
    draw(&mut h, 320, &ROWS);
    key(&mut h, NamedKey::Tab);
    let (scene, _, out) = draw(&mut h, 330, &ROWS);
    assert_eq!(out.focused, Some(menu_id().with(1_usize)));
    assert_eq!(lit(&scene, &ROWS), [1]);
}

#[test]
fn the_header_takes_no_key_and_no_click_and_is_read_as_text() {
    let layout = layout_of(&VARIANTS);
    assert_eq!(layout.rows[0].h, HEADER);
    assert_eq!(layout.rows[1].y, layout.panel.y + 6.0 + HEADER);
    let mut h = keyboard_open(&VARIANTS);
    assert_eq!(on_key(&mut h, 310, NamedKey::Home, &VARIANTS), [1]);
    let (x, y) = centre_of(&VARIANTS, 0);
    h.input.pointer_to(x, y).click(P);
    let (_, r, out) = draw(&mut h, 320, &VARIANTS);
    assert_eq!((r.picked, r.dismissed), (None, false));
    let node = out.tree.get(menu_id().with(0_usize)).expect("its node");
    assert_eq!(node.role, Role::StaticText);
    assert_eq!(node.name, "Serving on :4646");
    assert_eq!(node.description.as_deref(), Some("3 sessions"));
    assert!(!node.states.contains(State::Focusable));
    let (scene, _, _) = draw(&mut h, 330, &VARIANTS);
    assert_eq!(lit(&scene, &VARIANTS), [1], "its hover lights nothing");
}

#[test]
fn a_wide_header_widens_the_panel() {
    let name = "A header name well wider than the least width of a menu";
    let rows = [
        MenuRow::Header {
            name,
            status: "",
            tone: Tone::Ok,
        },
        entry("a", false),
    ];
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let spec = Spec::plain(
        name,
        Style::ui(DARK.font_base, DARK.menu_text_primary).medium(),
    );
    let nw = p.measure(&spec).0;
    let w = menu_layout(&mut p, &rows, AT).panel.w;
    assert_eq!(w, 12.0 + 8.0 + 8.0 + 8.0 + nw + 8.0);
}

/// The texts on the popup layer.
fn text_keys(scene: &Scene) -> Vec<TextKey> {
    let layer = &scene.layers[LayerId::Popup as usize];
    layer.texts.iter().map(|t| t.key).collect()
}

fn key_of(spec: &Spec) -> TextKey {
    TextKey(spec.cache_key(1.0))
}

#[test]
fn the_variants_draw_their_colours() {
    let t = DARK;
    let mut h = pointer_open(&VARIANTS);
    let (scene, _, _) = draw(&mut h, 310, &VARIANTS);
    let keys = text_keys(&scene);
    let label = |text, c| key_of(&Spec::plain(text, Style::ui(12.0, c)));
    let hint = |text, c| key_of(&Spec::plain(text, Style::ui(t.font_s, c)));
    let medium = Style::ui(t.font_base, t.menu_text_primary).medium();
    let status = Style::ui(t.font_s, t.menu_text_tertiary);
    for (what, k) in [
        ("name", key_of(&Spec::plain("Serving on :4646", medium))),
        ("status", key_of(&Spec::plain("3 sessions", status))),
        ("plain", label("Plain", t.menu_text_primary)),
        (
            "plain glyph",
            key_of(&icon::glyph(icon::ADD, 13.0, t.menu_text_tertiary)),
        ),
        ("plain hint", hint("Cmd+P", t.menu_text_quaternary)),
        ("disabled", label("Disabled", t.menu_text_disabled)),
        (
            "disabled glyph",
            key_of(&icon::glyph(icon::EDIT, 13.0, t.menu_text_disabled)),
        ),
        ("disabled hint", hint("Cmd+D", t.menu_text_disabled)),
        ("danger", label("Quit", t.err)),
        (
            "danger glyph",
            key_of(&icon::glyph(icon::CLOSE, 13.0, t.err)),
        ),
        (
            "chevron",
            key_of(&icon::glyph(
                icon::CHEVRON_RIGHT,
                12.0,
                t.menu_text_quaternary,
            )),
        ),
        (
            "chevron row hint",
            hint("beside the chevron", t.menu_text_quaternary),
        ),
    ] {
        assert!(keys.contains(&k), "{what}");
    }
}

/// A row with a hint and a chevron draws both: the chevron 8 in from the row's right edge, the
/// hint right-aligned 2 before it, and the panel counts 16 + hint + 2 + 12.
#[test]
fn a_row_draws_its_hint_and_its_chevron() {
    let rows = [MenuRow::Entry(MenuItem {
        text: "Work",
        shortcut: "3",
        chevron: true,
        ..MenuItem::default()
    })];
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let hint_w = p
        .measure(&Spec::plain(
            "3",
            Style::ui(DARK.font_s, DARK.menu_text_quaternary),
        ))
        .0;
    let label_w = p
        .measure(&Spec::plain(
            "Work",
            Style::ui(12.0, DARK.menu_text_primary),
        ))
        .0;
    let w = menu_layout(&mut p, &rows, AT).panel.w;
    let want = (12.0 + 8.0 + 14.0 + 8.0 + label_w + 16.0 + hint_w + 2.0 + 12.0 + 8.0).max(190.0);
    assert_eq!(w, want);
}

#[test]
fn the_header_dot_is_the_status_dot_in_its_tone() {
    let mut h = pointer_open(&VARIANTS);
    let (scene, _, _) = draw(&mut h, 310, &VARIANTS);
    let r = layout_of(&VARIANTS).rows[0];
    let want = [r.x + 8.0, r.y + 6.0 + 10.0 - 4.0, 8.0, 8.0];
    let dot = scene.layers[LayerId::Popup as usize]
        .geometry
        .under
        .iter()
        .find(|s| s.rect == want)
        .expect("the dot");
    assert_fill(dot, Tone::Ok.color(&DARK));
}

#[test]
fn an_item_row_is_an_entry_with_the_flags_off() {
    let entries: Vec<MenuRow<'_>> = ROWS
        .iter()
        .map(|r| match *r {
            MenuRow::Item {
                glyph,
                text,
                shortcut,
            } => MenuRow::Entry(MenuItem {
                glyph,
                text,
                shortcut,
                ..MenuItem::default()
            }),
            other => other,
        })
        .collect();
    let mut a = keyboard_open(&ROWS);
    let mut b = keyboard_open(&entries);
    assert_eq!(draw(&mut a, 310, &ROWS).0, draw(&mut b, 310, &entries).0);
}

/// The panel of an anchored menu of `ROWS`, measured through `menu_layout_at`.
fn anchored(rect: Rect, side: Side, align: PlaceAlign, gap: f32) -> Rect {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let at = MenuPlace::Anchor {
        rect,
        side,
        align,
        gap,
    };
    menu_layout_at(&mut p, &ROWS, &at).panel
}

#[test]
fn an_anchored_menu_flips_and_shifts_by_place() {
    let (w, h) = (layout_of(&ROWS).panel.w, layout_of(&ROWS).panel.h);
    let (b, s) = (Side::Bottom, PlaceAlign::Start);
    let below = anchored(Rect::new(100.0, 40.0, 80.0, 30.0), b, s, 4.0);
    assert_eq!(below, Rect::new(100.0, 74.0, w, h));
    let flipped = anchored(Rect::new(100.0, 340.0, 80.0, 30.0), b, s, 4.0);
    assert_eq!(flipped, Rect::new(100.0, 336.0 - h, w, h));
    let shifted = anchored(Rect::new(560.0, 40.0, 30.0, 30.0), b, s, 4.0);
    assert_eq!(shifted, Rect::new(WIN.0 - 8.0 - w, 74.0, w, h));
    let right = anchored(Rect::new(10.0, 40.0, 30.0, 30.0), Side::Right, s, 2.0);
    assert_eq!(right, Rect::new(42.0, 40.0, w, h));
    let end = anchored(Rect::new(300.0, 40.0, 80.0, 30.0), b, PlaceAlign::End, 0.0);
    assert_eq!(end, Rect::new(380.0 - w, 70.0, w, h));
}

#[test]
fn at_is_a_zero_size_anchor_below_the_point() {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let by_at = menu_layout_at(&mut p, &ROWS, &MenuPlace::At(AT));
    assert_eq!(by_at, menu_layout(&mut p, &ROWS, AT));
    assert_eq!((by_at.panel.x, by_at.panel.y), AT);
}

#[test]
fn the_menu_node_is_always_named() {
    let name_of = |name: &str| {
        let mut h = UiHarness::new(WIN);
        let options = MenuOptions {
            place: MenuPlace::At(AT),
            anchor_id: None,
            name,
        };
        let (_, out) = h.frame(|ui| {
            ui.menu_with("m", &ROWS, &options);
        });
        out.tree.get(menu_id()).expect("menu node").name.clone()
    };
    assert_eq!(name_of(""), "Menu");
    assert_eq!(name_of("Tab actions"), "Tab actions");
    let mut h = UiHarness::new(WIN);
    h.strings.insert("luneui.menu.name", "Menü").expect("key");
    let (_, _, out) = draw(&mut h, 0, &ROWS);
    assert_eq!(out.tree.get(menu_id()).expect("node").name, "Menü");
}

const TRIGGER: Rect = Rect::new(100.0, 40.0, 80.0, 30.0);

/// A trigger button that opens an anchored menu, as an app does; returns its clicks.
fn triggered(ui: &mut Ui<'_>, open: &mut bool, focusable: bool) -> bool {
    let id = ui.id("t");
    let clicked = if focusable {
        ui.button("t", TRIGGER, "Open", ButtonKind::Secondary)
            .clicked
    } else {
        ui.interact(id, TRIGGER, Sense::CLICK).clicked
    };
    *open |= clicked;
    if *open {
        let options = MenuOptions {
            place: MenuPlace::Anchor {
                rect: TRIGGER,
                side: Side::Bottom,
                align: PlaceAlign::Start,
                gap: 4.0,
            },
            anchor_id: Some(id),
            name: "",
        };
        let m = ui.menu_with("m", &ROWS, &options);
        if m.picked.is_some() || m.dismissed {
            *open = false;
        }
    }
    clicked
}

#[test]
fn a_press_on_the_trigger_while_open_closes_and_does_not_reopen() {
    let mut h = UiHarness::new(WIN);
    let mut open = false;
    h.at_ms(0).frame(|ui| {
        triggered(ui, &mut open, true);
    });
    h.input.pointer_to(140.0, 55.0).click(P);
    h.at_ms(10).frame(|ui| {
        triggered(ui, &mut open, true);
    });
    assert!(open);
    h.at_ms(300).frame(|ui| {
        triggered(ui, &mut open, true);
    });
    h.input.press(P);
    let mut clicked = false;
    h.at_ms(310)
        .frame(|ui| clicked |= triggered(ui, &mut open, true));
    assert!(!open, "dismissed");
    h.input.release(P);
    h.at_ms(320)
        .frame(|ui| clicked |= triggered(ui, &mut open, true));
    assert!(!clicked && !open, "the press reached nothing");
}

#[test]
fn anchor_id_takes_focus_on_close_when_nothing_was_focused_at_open() {
    let mut h = UiHarness::new(WIN);
    let mut open = true;
    h.at_ms(0).frame(|ui| {
        ui.button("t", TRIGGER, "Open", ButtonKind::Secondary);
        let options = MenuOptions {
            place: MenuPlace::At(AT),
            anchor_id: Some(ui.id("t")),
            name: "",
        };
        ui.menu_with("m", &ROWS, &options);
    });
    key(&mut h, NamedKey::Escape);
    let (_, out) = h.at_ms(10).frame(|ui| {
        ui.button("t", TRIGGER, "Open", ButtonKind::Secondary);
        let options = MenuOptions {
            place: MenuPlace::At(AT),
            anchor_id: Some(ui.id("t")),
            name: "",
        };
        open = !ui.menu_with("m", &ROWS, &options).dismissed;
    });
    assert!(!open);
    assert_eq!(
        out.focused,
        Some(menu_id().with(0_usize)),
        "still open here"
    );
    let (_, out) = h.at_ms(20).frame(|ui| {
        ui.button("t", TRIGGER, "Open", ButtonKind::Secondary);
    });
    assert_eq!(out.focused, Some(WidgetId::ROOT.with("t")));
}

#[test]
fn the_row_hover_conforms_under_every_tier_and_fades_through_the_ui() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&HOVER, &DARK.motion, tier, 0.0, 1.0)
            .unwrap_or_else(|e| panic!("{tier:?}: {e}"));
    }
    let (x, y) = centre_of(&ROWS, 1);
    let alpha = |scene: &Scene| {
        row_fills(scene, &ROWS)
            .first()
            .map_or(0.0, |f| f.1.color[3])
    };
    let full = f32::from(DARK.menu_item_hover[3]) / 255.0;
    for tier in [MotionTier::Full, MotionTier::Reduced] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        draw(&mut h, 0, &ROWS);
        draw(&mut h, 300, &ROWS);
        h.input.pointer_to(x, y);
        assert_eq!(alpha(&draw(&mut h, 310, &ROWS).0), 0.0, "{tier:?} starts");
        let half = alpha(&draw(&mut h, 370, &ROWS).0);
        assert!((half - full / 2.0).abs() < 0.01, "{tier:?}: {half}");
        let (scene, _, out) = draw(&mut h, 430, &ROWS);
        assert!(
            (alpha(&scene) - full).abs() < 1e-5,
            "{tier:?}: 120 ms, linear"
        );
        assert_eq!(out.repaint, Repaint::Idle, "{tier:?}");
    }
    let mut h = UiHarness::new(WIN);
    h.tier = MotionTier::Off;
    draw(&mut h, 0, &ROWS);
    h.input.pointer_to(x, y);
    assert!(
        (alpha(&draw(&mut h, 10, &ROWS).0) - full).abs() < 1e-5,
        "Off snaps"
    );
}

#[test]
fn the_exit_painter_snaps_the_fill_and_keeps_the_variants() {
    let under = |scene: &Scene| scene.layers[LayerId::Popup as usize].geometry.under.len();
    let mut h = keyboard_open(&VARIANTS);
    let (open, _, _) = draw(&mut h, 310, &VARIANTS);
    assert_eq!(lit(&open, &VARIANTS), [1]);
    h.at_ms(1000).frame(|_| {});
    let (closing, _) = h.at_ms(1060).frame(|_| {});
    assert_eq!(under(&closing), under(&open) - 1, "all but the row's fill");
    assert_eq!(text_keys(&closing).len(), text_keys(&open).len());
}

#[test]
fn a_row_the_pointer_lit_loses_its_fill_at_once_when_disabled() {
    let on = [entry("a", false), entry("b", false)];
    let off = [entry("a", false), entry("b", true)];
    let mut h = pointer_open(&on);
    let (x, y) = centre_of(&on, 1);
    h.input.pointer_to(x, y);
    draw(&mut h, 310, &on);
    assert_eq!(lit(&draw(&mut h, 500, &on).0, &on), [1]);
    assert_eq!(lit(&draw(&mut h, 510, &off).0, &off), [] as [usize; 0]);
    // Enabled again under the resting pointer, the row counts as entered.
    draw(&mut h, 520, &on);
    assert_eq!(lit(&draw(&mut h, 800, &on).0, &on), [1]);
}

/// The measures of the entry's Acceptance at scale 2: the panel is at least 380 device px wide
/// with a radius of 18, a row is 56 high, a separator 22, a hovered row is filled #FFFFFF16.
#[test]
fn at_scale_2_the_panel_is_380_wide_with_radius_18_rows_56_high_and_separators_22() {
    use lune_ui_core::testing::ScriptedInput;
    let rows = [
        entry("Alpha", false),
        MenuRow::Separator,
        entry("Beta", false),
    ];
    let lay = layout_of(&rows);
    // The layout is logical: half of each device measure.
    assert_eq!((lay.rows[0].h, lay.rows[1].h), (28.0, 11.0));
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    draw(&mut h, 0, &rows);
    let (x, y) = centre_of(&rows, 0);
    h.input.pointer_to(x, y);
    draw(&mut h, 300, &rows);
    let (scene, _, _) = draw(&mut h, 600, &rows);
    let under = &scene.layers[LayerId::Popup as usize].geometry.under;
    let panel = under
        .iter()
        .max_by(|a, b| (a.rect[2] * a.rect[3]).total_cmp(&(b.rect[2] * b.rect[3])))
        .expect("the panel");
    assert_eq!(panel.rect[2], 380.0, "the minimum width");
    assert_eq!(panel.radii, [18.0; 4]);
    let lit = under
        .iter()
        .find(|s| s.rect[3] == 56.0 && s.color[3] > 0.0 && s.rect[2] < panel.rect[2])
        .expect("a lit 56 px row");
    assert_fill(lit, [255, 255, 255, 0x16]);
}
