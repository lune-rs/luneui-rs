//! UI tokens read by the toolkit's widgets, and the Lunegit dark preset.
//!
//! A [`Theme`] is a plain `Copy` value carried by the [`Painter`](crate::paint::Painter), so one
//! process can draw with more than one theme. [`lunegit_dark`] holds the values copied from
//! Lunegit's `DarkTheme.axaml` and `App.axaml`, plus its glass layers. A theme file (JSON, read by
//! the bounded reader in [`crate::json`]) overlays some of those values: [`Theme::from_json`]. The
//! table from a file's colour key to the [`Theme`] field it sets is [`COLOR_KEYS`] and `color_slot`
//! below, in this file because the keys carry Lunegit's names.

use crate::backdrop::GlassLayer;
use crate::color::Rgba;
use crate::motion::MotionTokens;

mod file;

pub use file::{Skip, ThemeError, ThemeFile};

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
    /// The fill under selected text: `accent` at alpha 0x66 (the D51 derivation).
    pub text_selection: Rgba,
    /// Knob and check mark on an accent fill.
    pub on_accent: Rgba,
    /// Label on a primary button.
    pub on_accent_text: Rgba,
    pub dialog_backdrop: Rgba,
    pub dialog_footer: Rgba,
    pub menu_text_primary: Rgba,
    pub menu_text_tertiary: Rgba,
    pub menu_text_quaternary: Rgba,
    /// The 1 px ring of every glass surface (menu, list, popover, tooltip). A deliberate deviation
    /// from Lunegit's `PopupMenuOuterBorder` (#3AFFFFFF): white at 0x1F, quieter (D11).
    pub menu_popup_border: Rgba,
    pub menu_item_hover: Rgba,
    pub menu_separator: Rgba,
    /// The tint every glass surface draws over the blurred scene under it (straight alpha; file
    /// key `MenuGlassBackground`, which Lunegit does not have).
    pub menu_glass: Rgba,
    /// The menu row while pressed: Lunegit `DarkTheme.axaml:305` (#FFFFFF23).
    pub menu_item_pressed: Rgba,
    /// A disabled menu row's text: Lunegit `DarkTheme.axaml:307` (#FFFFFF58).
    pub menu_text_disabled: Rgba,
    /// The four toast tone colours, and the expanded log's fill and text: Lunegit
    /// `DarkTheme.axaml:335-341`. `toast_info` also tints a loading toast. The toast card itself
    /// is the glass surface (`menu_glass`, `menu_popup_border`, D8).
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
    /// The three acrylic layers a file's `GlassTintColor` and opacity keys set (D3); the preset's
    /// are [`lunegit_dark::WINDOW`], `PANEL` and `CARD`. Widgets still take their layer from their
    /// options: the app passes these.
    pub glass: ThemeGlass,
    pub font_s: f32,
    pub font_base: f32,
    pub font_m: f32,
    pub font_title: f32,
    pub line_title: f32,
    pub item_radius: f32,
    /// Motion durations and the Reduced map (`docs/motion.md`).
    pub motion: MotionTokens,
}

/// The window, panel and card acrylic layers of a theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeGlass {
    pub window: GlassLayer,
    pub panel: GlassLayer,
    pub card: GlassLayer,
}

/// How opaque the glass materials are, applied over a theme with [`Theme::with_acrylic`]
/// (Lunegit's `AcrylicLevel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AcrylicLevel {
    /// The theme's own opacities.
    #[default]
    Default,
    /// Each material opacity halved.
    LessOpaque,
    /// Each material opacity moved halfway to 1.
    MoreOpaque,
    /// Each material opacity 1.
    Opaque,
}

impl Theme {
    /// The theme with its three glass material opacities scaled by `level`, each clamped to
    /// [0, 1]. Tint opacities never scale. (Lunegit `ThemeApplicator.ApplyAcrylicSettings`: 0.7
    /// gives 0.35, 0.85 and 1; 0.8 gives 0.4, 0.9 and 1.)
    #[must_use]
    pub fn with_acrylic(mut self, level: AcrylicLevel) -> Theme {
        let scale = |m: f32| match level {
            AcrylicLevel::Default => m,
            AcrylicLevel::LessOpaque => m * 0.5,
            AcrylicLevel::MoreOpaque => m + (1.0 - m) * 0.5,
            AcrylicLevel::Opaque => 1.0,
        };
        let g = &mut self.glass;
        for layer in [&mut g.window, &mut g.panel, &mut g.card] {
            layer.material_opacity = scale(layer.material_opacity).clamp(0.0, 1.0);
        }
        self
    }

