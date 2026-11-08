//! The showcase's right pane: the global settings that every hosted page lives under.
//!
//! [`Settings`] is plain data with the rules that turn it into what the frame takes (a theme, a
//! tier, blur parameters, a material, a speed, a zoom); [`Settings::pane`] draws the controls.
//! The showcase answers the `PageState` hooks from it, and the window applies them. The settings
//! belong to the shell, not to a page: they survive a change of page and go back only by Reset.
//!
//! Everything here is the gallery's. The frame values the library takes (the noise strength, the
//! luminosity curve, the blur kernel's cap and divisor) each have a control that reaches the frame
//! through [`FrameTweaks`] or the composed theme.

use lune_ui::access::{Node, Role, State};
use lune_ui::acrylic::NOISE_OPACITY;
use lune_ui::color::Rgba;
use lune_ui::lunegit_dark::THEME;
use lune_ui::material::Material;
use lune_ui::motion::MotionTier;
use lune_ui::near_black;
use lune_ui::paint::Rect;
use lune_ui::scene::BlurLimits;
use lune_ui::text::{Spec, Style};
use lune_ui::theme::{AcrylicLevel, DARK, Theme, ThemeGlass};
use lune_ui::ui::{Sense, Ui};
use lune_ui::widgets::button::ButtonKind;
use lune_ui::widgets::radio_group::{ROW_HEIGHT, RadioOption};
use lune_ui::widgets::scroll_area::{Extent, ScrollOptions};
use lune_ui::widgets::slider::SliderOptions;

use super::knobs::Knobs;
use crate::page::{FrameTweaks, PageState};
use crate::scenes::{BLUR_SATURATION, BLUR_SIGMA};

/// Where a theme in [`THEMES`] comes from.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ThemeSource {
    /// A preset value, and the OS material it brings (`None` leaves the window's own choice).
    Preset(&'static Theme, Option<Material>),
    /// A theme file's bytes, loaded over the dark preset.
    File(&'static [u8]),
}

/// The theme the gallery starts under, the five files of Lunegit it can load, and the near-black
/// preset, which brings its own OS material.
pub(crate) const THEMES: [(&str, ThemeSource); 7] = [
    ("Lunegit dark (preset)", ThemeSource::Preset(&THEME, None)),
    (
        "Dark Modern",
        ThemeSource::File(include_bytes!(
            "../../assets/lunegit/themes/dark-modern.json"
        )),
    ),
    (
        "Blue",
        ThemeSource::File(include_bytes!("../../assets/lunegit/themes/blue.json")),
    ),
    (
        "Green",
        ThemeSource::File(include_bytes!("../../assets/lunegit/themes/green.json")),
    ),
    (
        "Orange",
        ThemeSource::File(include_bytes!("../../assets/lunegit/themes/orange.json")),
    ),
    (
        "Purple",
        ThemeSource::File(include_bytes!("../../assets/lunegit/themes/purple.json")),
    ),
    (
        "Near-black glass (preset)",
        ThemeSource::Preset(&near_black::THEME, Some(near_black::MATERIAL)),
    ),
];

/// The zoom steps.
pub(crate) const ZOOMS: [(&str, f32); 3] = [("1x", 1.0), ("1.5x", 1.5), ("2x", 2.0)];

/// The speed range: 0.1x to 2x of real time.
pub(crate) const SPEED: (f32, f32) = (0.1, 2.0);

/// The most blur sigma the slider offers, in physical pixels. The kernel is `ceil(3 sigma /
/// downsample)` texels a side, capped: at the default divisor 8 and cap 48 it stops growing at
/// sigma 128, and at the finest divisor 2 and the widest cap 128 at sigma 85, so the slider
/// never runs past what the limits can show.
pub(crate) const SIGMA_MAX: f32 = 64.0;
/// The strongest noise the slider offers; the default is `NOISE_OPACITY` and 0 is none.
pub(crate) const NOISE_MAX: f32 = 0.1;
/// The kernel cap the slider offers, in texels: 4 up to the toolkit's own limit.
pub(crate) const KERNEL_CAP: (f32, f32) = (4.0, BlurLimits::KERNEL_CAP_MAX);
/// The divisors of the blur's working picture, finest first.
pub(crate) const DOWNSAMPLES: [u32; 3] = [2, 4, 8];
/// The most saturation the slider offers.
pub(crate) const SATURATION_MAX: f32 = 3.0;

/// The OS materials, in the order the pane lists them.
pub(crate) const MATERIALS: [(&str, Material); 6] = [
    ("Vibrancy", Material::Vibrancy),
    (
        "Vibrancy, follows the window",
        Material::VibrancyFollowsWindow,
    ),
    ("Liquid, clear", Material::LiquidClear),
    ("Liquid, regular", Material::LiquidRegular),
    ("Opaque", Material::Opaque),
    ("Near-black (UltraDark)", near_black::MATERIAL),
];

/// The motion durations the pane edits, in milliseconds: the theme's five tokens and the
/// tooltip delay, with the range each slider has.
const DURATIONS: [(&str, f32); 6] = [
    ("Instant", 1000.0),
    ("Fast", 1000.0),
    ("Base", 1000.0),
    ("Moderate", 1500.0),
    ("Slow", 2000.0),
    ("Tooltip delay", 3000.0),
];

/// The tints a glass layer offers besides its theme's own.
const TINTS: [(&str, Option<Rgba>); 6] = [
    ("Theme", None),
    ("Black", Some([0, 0, 0, 255])),
    ("White", Some([255, 255, 255, 255])),
    ("Blue", Some([0x1E, 0x5A, 0xC8, 255])),
    ("Green", Some([0x1F, 0x8A, 0x4C, 255])),
    ("Red", Some([0xB0, 0x30, 0x38, 255])),
];

/// What the user changed about one glass layer; `None` keeps the theme's.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct LayerEdit {
    pub(crate) tint: Option<Rgba>,
    pub(crate) tint_opacity: Option<f32>,
    pub(crate) material_opacity: Option<f32>,
}

