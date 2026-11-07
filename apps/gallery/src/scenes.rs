//! The legacy scenes: fixed tables of id, scale, frame time and hover, each with a draw function
//! over the gallery's own hit type (the Painter API, D1). The `reference` and `effects` pages hold
//! them. Dump mode renders every row to `<id>@<scale>x.png`; window mode shows one row at a time.
//! Nothing here reads the clock: the time of a scene is its table value.

use lune_ui::chrome;
use lune_ui::chrome::TabSpan;
use lune_ui::color::{Rgba, over, premul};
use lune_ui::icon;
use lune_ui::lunegit_dark::{self, THEME};
use lune_ui::paint::{LayerId, Painter, Rect, StrokeStyle};
use lune_ui::scene::Scene;
use lune_ui::text::{Spec, Style, TextSystem};
use lune_ui::theme::Theme;
use lune_ui::widgets::dialog::{DialogGlass, DialogHits, DialogSpec, dialog};
use lune_ui::widgets::dropdown::dropdown;
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

pub(crate) const TITLE_BAND: f32 = 34.0;
pub(crate) const CARD_MARGIN: f32 = 10.0;

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
    Option(usize),
    Block,
    Confirm,
    Cancel,
    Caption(u8),
}

pub(crate) type P<'a> = Painter<'a, Hit>;

/// One legacy scene.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LegacyScene {
    pub(crate) id: &'static str,
    /// Device pixels per logical pixel in dump mode (1 or 2).
    pub(crate) scale: u32,
    /// Frame time in seconds, for animated widgets.
    pub(crate) time: f32,
    pub(crate) hover: Option<Hit>,
    pub(crate) draw: fn(&mut P<'_>, f32),
}

impl LegacyScene {
    /// `<id>@<scale>x`, the listing name.
    pub(crate) fn name(&self) -> String {
        format!("{}@{}x", self.id, self.scale)
    }

    /// Paints the scene at device scale `s` into a window of `size` physical pixels.
    pub(crate) fn paint(&self, text: &mut dyn TextSystem, s: f32, size: (u32, u32)) -> Scene {
        let win = Rect::new(0.0, 0.0, size.0 as f32 / s, size.1 as f32 / s);
        let mut p = Painter::new(s, win, text, self.hover, THEME);
        (self.draw)(&mut p, self.time);
        p.scene
    }
}

/// Time at which every animation is settled.
const SETTLED: f32 = 10.0;

/// The `reference` page: widgets, menus, dropdowns and dialogs.
pub(crate) const REFERENCE: &[LegacyScene] = &[
    LegacyScene {
        id: "controls",
        scale: 2,
        time: SETTLED,
        hover: Some(Hit::Secondary),
        draw: controls_scene,
    },
    LegacyScene {
        id: "controls",
        scale: 1,
        time: SETTLED,
        hover: Some(Hit::Secondary),
        draw: controls_scene,
    },
    LegacyScene {
        id: "menu",
        scale: 2,
        time: SETTLED,
        hover: Some(Hit::MenuItem(1)),
        draw: menu_scene,
    },
    LegacyScene {
        id: "menu-opening",
        scale: 2,
        time: 0.09,
        hover: None,
        draw: menu_scene,
    },
    LegacyScene {
        id: "dropdown",
        scale: 2,
        time: SETTLED,
        hover: None,
        draw: dropdown_scene,
    },
    LegacyScene {
        id: "dropdown-flipped",
        scale: 2,
        time: SETTLED,
        hover: None,
        draw: dropdown_flipped_scene,
    },
    LegacyScene {
        id: "dialog-confirm",
        scale: 2,
        time: SETTLED,
        hover: None,
        draw: dialog_confirm_scene,
    },
    LegacyScene {
        id: "dialog-message",
        scale: 2,
        time: SETTLED,
        hover: None,
        draw: dialog_message_scene,
    },
    LegacyScene {
        id: "dialog-body",
        scale: 2,
        time: SETTLED,
        hover: None,
        draw: dialog_body_scene,
    },
];

