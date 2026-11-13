//! The `docked-panels` page (`platform.desktop`, `platform.window-docking`, `platform.window-group`,
//! `platform.window-shape`, `overlay.menu-window`): three borderless, shaped panels docked in a
//! stack, and the menu one of them opens in a window of its own. The scenes draw the panels and the
//! menu in the canvas; with the page's "Show the panels" switch on, window mode opens the panels as
//! three real windows (`panels.rs`), which this file draws too, so a scene and a window are the same
//! drawing.
//!
//! The pictures are generated here from arithmetic (no bundled file, so no licence row): a sheet of
//! eight 6 by 6 sprites that every panel draws by part, and a meter picture. The scenes draw a fixed
//! meter; the live windows write another one every frame (`ImageStore::update`).

use lune_ui::access::{Node, Role};
use lune_ui::desktop::WindowShape;
use lune_ui::icon;
use lune_ui::image::{ImageData, ImageDraw, ImageId, ImagePart, Sampling};
use lune_ui::motion::MotionTier;
use lune_ui::paint::{LayerId, Painter, Rect};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::{Sense, Ui};
use lune_ui::widgets::menu::{MenuResponse, MenuRow, menu_size};

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "docked-panels",
    title: "Docked panels",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: &[],
    }),
};

/// The sprite sheet: eight 6 by 6 sprites in a row, drawn by part.
pub(crate) const SHEET: ImageId = ImageId(1300);
/// The meter the scenes draw: a fixed picture.
pub(crate) const METER_STILL: ImageId = ImageId(1301);
/// The meter the live panel windows draw: rewritten in place every frame.
pub(crate) const METER_LIVE: ImageId = ImageId(1302);

/// The width of a panel of the stack, in logical px.
const PANEL_W: f32 = 275.0;
/// The height of the two tall panels, and of the first one unshaded.
const PANEL_H: f32 = 116.0;
/// The height of the first panel rolled up to its strip.
const SHADED_H: f32 = 14.0;
/// The smallest panel's size.
const MINI: (f32, f32) = (140.0, 32.0);
/// The height of a panel's title strip, which drags the panels.
const STRIP_H: f32 = 14.0;
/// The side of a control sprite, in logical px.
const CONTROL: f32 = 12.0;

const PANEL_BG: [u8; 4] = [0x1E, 0x20, 0x26, 0xFF];
const STRIP_BG: [u8; 4] = [0x2B, 0x2E, 0x37, 0xFF];
const METER_PLATE: [u8; 4] = [0x12, 0x13, 0x17, 0xFF];
/// The flat colour that stands for the desktop in a dump.
const DESKTOP: [u8; 4] = [0x3A, 0x3C, 0x44, 0xFF];

/// The three panels, top of the stack first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelKind {
    /// The leader of the group: the one that shades and doubles.
    Main,
    /// The one that shows the meter.
    Levels,
    /// The smallest: the one that opens the menu.
    Mini,
}

impl PanelKind {
    pub(crate) const ALL: [PanelKind; 3] = [PanelKind::Main, PanelKind::Levels, PanelKind::Mini];

    /// The panel's place in [`Self::ALL`].
    pub(crate) const fn index(self) -> usize {
        match self {
            PanelKind::Main => 0,
            PanelKind::Levels => 1,
            PanelKind::Mini => 2,
        }
    }

    pub(crate) const fn title(self) -> &'static str {
        match self {
            PanelKind::Main => "main",
            PanelKind::Levels => "levels",
            PanelKind::Mini => "mini",
        }
    }

    /// The panel's size in logical px: only `Main` shades.
    pub(crate) const fn size(self, shaded: bool) -> (f32, f32) {
        match self {
            PanelKind::Main if shaded => (PANEL_W, SHADED_H),
            PanelKind::Main | PanelKind::Levels => (PANEL_W, PANEL_H),
            PanelKind::Mini => MINI,
        }
    }
}

/// The part of a panel of `size` that is cut away: a notch in its bottom left corner, 24 px wide
/// and a quarter of the height (4 to 12 px). A panel draws nothing in it and its window shape
/// leaves it out, so the desktop shows there, and takes clicks.
pub(crate) fn notch(size: (f32, f32)) -> Rect {
    let h = (size.1 / 4.0).clamp(4.0, 12.0).round();
    Rect::new(0.0, size.1 - h, 24.0, h)
}

