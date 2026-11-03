//! UI tokens read by the toolkit's widgets, and the Lunegit dark preset.
//!
//! A [`Theme`] is a plain `Copy` value carried by the [`Painter`](crate::paint::Painter), so one
//! process can draw with more than one theme. [`lunegit_dark`] holds the values copied from
//! Lunegit's `DarkTheme.axaml` and `App.axaml`, plus its glass layers.

use crate::color::Rgba;

/// Colours (straight alpha), the font-size scale and the control radius that widgets read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub fg_primary: Rgba,
    pub fg_secondary: Rgba,
    pub fg_tertiary: Rgba,
    pub surface_input: Rgba,
    pub surface_hover: Rgba,
    pub surface_selected: Rgba,
    pub border: Rgba,
    pub border_subtle: Rgba,
    pub accent: Rgba,
    pub accent_hover: Rgba,
    /// Knob and check mark on an accent fill.
    pub on_accent: Rgba,
    /// Label on a primary button.
    pub on_accent_text: Rgba,
    pub dropdown_bg: Rgba,
    pub dialog_backdrop: Rgba,
    pub dialog_footer: Rgba,
    pub menu_text_primary: Rgba,
    pub menu_text_tertiary: Rgba,
    pub menu_text_quaternary: Rgba,
    pub menu_popup_border: Rgba,
    pub menu_item_hover: Rgba,
    pub menu_separator: Rgba,
    pub menu_glass: Rgba,
    /// Caption close button fill while hovered, and its glyph colour.
    pub caption_close_hover: Rgba,
    pub caption_close_fg: Rgba,
    pub fg_muted: Rgba,
    pub fg_bright: Rgba,
    pub fg_quaternary: Rgba,
    pub fg_dim: Rgba,
    pub fg_faint: Rgba,
    pub surface_overlay: Rgba,
    pub surface_active: Rgba,
    pub surface_separator: Rgba,
    pub surface_divider: Rgba,
    /// Status tones: a success, a warning and an error colour (badges, dots, verdict rows).
    pub ok: Rgba,
    pub warn: Rgba,
    pub err: Rgba,
    pub status_warning: Rgba,
    pub status_warning_light: Rgba,
    pub status_warning_dark: Rgba,
    /// Data tone (chart series) and the text drawn on a data fill.
    pub data: Rgba,
    pub data_hi: Rgba,
    pub gold: Rgba,
    /// The neutral status dot.
    pub paused: Rgba,
    pub font_s: f32,
    pub font_base: f32,
    pub font_m: f32,
    pub font_title: f32,
    pub line_title: f32,
    pub item_radius: f32,
}

/// The dark preset's theme under a neutral name, for code and tests that need a theme but not a
/// particular preset.
pub const DARK: Theme = lunegit_dark::THEME;

/// Lunegit dark theme (`Lunegit.Desktop/Themes/DarkTheme.axaml`, `App.axaml`).
pub mod lunegit_dark {
    use super::Theme;
    use crate::GlassLayer;
    use crate::color::Rgba;

    pub const THEME: Theme = Theme {
        fg_primary: [0xFF, 0xFF, 0xFF, 0xCC],
        fg_secondary: [0xFF, 0xFF, 0xFF, 0xAA],
        fg_tertiary: [0xFF, 0xFF, 0xFF, 0x88],
        surface_input: [0xFF, 0xFF, 0xFF, 0x0A],
        surface_hover: [0xFF, 0xFF, 0xFF, 0x10],
        surface_selected: [0xFF, 0xFF, 0xFF, 0x18],
        border: [0xFF, 0xFF, 0xFF, 0x30],
        border_subtle: [0xFF, 0xFF, 0xFF, 0x1A],
        accent: [0x76, 0x7D, 0x85, 0xFF],
        accent_hover: [0x86, 0x8D, 0x95, 0xFF],
        on_accent: [0xFF, 0xFF, 0xFF, 0xFF],
        on_accent_text: [0xFF, 0xFF, 0xFF, 0xF0],
        dropdown_bg: [0x00, 0x00, 0x00, 0xF0],
        dialog_backdrop: [0x00, 0x00, 0x00, 0x80],
        dialog_footer: [0x00, 0x00, 0x00, 0x24],
        // Luneium browser context-menu tokens (`tokens.css`, `ContextMenu.css`).
        menu_text_primary: [0xFF, 0xFF, 0xFF, 0xCC],
        menu_text_tertiary: [0xFF, 0xFF, 0xFF, 0x88],
        menu_text_quaternary: [0xFF, 0xFF, 0xFF, 0x66],
        menu_popup_border: [0xFF, 0xFF, 0xFF, 0x3A],
        menu_item_hover: [0xFF, 0xFF, 0xFF, 0x16],
        menu_separator: [0xFF, 0xFF, 0xFF, 0x22],
        // `linear-gradient(rgba(0,0,0,.45)), rgba(165,165,165,.3)` flattened: alpha .615, grey 44.
        menu_glass: [44, 44, 44, 157],
        caption_close_hover: [0xC4, 0x2B, 0x1C, 0xFF],
        caption_close_fg: [0xFF, 0xFF, 0xFF, 0xFF],
        fg_muted: [0xB0, 0xB0, 0xB0, 0xFF],
        fg_bright: [0xFF, 0xFF, 0xFF, 0xDD],
        fg_quaternary: [0xFF, 0xFF, 0xFF, 0x66],
        fg_dim: [0xFF, 0xFF, 0xFF, 0x55],
        fg_faint: [0xFF, 0xFF, 0xFF, 0x44],
        surface_overlay: [0xFF, 0xFF, 0xFF, 0x15],
        surface_active: [0xFF, 0xFF, 0xFF, 0x20],
        surface_separator: [0xFF, 0xFF, 0xFF, 0x22],
        surface_divider: [0xFF, 0xFF, 0xFF, 0x40],
        ok: [0x73, 0xC9, 0x91, 0xFF],
        warn: [0xE2, 0xB9, 0x3D, 0xFF],
        err: [0xE0, 0x6C, 0x75, 0xFF],
        status_warning: [0xFF, 0xC1, 0x07, 0xFF],
        status_warning_light: [0xFF, 0xD6, 0x66, 0xFF],
        status_warning_dark: [0x5D, 0x43, 0x00, 0xFF],
        data: [0x5C, 0x8F, 0xE6, 0xFF],
        data_hi: [0x8F, 0xBB, 0xFF, 0xFF],
        gold: [0xE9, 0xC4, 0x6A, 0xFF],
        paused: [0x8A, 0x90, 0x99, 0xFF],
        font_s: 11.0,
        font_base: 13.0,
        font_m: 14.0,
        font_title: 18.0,
        line_title: 24.0,
        item_radius: 6.0,
    };