/// What the user changed about the luminosity curve of the glass layers (all three share it);
/// `None` keeps the theme's. The values are in 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct CurveEdit {
    pub(crate) mid_point: Option<f32>,
    pub(crate) white_max_opacity: Option<f32>,
    pub(crate) mid_point_max_opacity: Option<f32>,
    pub(crate) black_max_opacity: Option<f32>,
}

/// What the user changed about the one glass every floating surface draws (`menu_glass` and
/// `menu_popup_border`, D8); `None` keeps the theme's.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct SurfaceEdit {
    /// The tint's grey level, 0 to 255: the red, green and blue of `menu_glass`.
    pub(crate) tint: Option<f32>,
    /// The tint's opacity, 0 to 1: the alpha of `menu_glass`.
    pub(crate) opacity: Option<f32>,
    /// The ring's alpha, 0 to 255: the alpha of `menu_popup_border`.
    pub(crate) border: Option<f32>,
}

/// The blur kernel's half width in texels at scale 1: `ceil(3 sigma / downsample)`, capped.
pub(crate) fn kernel_radius(sigma: f32, kernel_cap: f32, downsample: u32) -> f32 {
    (3.0 * sigma / downsample.max(1) as f32)
        .ceil()
        .min(kernel_cap)
}

/// A 0 to 255 slider value as a channel.
fn channel(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Every global setting.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Settings {
    /// The index in [`THEMES`], and the theme it loaded.
    pub(crate) theme: usize,
    base: Theme,
    /// Why the chosen file did not load, when it did not (the preset then stands).
    pub(crate) theme_note: Option<String>,
    pub(crate) tier: MotionTier,
    /// Speed of the page's clock against the real one.
    pub(crate) speed: f32,
    pub(crate) zoom: f32,
    pub(crate) acrylic: AcrylicLevel,
    /// Window, panel and card.
    pub(crate) layers: [LayerEdit; 3],
    /// The glass of the floating surfaces.
    pub(crate) surface: SurfaceEdit,
    /// The durations of [`DURATIONS`] in milliseconds; `None` keeps the theme's.
    pub(crate) durations: [Option<f32>; 6],
    /// The luminosity curve of the three glass layers.
    pub(crate) curve: CurveEdit,
    /// The opacity of the noise texture; 0 is none.
    pub(crate) noise: f32,
    pub(crate) sigma: f32,
    /// The blur kernel's cap, in texels, and the divisor of its working picture.
    pub(crate) kernel_cap: f32,
    pub(crate) downsample: u32,
    pub(crate) saturation: f32,
    pub(crate) material: Material,
    /// What came of the last attempt to put `material` on the window.
    pub(crate) material_note: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: 0,
            base: THEME,
            theme_note: None,
            tier: MotionTier::Full,
            speed: 1.0,
            zoom: 1.0,
            acrylic: AcrylicLevel::Default,
            layers: [LayerEdit::default(); 3],
            surface: SurfaceEdit::default(),
            durations: [None; 6],
            curve: CurveEdit::default(),
            noise: NOISE_OPACITY,
            sigma: BLUR_SIGMA,
            kernel_cap: BlurLimits::KERNEL_CAP,
            downsample: BlurLimits::DOWNSAMPLE,
            saturation: BLUR_SATURATION,
            // The material the window is created with.
            material: Material::Vibrancy,
            material_note: None,
        }
    }
}

impl Settings {
    /// Chooses the theme `THEMES[index]`. A file that does not load leaves the preset and says why.
    pub(crate) fn set_theme(&mut self, index: usize) {
        let Some((_, source)) = THEMES.get(index) else {
            return;
        };
        self.theme = index;
        self.theme_note = None;
        self.base = match *source {
            ThemeSource::Preset(theme, material) => {
                if let Some(material) = material {
                    self.material = material;
                    self.material_note = None;
                }
                *theme
            }
            ThemeSource::File(bytes) => match Theme::from_json(bytes, &DARK) {
                Ok(loaded) => loaded.theme,
                Err(e) => {
                    self.theme_note = Some(format!("did not load: {e:?}"));
                    THEME
                }
            },
        };
    }

