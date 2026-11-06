//! The tray's pure logic (`platform.tray-icon`): the icon raster, the placement from the icon's
//! rect, the click rules, and the menu in a window of its own.

use std::time::Duration;

use lune_ui_core::access::Role;
use lune_ui_core::icon;
use lune_ui_core::input::{FrameOutput, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::tray::{
    GlyphTone, TrayAction, TrayBitmap, TrayPlatform, TrayState, TraySurface, TrayToggle,
    bar_from_tray, icon_bitmap, place_surface, tray_surface,
};
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::menu::{MenuItem, MenuResponse, MenuRow, menu_size};

const STATES: [TrayState; 3] = [TrayState::Serving, TrayState::Degraded, TrayState::Paused];

fn bitmap(state: TrayState, tone: GlyphTone, scale: u32) -> TrayBitmap {
    icon_bitmap(state, tone, scale, &DARK)
}

fn px(b: &TrayBitmap, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * b.width + x) * 4) as usize;
    [b.rgba[i], b.rgba[i + 1], b.rgba[i + 2], b.rgba[i + 3]]
}

/// A pixel centre's distance from the dot's centre, in bitmap pixels (the dot's centre is
/// `13.5 * scale`, its outer circle flush with the 18 px side).
fn dist(x: u32, y: u32, scale: u32) -> f32 {
    let c = 13.5 * scale as f32;
    let (dx, dy) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
    (dx * dx + dy * dy).sqrt()
}

fn dot_of(state: TrayState) -> [u8; 4] {
    match state {
        TrayState::Serving => DARK.ok,
        TrayState::Degraded => DARK.warn,
        TrayState::Paused => DARK.paused,
    }
}

fn each(b: &TrayBitmap) -> impl Iterator<Item = (u32, u32)> + use<'_> {
    (0..b.height).flat_map(|y| (0..b.width).map(move |x| (x, y)))
}

#[test]
fn the_bitmap_is_18_px_at_scale_1_and_36_at_2() {
    for (scale, side) in [(1, 18), (2, 36), (0, 36), (3, 36), (7, 36)] {
        let b = bitmap(TrayState::Serving, GlyphTone::Light, scale);
        assert_eq!((b.width, b.height), (side, side), "scale {scale}");
        assert_eq!(b.rgba.len() as u32, side * side * 4, "scale {scale}");
    }
}

#[test]
fn the_dots_centre_is_exactly_the_state_colour() {
    for scale in [1_u32, 2] {
        for tone in [GlyphTone::Light, GlyphTone::Dark] {
            for state in STATES {
                let b = bitmap(state, tone, scale);
                // The four pixels round the centre (13.5 * scale), all inside the dot.
                let c = (13.5 * scale as f32) as u32;
                for (x, y) in [(c, c), (c - 1, c - 1), (c, c - 1), (c - 1, c)] {
                    assert_eq!(px(&b, x, y), dot_of(state), "{state:?} {tone:?} x{scale}");
                }
            }
        }
    }
    assert_ne!(DARK.ok, DARK.warn);
    assert_ne!(DARK.warn, DARK.paused);
}

#[test]
fn the_ring_is_transparent_and_holds_no_glyph() {
    for scale in [1_u32, 2] {
        let k = scale as f32;
        for state in STATES {
            for tone in [GlyphTone::Light, GlyphTone::Dark] {
                let b = bitmap(state, tone, scale);
                let mut ring = 0;
                for (x, y) in each(&b) {
                    let d = dist(x, y, scale);
                    // Between the dot's anti-aliased edge and the ring's.
                    if (3.0 * k + 0.51..=4.5 * k - 0.51).contains(&d) {
                        assert_eq!(px(&b, x, y), [0; 4], "({x}, {y}) at {d}");
                        ring += 1;
                    }
                }
                assert!(ring >= 8, "x{scale}: {ring} ring pixels");
            }
        }
    }
}