/// The `effects` page: glass, blur, chrome, acrylic and the custom-draw hook.
pub(crate) const EFFECTS: &[LegacyScene] = &[
    LegacyScene {
        id: "custom-draw",
        scale: 2,
        time: SETTLED,
        hover: None,
        draw: custom_draw_scene,
    },
    LegacyScene {
        id: "glass-off",
        scale: 2,
        time: SETTLED,
        hover: Some(Hit::Secondary),
        draw: glass_off_scene,
    },
    LegacyScene {
        id: "unfocused",
        scale: 2,
        time: SETTLED,
        hover: Some(Hit::Secondary),
        draw: unfocused_scene,
    },
    LegacyScene {
        id: "blur-contrast",
        scale: 2,
        time: SETTLED,
        hover: None,
        draw: blur_contrast_scene,
    },
    LegacyScene {
        id: "chrome",
        scale: 2,
        time: SETTLED,
        hover: Some(Hit::Caption(2)),
        draw: chrome_scene,
    },
    LegacyScene {
        id: "acrylic",
        scale: 2,
        time: SETTLED,
        hover: None,
        draw: acrylic_scene,
    },
];

// ---- shared pieces ----

/// The window background: a panel layer over the whole window and the card on top (each
/// replaces what is below). `glass` off puts the layers over an opaque base; unfocused swaps
/// them for the solid inactive fill. Returns the card.
pub(crate) fn background<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    glass: bool,
    focused: bool,
    title: &str,
) -> Rect {
    background_in(p, &THEME, glass, focused, title)
}

/// The panel fill, the card fill and the noise flag of the window layers: `glass` off puts them
/// over an opaque base, unfocused swaps them for the solid inactive fill.
fn glass_fills(theme: &Theme, glass: bool, focused: bool) -> ([f32; 4], [f32; 4], bool) {
    if glass && !focused {
        let solid = premul(lunegit_dark::INACTIVE_FALLBACK);
        (solid, solid, false)
    } else {
        let mut panel = theme.glass.panel.premultiplied();
        let mut fill = theme.glass.card.premultiplied();
        if !glass {
            let base = premul(OPAQUE_BASE);
            panel = over(panel, base);
            fill = over(fill, base);
        }
        (panel, fill, true)
    }
}

/// Only the card of [`background_in`], at `card`: for a page that sits in a rect of a window
/// something else already painted.
pub(crate) fn card_in<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    theme: &Theme,
    glass: bool,
    focused: bool,
    card: Rect,
) -> Rect {
    // No layer change: the host's clip is still on, and `Painter::on` would drop it.
    let (panel, card_fill, noise) = glass_fills(theme, glass, focused);
    p.acrylic(card, chrome::CARD_RADIUS, card_fill, panel, noise);
    card
}

/// [`background`] with the layers and the title colour of `theme`: a page under a loaded theme
/// file draws its window from the theme's glass.
pub(crate) fn background_in<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    theme: &Theme,
    glass: bool,
    focused: bool,
    title: &str,
) -> Rect {
    p.on(LayerId::Base);
    let win = p.window;
    let card = Rect::new(
        CARD_MARGIN,
        TITLE_BAND,
        win.w - 2.0 * CARD_MARGIN,
        win.h - TITLE_BAND - CARD_MARGIN,
    );
    let (panel, card_fill, noise) = glass_fills(theme, glass, focused);
    p.acrylic(win, 0.0, panel, [0.0; 4], noise);
    p.acrylic(card, chrome::CARD_RADIUS, card_fill, panel, noise);
    let spec = Spec::plain(title, Style::ui(12.0, theme.fg_secondary).medium());
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

// Dropdown geometry (logical px, window 1040 x 660), from `widgets/dropdown.rs:18-23`: the list
// is `options.len() * 32 + 8` tall and opens above its anchor when
// `anchor.bottom() + 4 + h > 660 - 8`; its flipped top is `(anchor.y - 4 - h).max(8)`. The assert
// checks that this scene's list flips and that the flipped list fits above the anchor without the
// 8 px clamp.
const LIST_H: f32 = OPTIONS.len() as f32 * 32.0 + 8.0;
const FLIPPED_ANCHOR: Rect = Rect::new(34.0, 560.0, 210.0, 32.0);
const _: () = assert!(
    FLIPPED_ANCHOR.y + FLIPPED_ANCHOR.h + 4.0 + LIST_H > H as f32 - 8.0
        && FLIPPED_ANCHOR.y - 4.0 - LIST_H >= 8.0
);

const OPTIONS: [(&str, bool); 4] = [
    ("RznSans", true),
    ("Sans Serif", true),
    ("Monospace", true),
    ("Coming soon", false),
];

fn open_dropdown(p: &mut P<'_>, anchor: Rect) {
    p.on(LayerId::Popup);
    let win = p.window;
    p.hit(win, Hit::Dismiss);
    dropdown(p, anchor, &OPTIONS, 0, Hit::Option);
}

fn dropdown_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, true, true, "luneui gallery");
    let anchor = controls_content(p, card, true);
    open_dropdown(p, anchor);
}