/// The window shape of a panel of `size`: its rect without [`notch`], in logical px.
pub(crate) fn panel_shape(size: (f32, f32)) -> WindowShape {
    let (w, h) = size;
    let n = notch(size);
    WindowShape::new()
        .with_polygon(&[
            (0.0, 0.0),
            (w, 0.0),
            (w, h),
            (n.right(), h),
            (n.right(), n.y),
            (0.0, n.y),
        ])
        .unwrap_or_default()
}

/// A point inside the notch of a panel of `size`, in logical px.
#[allow(
    dead_code,
    reason = "the live smoke probes the notch; until then only the tests do"
)]
pub(crate) fn notch_probe(size: (f32, f32)) -> (f32, f32) {
    let n = notch(size);
    (n.x + n.w / 2.0, n.y + n.h / 2.0)
}

/// A point on the body of a panel of `size` (not in its notch), in logical px.
#[allow(
    dead_code,
    reason = "the live smoke probes the body; until then only the tests do"
)]
pub(crate) fn body_probe(size: (f32, f32)) -> (f32, f32) {
    let n = notch(size);
    (size.0 / 2.0, n.y / 2.0)
}

// ---- the pictures ---------------------------------------------------------------------------

/// The side of a sprite in the sheet, in picture pixels.
const SPRITE: u32 = 6;
const SHEET_W: u32 = 8 * SPRITE;

pub(crate) const GRIP: usize = 0;
pub(crate) const MINIMISE: usize = 1;
pub(crate) const CLOSE: usize = 2;
pub(crate) const ZOOM: usize = 3;
pub(crate) const LED_ON: usize = 4;
pub(crate) const LED_OFF: usize = 5;
pub(crate) const ARROW: usize = 6;
pub(crate) const CHECK: usize = 7;

/// The sprites, one 6 by 6 picture each: `#` is a drawn pixel.
const SPRITES: [[&str; 6]; 8] = [
    ["#.#.#.", ".#.#.#", "#.#.#.", ".#.#.#", "#.#.#.", ".#.#.#"],
    ["......", "......", "......", "......", "######", "######"],
    ["#....#", ".#..#.", "..##..", "..##..", ".#..#.", "#....#"],
    ["######", "#....#", "#....#", "#....#", "#....#", "######"],
    ["..##..", ".####.", "######", "######", ".####.", "..##.."],
    ["..##..", ".#..#.", "#....#", "#....#", ".#..#.", "..##.."],
    ["#.....", "###...", "#####.", "#####.", "###...", "#....."],
    [".....#", "....##", "#..##.", "##.##.", ".###..", "..#..."],
];

/// The colour of sprite `i`: the two LEDs have their own, the rest are light grey.
const fn sprite_colour(i: usize) -> [u8; 3] {
    match i {
        LED_ON | CHECK => [0x73, 0xC9, 0x91],
        LED_OFF => [0x8A, 0x90, 0x99],
        _ => [0xD8, 0xDA, 0xE0],
    }
}

/// The sheet: [`SPRITES`] side by side, straight alpha (the unlit pixels are see-through).
pub(crate) fn sprite_sheet() -> ImageData {
    let mut rgba = vec![0u8; (SHEET_W * SPRITE * 4) as usize];
    for (i, rows) in SPRITES.iter().enumerate() {
        let [r, g, b] = sprite_colour(i);
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.bytes().enumerate() {
                if c == b'#' {
                    let px = y as u32 * SHEET_W + i as u32 * SPRITE + x as u32;
                    let at = px as usize * 4;
                    rgba[at..at + 4].copy_from_slice(&[r, g, b, 255]);
                }
            }
        }
    }
    ImageData {
        width: SHEET_W,
        height: SPRITE,
        rgba,
    }
}

/// Sprite `i` of the sheet: a 6 by 6 part.
pub(crate) fn sprite_part(i: usize) -> ImagePart {
    ImagePart {
        x: i as u32 * SPRITE,
        y: 0,
        w: SPRITE,
        h: SPRITE,
    }
}

/// Draws sprite `i` into `dst`, nearest, so each of its pixels stays one block.
fn sprite(ui: &mut Ui<'_>, dst: Rect, i: usize) {
    let draw = ImageDraw {
        part: Some(sprite_part(i)),
        sampling: Sampling::Nearest,
        ..ImageDraw::default()
    };
    ui.painter().image_with(dst, SHEET, &draw);
}

