//! The reference scenes: a fixed table of id, scale, frame time and hover, each with a draw
//! function over the gallery's own hit type. Dump mode renders every row to
//! `<id>@<scale>x.png`; window mode shows one row at a time. Nothing here reads the clock: the
//! time of a scene is its table value.

use lune_ui::chrome;
use lune_ui::color::{Rgba, over, premul};
use lune_ui::icon;
use lune_ui::lunegit_dark::{self, THEME};
use lune_ui::paint::{LayerId, Painter, Rect};
use lune_ui::text::{Spec, Style};
use lune_ui::widgets::menu::{MENU_IN, MenuEntry, menu, menu_open_anim};
use lune_ui::widgets::{self, button, button_width, checkbox, combo, field, stepper, toggle};

/// Logical window size of every scene.
pub(crate) const W: u32 = 1040;
pub(crate) const H: u32 = 660;

/// Gaussian σ handed to the backdrop blur (the value the extraction was gated on).
pub(crate) const BLUR_SIGMA: f32 = 8.0;
/// Saturation boost on the blurred backdrop.
pub(crate) const BLUR_SATURATION: f32 = 1.8;

/// Base under the glass layers when the window has no OS glass.
const OPAQUE_BASE: Rgba = [0x1F, 0x1F, 0x1F, 0xFF];

const TITLE_BAND: f32 = 34.0;
const CARD_MARGIN: f32 = 10.0;

/// Hit regions of the gallery's controls (the painter is generic over the app's hit type).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Hit {
    Primary,
    Secondary,
    ToggleOn,
    ToggleOff,
    CheckOn,
    CheckOff,
    StepMinus,
    StepPlus,
    FieldEmpty,
    FieldFocused,
    Combo,
    Dismiss,
    MenuItem(u8),
}

pub(crate) type P<'a> = Painter<'a, Hit>;

/// One reference scene.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Scene {
    pub(crate) id: &'static str,
    /// Device pixels per logical pixel in dump mode (1 or 2).
    pub(crate) scale: u32,
    /// Frame time in seconds, for animated widgets.
    pub(crate) time: f32,
    pub(crate) hover: Option<Hit>,
    pub(crate) draw: fn(&mut P<'_>, f32),
}

impl Scene {
    /// `<id>@<scale>x`, the listing name.
    pub(crate) fn name(&self) -> String {
        format!("{}@{}x", self.id, self.scale)
    }
}

/// Time at which every animation is settled.
const SETTLED: f32 = 10.0;

pub(crate) const SCENES: &[Scene] = &[
    Scene {
        id: "controls",
        scale: 2,
        time: SETTLED,
        hover: Some(Hit::Secondary),
        draw: controls_scene,
    },
    Scene {
        id: "controls",
        scale: 1,
        time: SETTLED,
        hover: Some(Hit::Secondary),
        draw: controls_scene,
    },
    Scene {
        id: "menu",
        scale: 2,
        time: SETTLED,
        hover: Some(Hit::MenuItem(1)),
        draw: menu_scene,
    },
    Scene {
        id: "menu-opening",
        scale: 2,
        time: 0.09,
        hover: None,
        draw: menu_scene,
    },
    Scene {
        id: "custom-draw",
        scale: 2,
        time: SETTLED,
        hover: None,
        draw: custom_draw_scene,
    },
];

// ---- shared pieces ----

/// The window background: a panel layer over the whole window and the card on top (each
/// replaces what is below). `glass` off puts the layers over an opaque base; unfocused swaps
/// them for the solid inactive fill. Returns the card.
pub(crate) fn background(p: &mut P<'_>, glass: bool, focused: bool, title: &str) -> Rect {
    p.on(LayerId::Base);
    let win = p.window;
    let card = Rect::new(
        CARD_MARGIN,
        TITLE_BAND,
        win.w - 2.0 * CARD_MARGIN,
        win.h - TITLE_BAND - CARD_MARGIN,
    );
    let (panel, card_fill, noise) = if glass && !focused {
        let solid = premul(lunegit_dark::INACTIVE_FALLBACK);
        (solid, solid, false)
    } else {
        let mut panel = lunegit_dark::PANEL.premultiplied();
        let mut fill = lunegit_dark::CARD.premultiplied();
        if !glass {
            let base = premul(OPAQUE_BASE);
            panel = over(panel, base);
            fill = over(fill, base);
        }
        (panel, fill, true)
    };
    p.acrylic(win, 0.0, panel, [0.0; 4], noise);
    p.acrylic(card, chrome::CARD_RADIUS, card_fill, panel, noise);
    let spec = Spec::plain(title, Style::ui(12.0, THEME.fg_secondary).medium());
    p.text_centered(&spec, Rect::new(0.0, 0.0, win.w, TITLE_BAND - 2.0));
    card
}

fn heading(p: &mut P<'_>, text: &str, x: f32, y: f32) {
    p.text(
        &Spec::plain(text, Style::ui(THEME.font_s, THEME.fg_tertiary).medium()),
        x,
        y,
    );
}