fn dropdown_flipped_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, true, true, "luneui gallery");
    heading(
        p,
        "COMBO NEAR THE BOTTOM EDGE: THE LIST OPENS ABOVE",
        card.x + 24.0,
        card.y + 24.0,
    );
    combo(p, FLIPPED_ANCHOR, "RznSans", false, true, Hit::Combo);
    open_dropdown(p, FLIPPED_ANCHOR);
}

fn dialog_glass() -> DialogGlass {
    DialogGlass::of(&lunegit_dark::THEME)
}

const DIALOG_HITS: DialogHits<Hit> = DialogHits {
    block: Hit::Block,
    confirm: Hit::Confirm,
    cancel: Hit::Cancel,
};

fn dialog_confirm_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, true, true, "luneui gallery");
    controls_content(p, card, false);
    let spec = DialogSpec {
        title: "Discard changes",
        message: "The document has unsaved changes. Closing it will discard them.",
        extra_h: 0.0,
        confirm: "Discard",
        cancel: Some("Cancel"),
    };
    dialog(p, &spec, &dialog_glass(), &DIALOG_HITS, |_, _, _, _| {});
}

fn dialog_message_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, true, true, "luneui gallery");
    controls_content(p, card, false);
    let spec = DialogSpec {
        title: "About",
        message: "A message with a single button and no cancel.",
        extra_h: 0.0,
        confirm: "OK",
        cancel: None,
    };
    dialog(p, &spec, &dialog_glass(), &DIALOG_HITS, |_, _, _, _| {});
}

fn dialog_body_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, true, true, "luneui gallery");
    controls_content(p, card, false);
    let spec = DialogSpec {
        title: "Rename",
        message: "Enter a new name for this item.",
        // 12 gap + 18 caption + 32 field + 12 gap + 22 checkbox, plus 4 spare.
        extra_h: 100.0,
        confirm: "Rename",
        cancel: Some("Cancel"),
    };
    dialog(p, &spec, &dialog_glass(), &DIALOG_HITS, |p, x, y, inner| {
        let y = y + 12.0;
        heading(p, "NAME", x, y);
        let y = y + 18.0;
        field(
            p,
            Rect::new(x, y, inner, 32.0),
            "Untitled",
            "Name",
            true,
            Hit::FieldFocused,
        );
        checkbox(
            p,
            Rect::new(x, y + 44.0, inner, 22.0),
            false,
            "Do not ask again",
            Hit::CheckOff,
        );
    });
}

fn glass_off_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, false, true, "luneui gallery");
    controls_content(p, card, false);
}

fn unfocused_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, true, false, "luneui gallery");
    controls_content(p, card, false);
}

fn blur_contrast_scene(p: &mut P<'_>, _t: f32) {
    let t = THEME;
    p.on(LayerId::Base);
    let win = p.window;
    // 16 px black and white stripes across the whole window (1040 / 16 = 65 stripes).
    for i in 0..65 {
        let c: Rgba = if i % 2 == 0 {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        };
        p.fill(Rect::new(i as f32 * 16.0, 0.0, 16.0, win.h), 0.0, c);
    }
    // A panel whose backdrop is the blurred stripes.
    p.on(LayerId::Popup);
    let panel = Rect::new(320.0, 180.0, 400.0, 300.0);
    p.backdrop(panel, 9.0);
    p.fill(panel, 9.0, t.menu_glass);
    p.ring(panel, 9.0, 1.0, t.menu_popup_border, false);
    p.text_centered(
        &widgets::label(&t, "Backdrop blur over stripes", t.menu_text_primary),
        panel,
    );
}