/// The meter picture's size in pixels: twelve bars, each 3 pixels wide with a 1 pixel gap.
pub(crate) const METER_W: u32 = 48;
pub(crate) const METER_H: u32 = 8;
/// The length of one meter frame, in milliseconds: the live panels write a new picture each.
pub(crate) const METER_FRAME_MS: u64 = 33;

/// The height of bar `bar` (0 to 8 pixels) at `ms`: a triangle wave that moves a bar every
/// 40 ms and is offset by 5 steps between neighbours. A pure function of the time.
fn bar_height(ms: u64, bar: u32) -> u32 {
    let phase = (ms / 40 + u64::from(bar) * 5) % 32;
    let tri = if phase < 16 { phase } else { 31 - phase };
    (tri / 2) as u32 + u32::from(bar % 3 == 0)
}

/// The meter at `ms` of app time: lit pixels green low, amber in the middle and red at the top, on
/// a dark ground, fully opaque (so it covers what is under it). Deterministic in `ms`.
pub(crate) fn meter_pixels(ms: u64) -> ImageData {
    let mut rgba = Vec::with_capacity((METER_W * METER_H * 4) as usize);
    for y in 0..METER_H {
        for x in 0..METER_W {
            let lit = x % 4 < 3 && (METER_H - 1 - y) < bar_height(ms, x / 4);
            let px: [u8; 4] = if !lit {
                [0x1B, 0x1D, 0x22, 0xFF]
            } else if y < 2 {
                [0xE0, 0x55, 0x55, 0xFF]
            } else if y < 4 {
                [0xE2, 0xB9, 0x3D, 0xFF]
            } else {
                [0x73, 0xC9, 0x91, 0xFF]
            };
            rgba.extend_from_slice(&px);
        }
    }
    ImageData {
        width: METER_W,
        height: METER_H,
        rgba,
    }
}

/// The pictures the page draws: the sheet and both meters (the live one as the host starts it).
pub(crate) fn pictures() -> Vec<(ImageId, ImageData)> {
    vec![
        (SHEET, sprite_sheet()),
        (METER_STILL, meter_pixels(600)),
        (METER_LIVE, meter_pixels(0)),
    ]
}

// ---- one panel ------------------------------------------------------------------------------

/// What a panel is drawn with.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PanelView {
    pub(crate) kind: PanelKind,
    pub(crate) shaded: bool,
    /// The group is kept above other windows: the first panel's LED shows it.
    pub(crate) on_top: bool,
    /// The meter picture the levels panel draws.
    pub(crate) meter: ImageId,
}

/// What one panel's frame reported.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PanelFrame {
    /// A primary press started on the title strip this frame.
    pub(crate) press_started: bool,
    /// A double click on the title strip.
    pub(crate) double_clicked: bool,
    /// A secondary click on the panel.
    pub(crate) secondary_clicked: bool,
    /// The double size control was clicked.
    pub(crate) zoom: bool,
    pub(crate) minimise: bool,
    pub(crate) close: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    Close,
    Minimise,
    Zoom,
}

impl Control {
    const fn sprite(self) -> usize {
        match self {
            Control::Close => CLOSE,
            Control::Minimise => MINIMISE,
            Control::Zoom => ZOOM,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Control::Close => "Close",
            Control::Minimise => "Minimise",
            Control::Zoom => "Double size",
        }
    }
}

/// A panel's controls, rightmost first: only the first panel doubles.
fn controls(kind: PanelKind) -> &'static [Control] {
    match kind {
        PanelKind::Main => &[Control::Close, Control::Minimise, Control::Zoom],
        PanelKind::Levels | PanelKind::Mini => &[Control::Close, Control::Minimise],
    }
}

/// Where control `i` (0 is the rightmost) is, for a panel at `at` that is `width` wide.
fn control_rect(at: (f32, f32), width: f32, i: usize) -> Rect {
    Rect::new(
        at.0 + width - 16.0 - 14.0 * i as f32,
        at.1 + 1.0,
        CONTROL,
        CONTROL,
    )
}

