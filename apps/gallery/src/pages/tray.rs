//! The `tray` page (`platform.tray-icon`, M10): the tray icon's three states at both scales as the
//! CPU raster draws them, the icon on a light bar, where the flyout and the menu go from the
//! icon's rect on macOS and on Windows, and the two surfaces filling a window of their own. In
//! the window on macOS and Windows the page also puts a real item in the tray (`popover.rs`): its
//! state follows the page's buttons, and a click opens the surface in a second window.
//!
//! The scenes are dumps, which have no OS material, so the two surface scenes draw over a flat
//! stand-in for the desktop; the OS glass is what the live window adds.

use lune_ui::icon;
use lune_ui::image::{Fit, ImageAlign, ImageData, ImageId};
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Painter, Rect};
use lune_ui::text::{Spec, Style};
use lune_ui::tray::{
    GlyphTone, TrayBitmap, TrayState, TraySurface, bar_from_tray, icon_bitmap, place_surface,
};
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::flyout::{Flyout, flyout_size};
use lune_ui::widgets::menu::{MenuItem, MenuRow, menu_size};
use lune_ui::widgets::popover::{PlaceAlign, Side};

use super::flyout::{ROUTES, rows, state_text, state_tone, stats};
use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "tray",
    title: "Tray",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The bar colours the icon scenes draw on: a dark menu bar and a light one.
const DARK_BAR: [u8; 4] = [0x25, 0x25, 0x28, 0xFF];
const LIGHT_BAR: [u8; 4] = [0xE6, 0xE6, 0xE9, 0xFF];
/// The flat colour that stands for the desktop behind a surface in a dump.
const DESKTOP: [u8; 4] = [0x3A, 0x3C, 0x44, 0xFF];
/// The side, in physical pixels, of the enlarged copy: 8 times the 18 px bitmap, 4 times the 36 px.
const ENLARGED: u32 = 144;

const STATES: [TrayState; 3] = [TrayState::Serving, TrayState::Degraded, TrayState::Paused];

fn state_index(state: TrayState) -> u64 {
    match state {
        TrayState::Serving => 0,
        TrayState::Degraded => 1,
        TrayState::Paused => 2,
    }
}

fn state_name(state: TrayState) -> &'static str {
    match state {
        TrayState::Serving => "serving",
        TrayState::Degraded => "degraded",
        TrayState::Paused => "paused",
    }
}

/// The id of the icon for `state`: the bitmap itself (`false`) or its enlarged copy (`true`).
fn image_id(state: TrayState, enlarged: bool) -> ImageId {
    ImageId(1100 + state_index(state) * 2 + u64::from(enlarged))
}

fn picture(bitmap: &TrayBitmap) -> ImageData {
    ImageData {
        width: bitmap.width,
        height: bitmap.height,
        rgba: bitmap.rgba.clone(),
    }
}

/// `bitmap` with every pixel repeated `factor` times each way: nearest neighbour, so the pixels
/// can be judged one by one.
fn enlarge(bitmap: &TrayBitmap, factor: u32) -> ImageData {
    let (w, h, f) = (
        bitmap.width as usize,
        bitmap.height as usize,
        factor as usize,
    );
    let mut rgba = Vec::with_capacity(w * h * f * f * 4);
    for y in 0..h * f {
        for x in 0..w * f {
            let at = ((y / f) * w + x / f) * 4;
            rgba.extend_from_slice(&bitmap.rgba[at..at + 4]);
        }
    }
    ImageData {
        width: bitmap.width * factor,
        height: bitmap.height * factor,
        rgba,
    }
}

/// The pictures the icon of `state` draws at `scale`, in `tone`.
fn pictures(state: TrayState, tone: GlyphTone, scale: u32) -> Vec<(ImageId, ImageData)> {
    let bitmap = icon_bitmap(state, tone, scale, &THEME);
    let factor = ENLARGED / bitmap.width;
    vec![
        (image_id(state, false), picture(&bitmap)),
        (image_id(state, true), enlarge(&bitmap, factor)),
    ]
}