#[test]
fn the_states_differ_only_inside_the_dots_circle() {
    for scale in [1_u32, 2] {
        for tone in [GlyphTone::Light, GlyphTone::Dark] {
            let bitmaps: Vec<TrayBitmap> = STATES.iter().map(|s| bitmap(*s, tone, scale)).collect();
            for (a, b) in [(0, 1), (1, 2), (0, 2)] {
                let mut differ = 0;
                for (x, y) in each(&bitmaps[a]) {
                    if px(&bitmaps[a], x, y) != px(&bitmaps[b], x, y) {
                        // The dot plus half a pixel of corner.
                        assert!(
                            dist(x, y, scale) <= 3.0 * scale as f32 + 0.71,
                            "({x}, {y}) x{scale}"
                        );
                        differ += 1;
                    }
                }
                assert!(differ >= 20, "states {a} and {b}: {differ} pixels differ");
            }
        }
    }
}

#[test]
fn the_tones_differ_only_in_glyph_pixels() {
    for scale in [1_u32, 2] {
        for state in STATES {
            let light = bitmap(state, GlyphTone::Light, scale);
            let dark = bitmap(state, GlyphTone::Dark, scale);
            let mut differ = 0;
            for (x, y) in each(&light) {
                let (l, d) = (px(&light, x, y), px(&dark, x, y));
                if l == d {
                    continue;
                }
                differ += 1;
                // Outside the dot, and a glyph pixel in both: the same coverage, white or black.
                assert!(dist(x, y, scale) > 3.0 * scale as f32 + 0.71, "({x}, {y})");
                assert_eq!(l[3], d[3], "({x}, {y}): the same coverage");
                assert!(l[3] > 0);
                assert_eq!(l[..3], [255; 3]);
                assert_eq!(d[..3], [0; 3]);
            }
            assert!(differ > 20, "x{scale}: {differ} glyph pixels");
        }
    }
}

#[test]
fn the_glyph_is_in_its_box_and_the_bitmap_is_not_empty() {
    for scale in [1_u32, 2] {
        let b = bitmap(TrayState::Serving, GlyphTone::Light, scale);
        let boxed = 16 * scale;
        let in_box = each(&b)
            .filter(|&(x, y)| x < boxed && y < boxed && px(&b, x, y)[3] > 0)
            .count();
        assert!(in_box > 30 * (scale * scale) as usize, "x{scale}: {in_box}");
        // The crescent's left side is a stroke near x 4.7 (of 16) at mid height.
        let (x0, x1, y) = (4 * scale, 6 * scale, 8 * scale);
        assert!((x0..=x1).any(|x| px(&b, x, y)[3] > 200), "x{scale}");
        // Nothing past the glyph box except the dot's overhang (the strip right and below it).
        for (x, y) in each(&b) {
            let a = px(&b, x, y)[3];
            if (x >= boxed || y >= boxed) && a > 0 {
                assert!(dist(x, y, scale) <= 3.0 * scale as f32 + 0.71, "({x}, {y})");
            }
        }
    }
}

#[test]
fn the_raster_is_deterministic() {
    for scale in [1_u32, 2] {
        let a = bitmap(TrayState::Degraded, GlyphTone::Dark, scale);
        let b = bitmap(TrayState::Degraded, GlyphTone::Dark, scale);
        assert_eq!(a, b);
    }
}

#[test]
fn the_dot_follows_the_theme() {
    let mut theme = DARK;
    theme.ok = [1, 2, 3, 255];
    let b = icon_bitmap(TrayState::Serving, GlyphTone::Light, 1, &theme);
    assert_eq!(px(&b, 13, 13), [1, 2, 3, 255]);
}

const SCREEN: Rect = Rect::new(0.0, 0.0, 1920.0, 1080.0);
const FLYOUT: (f32, f32) = (340.0, 420.0);
const MENU: (f32, f32) = (200.0, 150.0);

#[test]
fn the_entrys_known_answers_for_placement() {
    // macOS: a menu bar 24 high; the menu is 4 under it, flush right.
    let bar = Rect::new(0.0, 0.0, 1920.0, 24.0);
    let r = place_surface(TraySurface::MenuUnderBar, SCREEN, bar, MENU);
    assert_eq!((r.y, r.right()), (28.0, 1920.0));
    assert_eq!((r.w, r.h), MENU);
    // Windows: a taskbar whose top is at 1040 of 1080.
    let bar = Rect::new(0.0, 1040.0, 1920.0, 40.0);
    let r = place_surface(TraySurface::FlyoutOverBar, SCREEN, bar, FLYOUT);
    assert_eq!((r.bottom(), r.right()), (1028.0, 1908.0));
    let r = place_surface(TraySurface::MenuOverBar, SCREEN, bar, MENU);
    assert_eq!((r.bottom(), r.right()), (1032.0, 1908.0));
}