fn chrome_scene(p: &mut P<'_>, _t: f32) {
    let card = background(p, true, true, "luneui gallery");
    let t = THEME;
    let tab = TabSpan {
        left: 140.0,
        right: 300.0,
    };
    let outline = chrome::outline(card.w, card.h, 32.0, Some(tab));
    let pts: Vec<[f32; 2]> = outline
        .iter()
        .map(|q| [q[0] + card.x, q[1] + card.y])
        .collect();
    p.stroke(&pts, true, StrokeStyle::solid(t.border, chrome::STROKE));
    for (left, text, color) in [
        (0.0, "Inactive", t.fg_tertiary),
        (tab.left, "Active tab", t.fg_primary),
    ] {
        let r = Rect::new(card.x + left, card.y, tab.right - tab.left, 32.0);
        p.text_centered(&widgets::label(&t, text, color), r);
    }
    let win = p.window;
    lune_ui::window::caption_buttons(
        p,
        win,
        TITLE_BAND - 4.0,
        false,
        [Hit::Caption(0), Hit::Caption(1), Hit::Caption(2)],
    );
}

fn acrylic_scene(p: &mut P<'_>, _t: f32) {
    let t = THEME;
    p.on(LayerId::Base);
    let win = p.window;
    let base: Rgba = [0x70, 0x70, 0x70, 0xFF];
    p.fill(win, 0.0, base);
    // Acrylic replaces what is below it, so the panels go on the page layer above the base fill.
    p.on(LayerId::Page);
    let layers = [
        ("WINDOW", lunegit_dark::WINDOW),
        ("PANEL", lunegit_dark::PANEL),
        ("CARD", lunegit_dark::CARD),
    ];
    for (i, (name, layer)) in layers.into_iter().enumerate() {
        let r = Rect::new(40.0 + i as f32 * 330.0, 120.0, 300.0, 400.0);
        p.acrylic(r, 8.0, layer.premultiplied(), premul(base), true);
        p.text_centered(&widgets::label(&t, name, t.fg_primary), r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lune_ui::scene::Acrylic;
    use lune_ui::testing::FakeText;

    fn scene_of(id: &str) -> Scene {
        let row = EFFECTS.iter().find(|s| s.id == id).expect("scene row");
        let (w, h) = (W * row.scale, H * row.scale);
        row.paint(&mut FakeText, row.scale as f32, (w, h))
    }

    fn acrylics(scene: &Scene) -> Vec<Acrylic> {
        scene
            .in_order()
            .into_iter()
            .flat_map(|l| l.geometry.acrylic.iter().copied())
            .collect()
    }

    /// The inactive window draws the solid fallback in place of the glass layers: the window and
    /// the card are both `window_fallback`, premultiplied, with noise off, and no acrylic item
    /// carries a glass layer's colour or noise. The fallback goes through the acrylic path (its
    /// `Src` semantics antialias the card's corners against the panel), so the "fill" is an
    /// acrylic item whose colour is the fallback, never a glass one. The focused scene beside it
    /// shows the glass.
    #[test]
    fn the_unfocused_scene_draws_the_window_fallback_and_no_glass_layer() {
        let fallback = premul(THEME.window_fallback);
        let off = acrylics(&scene_of("unfocused"));
        assert_eq!(off.len(), 2, "the window and the card");
        for a in &off {
            assert_eq!(a.shape.color, fallback, "an inactive layer is the fallback");
            assert_eq!(a.shape.params[3], 0.0, "no noise on the fallback");
        }
        // The card's `under` is the window layer, which is the fallback too.
        assert_eq!(off[1].shape.under, fallback);
        // Instrument check: the same background with focus is the glass layers, with noise.
        let mut text = FakeText;
        let win = Rect::new(0.0, 0.0, W as f32, H as f32);
        let mut p: Painter<'_, Hit> = Painter::new(2.0, win, &mut text, None, THEME);
        background(&mut p, true, true, "t");
        let on = acrylics(&p.scene);
        assert_eq!(on.len(), 2);
        assert_eq!(on[0].shape.color, THEME.glass.panel.premultiplied());
        assert_eq!(on[1].shape.color, THEME.glass.card.premultiplied());
        assert!(on.iter().all(|a| a.shape.params[3] == 1.0));
    }
}