/// The icon of one state at one scale, on a bar, and the same pixels enlarged.
struct Icon {
    state: TrayState,
    tone: GlyphTone,
    scale: u32,
    bar: [u8; 4],
}

impl PageState for Icon {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "Tray icon");
        let (x, y) = (area.x + 24.0, area.y + 24.0);
        heading(ui, "TRAY ICON", x, y);
        let side = 18 * self.scale;
        caption(
            ui,
            &format!(
                "{}, scale {}: a {side} px bitmap drawn 1:1, so one bitmap pixel is one screen pixel",
                state_name(self.state),
                self.scale
            ),
            x,
            y + 20.0,
        );
        let strip = Rect::new(x, y + 56.0, 260.0, 32.0);
        let icon_at = Rect::new(strip.right() - 32.0, strip.y + 7.0, 18.0, 18.0);
        heading(ui, "PIXELS, ENLARGED", x, y + 108.0);
        let enlarged = ENLARGED as f32 / self.scale as f32;
        let well = Rect::new(x, y + 130.0, enlarged + 16.0, enlarged + 16.0);
        let pic = Rect::new(well.x + 8.0, well.y + 8.0, enlarged, enlarged);
        let p = ui.painter();
        p.fill(strip, 4.0, self.bar);
        p.image(
            icon_at,
            image_id(self.state, false),
            Fit::Contain,
            ImageAlign::CENTER,
            1.0,
        );
        p.fill(well, 4.0, self.bar);
        p.image(
            pic,
            image_id(self.state, true),
            Fit::Contain,
            ImageAlign::CENTER,
            1.0,
        );
    }

    fn images(&self) -> Vec<(ImageId, ImageData)> {
        pictures(self.state, self.tone, self.scale)
    }
}

/// `icon::<STATE, SCALE>` is the state of one icon scene; `STATE` indexes [`STATES`].
fn icon_scene<const STATE: usize, const SCALE: u32>() -> Box<dyn PageState> {
    Box::new(Icon {
        state: STATES[STATE],
        tone: GlyphTone::Light,
        scale: SCALE,
        bar: DARK_BAR,
    })
}

/// The paused icon on a light bar with a dark glyph: the ring is see-through, so no ring colour
/// can be wrong on it.
fn light_tray() -> Box<dyn PageState> {
    Box::new(Icon {
        state: TrayState::Paused,
        tone: GlyphTone::Dark,
        scale: 2,
        bar: LIGHT_BAR,
    })
}

/// A screen, its bar, the tray icon's rect and the surface placed from it, drawn at `k` times
/// life size with its top left at (`ox`, `oy`). Returns the line under it.
struct Mock {
    screen: Rect,
    tray: Rect,
    surface: TraySurface,
    size: (f32, f32),
    label: &'static str,
}

impl Mock {
    fn draw(&self, ui: &mut Ui<'_>, ox: f32, oy: f32, k: f32) {
        let t = *ui.theme();
        let bar = bar_from_tray(self.screen, self.tray);
        let placed = place_surface(self.surface, self.screen, bar, self.size);
        let to = |r: Rect| Rect::new(ox + r.x * k, oy + r.y * k, r.w * k, r.h * k);
        {
            let p = ui.painter();
            p.fill(to(self.screen), 4.0, [0x14, 0x14, 0x17, 0xFF]);
            p.fill(to(bar), 0.0, [0x2C, 0x2C, 0x32, 0xFF]);
            p.fill(to(self.tray), 2.0, t.accent);
            p.fill(to(placed), 4.0, [0x3F, 0x3F, 0x4A, 0xFF]);
            p.ring(to(placed), 4.0, 1.0, t.border, false);
        }
        let line = format!(
            "{}: x {:.0}, y {:.0}, {:.0} x {:.0} (bar y {:.0}, h {:.0}; screen {:.0} x {:.0})",
            self.label,
            placed.x,
            placed.y,
            placed.w,
            placed.h,
            bar.y,
            bar.h,
            self.screen.w,
            self.screen.h
        );
        let below = to(self.screen).bottom() + 8.0;
        let spec = Spec::plain(line, Style::ui(t.font_s, t.fg_secondary));
        ui.painter().text(&spec, ox, below);
    }
}