/// Draws the panel at `at` (its top left, in the frame's logical px) as `view` says and reports
/// what the frame did to it. The panel is the title strip, its controls and a body, over a
/// background that leaves its [`notch`] empty.
pub(crate) fn draw_panel(ui: &mut Ui<'_>, at: (f32, f32), view: &PanelView) -> PanelFrame {
    let t = *ui.theme();
    let size = view.kind.size(view.shaded);
    let key = view.kind.index() as u64;
    ui.scope(("panel", key), |ui| {
        let whole = Rect::new(at.0, at.1, size.0, size.1);
        let strip = Rect::new(at.0, at.1, size.0, STRIP_H);
        let mut out = PanelFrame::default();
        // The hit regions, lowest first: the body, the strip over it, the controls over that.
        let id = ui.id("body");
        let body = ui.interact(id, whole, Sense::CLICK);
        let id = ui.id("strip");
        let strip_hit = ui.interact(id, strip, Sense::DRAG);
        out.press_started = strip_hit.press_started;
        out.double_clicked = strip_hit.double_clicked;
        out.secondary_clicked = body.secondary_clicked || strip_hit.secondary_clicked;
        let mut hovered = [false; 3];
        for (i, c) in controls(view.kind).iter().enumerate() {
            let id = ui.id(("control", i as u64));
            let rect = control_rect(at, size.0, i);
            let r = ui.interact(id, rect, Sense::CLICK);
            ui.node(Node::new(id, Role::Button, c.name(), rect));
            hovered[i] = r.hovered;
            match c {
                Control::Close => out.close = r.clicked,
                Control::Minimise => out.minimise = r.clicked,
                Control::Zoom => out.zoom = r.clicked,
            }
        }
        // The background, in two rects, so the notch stays empty.
        let n = notch(size);
        let above = size.1 - n.h;
        let p = ui.painter();
        p.fill(Rect::new(at.0, at.1, size.0, above), 0.0, PANEL_BG);
        p.fill(
            Rect::new(at.0 + n.w, at.1 + above, size.0 - n.w, n.h),
            0.0,
            PANEL_BG,
        );
        // The strip: its part above the notch, and (a rolled up panel) the part beside it.
        p.fill(
            Rect::new(at.0, at.1, size.0, STRIP_H.min(above)),
            0.0,
            STRIP_BG,
        );
        if STRIP_H > above {
            p.fill(
                Rect::new(at.0 + n.w, at.1 + above, size.0 - n.w, STRIP_H - above),
                0.0,
                STRIP_BG,
            );
        }
        let left = at.0 + n.right() + 4.0;
        sprite(ui, Rect::new(left, at.1 + 1.0, CONTROL, CONTROL), GRIP);
        let title = Spec::plain(
            view.kind.title(),
            Style::ui(t.font_s, t.fg_primary).medium(),
        );
        ui.painter().text(&title, left + 16.0, at.1 + 1.0);
        for (i, c) in controls(view.kind).iter().enumerate() {
            let rect = control_rect(at, size.0, i);
            if hovered[i] {
                ui.painter().fill(rect, 2.0, t.surface_hover);
            }
            sprite(ui, rect, c.sprite());
        }
        if !(view.kind == PanelKind::Main && view.shaded) {
            draw_body(ui, at, view);
        }
        out
    })
}

/// A panel's body, under its strip.
fn draw_body(ui: &mut Ui<'_>, at: (f32, f32), view: &PanelView) {
    let t = *ui.theme();
    let line = |ui: &mut Ui<'_>, text: &str, x: f32, y: f32| {
        let spec = Spec::plain(text, Style::ui(t.font_s, t.fg_secondary));
        ui.painter().text(&spec, at.0 + x, at.1 + y);
    };
    match view.kind {
        PanelKind::Main => {
            line(
                ui,
                "The strip drags the panels it is docked with.",
                28.0,
                22.0,
            );
            line(ui, "Double-click the strip to shade it.", 28.0, 40.0);
            for i in 0..8 {
                let dst = Rect::new(at.0 + 28.0 + i as f32 * 16.0, at.1 + 62.0, CONTROL, CONTROL);
                sprite(ui, dst, i);
            }
            let led = if view.on_top { LED_ON } else { LED_OFF };
            sprite(
                ui,
                Rect::new(at.0 + 28.0, at.1 + 84.0, CONTROL, CONTROL),
                led,
            );
            line(ui, "always on top", 46.0, 84.0);
        }
        PanelKind::Levels => {
            sprite(
                ui,
                Rect::new(at.0 + 28.0, at.1 + 21.0, CONTROL, CONTROL),
                ARROW,
            );
            line(ui, "a picture written every frame", 46.0, 22.0);
            let dst = Rect::new(at.0 + 28.0, at.1 + 44.0, 192.0, 32.0);
            let p = ui.painter();
            p.fill(
                Rect::new(dst.x - 3.0, dst.y - 3.0, dst.w + 6.0, dst.h + 6.0),
                2.0,
                METER_PLATE,
            );
            let draw = ImageDraw {
                sampling: Sampling::Nearest,
                ..ImageDraw::default()
            };
            p.image_with(dst, view.meter, &draw);
        }
        PanelKind::Mini => {
            line(ui, "right-click: menu", 28.0, 16.0);
        }
    }
}

