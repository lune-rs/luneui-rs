//! The menu (C4 retrofit, `overlay.menu` acceptance): bit-exact rest and motion against the
//! Painter-API `menu` (D7), exit retention per tier, curve conformance, picking, dismissal and the
//! semantic tree.

use lune_ui_core::access::{Action, Role, State};
use lune_ui_core::icon;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Sense, WidgetId};
use lune_ui_core::widgets::menu::{
    CLOSE, MenuEntry, MenuResponse, MenuRow, OPEN, menu, menu_close_anim, menu_layout,
    menu_open_anim,
};

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
        assert_eq!(idle_after, ms, "{tier:?}");
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