    pub const TINT: Rgba = [0, 0, 0, 255];

    /// Base window layer (`GlassTintDarkOpacity` 1, `GlassMaterialDarkOpacity` 0.7).
    pub const WINDOW: GlassLayer = GlassLayer {
        tint: TINT,
        tint_opacity: 1.0,
        material_opacity: 0.7,
    };

    /// Panel layer (`GlassTintLightOpacity` 0.8, `GlassMaterialLightOpacity` 0.7).
    pub const PANEL: GlassLayer = GlassLayer {
        tint: TINT,
        tint_opacity: 0.8,
        material_opacity: 0.7,
    };

    /// Card and header-tint layer (tint 1, `GlassMaterialDarkHeaderOpacity` 0.8).
    pub const CARD: GlassLayer = GlassLayer {
        tint: TINT,
        tint_opacity: 1.0,
        material_opacity: 0.8,
    };

    /// Solid fill shown while the window is inactive (tint lightened 20%), replacing all
    /// acrylic layers. Lunegit works around AvaloniaUI/Avalonia#5440 this way.
    pub const INACTIVE_FALLBACK: Rgba = [0x33, 0x33, 0x33, 0xFF];

    pub const CARD_CORNER_RADIUS: f32 = 8.0;
    pub const TAB_HEADER_HEIGHT: f32 = 32.0;
    pub const BODY_PADDING: f32 = 10.0;
}

#[cfg(test)]
mod tests {
    use super::lunegit_dark::THEME;

    #[test]
    fn the_status_and_surface_tokens_hold_their_values() {
        let expected = [
            (THEME.fg_muted, [0xB0, 0xB0, 0xB0, 0xFF]),
            (THEME.fg_bright, [0xFF, 0xFF, 0xFF, 0xDD]),
            (THEME.fg_quaternary, [0xFF, 0xFF, 0xFF, 0x66]),
            (THEME.fg_dim, [0xFF, 0xFF, 0xFF, 0x55]),
            (THEME.fg_faint, [0xFF, 0xFF, 0xFF, 0x44]),
            (THEME.surface_overlay, [0xFF, 0xFF, 0xFF, 0x15]),
            (THEME.surface_active, [0xFF, 0xFF, 0xFF, 0x20]),
            (THEME.surface_separator, [0xFF, 0xFF, 0xFF, 0x22]),
            (THEME.surface_divider, [0xFF, 0xFF, 0xFF, 0x40]),
            (THEME.ok, [0x73, 0xC9, 0x91, 0xFF]),
            (THEME.warn, [0xE2, 0xB9, 0x3D, 0xFF]),
            (THEME.err, [0xE0, 0x6C, 0x75, 0xFF]),
            (THEME.status_warning, [0xFF, 0xC1, 0x07, 0xFF]),
            (THEME.status_warning_light, [0xFF, 0xD6, 0x66, 0xFF]),
            (THEME.status_warning_dark, [0x5D, 0x43, 0x00, 0xFF]),
            (THEME.data, [0x5C, 0x8F, 0xE6, 0xFF]),
            (THEME.data_hi, [0x8F, 0xBB, 0xFF, 0xFF]),
            (THEME.gold, [0xE9, 0xC4, 0x6A, 0xFF]),
            (THEME.paused, [0x8A, 0x90, 0x99, 0xFF]),
        ];
        assert_eq!(expected.len(), 19);
        for (got, want) in expected {
            assert_eq!(got, want);
        }
        assert_eq!(THEME.font_title, 18.0);
        assert_eq!(THEME.line_title, 24.0);
    }
}