    /// The colour a theme file's colour `key` sets in this theme (the first field it sets, when
    /// it sets more than one), or `None` for a key that is not in [`COLOR_KEYS`].
    pub fn color_key(&self, key: &str) -> Option<Rgba> {
        let mut copy = *self;
        color_slot(&mut copy, key).map(|slot| slot.get())
    }
}

/// The fields one colour key writes.
pub(crate) enum Slot<'a> {
    One(&'a mut Rgba),
    Two(&'a mut Rgba, &'a mut Rgba),
    /// The tint of all three glass layers.
    Tints(&'a mut ThemeGlass),
}

impl Slot<'_> {
    pub(crate) fn set(self, c: Rgba) {
        match self {
            Slot::One(a) => *a = c,
            Slot::Two(a, b) => {
                *a = c;
                *b = c;
            }
            Slot::Tints(g) => {
                g.window.tint = c;
                g.panel.tint = c;
                g.card.tint = c;
            }
        }
    }

    fn get(&self) -> Rgba {
        match self {
            Slot::One(a) | Slot::Two(a, _) => **a,
            Slot::Tints(g) => g.window.tint,
        }
    }
}

/// One table, two products: the key list [`COLOR_KEYS`] and `color_slot`, the `match` from a key
/// to the fields it sets. Every key is Lunegit's resource name; its value in the dark preset
/// equals the `Theme` field's (checked against `DarkTheme.axaml`).
macro_rules! color_keys {
    ($($key:literal => |$t:ident| $slot:expr),+ $(,)?) => {
        /// The colour keys a theme file can set, in table order.
        pub const COLOR_KEYS: &[&str] = &[$($key),+];

        pub(crate) fn color_slot<'a>(theme: &'a mut Theme, key: &str) -> Option<Slot<'a>> {
            match key {
                $($key => {
                    let $t = theme;
                    Some($slot)
                })+
                _ => None,
            }
        }
    };
}

