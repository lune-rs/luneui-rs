//! UI tokens read by the toolkit's widgets, and the Lunegit dark preset.
//!
//! A [`Theme`] is a plain `Copy` value carried by the [`Painter`](crate::paint::Painter), so one
//! process can draw with more than one theme. [`lunegit_dark`] holds the values copied from
//! Lunegit's `DarkTheme.axaml` and `App.axaml`, plus its glass layers.

use crate::color::Rgba;
use crate::motion::MotionTokens;

/// Colours (straight alpha), the font-size scale, the control radius and the motion tokens that
/// widgets read.
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
    /// The scrollbar thumb: `accent` at alpha 0x8C (Wane's `--scroll-thumb`).
    pub scroll_thumb: Rgba,
    /// The thumb while hovered or held: `accent` at alpha 0xE6.
    pub scroll_thumb_hover: Rgba,
    /// The fill under selected text: `accent` at alpha 0x66 (the [D51] derivation).
    pub text_selection: Rgba,
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
    /// The menu row while pressed: Lunegit `DarkTheme.axaml:305` (#FFFFFF23).
    pub menu_item_pressed: Rgba,
    /// A disabled menu row's text: Lunegit `DarkTheme.axaml:307` (#FFFFFF58).
    pub menu_text_disabled: Rgba,
    /// Toast fill, border and the four tone colours, and the expanded log's fill and text:
    /// Lunegit `DarkTheme.axaml:333-341`. `toast_info` also tints a loading toast.
    pub toast_bg: Rgba,
    pub toast_border: Rgba,
    pub toast_info: Rgba,
    pub toast_success: Rgba,
    pub toast_warning: Rgba,
    pub toast_error: Rgba,
    pub toast_log_bg: Rgba,
    pub toast_log_fg: Rgba,
    /// The icon rail's dirty dot: Lunegit `DarkTheme.axaml:68` (#D98A3D).
    pub dirty_indicator: Rgba,
    /// The icon rail's count badge: Lunegit `DarkTheme.axaml:57` (#E74856).
    pub count_badge: Rgba,
    /// Caption close button fill while hovered, and its glyph colour.
    pub caption_close_hover: Rgba,
    pub caption_close_fg: Rgba,
    pub fg_muted: Rgba,
    /// Lunegit `ForegroundSubtle`: the toolbar's label.
    pub fg_subtle: Rgba,
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
    /// The error counterpart of `status_warning_light` (Lunegit `StatusErrorLight`): the error
    /// tint of a busy cue and a tab's progress spinner.
    pub status_error_light: Rgba,
    /// The error status colour itself (Lunegit `StatusError`): the command palette's error line
    /// and its destructive mark.
    pub status_error: Rgba,
    pub status_warning_dark: Rgba,
    /// Data tone (chart series) and the text drawn on a data fill.
    pub data: Rgba,
    pub data_hi: Rgba,
    pub gold: Rgba,
    /// The neutral status dot.
    pub paused: Rgba,
    /// The window fill a toolbar shows when the window is inactive or no glass material is
    /// available: Lunegit's `WindowFallbackBackground` #FF333333 (`DarkTheme.axaml:245`, D128).
    /// Opaque, so the toolbar never reads as see-through without a backdrop.
    pub window_fallback: Rgba,
    /// A link, a quote bar and a diff hunk header: Lunegit's `MarkdownLinkForeground` and
    /// `DiffHunkForeground` #61AFEF (D14).
    pub link: Rgba,
    /// The fill behind inline code (`MarkdownInlineCodeBackground` #14FFFFFF).
    pub code_inline: Rgba,
    /// A diff line's fill: added (`DiffAddedBackground` #1A73C991) and removed (#1AE06C75).
    pub diff_added: Rgba,
    pub diff_removed: Rgba,
    /// The intra-line mark under changed words, the line fill's hue at alpha 0x40. No source
    /// reads them: a deliberate look deviation (D14).
    pub diff_added_mark: Rgba,
    pub diff_removed_mark: Rgba,
    pub font_s: f32,
    pub font_base: f32,
    pub font_m: f32,
    pub font_title: f32,
    pub line_title: f32,
    pub item_radius: f32,
    /// Motion durations and the Reduced map (`docs/motion.md`).
    pub motion: MotionTokens,
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
        scroll_thumb: [0x76, 0x7D, 0x85, 0x8C],
        scroll_thumb_hover: [0x76, 0x7D, 0x85, 0xE6],
        text_selection: [0x76, 0x7D, 0x85, 0x66],
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
        // Lunegit `DarkTheme.axaml:305,307`.
        menu_item_pressed: [0xFF, 0xFF, 0xFF, 0x23],
        menu_text_disabled: [0xFF, 0xFF, 0xFF, 0x58],
        // Lunegit `DarkTheme.axaml:333-341`.
        toast_bg: [0x18, 0x18, 0x18, 0xE6],
        toast_border: [0xFF, 0xFF, 0xFF, 0x33],
        toast_info: [0x61, 0xAF, 0xEF, 0xFF],
        toast_success: [0x4C, 0xAF, 0x50, 0xFF],
        toast_warning: [0xE2, 0xB9, 0x3D, 0xFF],
        toast_error: [0xE0, 0x6C, 0x75, 0xFF],
        toast_log_bg: [0x0D, 0x0D, 0x0D, 0xFF],
        toast_log_fg: [0xCC, 0xCC, 0xCC, 0xFF],
        // Lunegit `DarkTheme.axaml:68` and `:57`.
        dirty_indicator: [0xD9, 0x8A, 0x3D, 0xFF],
        count_badge: [0xE7, 0x48, 0x56, 0xFF],
        caption_close_hover: [0xC4, 0x2B, 0x1C, 0xFF],
        caption_close_fg: [0xFF, 0xFF, 0xFF, 0xFF],
        fg_muted: [0xB0, 0xB0, 0xB0, 0xFF],
        fg_subtle: [0x80, 0x80, 0x80, 0xFF],
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
        status_error_light: [0xEF, 0x9A, 0x9A, 0xFF],
        status_error: [0xF4, 0x43, 0x36, 0xFF],
        status_warning_dark: [0x5D, 0x43, 0x00, 0xFF],
        data: [0x5C, 0x8F, 0xE6, 0xFF],
        data_hi: [0x8F, 0xBB, 0xFF, 0xFF],
        gold: [0xE9, 0xC4, 0x6A, 0xFF],
        paused: [0x8A, 0x90, 0x99, 0xFF],
        // Lunegit `Themes/DarkTheme.axaml:245`.
        window_fallback: [0x33, 0x33, 0x33, 0xFF],
        // Lunegit `DarkTheme.axaml:106-110,134-141`; the marks are derived (D14).
        link: [0x61, 0xAF, 0xEF, 0xFF],
        code_inline: [0xFF, 0xFF, 0xFF, 0x14],
        diff_added: [0x73, 0xC9, 0x91, 0x1A],
        diff_removed: [0xE0, 0x6C, 0x75, 0x1A],
        diff_added_mark: [0x73, 0xC9, 0x91, 0x40],
        diff_removed_mark: [0xE0, 0x6C, 0x75, 0x40],
        font_s: 11.0,
        font_base: 13.0,
        font_m: 14.0,
        font_title: 18.0,
        line_title: 24.0,
        item_radius: 6.0,
        motion: crate::motion::MOTION,
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
            (THEME.fg_subtle, [0x80, 0x80, 0x80, 0xFF]),
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
            (THEME.status_error_light, [0xEF, 0x9A, 0x9A, 0xFF]),
            (THEME.status_error, [0xF4, 0x43, 0x36, 0xFF]),
            (THEME.status_warning_dark, [0x5D, 0x43, 0x00, 0xFF]),
            (THEME.data, [0x5C, 0x8F, 0xE6, 0xFF]),
            (THEME.data_hi, [0x8F, 0xBB, 0xFF, 0xFF]),
            (THEME.gold, [0xE9, 0xC4, 0x6A, 0xFF]),
            (THEME.paused, [0x8A, 0x90, 0x99, 0xFF]),
            (THEME.window_fallback, [0x33, 0x33, 0x33, 0xFF]),
            (THEME.menu_item_pressed, [0xFF, 0xFF, 0xFF, 0x23]),
            (THEME.menu_text_disabled, [0xFF, 0xFF, 0xFF, 0x58]),
            (THEME.toast_bg, [0x18, 0x18, 0x18, 0xE6]),
            (THEME.toast_border, [0xFF, 0xFF, 0xFF, 0x33]),
            (THEME.toast_info, [0x61, 0xAF, 0xEF, 0xFF]),
            (THEME.toast_success, [0x4C, 0xAF, 0x50, 0xFF]),
            (THEME.toast_warning, [0xE2, 0xB9, 0x3D, 0xFF]),
            (THEME.toast_error, [0xE0, 0x6C, 0x75, 0xFF]),
            (THEME.toast_log_bg, [0x0D, 0x0D, 0x0D, 0xFF]),
            (THEME.toast_log_fg, [0xCC, 0xCC, 0xCC, 0xFF]),
            (THEME.dirty_indicator, [0xD9, 0x8A, 0x3D, 0xFF]),
            (THEME.count_badge, [0xE7, 0x48, 0x56, 0xFF]),
        ];
        assert_eq!(expected.len(), 35);
        for (got, want) in expected {
            assert_eq!(got, want);
        }
        assert_eq!(THEME.font_title, 18.0);
        assert_eq!(THEME.line_title, 24.0);
    }
}