/// The macOS menu under a 24 px bar, its right edge on the screen's.
struct PlacementMac;

impl PageState for PlacementMac {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "Tray placement");
        let (x, y) = (area.x + 24.0, area.y + 24.0);
        heading(ui, "MACOS", x, y);
        caption(
            ui,
            "The menu sits 4 px under the menu bar and flush with the screen's right edge.",
            x,
            y + 20.0,
        );
        Mock {
            screen: Rect::new(0.0, 0.0, 1512.0, 982.0),
            tray: Rect::new(1380.0, 0.0, 34.0, 24.0),
            surface: TraySurface::MenuUnderBar,
            size: (220.0, 270.0),
            label: "menu",
        }
        .draw(ui, x, y + 56.0, 0.5);
    }
}

/// The Windows flyout and menu over a 40 px taskbar.
struct PlacementWindows;

impl PageState for PlacementWindows {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "Tray placement");
        let (x, y) = (area.x + 24.0, area.y + 24.0);
        heading(ui, "WINDOWS", x, y);
        caption(
            ui,
            "The flyout sits 12 px above the taskbar and 12 px in from the right edge; the menu 8 px above.",
            x,
            y + 20.0,
        );
        let screen = Rect::new(0.0, 0.0, 1920.0, 1080.0);
        let tray = Rect::new(1640.0, 1040.0, 32.0, 40.0);
        let k = 0.24;
        Mock {
            screen,
            tray,
            surface: TraySurface::FlyoutOverBar,
            size: (340.0, 420.0),
            label: "flyout",
        }
        .draw(ui, x, y + 56.0, k);
        Mock {
            screen,
            tray,
            surface: TraySurface::MenuOverBar,
            size: (220.0, 270.0),
            label: "menu",
        }
        .draw(ui, x + 1920.0 * k + 24.0, y + 56.0, k);
    }
}

/// One menu row, no glyph; `hint` is the text at the row's right end.
fn entry(text: &'static str, hint: &'static str, chevron: bool, danger: bool) -> MenuRow<'static> {
    MenuRow::Entry(MenuItem {
        glyph: None,
        text,
        shortcut: hint,
        disabled: false,
        danger,
        chevron,
    })
}

/// A tray menu's rows, as the mock has them per surface (`D227`). `MenuUnderBar` is macOS's: the
/// state header, then the actions with their key hints (`Cmd+O`: no bundled font has the command
/// symbol). `MenuOverBar` is Windows': no header, `Unload local model` with the memory it frees,
/// and the quit row in the error colour. The flyout's own rows are the `flyout` page's.
pub(crate) fn menu_rows(surface: TraySurface, state: TrayState) -> Vec<MenuRow<'static>> {
    let pause = if state == TrayState::Paused {
        "Resume compression"
    } else {
        "Pause compression"
    };
    if surface == TraySurface::MenuOverBar {
        return vec![
            entry("Open Wane", "", false, false),
            entry("Profile", "Work", true, false),
            entry(pause, "", false, false),
            entry("Unload local model", "frees 19.3 GB", false, false),
            MenuRow::Separator,
            entry("Quit Wane (stops proxy)", "", false, true),
        ];
    }
    let (name, status) = match state {
        TrayState::Serving => (
            "Wane is serving on :4646",
            "RM 48.20 today \u{b7} 14.6% saved",
        ),
        TrayState::Degraded => ("Wane \u{b7} anthropic degraded", "falling back to local"),
        TrayState::Paused => ("Wane \u{b7} paused", "compression is off"),
    };
    vec![
        MenuRow::Header {
            name,
            status,
            tone: state_tone(state),
        },
        MenuRow::Separator,
        entry("Open Wane", "Cmd+O", false, false),
        entry("Profile", "Work", true, false),
        entry(pause, "", false, false),
        MenuRow::Separator,
        entry("Live sessions", "3", true, false),
        entry("Unload local model", "", false, false),
        MenuRow::Separator,
        entry("Quit Wane", "Cmd+Q", false, false),
    ]
}