    /// The theme the settings start from: the chosen file, with the acrylic level.
    fn leveled(&self, base: Theme) -> Theme {
        base.with_acrylic(self.acrylic)
    }

    /// `base` (the chosen theme, or the one a hosted page brings) under the acrylic level, the
    /// glass edits and the motion edits.
    pub(crate) fn compose(&self, base: Theme) -> Theme {
        let mut theme = self.leveled(base);
        let ThemeGlass {
            window,
            panel,
            card,
        } = &mut theme.glass;
        for (layer, edit) in [window, panel, card].into_iter().zip(self.layers) {
            if let Some(tint) = edit.tint {
                layer.tint = tint;
            }
            if let Some(v) = edit.tint_opacity {
                layer.tint_opacity = v;
            }
            if let Some(v) = edit.material_opacity {
                layer.material_opacity = v;
            }
            let curve = &mut layer.curve;
            let c = self.curve;
            for (slot, edit) in [
                (&mut curve.mid_point, c.mid_point),
                (&mut curve.white_max_opacity, c.white_max_opacity),
                (&mut curve.mid_point_max_opacity, c.mid_point_max_opacity),
                (&mut curve.black_max_opacity, c.black_max_opacity),
            ] {
                if let Some(v) = edit {
                    *slot = f64::from(v);
                }
            }
        }
        if let Some(v) = self.surface.tint {
            let g = channel(v);
            theme.menu_glass[..3].fill(g);
        }
        if let Some(v) = self.surface.opacity {
            theme.menu_glass[3] = channel(v * 255.0);
        }
        if let Some(v) = self.surface.border {
            theme.menu_popup_border[3] = channel(v);
        }
        let m = &mut theme.motion;
        let ms = |v: f32| std::time::Duration::from_millis(v.max(0.0).round() as u64);
        for (slot, edit) in [
            &mut m.instant,
            &mut m.fast,
            &mut m.base,
            &mut m.moderate,
            &mut m.slow,
            &mut m.tooltip_delay,
        ]
        .into_iter()
        .zip(self.durations)
        {
            if let Some(v) = edit {
                *slot = ms(v);
            }
        }
        theme
    }

    /// The chosen theme under the settings, for a page that brings none.
    pub(crate) fn theme(&self) -> Theme {
        self.compose(self.base)
    }

    /// The draw parameters.
    pub(crate) fn tweaks(&self) -> FrameTweaks {
        FrameTweaks {
            blur_sigma: self.sigma,
            saturation: self.saturation,
            noise: self.noise,
            blur: BlurLimits {
                kernel_cap: self.kernel_cap,
                downsample: self.downsample,
            },
        }
    }

    /// Everything back to what the gallery starts with. The window learns of a material change
    /// from the next frame's hook.
    pub(crate) fn reset(&mut self) {
        *self = Settings::default();
    }

    /// Draws the pane in `rect`: a scrolling column of sections.
    pub(crate) fn pane(
        &mut self,
        ui: &mut Ui<'_>,
        rect: Rect,
        page: &mut dyn PageState,
        generation: u64,
    ) {
        let opts = ScrollOptions::default();
        ui.scroll_area("settings", rect, &opts, |ui, _| {
            let mut col = Col {
                x: rect.x + 4.0,
                w: rect.w - 16.0,
                y: rect.y + 4.0,
            };
            // The hosted page's own knobs come first, under a scope of the page's choice so a
            // new page starts with fresh widgets.
            col.section(ui, "KNOBS");
            let mut knobs = Knobs::new(&mut col);
            ui.scope(("knobs", generation), |ui| page.knobs(ui, &mut knobs));
            if knobs.is_empty() {
                col.note(ui, "This page has no knobs yet.");
            }
            self.sections(ui, &mut col);
            let height = f64::from(col.y - rect.y + 8.0);
            ((), Extent::new(f64::from(rect.w), height))
        });
    }