#[test]
fn a_bar_on_the_top_or_a_side_places_on_the_screen_side() {
    // A taskbar on the top: the flyout hangs 12 under it.
    let top = Rect::new(0.0, 0.0, 1920.0, 40.0);
    let r = place_surface(TraySurface::FlyoutOverBar, SCREEN, top, FLYOUT);
    assert_eq!((r.y, r.right()), (52.0, 1908.0));
    // On the right, the surface goes left of it, its bottom on the screen's less the margin.
    let right = Rect::new(1880.0, 0.0, 40.0, 1080.0);
    let r = place_surface(TraySurface::FlyoutOverBar, SCREEN, right, FLYOUT);
    assert_eq!((r.right(), r.bottom()), (1880.0 - 12.0, 1080.0 - 12.0));
    let r = place_surface(TraySurface::MenuOverBar, SCREEN, right, MENU);
    assert_eq!((r.right(), r.bottom()), (1880.0 - 8.0, 1080.0 - 12.0));
    // On the left, to the right of it.
    let left = Rect::new(0.0, 0.0, 40.0, 1080.0);
    let r = place_surface(TraySurface::FlyoutOverBar, SCREEN, left, FLYOUT);
    assert_eq!((r.x, r.bottom()), (52.0, 1080.0 - 12.0));
}

#[test]
fn a_surface_taller_than_the_screen_stays_inside_as_place_allows() {
    let bar = Rect::new(0.0, 1040.0, 1920.0, 40.0);
    let r = place_surface(TraySurface::FlyoutOverBar, SCREEN, bar, (340.0, 2000.0));
    // It keeps its size and sits on the top margin; the right edge still holds.
    assert_eq!((r.y, r.h, r.right()), (12.0, 2000.0, 1908.0));
    let bar = Rect::new(0.0, 0.0, 1920.0, 24.0);
    let r = place_surface(TraySurface::MenuUnderBar, SCREEN, bar, (200.0, 2000.0));
    assert_eq!((r.y, r.h, r.right()), (0.0, 2000.0, 1920.0));
    // Wider than the screen: it keeps the left margin and its size.
    let bar = Rect::new(0.0, 1040.0, 1920.0, 40.0);
    let r = place_surface(TraySurface::FlyoutOverBar, SCREEN, bar, (3000.0, 100.0));
    assert_eq!((r.x, r.w), (12.0, 3000.0));
}

#[test]
fn a_second_monitor_places_inside_its_own_screen() {
    let screen = Rect::new(1920.0, 0.0, 1920.0, 1080.0);
    let bar = Rect::new(1920.0, 1040.0, 1920.0, 40.0);
    let r = place_surface(TraySurface::FlyoutOverBar, screen, bar, FLYOUT);
    assert_eq!((r.bottom(), r.right()), (1028.0, 3840.0 - 12.0));
}

#[test]
fn the_bar_comes_from_the_tray_rect() {
    // The probe's measured case: a 2400 x 1350 screen, the icon at (1863, -1) 34 x 33.
    let screen = Rect::new(0.0, 0.0, 2400.0, 1350.0);
    let bar = bar_from_tray(screen, Rect::new(1863.0, -1.0, 34.0, 33.0));
    assert_eq!(bar, Rect::new(0.0, 0.0, 2400.0, 32.0));
    let r = place_surface(TraySurface::MenuUnderBar, screen, bar, MENU);
    assert_eq!((r.y, r.right()), (36.0, 2400.0));
    // A taskbar at the bottom: the strip starts at the icon's top.
    let bar = bar_from_tray(SCREEN, Rect::new(1700.0, 1044.0, 24.0, 32.0));
    assert_eq!(bar, Rect::new(0.0, 1044.0, 1920.0, 36.0));
    // On a side: the icon near the right edge of a vertical taskbar.
    let bar = bar_from_tray(SCREEN, Rect::new(1884.0, 1000.0, 24.0, 24.0));
    assert_eq!(bar, Rect::new(1884.0, 0.0, 36.0, 1080.0));
    let bar = bar_from_tray(SCREEN, Rect::new(8.0, 1000.0, 24.0, 24.0));
    assert_eq!(bar, Rect::new(0.0, 0.0, 32.0, 1080.0));
}