// ---- the menu -------------------------------------------------------------------------------

/// What the panels' menu shows checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MenuState {
    pub(crate) on_top: bool,
    pub(crate) doubled: bool,
    pub(crate) snap: bool,
}

/// What a row of the menu does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuPick {
    OnTop,
    Double,
    Snap,
    Close,
}

/// The menu's rows, a check before the ones that are on.
pub(crate) fn menu_rows(s: MenuState) -> [MenuRow<'static>; 4] {
    let mark = |on: bool| on.then_some(icon::CHECK);
    let row = |glyph, text| MenuRow::Item {
        glyph,
        text,
        shortcut: "",
    };
    [
        row(mark(s.on_top), "Always on top"),
        row(mark(s.doubled), "Double size"),
        row(mark(s.snap), "Snap to edges"),
        row(None, "Close panels"),
    ]
}

/// The row at `index` of [`menu_rows`], as what it does.
pub(crate) fn menu_pick(index: usize) -> Option<MenuPick> {
    [
        MenuPick::OnTop,
        MenuPick::Double,
        MenuPick::Snap,
        MenuPick::Close,
    ]
    .get(index)
    .copied()
}

/// The size, in logical px, of the window the menu fills.
pub(crate) fn menu_window_size<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    s: MenuState,
) -> (f32, f32) {
    menu_size(p, &menu_rows(s))
}

/// The menu filling `area` from its corner: what the menu window draws, and what the scene draws
/// in a pane.
pub(crate) fn draw_menu(ui: &mut Ui<'_>, area: Rect, s: MenuState) -> MenuResponse {
    ui.menu_window_in("panel-menu", area, &menu_rows(s))
}

// ---- the page -------------------------------------------------------------------------------

/// The state both the window and the still scenes draw: the three panels docked in a stack.
struct Live {
    /// The page's switch: window mode opens the panels as windows while it is on.
    show: bool,
}

fn live() -> Box<dyn PageState> {
    Box::new(Live { show: false })
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "Docked panels");
        let (x, y) = (area.x + 24.0, area.y + 24.0);
        heading(ui, "THREE PANELS, DOCKED", x, y);
        caption(
            ui,
            "Borderless windows with a notch cut out of each: the desktop shows, and takes clicks, in the notch.",
            x,
            y + 20.0,
        );
        let view = |kind| PanelView {
            kind,
            shaded: false,
            on_top: false,
            meter: METER_STILL,
        };
        let mut top = y + 56.0;
        for kind in PanelKind::ALL {
            draw_panel(ui, (x, top), &view(kind));
            top += kind.size(false).1;
        }
        let note = x + PANEL_W + 40.0;
        heading(ui, "IN THE WINDOW", note, y + 56.0);
        for (i, text) in [
            "Turn on Show the panels in the settings pane to open these as three windows.",
            "Drag a strip: the first panel takes the whole set, the others go alone and snap to edges.",
            "Double-click the first strip to shade it; its Double size control doubles every panel.",
            "Right-click the small panel for a menu in a window of its own.",
            "The levels panel writes its picture again every frame.",
        ]
        .into_iter()
        .enumerate()
        {
            caption(ui, text, note, y + 80.0 + i as f32 * 20.0);
        }
    }

    fn images(&self) -> Vec<(ImageId, ImageData)> {
        pictures()
    }

    fn panels(&self) -> bool {
        self.show
    }

    fn panels_closed(&mut self) {
        self.show = false;
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "Show the panels", &mut self.show);
    }
}

/// The menu in a pane, over a flat stand-in for the desktop (a dump has no OS material).
struct MenuScene {
    open: bool,
}

fn menu_scene() -> Box<dyn PageState> {
    Box::new(MenuScene { open: true })
}