    fn sections(&mut self, ui: &mut Ui<'_>, col: &mut Col) {
        col.section(ui, "THEME");
        let options: Vec<RadioOption<'_>> = THEMES
            .iter()
            .enumerate()
            .map(|(i, (label, _))| RadioOption {
                key: i as u64,
                label,
                enabled: true,
            })
            .collect();
        let mut chosen = self.theme as u64;
        if col.radio(ui, "theme", &options, &mut chosen, "Theme") {
            self.set_theme(chosen as usize);
        }
        if let Some(note) = self.theme_note.clone() {
            col.note(ui, &note);
        }
        col.note(ui, "A page that brings its own theme keeps it.");

        col.section(ui, "MOTION");
        let tiers = [
            RadioOption {
                key: 0,
                label: "Full",
                enabled: true,
            },
            RadioOption {
                key: 1,
                label: "Reduced",
                enabled: true,
            },
            RadioOption {
                key: 2,
                label: "Off",
                enabled: true,
            },
        ];
        let mut tier = match self.tier {
            MotionTier::Full => 0,
            MotionTier::Reduced => 1,
            MotionTier::Off => 2,
        };
        if col.radio(ui, "tier", &tiers, &mut tier, "Motion tier") {
            self.tier = [MotionTier::Full, MotionTier::Reduced, MotionTier::Off][tier as usize % 3];
        }
        col.slider(
            ui,
            "speed",
            "Speed",
            &mut self.speed,
            (SPEED.0, SPEED.1, 0.1),
            |v| format!("{v:.1}x"),
        );
        let motion = self.compose(self.base).motion;
        let current = [
            motion.instant,
            motion.fast,
            motion.base,
            motion.moderate,
            motion.slow,
            motion.tooltip_delay,
        ];
        for (i, (label, max)) in DURATIONS.iter().enumerate() {
            let mut ms = self.durations[i].unwrap_or(current[i].as_secs_f32() * 1000.0);
            if col.slider(
                ui,
                ("duration", i),
                label,
                &mut ms,
                (0.0, *max, 10.0),
                |v| format!("{v:.0} ms"),
            ) {
                self.durations[i] = Some(ms);
            }
        }

        col.section(ui, "ZOOM");
        let zooms: Vec<RadioOption<'_>> = ZOOMS
            .iter()
            .enumerate()
            .map(|(i, (label, _))| RadioOption {
                key: i as u64,
                label,
                enabled: true,
            })
            .collect();
        let mut zoom = ZOOMS
            .iter()
            .position(|(_, z)| (*z - self.zoom).abs() < 1e-3)
            .unwrap_or(0) as u64;
        if col.radio(ui, "zoom", &zooms, &mut zoom, "Zoom") {
            self.zoom = ZOOMS[zoom as usize % ZOOMS.len()].1;
        }

        col.section(ui, "GLASS");
        let leveled = self.leveled(self.base).glass;
        let layers = [leveled.window, leveled.panel, leveled.card];
        for (i, name) in ["Window", "Panel", "Card"].into_iter().enumerate() {
            col.label(ui, name);
            let edit = &mut self.layers[i];
            col.swatches(ui, ("tint", i), &mut edit.tint);
            let mut v = edit.tint_opacity.unwrap_or(layers[i].tint_opacity);
            if col.slider(
                ui,
                ("tint-opacity", i),
                "Tint opacity",
                &mut v,
                (0.0, 1.0, 0.0),
                |v| format!("{v:.2}"),
            ) {
                edit.tint_opacity = Some(v);
            }
            let mut v = edit.material_opacity.unwrap_or(layers[i].material_opacity);
            if col.slider(
                ui,
                ("material-opacity", i),
                "Material",
                &mut v,
                (0.0, 1.0, 0.0),
                |v| format!("{v:.2}"),
            ) {
                edit.material_opacity = Some(v);
            }
        }
        let levels = [
            ("Default", AcrylicLevel::Default),
            ("Less opaque", AcrylicLevel::LessOpaque),
            ("More opaque", AcrylicLevel::MoreOpaque),
            ("Opaque", AcrylicLevel::Opaque),
        ];
        let options: Vec<RadioOption<'_>> = levels
            .iter()
            .enumerate()
            .map(|(i, (label, _))| RadioOption {
                key: i as u64,
                label,
                enabled: true,
            })
            .collect();
        let mut level = levels
            .iter()
            .position(|(_, l)| *l == self.acrylic)
            .unwrap_or(0) as u64;
        col.label(ui, "Acrylic level");
        if col.radio(ui, "level", &options, &mut level, "Acrylic level") {
            self.acrylic = levels[level as usize % levels.len()].1;
        }
        col.slider(
            ui,
            "noise",
            "Noise",
            &mut self.noise,
            (0.0, NOISE_MAX, 0.0025),
            |v| {
                if v <= 0.0 {
                    "off".to_owned()
                } else {
                    format!("{v:.4}")
                }
            },
        );
        col.label(ui, "Luminosity curve");
        let shown = leveled.window.curve;
        let rows: [(&str, &str, f64, &mut Option<f32>); 4] = [
            (
                "lum-mid",
                "Mid point",
                shown.mid_point,
                &mut self.curve.mid_point,
            ),
            (
                "lum-white",
                "White max",
                shown.white_max_opacity,
                &mut self.curve.white_max_opacity,
            ),
            (
                "lum-middle",
                "Mid max",
                shown.mid_point_max_opacity,
                &mut self.curve.mid_point_max_opacity,
            ),
            (
                "lum-black",
                "Black max",
                shown.black_max_opacity,
                &mut self.curve.black_max_opacity,
            ),
        ];
        for (key, label, current, edit) in rows {
            let mut v = edit.unwrap_or(current as f32);
            if col.slider(ui, key, label, &mut v, (0.0, 1.0, 0.0), |v| {
                format!("{v:.2}")
            }) {
                *edit = Some(v);
            }
        }
        col.note(ui, "How a dark or light tint suppresses its opacity.");

        col.label(ui, "Floating surfaces");
        let current = self.leveled(self.base);
        let mut v = self
            .surface
            .tint
            .unwrap_or(f32::from(current.menu_glass[0]));
        if col.slider(
            ui,
            "glass-tint",
            "Glass tint",
            &mut v,
            (0.0, 255.0, 1.0),
            |v| format!("{v:.0}"),
        ) {
            self.surface.tint = Some(v);
        }
        let mut v = self
            .surface
            .opacity
            .unwrap_or(f32::from(current.menu_glass[3]) / 255.0);
        if col.slider(
            ui,
            "glass-opacity",
            "Glass opacity",
            &mut v,
            (0.0, 1.0, 0.0),
            |v| format!("{v:.2}"),
        ) {
            self.surface.opacity = Some(v);
        }
        let mut v = self
            .surface
            .border
            .unwrap_or(f32::from(current.menu_popup_border[3]));
        if col.slider(
            ui,
            "glass-border",
            "Glass border",
            &mut v,
            (0.0, 255.0, 1.0),
            |v| format!("{v:.0}"),
        ) {
            self.surface.border = Some(v);
        }
        col.note(ui, "Menus, lists, tooltips, toasts, dialogs.");

        col.section(ui, "BLUR");
        col.slider(
            ui,
            "sigma",
            "Sigma",
            &mut self.sigma,
            (1.0, SIGMA_MAX, 0.5),
            |v| {
                if v >= SIGMA_MAX {
                    format!("{v:.1} (cap)")
                } else {
                    format!("{v:.1}")
                }
            },
        );
        col.slider(
            ui,
            "saturation",
            "Saturation",
            &mut self.saturation,
            (0.0, SATURATION_MAX, 0.1),
            |v| format!("{v:.1}"),
        );
        col.slider(
            ui,
            "kernel-cap",
            "Kernel cap",
            &mut self.kernel_cap,
            (KERNEL_CAP.0, KERNEL_CAP.1, 1.0),
            |v| format!("{v:.0} texels"),
        );
        let options: Vec<RadioOption<'_>> = DOWNSAMPLES
            .iter()
            .map(|d| RadioOption {
                key: u64::from(*d),
                label: match d {
                    2 => "1/2",
                    4 => "1/4",
                    _ => "1/8",
                },
                enabled: true,
            })
            .collect();
        col.label(ui, "Working size");
        let mut chosen = u64::from(self.downsample);
        if col.radio(ui, "downsample", &options, &mut chosen, "Blur working size") {
            self.downsample = chosen as u32;
        }
        let radius = kernel_radius(self.sigma, self.kernel_cap, self.downsample);
        col.note(ui, &format!("The kernel is {radius:.0} texels a side."));

        col.section(ui, "OS MATERIAL");
        let options: Vec<RadioOption<'_>> = MATERIALS
            .iter()
            .enumerate()
            .map(|(i, (label, _))| RadioOption {
                key: i as u64,
                label,
                enabled: true,
            })
            .collect();
        let mut chosen = MATERIALS
            .iter()
            .position(|(_, m)| *m == self.material)
            .unwrap_or(0) as u64;
        if col.radio(ui, "material", &options, &mut chosen, "OS material") {
            self.material = MATERIALS[chosen as usize % MATERIALS.len()].1;
            self.material_note = None;
        }
        if let Some(note) = self.material_note.clone() {
            col.note(ui, &note);
        }

        col.y += 8.0;
        if col.button(ui, "reset", "Reset all settings") {
            self.reset();
        }
    }
}