#[test]
fn a_click_chooses_the_surface_by_platform_and_button() {
    assert_eq!(
        tray_surface(TrayPlatform::MacOs, true),
        TraySurface::MenuUnderBar
    );
    assert_eq!(
        tray_surface(TrayPlatform::MacOs, false),
        TraySurface::MenuUnderBar
    );
    assert_eq!(
        tray_surface(TrayPlatform::Windows, true),
        TraySurface::FlyoutOverBar
    );
    assert_eq!(
        tray_surface(TrayPlatform::Windows, false),
        TraySurface::MenuOverBar
    );
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

const FLY: TraySurface = TraySurface::FlyoutOverBar;
const MEN: TraySurface = TraySurface::MenuOverBar;

#[test]
fn a_click_opens_and_the_next_closes() {
    let mut t = TrayToggle::new();
    assert_eq!(t.click(FLY, ms(0)), TrayAction::Open(FLY));
    assert_eq!(t.open(), Some(FLY));
    assert_eq!(t.click(FLY, ms(1000)), TrayAction::Close);
    assert_eq!(t.open(), None);
    assert_eq!(t.click(FLY, ms(2000)), TrayAction::Open(FLY));
}

#[test]
fn the_click_that_took_focus_does_not_reopen() {
    // The click on the icon takes focus from the open surface: focus lost, then the click.
    let mut t = TrayToggle::new();
    t.click(FLY, ms(0));
    assert_eq!(t.focus_lost(ms(5000)), TrayAction::Close);
    assert_eq!(t.click(FLY, ms(5100)), TrayAction::None);
    assert_eq!(t.open(), None);

    // A later click opens.
    let mut t = TrayToggle::new();
    t.click(FLY, ms(0));
    t.focus_lost(ms(5000));
    assert_eq!(t.click(FLY, ms(5300)), TrayAction::Open(FLY));

    // The guard is 250 ms: just inside holds, at it opens.
    let mut t = TrayToggle::new();
    t.click(FLY, ms(0));
    t.focus_lost(ms(5000));
    assert_eq!(t.click(FLY, ms(5249)), TrayAction::None);
    let mut t = TrayToggle::new();
    t.click(FLY, ms(0));
    t.focus_lost(ms(5000));
    assert_eq!(t.click(FLY, ms(5250)), TrayAction::Open(FLY));

    // The ignored click is spent: the next one opens.
    let mut t = TrayToggle::new();
    t.click(FLY, ms(0));
    t.focus_lost(ms(5000));
    t.click(FLY, ms(5100));
    assert_eq!(t.click(FLY, ms(5150)), TrayAction::Open(FLY));
}

#[test]
fn a_close_of_another_kind_leaves_no_guard() {
    let mut t = TrayToggle::new();
    t.click(FLY, ms(0));
    t.closed();
    assert_eq!(t.open(), None);
    assert_eq!(t.click(FLY, ms(10)), TrayAction::Open(FLY));
}

#[test]
fn a_right_click_with_the_flyout_open_swaps_to_the_menu() {
    let mut t = TrayToggle::new();
    t.click(FLY, ms(0));
    assert_eq!(t.click(MEN, ms(100)), TrayAction::Open(MEN));
    assert_eq!(t.open(), Some(MEN));
    assert_eq!(t.click(MEN, ms(200)), TrayAction::Close);
}

#[test]
fn focus_lost_when_closed_is_nothing() {
    let mut t = TrayToggle::new();
    assert_eq!(t.focus_lost(ms(100)), TrayAction::None);
    // And it leaves no guard behind: the click right after opens.
    assert_eq!(t.click(FLY, ms(110)), TrayAction::Open(FLY));
}

// The menu in a window of its own.

const KEY: &str = "tray-menu";

fn item(text: &'static str, disabled: bool) -> MenuRow<'static> {
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::ADD),
        text,
        shortcut: "",
        disabled,
        danger: false,
        chevron: false,
    })
}