/// What a tray surface's frame reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SurfaceFrame {
    pub(crate) dismissed: bool,
    pub(crate) picked: Option<usize>,
}

/// The surface `surface` fills the window with, in the state `state`. A stand-in app: the
/// flyout's rows are the ones of the `flyout` page, the menus' are [`menu_rows`].
pub(crate) fn draw_surface(
    ui: &mut Ui<'_>,
    surface: TraySurface,
    state: TrayState,
) -> SurfaceFrame {
    if surface == TraySurface::FlyoutOverBar {
        let (stats, rows) = (stats(), rows(state));
        let flyout = Flyout {
            anchor: Rect::new(0.0, 0.0, 0.0, 0.0),
            anchor_id: None,
            side: Side::Top,
            align: PlaceAlign::End,
            mark: Some(icon::INFO),
            wordmark: "Wane",
            state: state_text(state),
            tone: state_tone(state),
            stats: &stats,
            routes: &ROUTES,
            rows: &rows,
            name: "Wane tray menu",
        };
        let r = ui.flyout_window("tray-flyout", &flyout);
        SurfaceFrame {
            dismissed: r.dismissed,
            picked: r.picked,
        }
    } else {
        let r = ui.menu_window("tray-menu", &menu_rows(surface, state));
        SurfaceFrame {
            dismissed: r.dismissed,
            picked: r.picked,
        }
    }
}

/// The size, in logical pixels, of the window `surface` fills in `state`.
pub(crate) fn surface_size<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    surface: TraySurface,
    state: TrayState,
) -> (f32, f32) {
    if surface == TraySurface::FlyoutOverBar {
        let (stats, rows) = (stats(), rows(state));
        let flyout = Flyout {
            anchor: Rect::new(0.0, 0.0, 0.0, 0.0),
            anchor_id: None,
            side: Side::Top,
            align: PlaceAlign::End,
            mark: Some(icon::INFO),
            wordmark: "Wane",
            state: state_text(state),
            tone: state_tone(state),
            stats: &stats,
            routes: &ROUTES,
            rows: &rows,
            name: "Wane tray menu",
        };
        flyout_size(p, &flyout)
    } else {
        menu_size(p, &menu_rows(surface, state))
    }
}

/// A surface in a window of its own, over a flat stand-in for the desktop.
struct Surface {
    surface: TraySurface,
}

impl PageState for Surface {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let t = *ui.theme();
        let win = stage.area(ui);
        {
            let p = ui.painter();
            stage.on(p, lune_ui::paint::LayerId::Base);
            p.fill(win, 0.0, DESKTOP);
        }
        for (i, line) in [
            "The window of a tray surface: the card fills it from the corner.",
            "The desktop here is a flat stand-in for the OS glass.",
        ]
        .into_iter()
        .enumerate()
        {
            let spec = Spec::plain(line, Style::ui(t.font_base, t.fg_secondary));
            ui.painter()
                .text(&spec, win.x + 400.0, win.y + 40.0 + i as f32 * 22.0);
        }
        if stage.rect().is_some() {
            // The surface is an overlay the toolkit puts at the corner of the window, so it
            // cannot be moved into a pane: say so instead of drawing it at the window's corner.
            let spec = Spec::plain(
                "This surface fills a window of its own, so a pane cannot show it. The tray page opens it for real.",
                Style::ui(t.font_base, t.fg_tertiary),
            );
            ui.painter().text(&spec, win.x + 400.0, win.y + 94.0);
            return;
        }
        draw_surface(ui, self.surface, TrayState::Serving);
    }
}

fn flyout_window() -> Box<dyn PageState> {
    Box::new(Surface {
        surface: TraySurface::FlyoutOverBar,
    })
}

fn menu_window() -> Box<dyn PageState> {
    Box::new(Surface {
        surface: TraySurface::MenuUnderBar,
    })
}