/// The column the sections stack down: its left edge, width and the next row's top.
#[derive(Debug)]
pub(crate) struct Col {
    pub(crate) x: f32,
    pub(crate) w: f32,
    pub(crate) y: f32,
}

/// A slider row's label width and value width.
const LABEL_W: f32 = 96.0;
const VALUE_W: f32 = 64.0;

impl Col {
    fn text(ui: &mut Ui<'_>, text: &str, x: f32, y: f32, tertiary: bool) {
        let t = *ui.theme();
        let color = if tertiary {
            t.fg_tertiary
        } else {
            t.fg_secondary
        };
        let spec = Spec::plain(text, Style::ui(t.font_s, color));
        ui.painter().text(&spec, x, y);
    }

    pub(crate) fn section(&mut self, ui: &mut Ui<'_>, title: &str) {
        self.y += 14.0;
        let t = *ui.theme();
        let spec = Spec::plain(title, Style::ui(t.font_s, t.fg_tertiary).medium());
        ui.painter().text(&spec, self.x, self.y);
        self.y += 22.0;
    }

    pub(crate) fn label(&mut self, ui: &mut Ui<'_>, text: &str) {
        let t = *ui.theme();
        let spec = Spec::plain(text, Style::ui(t.font_base, t.fg_primary));
        ui.painter().text(&spec, self.x, self.y + 3.0);
        self.y += 24.0;
    }