/// The widget showcase inside `card`. Returns the combo's rect (a dropdown anchors to it);
/// `combo_open` draws the combo in its open state.
fn controls_content(p: &mut P<'_>, card: Rect, combo_open: bool) -> Rect {
    let t = THEME;
    let x = card.x + 24.0;
    let y = card.y + 24.0;

    heading(p, "BUTTONS", x, y);
    let by = y + 20.0;
    let pw = button_width(p, "Primary");
    button(p, Rect::new(x, by, pw, 32.0), "Primary", true, Hit::Primary);
    let sw = button_width(p, "Secondary");
    button(
        p,
        Rect::new(x + pw + 8.0, by, sw, 32.0),
        "Secondary",
        false,
        Hit::Secondary,
    );

    let y = y + 68.0;
    heading(p, "TOGGLES AND CHECKBOXES", x, y);
    let ty = y + 20.0;
    let (h_on, h_off) = (p.hovered(Hit::ToggleOn), p.hovered(Hit::ToggleOff));
    toggle(p, Rect::new(x, ty, 40.0, 24.0), true, h_on, Hit::ToggleOn);
    toggle(
        p,
        Rect::new(x + 64.0, ty, 40.0, 24.0),
        false,
        h_off,
        Hit::ToggleOff,
    );
    checkbox(
        p,
        Rect::new(x + 150.0, ty, 140.0, 24.0),
        true,
        "Checked",
        Hit::CheckOn,
    );
    checkbox(
        p,
        Rect::new(x + 300.0, ty, 140.0, 24.0),
        false,
        "Unchecked",
        Hit::CheckOff,
    );

    let y = y + 60.0;
    heading(p, "STEPPER", x, y);
    stepper(
        p,
        Rect::new(x, y + 20.0, 140.0, 32.0),
        12,
        Hit::StepMinus,
        Hit::StepPlus,
    );

    let y = y + 60.0;
    heading(p, "FIELDS", x, y);
    field(
        p,
        Rect::new(x, y + 20.0, 210.0, 32.0),
        "",
        "Search",
        false,
        Hit::FieldEmpty,
    );
    field(
        p,
        Rect::new(x + 230.0, y + 20.0, 210.0, 32.0),
        "lune",
        "Search",
        true,
        Hit::FieldFocused,
    );

    let y = y + 60.0;
    heading(p, "COMBO", x, y);
    let combo_rect = Rect::new(x, y + 20.0, 210.0, 32.0);
    combo(p, combo_rect, "RznSans", false, combo_open, Hit::Combo);

    // Right column: labels and icons.
    let rx = x + 480.0;
    let ry = card.y + 24.0;
    heading(p, "LABELS", rx, ry);
    let lines = [
        ("Primary text", t.fg_primary, t.font_base),
        ("Secondary text", t.fg_secondary, t.font_base),
        ("Tertiary text", t.fg_tertiary, t.font_base),
    ];
    for (i, (text, color, size)) in lines.into_iter().enumerate() {
        p.text(
            &Spec::plain(text, Style::ui(size, color)),
            rx,
            ry + 20.0 + i as f32 * 22.0,
        );
    }
    p.text(
        &Spec::plain("Title text", Style::ui(t.font_m, t.fg_primary).medium()),
        rx,
        ry + 20.0 + 3.0 * 22.0,
    );
    p.text(
        &Spec::plain("Small caption", Style::ui(t.font_s, t.fg_tertiary)),
        rx,
        ry + 20.0 + 4.0 * 22.0 + 4.0,
    );

    let iy = ry + 168.0;
    heading(p, "ICONS", rx, iy);
    let glyphs = [
        icon::ADD,
        icon::GEAR,
        icon::ELLIPSIS,
        icon::CHROME_CLOSE,
        icon::EDIT,
        icon::COPY,
        icon::INFO,
        icon::CHEVRON_DOWN,
        icon::ARROW_LEFT,
        icon::CHECK,
    ];
    for (i, g) in glyphs.into_iter().enumerate() {
        let r = Rect::new(rx + i as f32 * 36.0, iy + 20.0, 28.0, 28.0);
        p.text_centered(&icon::glyph(g, 18.0, t.fg_primary), r);
    }
    combo_rect
}

// ---- scenes ----

fn controls_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, true, true, "luneui gallery");
    controls_content(p, card, false);
}

fn menu_scene(p: &mut P<'_>, t: f32) {
    let card = background(p, true, true, "luneui gallery");
    controls_content(p, card, false);
    p.on(LayerId::Popup);
    let win = p.window;
    p.hit(win, Hit::Dismiss);
    let item = |glyph, text, shortcut, n| MenuEntry::Item {
        glyph,
        text,
        shortcut,
        hit: Hit::MenuItem(n),
    };
    let entries = [
        item(Some(icon::ADD), "New Window", "Cmd+N", 0),
        item(Some(icon::EDIT), "Rename...", "", 1),
        item(Some(icon::COPY), "Copy", "Cmd+C", 2),
        MenuEntry::Separator,
        item(Some(icon::GEAR), "Settings...", "Cmd+,", 3),
        item(Some(icon::INFO), "About", "", 4),
    ];
    menu(
        p,
        &entries,
        (500.0, 120.0),
        menu_open_anim(t / MENU_IN),
        true,
    );
}

fn custom_draw_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, true, true, "luneui gallery");
    let t = THEME;
    let x = card.x + 24.0;
    let y = card.y + 24.0;
    heading(p, "CUSTOM DRAW SLOT", x, y);
    let slot = Rect::new(x, y + 20.0, 320.0, 200.0);
    // App-owned drawing: the fill lands inside the slot's clip, under this layer's text.
    p.custom(0, slot);
    p.ring(slot.inset(-1.0, -1.0), 2.0, 1.0, t.border, true);
    p.text(
        &widgets::label(&t, "Drawn by the app, clipped to the slot", t.fg_secondary),
        x,
        slot.bottom() + 12.0,
    );
}