/// The window's page: the state buttons, the three icons, and what the live tray does.
struct Live {
    state: TrayState,
}

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        state: TrayState::Serving,
    })
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "Tray");
        let (x, y) = (area.x + 24.0, area.y + 24.0);
        heading(ui, "TRAY ITEM", x, y);
        let note = if cfg!(any(target_os = "macos", target_os = "windows")) {
            "This page puts an item in the tray. Click it to open the surface in a window of its own."
        } else {
            "This platform has no tray in the toolkit (Unsupported); the icons below are drawn anyway."
        };
        caption(ui, note, x, y + 20.0);
        if stage.rect().is_some() {
            // Only on the showcase's stage: the scenes' pictures stay as they are.
            caption(
                ui,
                "Real: the tray item and the clicks on it. Scripted: the scenes' pointer, and the surface they draw.",
                x,
                y + 38.0,
            );
        }
        let mut bx = x;
        for s in STATES {
            let label = state_name(s);
            let w = button_width(ui.painter(), label);
            let kind = if s == self.state {
                ButtonKind::Primary
            } else {
                ButtonKind::Secondary
            };
            let r = ui.button(
                ("state", state_index(s)),
                Rect::new(bx, y + 52.0, w, 32.0),
                label,
                kind,
            );
            if r.clicked {
                self.state = s;
            }
            bx += w + 8.0;
        }
        heading(ui, "ICONS (36 px BITMAPS)", x, y + 108.0);
        let strip = Rect::new(x, y + 130.0, 160.0, 32.0);
        let p = ui.painter();
        p.fill(strip, 4.0, DARK_BAR);
        for (i, s) in STATES.into_iter().enumerate() {
            let at = Rect::new(strip.x + 14.0 + i as f32 * 50.0, strip.y + 7.0, 18.0, 18.0);
            p.image(
                at,
                image_id(s, false),
                Fit::Contain,
                ImageAlign::CENTER,
                1.0,
            );
        }
    }

    fn images(&self) -> Vec<(ImageId, ImageData)> {
        STATES
            .into_iter()
            .flat_map(|s| pictures(s, GlyphTone::Light, 2))
            .collect()
    }

    fn tray_state(&self) -> Option<TrayState> {
        Some(self.state)
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let names: Vec<&str> = STATES.iter().map(|s| state_name(*s)).collect();
        let mut at = STATES.iter().position(|s| *s == self.state).unwrap_or(0);
        if panel.choice(ui, "Tray state", &names, &mut at) {
            self.state = STATES[at];
        }
    }
}

const fn scene(
    id: &'static str,
    scale: u32,
    state: fn() -> Box<dyn PageState>,
    capture_ms: u32,
) -> UiScene {
    UiScene {
        id,
        scale: scale as f32,
        tier: MotionTier::Full,
        state,
        script: &[],
        capture_ms,
        expect: Expect::Settled,
    }
}

const SCENES: &[UiScene] = &[
    scene("tray-icon-serving", 1, icon_scene::<0, 1>, 0),
    scene("tray-icon-serving", 2, icon_scene::<0, 2>, 0),
    scene("tray-icon-degraded", 1, icon_scene::<1, 1>, 0),
    scene("tray-icon-degraded", 2, icon_scene::<1, 2>, 0),
    scene("tray-icon-paused", 1, icon_scene::<2, 1>, 0),
    scene("tray-icon-paused", 2, icon_scene::<2, 2>, 0),
    scene("tray-icon-light-tray", 2, light_tray, 0),
    scene(
        "tray-icon-placement",
        2,
        || -> Box<dyn PageState> { Box::new(PlacementMac) },
        0,
    ),
    scene(
        "tray-icon-placement-windows",
        2,
        || -> Box<dyn PageState> { Box::new(PlacementWindows) },
        0,
    ),
    // The open is `fast` (180 ms): settled by 500 ms.
    scene("tray-flyout-window", 2, flyout_window, 500),
    scene("tray-menu-window", 2, menu_window, 500),
];