color_keys! {
    "ForegroundPrimary" => |t| Slot::One(&mut t.fg_primary),
    "ForegroundSecondary" => |t| Slot::One(&mut t.fg_secondary),
    "ForegroundTertiary" => |t| Slot::One(&mut t.fg_tertiary),
    "ForegroundQuaternary" => |t| Slot::One(&mut t.fg_quaternary),
    "ForegroundDim" => |t| Slot::One(&mut t.fg_dim),
    "ForegroundFaint" => |t| Slot::One(&mut t.fg_faint),
    "ForegroundMuted" => |t| Slot::One(&mut t.fg_muted),
    "ForegroundSubtle" => |t| Slot::One(&mut t.fg_subtle),
    "ForegroundBright" => |t| Slot::One(&mut t.fg_bright),
    "SurfaceInput" => |t| Slot::One(&mut t.surface_input),
    "SurfaceHover" => |t| Slot::One(&mut t.surface_hover),
    "SurfaceSelected" => |t| Slot::One(&mut t.surface_selected),
    "SurfaceOverlay" => |t| Slot::One(&mut t.surface_overlay),
    "SurfaceActive" => |t| Slot::One(&mut t.surface_active),
    "SurfaceSeparator" => |t| Slot::Two(&mut t.surface_separator, &mut t.menu_separator),
    "SurfaceDivider" => |t| Slot::One(&mut t.surface_divider),
    "SurfaceBorder" => |t| Slot::One(&mut t.border),
    "SurfaceBorderSubtle" => |t| Slot::One(&mut t.border_subtle),
    "AccentPrimaryColor" => |t| Slot::One(&mut t.accent),
    "DialogBackdropBrush" => |t| Slot::One(&mut t.dialog_backdrop),
    "DialogDarkerBlockBackground" => |t| Slot::One(&mut t.dialog_footer),
    "PopupMenuOuterBorder" => |t| Slot::One(&mut t.menu_popup_border),
    "MenuGlassBackground" => |t| Slot::One(&mut t.menu_glass),
    "PopupMenuItemHover" => |t| Slot::One(&mut t.menu_item_hover),
    "PopupMenuItemPressed" => |t| Slot::One(&mut t.menu_item_pressed),
    "PopupMenuDisabledForeground" => |t| Slot::One(&mut t.menu_text_disabled),
    "ToastInfoAccent" => |t| Slot::One(&mut t.toast_info),
    "ToastSuccessAccent" => |t| Slot::One(&mut t.toast_success),
    "ToastWarningAccent" => |t| Slot::One(&mut t.toast_warning),
    "ToastErrorAccent" => |t| Slot::One(&mut t.toast_error),
    "ToastTerminalBackground" => |t| Slot::One(&mut t.toast_log_bg),
    "ToastTerminalForeground" => |t| Slot::One(&mut t.toast_log_fg),
    "DirtyIndicator" => |t| Slot::One(&mut t.dirty_indicator),
    "StatusDeleted" => |t| Slot::One(&mut t.count_badge),
    "FileStatusAdded" => |t| Slot::One(&mut t.ok),
    "FileStatusModified" => |t| Slot::One(&mut t.warn),
    "FileStatusDeleted" => |t| Slot::One(&mut t.err),
    "StatusWarning" => |t| Slot::One(&mut t.status_warning),
    "StatusWarningLight" => |t| Slot::One(&mut t.status_warning_light),
    "StatusWarningDark" => |t| Slot::One(&mut t.status_warning_dark),
    "StatusError" => |t| Slot::One(&mut t.status_error),
    "StatusErrorLight" => |t| Slot::One(&mut t.status_error_light),
    "MarkdownLinkForeground" => |t| Slot::One(&mut t.link),
    "MarkdownInlineCodeBackground" => |t| Slot::One(&mut t.code_inline),
    "DiffAddedBackground" => |t| Slot::One(&mut t.diff_added),
    "DiffRemovedBackground" => |t| Slot::One(&mut t.diff_removed),
    "GlassTintColor" => |t| Slot::Tints(&mut t.glass),
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
        dialog_backdrop: [0x00, 0x00, 0x00, 0x80],
        dialog_footer: [0x00, 0x00, 0x00, 0x24],
        // Luneium browser context-menu tokens (`tokens.css`, `ContextMenu.css`).
        menu_text_primary: [0xFF, 0xFF, 0xFF, 0xCC],
        menu_text_tertiary: [0xFF, 0xFF, 0xFF, 0x88],
        menu_text_quaternary: [0xFF, 0xFF, 0xFF, 0x66],
        menu_popup_border: [0xFF, 0xFF, 0xFF, 0x1F],
        menu_item_hover: [0xFF, 0xFF, 0xFF, 0x16],
        menu_separator: [0xFF, 0xFF, 0xFF, 0x22],
        // `linear-gradient(rgba(0,0,0,.45)), rgba(165,165,165,.3)` flattened: alpha .615, grey 44.
        menu_glass: [44, 44, 44, 157],
        // Lunegit `DarkTheme.axaml:305,307`.
        menu_item_pressed: [0xFF, 0xFF, 0xFF, 0x23],
        menu_text_disabled: [0xFF, 0xFF, 0xFF, 0x58],
        // Lunegit `DarkTheme.axaml:335-341`.
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
        // Lunegit `DarkTheme.axaml:106-110,134-141`; the marks are derived (D14).
        link: [0x61, 0xAF, 0xEF, 0xFF],
        code_inline: [0xFF, 0xFF, 0xFF, 0x14],
        diff_added: [0x73, 0xC9, 0x91, 0x1A],
        diff_removed: [0xE0, 0x6C, 0x75, 0x1A],
        diff_added_mark: [0x73, 0xC9, 0x91, 0x40],
        diff_removed_mark: [0xE0, 0x6C, 0x75, 0x40],
        glass: super::ThemeGlass {
            window: WINDOW,
            panel: PANEL,
            card: CARD,
        },
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

    pub const CARD_CORNER_RADIUS: f32 = 8.0;
    pub const TAB_HEADER_HEIGHT: f32 = 32.0;
    pub const BODY_PADDING: f32 = 10.0;
}