impl PageState for MenuScene {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "Docked panels menu");
        let (x, y) = (area.x + 24.0, area.y + 24.0);
        heading(ui, "THE MENU", x, y);
        caption(
            ui,
            "A secondary click on the small panel opens it in a window of its own, placed from the pointer.",
            x,
            y + 20.0,
        );
        let pane = Rect::new(x, y + 56.0, 260.0, 220.0);
        {
            let p = ui.painter();
            p.on(LayerId::Base);
            p.fill(pane, 4.0, DESKTOP);
        }
        let state = MenuState {
            on_top: true,
            doubled: false,
            snap: true,
        };
        if self.open && draw_menu(ui, pane, state).dismissed {
            self.open = false;
        }
        draw_panel(
            ui,
            (x + 300.0, y + 56.0),
            &PanelView {
                kind: PanelKind::Mini,
                shaded: false,
                on_top: true,
                meter: METER_STILL,
            },
        );
    }

    fn images(&self) -> Vec<(ImageId, ImageData)> {
        pictures()
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "Open", &mut self.open);
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
    scene("docked-panels", 1, live, 0),
    scene("docked-panels", 2, live, 0),
    // The menu's open is `fast` (180 ms): settled by 500 ms.
    scene("docked-panels-menu", 2, menu_scene, 500),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::testing::FakeText;

    #[test]
    fn the_sheet_holds_eight_sprites_in_a_row_and_each_part_is_inside_it() {
        let sheet = sprite_sheet();
        assert_eq!((sheet.width, sheet.height), (48, 6));
        assert!(sheet.length_ok());
        for i in 0..8 {
            let part = sprite_part(i);
            assert!(part.x + part.w <= sheet.width && part.y + part.h <= sheet.height);
            // Every sprite lights at least one pixel and none lights them all.
            let lit = (0..6u32)
                .flat_map(|y| (0..6u32).map(move |x| (x, y)))
                .filter(|(x, y)| {
                    let at = ((y * sheet.width + part.x + x) * 4 + 3) as usize;
                    sheet.rgba[at] == 255
                })
                .count();
            assert!((1..36).contains(&lit), "sprite {i} lights {lit} pixels");
        }
    }

    #[test]
    fn the_meter_is_a_pure_function_of_the_time_and_moves() {
        let a = meter_pixels(1000);
        assert_eq!(a, meter_pixels(1000));
        assert!(a.length_ok());
        assert_eq!((a.width, a.height), (METER_W, METER_H));
        // A frame later it has moved.
        assert_ne!(a, meter_pixels(1000 + 40));
        // Every pixel is opaque, so the picture covers its plate.
        assert!(a.rgba.chunks(4).all(|px| px[3] == 255));
        // The gaps between bars are never lit.
        for ms in [0, 200, 640, 1234] {
            let m = meter_pixels(ms);
            for y in 0..METER_H {
                let at = ((y * METER_W + 3) * 4) as usize;
                assert_eq!(m.rgba[at..at + 3], [0x1B, 0x1D, 0x22], "ms {ms} y {y}");
            }
        }
    }

    #[test]
    fn the_notch_is_cut_out_of_every_panels_shape() {
        for kind in PanelKind::ALL {
            for shaded in [false, true] {
                let size = kind.size(shaded);
                let shape = panel_shape(size);
                assert!(!shape.is_empty());
                let (nx, ny) = notch_probe(size);
                assert!(!shape.contains((nx, ny)), "{kind:?} shaded {shaded}");
                let (bx, by) = body_probe(size);
                assert!(shape.contains((bx, by)), "{kind:?} shaded {shaded}");
                // The strip's right end and the far corner are on the body.
                assert!(shape.contains((size.0 - 2.0, 2.0)));
                assert!(shape.contains((size.0 - 2.0, size.1 - 2.0)));
            }
        }
    }

    #[test]
    fn the_small_panel_is_the_smallest_and_smaller_than_the_menu_on_both_axes() {
        let mut text = FakeText;
        let win = Rect::new(0.0, 0.0, 4096.0, 4096.0);
        let mut p: Painter<'_, ()> =
            Painter::new(1.0, win, &mut text, None, lune_ui::lunegit_dark::THEME);
        let menu = menu_window_size(
            &mut p,
            MenuState {
                on_top: false,
                doubled: false,
                snap: false,
            },
        );
        let mini = PanelKind::Mini.size(false);
        assert!(mini.0 < menu.0 && mini.1 < menu.1, "{mini:?} vs {menu:?}");
        for kind in [PanelKind::Main, PanelKind::Levels] {
            let s = kind.size(false);
            assert!(mini.0 < s.0 && mini.1 < s.1);
        }
        assert_eq!(PanelKind::Main.size(true), (275.0, 14.0));
        assert_eq!(PanelKind::Main.size(false), (275.0, 116.0));
    }

    #[test]
    fn the_menu_rows_check_what_is_on_and_each_row_does_one_thing() {
        let on = menu_rows(MenuState {
            on_top: true,
            doubled: false,
            snap: true,
        });
        let glyphs: Vec<bool> = on
            .iter()
            .map(|r| matches!(r, MenuRow::Item { glyph: Some(_), .. }))
            .collect();
        assert_eq!(glyphs, [true, false, true, false]);
        assert_eq!(menu_pick(0), Some(MenuPick::OnTop));
        assert_eq!(menu_pick(3), Some(MenuPick::Close));
        assert_eq!(menu_pick(4), None);
    }

    #[test]
    fn the_scenes_draw_the_panels_and_settle_without_asking_for_a_frame() {
        for scene in SCENES {
            let (_, out) = run_scene("docked-panels", scene, &mut FakeText).expect("runs");
            assert_eq!(
                out.repaint,
                lune_ui::input::Repaint::Idle,
                "{}",
                scene.name()
            );
        }
        assert_eq!(
            SCENES.iter().map(UiScene::name).collect::<Vec<_>>(),
            [
                "docked-panels@1x",
                "docked-panels@2x",
                "docked-panels-menu@2x"
            ]
        );
    }

    /// The panel's frame reports its strip press, its controls and a secondary click, and a click
    /// on a control is no press of the strip under it.
    #[test]
    fn a_panel_reports_the_strip_the_controls_and_the_secondary_click() {
        use lune_ui::input::PointerButton;
        use lune_ui::lunegit_dark::THEME;
        use lune_ui::motion::{Cadence, MotionTier};
        use lune_ui::shortcuts::Shortcuts;
        use lune_ui::strings::{DisplayFormat, StringTable};
        use lune_ui::testing::ScriptedInput;
        use lune_ui::ui::{FrameConfig, UiStore};
        use std::time::Duration;

        let view = PanelView {
            kind: PanelKind::Main,
            shaded: false,
            on_top: false,
            meter: METER_STILL,
        };
        let size = PanelKind::Main.size(false);
        let mut store = UiStore::default();
        let strings = StringTable::default();
        let shortcuts = Shortcuts::default();
        let mut text = FakeText;
        let mut frame = |input: &mut ScriptedInput| {
            let raw = input.end_frame();
            let config = FrameConfig {
                theme: THEME,
                tier: MotionTier::Full,
                busy_cadence: Cadence::BUSY,
                strings: &strings,
                arg_format: &DisplayFormat,
                keymap: lune_ui::input::Keymap::MAC,
                shortcuts: &shortcuts,
            };
            let mut ui = Ui::begin(&mut store, &raw, config, &mut text);
            let out = draw_panel(&mut ui, (0.0, 0.0), &view);
            ui.finish();
            out
        };
        let mut input = ScriptedInput::new(size, 1.0);
        input.set_time(Duration::ZERO);
        // The first frame establishes the hit regions.
        frame(&mut input);
        // Press on the strip, left of the controls.
        let strip_x = 100.0;
        input.pointer_to(strip_x, 6.0);
        frame(&mut input);
        input.press(PointerButton::Primary);
        let pressed = frame(&mut input);
        assert!(pressed.press_started, "{pressed:?}");
        input.release(PointerButton::Primary);
        frame(&mut input);
        // A click on the close control (the rightmost) is a close and not a press of the strip.
        let close = control_rect((0.0, 0.0), size.0, 0);
        input.pointer_to(close.x + 6.0, close.y + 6.0);
        frame(&mut input);
        input.press(PointerButton::Primary);
        let pressed = frame(&mut input);
        assert!(!pressed.press_started, "{pressed:?}");
        input.release(PointerButton::Primary);
        let released = frame(&mut input);
        assert!(released.close, "{released:?}");
        // A secondary click on the body is reported.
        input.pointer_to(strip_x, 50.0);
        frame(&mut input);
        input.press(PointerButton::Secondary);
        frame(&mut input);
        input.release(PointerButton::Secondary);
        let sec = frame(&mut input);
        assert!(sec.secondary_clicked, "{sec:?}");
    }
}