/// What the bench replays: the state buttons, one after the other.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 300,
        events: &[Ev::Tab],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use lune_ui::tray::TrayPlatform;

    #[test]
    fn the_surfaces_a_click_opens_are_the_ones_the_page_draws() {
        // The placement scenes use the same surfaces and sizes the live tray places.
        for (platform, left, surface) in [
            (TrayPlatform::MacOs, true, TraySurface::MenuUnderBar),
            (TrayPlatform::Windows, true, TraySurface::FlyoutOverBar),
            (TrayPlatform::Windows, false, TraySurface::MenuOverBar),
        ] {
            assert_eq!(lune_ui::tray::tray_surface(platform, left), surface);
        }
    }

    #[test]
    fn the_enlarged_copy_repeats_each_pixel_whole() {
        let bitmap = icon_bitmap(TrayState::Serving, GlyphTone::Light, 1, &THEME);
        let big = enlarge(&bitmap, 8);
        assert_eq!((big.width, big.height), (144, 144));
        assert!(big.length_ok());
        // Every 8 x 8 block is one pixel of the bitmap.
        for y in 0..144usize {
            for x in 0..144usize {
                let from = ((y / 8) * 18 + x / 8) * 4;
                let at = (y * 144 + x) * 4;
                assert_eq!(big.rgba[at..at + 4], bitmap.rgba[from..from + 4]);
            }
        }
    }

    #[test]
    fn the_macos_menu_starts_with_the_state_header_and_the_windows_one_has_none() {
        for s in STATES {
            let mac = menu_rows(TraySurface::MenuUnderBar, s);
            assert!(matches!(mac[0], MenuRow::Header { .. }));
            assert!(matches!(mac[1], MenuRow::Separator));
            assert_eq!(mac.len(), 10);
            assert!(
                mac.iter()
                    .skip(1)
                    .all(|r| !matches!(r, MenuRow::Header { .. }))
            );
            let win = menu_rows(TraySurface::MenuOverBar, s);
            assert!(!win.iter().any(|r| matches!(r, MenuRow::Header { .. })));
            assert_eq!(win.len(), 6);
        }
    }

    fn text(r: &MenuRow<'static>) -> Option<(&'static str, &'static str, bool, bool)> {
        match r {
            MenuRow::Entry(i) => Some((i.text, i.shortcut, i.chevron, i.danger)),
            _ => None,
        }
    }

    #[test]
    fn the_rows_follow_the_surface_and_the_state() {
        let mac = menu_rows(TraySurface::MenuUnderBar, TrayState::Serving);
        assert_eq!(text(&mac[2]), Some(("Open Wane", "Cmd+O", false, false)));
        assert_eq!(text(&mac[3]), Some(("Profile", "Work", true, false)));
        assert_eq!(text(&mac[6]), Some(("Live sessions", "3", true, false)));
        assert_eq!(text(&mac[9]), Some(("Quit Wane", "Cmd+Q", false, false)));
        assert!(
            mac.iter()
                .all(|r| text(r).is_none_or(|(_, _, _, danger)| !danger))
        );
        let win = menu_rows(TraySurface::MenuOverBar, TrayState::Serving);
        assert_eq!(
            text(&win[3]),
            Some(("Unload local model", "frees 19.3 GB", false, false))
        );
        assert_eq!(
            text(&win[5]),
            Some(("Quit Wane (stops proxy)", "", false, true))
        );
        for surface in [TraySurface::MenuUnderBar, TraySurface::MenuOverBar] {
            let rows = menu_rows(surface, TrayState::Paused);
            assert!(
                rows.iter()
                    .any(|r| text(r).is_some_and(|t| t.0 == "Resume compression"))
            );
            let rows = menu_rows(surface, TrayState::Serving);
            assert!(
                rows.iter()
                    .any(|r| text(r).is_some_and(|t| t.0 == "Pause compression"))
            );
        }
        let MenuRow::Header { name, .. } =
            menu_rows(TraySurface::MenuUnderBar, TrayState::Paused)[0]
        else {
            panic!("no header")
        };
        assert_eq!(name, "Wane \u{b7} paused");
    }
}