#[cfg(test)]
mod tests {
    use super::lunegit_dark::{CARD, PANEL, THEME, WINDOW};

    #[test]
    fn the_preset_glass_is_the_old_constants() {
        assert_eq!(THEME.glass.window, WINDOW);
        assert_eq!(THEME.glass.panel, PANEL);
        assert_eq!(THEME.glass.card, CARD);
    }

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
            (THEME.menu_item_pressed, [0xFF, 0xFF, 0xFF, 0x23]),
            (THEME.menu_text_disabled, [0xFF, 0xFF, 0xFF, 0x58]),
            (THEME.toast_info, [0x61, 0xAF, 0xEF, 0xFF]),
            (THEME.toast_success, [0x4C, 0xAF, 0x50, 0xFF]),
            (THEME.toast_warning, [0xE2, 0xB9, 0x3D, 0xFF]),
            (THEME.toast_error, [0xE0, 0x6C, 0x75, 0xFF]),
            (THEME.toast_log_bg, [0x0D, 0x0D, 0x0D, 0xFF]),
            (THEME.toast_log_fg, [0xCC, 0xCC, 0xCC, 0xFF]),
            (THEME.dirty_indicator, [0xD9, 0x8A, 0x3D, 0xFF]),
            (THEME.count_badge, [0xE7, 0x48, 0x56, 0xFF]),
        ];
        assert_eq!(expected.len(), 32);
        for (got, want) in expected {
            assert_eq!(got, want);
        }
        assert_eq!(THEME.font_title, 18.0);
        assert_eq!(THEME.line_title, 24.0);
    }

    // ── The preset against Lunegit's `DarkTheme.axaml` and `App.axaml` (read at M11) ─────────
    //
    // `src/Lunegit.Desktop/Themes/DarkTheme.axaml` and `src/Lunegit.Desktop/App.axaml` of the
    // Lunegit repo. Each value below was read from the file at the line named beside it, never from
    // `THEME`. (The entry's old range, lines 200-233, has moved: the colour brushes are at 7-159,
    // 298-341 and the glass levels at 226-235.)

    use super::{COLOR_KEYS, Theme};
    use crate::GlassLayer;

    /// (axaml key, its line in `DarkTheme.axaml`, the file's literal, the RGBA it means).
    /// `GlassTintColor` (`Black`, :226) is the one key not here; see the glass test.
    const AXAML: &[(&str, u32, &str, [u8; 4])] = &[
        (
            "ForegroundPrimary",
            7,
            "#CCFFFFFF",
            [0xFF, 0xFF, 0xFF, 0xCC],
        ),
        (
            "ForegroundSecondary",
            8,
            "#AAFFFFFF",
            [0xFF, 0xFF, 0xFF, 0xAA],
        ),
        (
            "ForegroundTertiary",
            9,
            "#88FFFFFF",
            [0xFF, 0xFF, 0xFF, 0x88],
        ),
        (
            "ForegroundQuaternary",
            10,
            "#66FFFFFF",
            [0xFF, 0xFF, 0xFF, 0x66],
        ),
        ("ForegroundDim", 11, "#55FFFFFF", [0xFF, 0xFF, 0xFF, 0x55]),
        ("ForegroundFaint", 12, "#44FFFFFF", [0xFF, 0xFF, 0xFF, 0x44]),
        ("ForegroundMuted", 13, "#B0B0B0", [0xB0, 0xB0, 0xB0, 0xFF]),
        ("ForegroundSubtle", 14, "#808080", [0x80, 0x80, 0x80, 0xFF]),
        (
            "ForegroundBright",
            17,
            "#DDFFFFFF",
            [0xFF, 0xFF, 0xFF, 0xDD],
        ),
        ("SurfaceInput", 25, "#0AFFFFFF", [0xFF, 0xFF, 0xFF, 0x0A]),
        ("SurfaceHover", 26, "#10FFFFFF", [0xFF, 0xFF, 0xFF, 0x10]),
        ("SurfaceSelected", 28, "#18FFFFFF", [0xFF, 0xFF, 0xFF, 0x18]),
        ("SurfaceOverlay", 27, "#15FFFFFF", [0xFF, 0xFF, 0xFF, 0x15]),
        ("SurfaceActive", 29, "#20FFFFFF", [0xFF, 0xFF, 0xFF, 0x20]),
        (
            "SurfaceSeparator",
            30,
            "#22FFFFFF",
            [0xFF, 0xFF, 0xFF, 0x22],
        ),
        ("SurfaceDivider", 36, "#40FFFFFF", [0xFF, 0xFF, 0xFF, 0x40]),
        ("SurfaceBorder", 35, "#30FFFFFF", [0xFF, 0xFF, 0xFF, 0x30]),
        (
            "SurfaceBorderSubtle",
            34,
            "#1AFFFFFF",
            [0xFF, 0xFF, 0xFF, 0x1A],
        ),
        (
            "AccentPrimaryColor",
            158,
            "#FF767D85",
            [0x76, 0x7D, 0x85, 0xFF],
        ),
        (
            "DialogBackdropBrush",
            317,
            "#80000000",
            [0x00, 0x00, 0x00, 0x80],
        ),
        (
            "DialogDarkerBlockBackground",
            318,
            "#24000000",
            [0x00, 0x00, 0x00, 0x24],
        ),
        (
            "PopupMenuItemHover",
            304,
            "#16FFFFFF",
            [0xFF, 0xFF, 0xFF, 0x16],
        ),
        (
            "PopupMenuItemPressed",
            305,
            "#23FFFFFF",
            [0xFF, 0xFF, 0xFF, 0x23],
        ),
        (
            "PopupMenuDisabledForeground",
            307,
            "#58FFFFFF",
            [0xFF, 0xFF, 0xFF, 0x58],
        ),
        ("ToastInfoAccent", 335, "#61AFEF", [0x61, 0xAF, 0xEF, 0xFF]),
        (
            "ToastSuccessAccent",
            336,
            "#4CAF50",
            [0x4C, 0xAF, 0x50, 0xFF],
        ),
        (
            "ToastWarningAccent",
            337,
            "#E2B93D",
            [0xE2, 0xB9, 0x3D, 0xFF],
        ),
        ("ToastErrorAccent", 338, "#E06C75", [0xE0, 0x6C, 0x75, 0xFF]),
        (
            "ToastTerminalBackground",
            340,
            "#FF0D0D0D",
            [0x0D, 0x0D, 0x0D, 0xFF],
        ),
        (
            "ToastTerminalForeground",
            341,
            "#CCCCCC",
            [0xCC, 0xCC, 0xCC, 0xFF],
        ),
        ("DirtyIndicator", 68, "#D98A3D", [0xD9, 0x8A, 0x3D, 0xFF]),
        ("StatusDeleted", 57, "#E74856", [0xE7, 0x48, 0x56, 0xFF]),
        ("FileStatusAdded", 116, "#73C991", [0x73, 0xC9, 0x91, 0xFF]),
        (
            "FileStatusModified",
            115,
            "#E2B93D",
            [0xE2, 0xB9, 0x3D, 0xFF],
        ),
        (
            "FileStatusDeleted",
            117,
            "#E06C75",
            [0xE0, 0x6C, 0x75, 0xFF],
        ),
        ("StatusWarning", 60, "#FFC107", [0xFF, 0xC1, 0x07, 0xFF]),
        (
            "StatusWarningLight",
            62,
            "#FFD666",
            [0xFF, 0xD6, 0x66, 0xFF],
        ),
        ("StatusWarningDark", 61, "#5D4300", [0x5D, 0x43, 0x00, 0xFF]),
        ("StatusError", 63, "#F44336", [0xF4, 0x43, 0x36, 0xFF]),
        ("StatusErrorLight", 64, "#EF9A9A", [0xEF, 0x9A, 0x9A, 0xFF]),
        (
            "MarkdownLinkForeground",
            140,
            "#61AFEF",
            [0x61, 0xAF, 0xEF, 0xFF],
        ),
        (
            "MarkdownInlineCodeBackground",
            134,
            "#14FFFFFF",
            [0xFF, 0xFF, 0xFF, 0x14],
        ),
        (
            "DiffAddedBackground",
            106,
            "#1A73C991",
            [0x73, 0xC9, 0x91, 0x1A],
        ),
        (
            "DiffRemovedBackground",
            107,
            "#1AE06C75",
            [0xE0, 0x6C, 0x75, 0x1A],
        ),
    ];

    #[test]
    fn every_color_key_holds_the_dark_theme_axaml_value() {
        // 44 rows plus three keys with no row here: `GlassTintColor`, `MenuGlassBackground` (not in
        // Lunegit) and `PopupMenuOuterBorder` (a deviation, tested below): all 47 keys of the
        // table are covered, none twice.
        let mut keys: Vec<&str> = AXAML.iter().map(|r| r.0).collect();
        keys.push("GlassTintColor");
        keys.push("MenuGlassBackground");
        keys.push("PopupMenuOuterBorder");
        keys.sort_unstable();
        let mut table = COLOR_KEYS.to_vec();
        table.sort_unstable();
        assert_eq!(keys, table);
        for (key, line, literal, rgba) in AXAML {
            assert_eq!(
                THEME.color_key(key),
                Some(*rgba),
                "{key}: DarkTheme.axaml:{line} says {literal}"
            );
        }
        // `GlassTintColor` is `Black` (:226) and sets the tint of all three layers.
        assert_eq!(THEME.color_key("GlassTintColor"), Some([0, 0, 0, 255]));
        for layer in [THEME.glass.window, THEME.glass.panel, THEME.glass.card] {
            assert_eq!(layer.tint, [0, 0, 0, 255]);
        }
    }

    /// `GlassTintDarkOpacity` 1 (:227), `GlassMaterialDarkOpacity` 0.7 (:228),
    /// `GlassMaterialDarkHeaderOpacity` 0.8 (:229), `GlassTintLightOpacity` 0.8 (:230),
    /// `GlassMaterialLightOpacity` 0.7 (:231). `WINDOW` takes the Dark pair, `CARD` the tint 1 and
    /// the header's 0.8, `PANEL` the Light pair (D53 names PANEL's values, tint 0.8 and material
    /// 0.7). The 0.85 of `GlassMaterialDarkPanelOpacity` (:235) is Lunegit's terminal panel, a layer
    /// the toolkit does not have; `Theme::with_acrylic` produces 0.85 from 0.7 (D208), and the
    /// `Solid` levels (:232-233) are `AcrylicLevel::Opaque`.
    #[test]
    fn the_glass_layers_hold_the_dark_theme_axaml_opacities() {
        let layer = |tint_opacity, material_opacity| GlassLayer {
            tint: [0, 0, 0, 255],
            tint_opacity,
            material_opacity,
        };
        assert_eq!(THEME.glass.window, layer(1.0, 0.7));
        assert_eq!(THEME.glass.card, layer(1.0, 0.8));
        assert_eq!(THEME.glass.panel, layer(0.8, 0.7));
    }

    /// `PopupMenuOuterBorder` is #3AFFFFFF in `DarkTheme.axaml:301`; the toolkit's glass ring is
    /// white at 0x1F, a deliberate deviation (D11, R8). `MenuGlassBackground` is the toolkit's own key.
    #[test]
    fn the_glass_ring_and_tint_keys_hold_the_toolkits_values() {
        assert_eq!(
            THEME.color_key("PopupMenuOuterBorder"),
            Some([0xFF, 0xFF, 0xFF, 0x1F])
        );
        assert_ne!(THEME.menu_popup_border, [0xFF, 0xFF, 0xFF, 0x3A]);
        assert_eq!(
            THEME.color_key("MenuGlassBackground"),
            Some([44, 44, 44, 157])
        );
        assert_eq!(THEME.color_key("BackdropDark"), None);
    }

    /// Fields with no key of their own, each with the row that sets its value.
    #[test]
    fn the_tokens_without_an_axaml_key_follow_their_decision_rows() {
        let accent = THEME.accent;
        // `ScrollThumbFill` accent at Opacity 0.55 (:164) and `ScrollThumbFillHover` at 0.9 (:165):
        // 0.55 * 255 = 140.25 -> 0x8C, 0.9 * 255 = 229.5 -> 0xE6 (D208).
        assert_eq!(THEME.scroll_thumb, [accent[0], accent[1], accent[2], 0x8C]);
        assert_eq!(
            THEME.scroll_thumb_hover,
            [accent[0], accent[1], accent[2], 0xE6]
        );
        // The selection fill is the accent at alpha 0x66 (D51, D208), not `SelectionOverlay`
        // #33767D85 (:48), which the toolkit's text widgets do not read.
        assert_eq!(
            THEME.text_selection,
            [accent[0], accent[1], accent[2], 0x66]
        );
        // `accent_hover`: each channel plus 0x10, saturating (D208).
        assert_eq!(THEME.accent_hover, [0x86, 0x8D, 0x95, 0xFF]);
        // The diff intra-line marks: the line fill's hue at alpha 0x40 (D14, D208).
        assert_eq!(THEME.diff_added_mark, [0x73, 0xC9, 0x91, 0x40]);
        assert_eq!(THEME.diff_removed_mark, [0xE0, 0x6C, 0x75, 0x40]);
        // The menu's text tokens are the matching `fg_*` (D208), its separator the surface
        // separator's value (D58 says they are equal; the key table sets both, D207). Lunegit's
        // own `PopupMenuSeparator` is #18FFFFFF (:306): the menu entry builds on Luneium's
        // ContextMenu.css, and the moved value stays.
        assert_eq!(THEME.menu_text_primary, THEME.fg_primary);
        assert_eq!(THEME.menu_text_tertiary, THEME.fg_tertiary);
        assert_eq!(THEME.menu_text_quaternary, THEME.fg_quaternary);
        assert_eq!(THEME.menu_separator, THEME.surface_separator);
        assert_ne!(THEME.menu_separator, [0xFF, 0xFF, 0xFF, 0x18]);
        // Wane only, no Lunegit colour for the meaning (D58): data, data_hi, gold, paused.
        assert_eq!(THEME.data, [0x5C, 0x8F, 0xE6, 0xFF]);
        assert_eq!(THEME.data_hi, [0x8F, 0xBB, 0xFF, 0xFF]);
        assert_eq!(THEME.gold, [0xE9, 0xC4, 0x6A, 0xFF]);
        assert_eq!(THEME.paused, [0x8A, 0x90, 0x99, 0xFF]);
    }

    /// `FontSizeS` 11, `FontSizeBase` 13, `FontSizeM` 14, `FontSizeL` 18 (`App.axaml:159-162`).
    #[test]
    fn the_font_sizes_hold_the_app_axaml_typography_scale() {
        assert_eq!(THEME.font_s, 11.0);
        assert_eq!(THEME.font_base, 13.0);
        assert_eq!(THEME.font_m, 14.0);
        assert_eq!(THEME.font_title, 18.0);
    }

    // One test per motion duration (`App.axaml:151-155`, seconds there, milliseconds here).

    fn ms(theme: &Theme, f: impl Fn(&crate::motion::MotionTokens) -> std::time::Duration) -> u128 {
        f(&theme.motion).as_millis()
    }

    #[test]
    fn anim_duration_instant_is_120_ms() {
        assert_eq!(ms(&THEME, |m| m.instant), 120); // 0:0:0.12, :151
    }

    #[test]
    fn anim_duration_fast_is_180_ms() {
        assert_eq!(ms(&THEME, |m| m.fast), 180); // 0:0:0.18, :152
    }

    #[test]
    fn anim_duration_base_is_250_ms() {
        assert_eq!(ms(&THEME, |m| m.base), 250); // 0:0:0.25, :153
    }

    #[test]
    fn anim_duration_moderate_is_350_ms() {
        assert_eq!(ms(&THEME, |m| m.moderate), 350); // 0:0:0.35, :154
    }

    #[test]
    fn anim_duration_slow_is_500_ms() {
        assert_eq!(ms(&THEME, |m| m.slow), 500); // 0:0:0.5, :155
    }
}