    pub(crate) fn note(&mut self, ui: &mut Ui<'_>, text: &str) {
        Self::text(ui, text, self.x, self.y, true);
        self.y += 20.0;
    }

    /// A radio group; true when the choice changed.
    pub(crate) fn radio(
        &mut self,
        ui: &mut Ui<'_>,
        key: &str,
        options: &[RadioOption<'_>],
        selected: &mut u64,
        name: &str,
    ) -> bool {
        let rect = Rect::new(self.x, self.y, self.w, options.len() as f32 * ROW_HEIGHT);
        let changed = ui.radio_group(key, rect, options, selected, name).changed;
        self.y += rect.h + 4.0;
        changed
    }

    /// A labelled slider with its value to the right; true when the user moved it.
    pub(crate) fn slider(
        &mut self,
        ui: &mut Ui<'_>,
        key: impl std::hash::Hash,
        label: &str,
        value: &mut f32,
        (min, max, step): (f32, f32, f32),
        show: impl Fn(f32) -> String,
    ) -> bool {
        Self::text(ui, label, self.x, self.y + 4.0, false);
        let track = Rect::new(self.x + LABEL_W, self.y, self.w - LABEL_W - VALUE_W, 24.0);
        let opts = SliderOptions {
            min,
            max,
            step,
            name: label,
        };
        let changed = ui.slider(key, track, value, &opts).changed;
        let t = *ui.theme();
        let spec = Spec::plain(show(*value), Style::mono(t.font_s, 16.0, t.fg_secondary));
        let w = ui.painter().measure(&spec).0;
        ui.painter().text(&spec, self.x + self.w - w, self.y + 4.0);
        self.y += 28.0;
        changed
    }

    pub(crate) fn toggle(&mut self, ui: &mut Ui<'_>, key: &str, label: &str, on: &mut bool) {
        Self::text(ui, label, self.x, self.y + 4.0, false);
        let rect = Rect::new(self.x + self.w - 44.0, self.y, 40.0, 24.0);
        ui.toggle(key, rect, on, label);
        self.y += 28.0;
    }

    pub(crate) fn button(&mut self, ui: &mut Ui<'_>, key: &str, text: &str) -> bool {
        let rect = Rect::new(self.x, self.y, self.w, 32.0);
        let clicked = ui.button(key, rect, text, ButtonKind::Secondary).clicked;
        self.y += 40.0;
        clicked
    }

    /// A row of tint swatches; `edit` is the chosen tint, `None` for the theme's own.
    fn swatches(&mut self, ui: &mut Ui<'_>, key: (&str, usize), edit: &mut Option<Rgba>) {
        let t = *ui.theme();
        for (i, (name, tint)) in TINTS.iter().enumerate() {
            let rect = Rect::new(self.x + i as f32 * 32.0, self.y, 26.0, 24.0);
            let id = ui.id((key, i));
            let r = ui.interact(id, rect, Sense::CLICK_FOCUS);
            let selected = *edit == *tint;
            let mut node = Node::new(id, Role::Button, name, rect);
            node.states.insert(State::Focusable);
            if selected {
                node.states.insert(State::Selected);
            }
            ui.node(node);
            if r.clicked {
                *edit = *tint;
            }
            let p = ui.painter();
            match tint {
                Some(c) => p.fill(rect, 4.0, *c),
                None => {
                    // The theme's own: an empty box with a slash.
                    p.fill(rect, 4.0, t.surface_input);
                    p.text_centered(&Spec::plain("T", Style::ui(t.font_s, t.fg_secondary)), rect);
                }
            }
            let (width, color) = if selected {
                (2.0, t.accent)
            } else if r.hovered {
                (1.0, t.fg_secondary)
            } else {
                (1.0, t.border)
            };
            p.ring(rect, 4.0, width, color, true);
        }
        self.y += 30.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_change_nothing() {
        let s = Settings::default();
        assert_eq!(s.theme(), THEME);
        assert_eq!(s.tweaks(), FrameTweaks::DEFAULT);
        assert_eq!(s.tier, MotionTier::Full);
        assert!((s.speed - 1.0).abs() < f32::EPSILON && (s.zoom - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn every_theme_file_loads() {
        for (i, (name, _)) in THEMES.iter().enumerate() {
            let mut s = Settings::default();
            s.set_theme(i);
            assert_eq!(s.theme_note, None, "{name}");
        }
        let mut s = Settings::default();
        s.set_theme(2);
        assert_ne!(s.theme(), THEME, "the blue file is not the preset");
    }

    #[test]
    fn the_near_black_entries_are_last_and_blue_stays_at_two() {
        assert_eq!(THEMES.len(), 7);
        assert_eq!(THEMES[2].0, "Blue");
        assert_eq!(THEMES[6].0, "Near-black glass (preset)");
        assert_eq!(MATERIALS.len(), 6);
        assert_eq!(
            MATERIALS[5],
            ("Near-black (UltraDark)", near_black::MATERIAL)
        );
    }

    #[test]
    fn selecting_near_black_sets_its_theme_and_material() {
        let mut s = Settings {
            material_note: Some("earlier note".to_owned()),
            ..Settings::default()
        };
        s.set_theme(6);
        assert_eq!(s.theme_note, None);
        assert_eq!(s.theme(), near_black::THEME);
        assert_eq!(s.material, near_black::MATERIAL);
        assert_eq!(s.material_note, None);
        let radio = MATERIALS.iter().position(|(_, m)| *m == s.material);
        assert_eq!(
            radio,
            Some(5),
            "the material radio shows the near-black entry"
        );
        s.material = Material::Opaque;
        assert_eq!(
            s.material,
            Material::Opaque,
            "the radio still changes it by hand"
        );
    }

    #[test]
    fn selecting_the_other_themes_leaves_the_material_alone() {
        let mut s = Settings::default();
        s.set_theme(2);
        assert_eq!(s.material, Material::Vibrancy);
        s.set_theme(0);
        assert_eq!(s.material, Material::Vibrancy);
    }

    #[test]
    fn the_edits_reach_the_theme_and_only_what_was_edited() {
        let mut s = Settings::default();
        s.layers[1].tint = Some([1, 2, 3, 255]);
        s.layers[1].tint_opacity = Some(0.25);
        s.layers[2].material_opacity = Some(0.5);
        s.durations[1] = Some(400.0);
        s.durations[5] = Some(100.0);
        let t = s.theme();
        assert_eq!(t.glass.panel.tint, [1, 2, 3, 255]);
        assert!((t.glass.panel.tint_opacity - 0.25).abs() < 1e-6);
        assert!((t.glass.card.material_opacity - 0.5).abs() < 1e-6);
        assert_eq!(t.glass.window, THEME.glass.window);
        assert_eq!(t.motion.fast, std::time::Duration::from_millis(400));
        assert_eq!(
            t.motion.tooltip_delay,
            std::time::Duration::from_millis(100)
        );
        assert_eq!(t.motion.base, THEME.motion.base);
        // The acrylic level scales the material opacities it is given, under the edits.
        s.acrylic = AcrylicLevel::Opaque;
        let t = s.theme();
        assert!((t.glass.panel.material_opacity - 1.0).abs() < 1e-6);
        assert!(
            (t.glass.card.material_opacity - 0.5).abs() < 1e-6,
            "an edit wins"
        );
    }

    #[test]
    fn a_page_theme_is_the_base_and_the_edits_go_over_it() {
        let mut page = THEME;
        page.glass.panel.tint_opacity = 0.9;
        let mut s = Settings::default();
        assert_eq!(s.compose(page).glass.panel.tint_opacity, 0.9);
        s.layers[1].tint_opacity = Some(0.1);
        assert!((s.compose(page).glass.panel.tint_opacity - 0.1).abs() < 1e-6);
    }

    #[test]
    fn each_glass_slider_changes_its_own_token_and_only_that() {
        let base = Settings::default().theme();
        // Tint: the grey of `menu_glass`; its alpha stays.
        let mut s = Settings::default();
        s.surface.tint = Some(90.0);
        let t = s.theme();
        assert_eq!(t.menu_glass, [90, 90, 90, base.menu_glass[3]]);
        assert_eq!(
            Theme {
                menu_glass: base.menu_glass,
                ..t
            },
            base
        );
        // Opacity: the alpha of `menu_glass`; its grey stays.
        let mut s = Settings::default();
        s.surface.opacity = Some(0.25);
        let t = s.theme();
        assert_eq!(
            t.menu_glass,
            [
                base.menu_glass[0],
                base.menu_glass[1],
                base.menu_glass[2],
                64
            ]
        );
        assert_eq!(
            Theme {
                menu_glass: base.menu_glass,
                ..t
            },
            base
        );
        // Border: the alpha of `menu_popup_border`; its colour stays.
        let mut s = Settings::default();
        s.surface.border = Some(200.0);
        let t = s.theme();
        assert_eq!(t.menu_popup_border[3], 200);
        assert_eq!(t.menu_popup_border[..3], base.menu_popup_border[..3]);
        assert_eq!(
            Theme {
                menu_popup_border: base.menu_popup_border,
                ..t
            },
            base
        );
    }

    #[test]
    fn a_page_theme_keeps_its_glass_and_the_sliders_go_over_it() {
        let mut page = THEME;
        page.menu_glass = [10, 20, 30, 100];
        let mut s = Settings::default();
        assert_eq!(s.compose(page), page, "the defaults change nothing");
        s.surface.opacity = Some(1.0);
        assert_eq!(s.compose(page).menu_glass, [10, 20, 30, 255]);
    }

    #[test]
    fn reset_restores_every_setting() {
        let mut s = Settings::default();
        s.set_theme(3);
        s.tier = MotionTier::Off;
        s.speed = 0.3;
        s.zoom = 2.0;
        s.noise = 0.0;
        s.sigma = 12.0;
        s.kernel_cap = 10.0;
        s.downsample = 2;
        s.curve.mid_point = Some(0.2);
        s.material = Material::Opaque;
        s.durations[0] = Some(5.0);
        s.surface.tint = Some(10.0);
        s.reset();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn the_blur_slider_never_runs_past_what_the_limits_can_show() {
        // The kernel is ceil(3 sigma / downsample) texels, capped (`blur.rs`).
        let default = BlurLimits::default();
        assert!(
            kernel_radius(SIGMA_MAX, default.kernel_cap, default.downsample) < default.kernel_cap
        );
        // Even at the finest divisor the widest cap holds the slider's top.
        assert!(kernel_radius(SIGMA_MAX, KERNEL_CAP.1, 2) < KERNEL_CAP.1);
        const { assert!(SIGMA_MAX > 16.0, "past the old stop") };
        assert_eq!(
            kernel_radius(200.0, 48.0, 8),
            48.0,
            "a wide sigma is capped"
        );
    }

    #[test]
    fn the_noise_slider_reaches_the_frame_and_zero_is_off() {
        let mut s = Settings::default();
        assert_eq!(s.tweaks().noise, NOISE_OPACITY);
        s.noise = 0.05;
        assert_eq!(s.tweaks().noise, 0.05);
        let mut p = s.tweaks().params((10, 10), 1.0);
        assert_eq!(p.noise_opacity, 0.05);
        s.noise = 0.0;
        p = s.tweaks().params((10, 10), 1.0);
        assert_eq!(p.noise_opacity, 0.0);
        assert_eq!(
            s.tweaks(),
            FrameTweaks {
                noise: 0.0,
                ..FrameTweaks::DEFAULT
            }
        );
    }

    #[test]
    fn each_luminosity_control_changes_its_own_value_on_all_three_layers() {
        let base = Settings::default().theme();
        for field in 0..4 {
            let mut s = Settings::default();
            match field {
                0 => s.curve.mid_point = Some(0.3),
                1 => s.curve.white_max_opacity = Some(0.9),
                2 => s.curve.mid_point_max_opacity = Some(0.8),
                _ => s.curve.black_max_opacity = Some(0.1),
            }
            let t = s.theme();
            for (got, was) in [
                (t.glass.window, base.glass.window),
                (t.glass.panel, base.glass.panel),
                (t.glass.card, base.glass.card),
            ] {
                let (g, w) = (got.curve, was.curve);
                let same = [
                    g.mid_point == w.mid_point,
                    g.white_max_opacity == w.white_max_opacity,
                    g.mid_point_max_opacity == w.mid_point_max_opacity,
                    g.black_max_opacity == w.black_max_opacity,
                ];
                for (i, same) in same.into_iter().enumerate() {
                    assert_eq!(same, i != field, "field {field}, value {i}");
                }
                assert_eq!((got.tint, got.tint_opacity), (was.tint, was.tint_opacity));
            }
        }
        // It reaches the colour a layer draws: a white tint's suppression moves.
        let mut s = Settings::default();
        s.layers[0].tint = Some([255, 255, 255, 255]);
        let before = s.theme().glass.window.acrylic_colors().tint[3];
        s.curve.white_max_opacity = Some(0.9);
        assert_ne!(s.theme().glass.window.acrylic_colors().tint[3], before);
        // A page that brings its own curve keeps it until a control is touched.
        let mut page = THEME;
        page.glass.window.curve.white_max_opacity = 0.33;
        let mut s = Settings::default();
        assert_eq!(s.compose(page).glass.window.curve.white_max_opacity, 0.33);
        s.curve.mid_point = Some(0.4);
        let t = s.compose(page).glass.window.curve;
        assert_eq!(
            (t.white_max_opacity, t.mid_point),
            (0.33, f64::from(0.4_f32))
        );
    }

    #[test]
    fn the_kernel_cap_and_the_divisor_reach_the_frames_blur_limits() {
        let mut s = Settings::default();
        assert_eq!(s.tweaks().blur, BlurLimits::default());
        s.kernel_cap = 20.0;
        assert_eq!(
            s.tweaks().blur,
            BlurLimits {
                kernel_cap: 20.0,
                downsample: 8
            }
        );
        s.downsample = 2;
        let p = s.tweaks().params((10, 10), 1.0);
        assert_eq!(
            p.blur,
            BlurLimits {
                kernel_cap: 20.0,
                downsample: 2
            }
        );
        for d in DOWNSAMPLES {
            assert_eq!(
                BlurLimits {
                    kernel_cap: 48.0,
                    downsample: d
                }
                .effective()
                .downsample,
                d
            );
        }
    }

    #[test]
    fn sigma_goes_past_sixteen() {
        let s = Settings {
            sigma: 40.0,
            ..Settings::default()
        };
        assert_eq!(s.tweaks().blur_sigma, 40.0);
    }
}