/// Row 1 is disabled; the picks are 0, 2, 3.
fn rows() -> [MenuRow<'static>; 5] {
    [
        item("Open app", false),
        item("Pause", true),
        item("Profile", false),
        MenuRow::Separator,
        item("Quit app", false),
    ]
}

fn size() -> (f32, f32) {
    let mut text = FakeText;
    let win = Rect::new(0.0, 0.0, 800.0, 600.0);
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    menu_size(&mut p, &rows())
}

fn draw(
    h: &mut UiHarness,
    ms: u64,
    on: bool,
) -> (lune_ui_core::scene::Scene, MenuResponse, FrameOutput) {
    let rows = rows();
    let mut resp = MenuResponse::default();
    let (scene, out) = h.at_ms(ms).frame(|ui| {
        if on {
            resp = ui.menu_window(KEY, &rows);
        }
    });
    (scene, resp, out)
}

/// A harness whose window is the menu's size, opened and settled past the 180 ms open.
fn opened() -> UiHarness {
    let mut h = UiHarness::new(size());
    draw(&mut h, 0, true);
    draw(&mut h, 300, true);
    h
}

fn menu_id() -> WidgetId {
    WidgetId::ROOT.with(KEY)
}

fn row_centre(out: &FrameOutput, n: usize) -> (f32, f32) {
    let r = out
        .tree
        .nodes
        .iter()
        .filter(|node| node.role == Role::MenuItem)
        .nth(n)
        .expect("row node")
        .bounds;
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

#[test]
fn the_menu_card_fills_the_window() {
    let (w, h) = size();
    assert!(w >= 190.0 && h > 100.0);
    let mut ui = opened();
    let (scene, ..) = draw(&mut ui, 600, true);
    let card = scene.layers[LayerId::Popup as usize].geometry.under[0].rect;
    assert_eq!(card, [0.0, 0.0, w, h]);
    assert!(scene.backdrop.is_some(), "a glass card asks for the blur");
}

#[test]
fn the_window_menu_is_a_menu_node_with_a_node_per_row() {
    let mut h = opened();
    let (_, _, out) = draw(&mut h, 600, true);
    let menu = out.tree.get(menu_id()).expect("menu node");
    assert_eq!(menu.role, Role::Menu);
    assert_eq!(menu.name, "Menu");
    // A node per item; the separator has none.
    assert_eq!(menu.children.len(), 4);
    let (w, hgt) = size();
    assert_eq!(
        (menu.bounds.x, menu.bounds.y, menu.bounds.w, menu.bounds.h),
        (0.0, 0.0, w, hgt)
    );
}

#[test]
fn escape_an_outside_press_and_focus_loss_dismiss_the_window_menu() {
    let mut h = opened();
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    assert!(draw(&mut h, 310, true).1.dismissed);

    // The window is the menu: a press outside it is a press in the app's other window, which
    // reaches this Ui as a focus loss. A press at a corner outside the card still dismisses.
    let mut h = opened();
    h.input.focus(false);
    assert!(draw(&mut h, 310, true).1.dismissed);

    // A press on a separator or the padding is not outside.
    let mut h = opened();
    for (x, y) in [(3.0, 3.0), (100.0, 3.0)] {
        h.input.pointer_to(x, y).click(PointerButton::Primary);
        let (_, r, _) = draw(&mut h, 330, true);
        assert!(!r.dismissed && r.picked.is_none(), "({x}, {y})");
    }
}

#[test]
fn a_focus_loss_before_the_menu_first_opened_does_nothing() {
    let mut h = UiHarness::new(size());
    h.input.focus(false);
    let (_, r, _) = draw(&mut h, 0, false);
    assert!(!r.dismissed);
    // Nothing was open, so the first frame that opens it is not dismissed either.
    let (_, r, _) = draw(&mut h, 10, true);
    assert!(!r.dismissed && r.picked.is_none());
    let (_, r, _) = draw(&mut h, 400, true);
    assert!(!r.dismissed);
}

#[test]
fn a_click_picks_once_and_a_disabled_row_picks_nothing() {
    let mut h = opened();
    let (_, _, out) = draw(&mut h, 310, true);
    let (x, y) = row_centre(&out, 2);
    h.input.pointer_to(x, y);
    draw(&mut h, 320, true);
    h.input.click(PointerButton::Primary);
    let (_, r, _) = draw(&mut h, 330, true);
    assert_eq!(r.picked, Some(2));
    assert_eq!(draw(&mut h, 340, true).1.picked, None, "once");
    let (x, y) = row_centre(&out, 1);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    let (_, r, _) = draw(&mut h, 350, true);
    assert!(r.picked.is_none() && !r.dismissed);
}

#[test]
fn enter_and_space_pick_the_first_row_once_and_the_arrows_skip_a_disabled_one() {
    for k in [NamedKey::Enter, NamedKey::Space] {
        let mut h = opened();
        draw(&mut h, 310, true);
        h.input.key_down(Key::Named(k), None);
        assert_eq!(draw(&mut h, 320, true).1.picked, Some(0), "{k:?}");
        assert_eq!(draw(&mut h, 330, true).1.picked, None, "{k:?}: once");
    }
    let mut h = opened();
    draw(&mut h, 310, true);
    // Row 3 is the separator: the picks are 0, 2, 4.
    for (ms, k, row) in [
        (320, NamedKey::ArrowDown, 0),
        (330, NamedKey::ArrowDown, 2),
        (340, NamedKey::ArrowDown, 4),
        (350, NamedKey::ArrowDown, 0),
        (360, NamedKey::ArrowUp, 4),
        (370, NamedKey::Home, 0),
        (380, NamedKey::End, 4),
    ] {
        h.input.key_down(Key::Named(k), None);
        draw(&mut h, ms, true);
        h.input.key_down(Key::Named(NamedKey::Enter), None);
        let (_, r, _) = draw(&mut h, ms + 5, true);
        assert_eq!(r.picked, Some(row), "{k:?}");
    }
}

#[test]
fn the_window_menu_opens_and_closes_with_the_menus_motion_and_goes_idle() {
    let mut h = UiHarness::new(size());
    // Opening: opacity only grows and is settled by 180 ms.
    let mut last = 0.0;
    let rest = {
        let mut r = UiHarness::new(size());
        draw(&mut r, 0, true);
        let (scene, ..) = draw(&mut r, 400, true);
        scene.layers[LayerId::Popup as usize].geometry.under[0].color[3]
    };
    for k in 0..=15_u64 {
        let dt = (k * 1000 + 30) / 60;
        let (scene, _, out) = draw(&mut h, dt, true);
        let a = scene.layers[LayerId::Popup as usize].geometry.under[0].color[3];
        assert!(a >= last && a.is_finite(), "{dt}: {a} after {last}");
        last = a;
        if dt >= 180 + 17 {
            assert_eq!(a, rest, "settled at {dt} ms");
            assert_eq!(out.repaint, Repaint::Idle);
        }
    }
    // Closing: the fade replays the card and the blur for 120 ms, then idle.
    let mut t = 1000;
    let mut alphas = Vec::new();
    loop {
        let (scene, _, out) = draw(&mut h, t, false);
        if let Some(s) = scene.layers[LayerId::Popup as usize].geometry.under.first() {
            alphas.push(s.color[3]);
            assert!(scene.backdrop.is_some(), "the blur goes with the fade");
        }
        if out.repaint == Repaint::Idle {
            break;
        }
        t += 10;
        assert!(t < 2000, "never settled");
    }
    assert_eq!(t - 1000, 120);
    assert_eq!(alphas.len(), 12);
    assert!(alphas.windows(2).all(|w| w[1] <= w[0]));
    let (scene, ..) = draw(&mut h, t + 50, false);
    assert!(scene.backdrop.is_none(), "gone once the close is over");
}
