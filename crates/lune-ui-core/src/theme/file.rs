//! Loading a theme from a JSON file: bytes in, a new [`Theme`] and a list of what was skipped out.
//! The file is untrusted. Parsing goes through the bounded reader ([`crate::json`]); a colour
//! string is checked byte by byte; a number outside its range is skipped. Nothing here panics and no
//! value outside its type's range enters a [`Theme`].
//!
//! The file is `{ name, author?, version?, colors: {key: "#AARRGGBB"}, acrylic: {key: number},
//! glass? }`. It overlays a base theme: a key the file omits keeps the base value, so an empty
//! `colors` returns the base. Which colour key sets which field is `COLOR_KEYS` in the parent
//! module; the acrylic keys are below. The app reads the bytes and swaps the new theme in at a
//! frame boundary (the toolkit reads no file and watches none).
//!
//! The optional `glass` section is read after `acrylic`, so it wins over the acrylic keys it
//! overlaps. Keys are camelCase and case-sensitive, every member is optional:
//!
//! - `window`, `panel`, `card`: a layer, `{ tint: "#AARRGGBB" | "#RRGGBB", tintOpacity,
//!   materialOpacity, midPoint, whiteMaxOpacity, midPointMaxOpacity, blackMaxOpacity }`, the
//!   numbers in [0, 1]. A layer member the file omits keeps the base value.
//! - `material`: the OS material, which lands in [`ThemeFile::material`] and not in the [`Theme`]
//!   (the window backdrop is applied once by the app; a frame never reads it). Either one of the
//!   strings `"Vibrancy"`, `"VibrancyFollowsWindow"`, `"LiquidClear"`, `"LiquidRegular"`,
//!   `"Opaque"`, or `{ macos?, windows? }` with
//!   `macos = { kind: "vibrancy", effect, state, appearance? } | { kind: "liquid", style, tint?,
//!   opaque? }` and `windows = { kind: "acrylic", tint?, dark? } | { kind: "mica", dark? } |
//!   { kind: "tabbed", dark? }`. `effect`, `state`, `appearance` and `style` are the Rust variant
//!   names of `VibrancyEffect`, `MaterialState`, `MaterialAppearance` and `LiquidStyle`.
//!
//! A `glass` that is not an object is [`ThemeError::WrongType`]. A fault in a layer member skips
//! that member and applies the rest. The material is all or nothing: any fault in it gives
//! `material: None` and one [`Skip::BadMaterial`].

use crate::GlassLayer;
use crate::backdrop::{
    LiquidStyle, MacosMaterial, Material, MaterialAppearance, MaterialState, OsMaterial,
    VibrancyEffect, WindowsMaterial,
};
use crate::color::{Rgba, with_alpha_byte};
use crate::json::{JsonError, JsonErrorKind, Value, parse};

use super::{Theme, color_slot};

/// A loaded theme file.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeFile {
    /// The base theme with the file's keys applied.
    pub theme: Theme,
    pub name: String,
    pub author: Option<String>,
    /// A monotonic integer an app uses to refresh bundled themes; 0 when the file has none.
    pub version: i64,
    /// The OS material of the file's `glass.material`; `None` when the file has none or the
    /// material had a fault (then [`Skip::BadMaterial`] is in `skipped`).
    pub material: Option<Material>,
    /// What the file said that the loader did not apply, in file order. Apps may ignore these:
    /// real files carry about fifty colour keys no `Theme` field reads.
    pub skipped: Vec<Skip>,
}

/// A key of the file that was not applied; the string is the key (at most 256 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    /// A colour key with no `Theme` field.
    UnknownColor(String),
    /// A known colour key whose value is not `#RRGGBB` or `#AARRGGBB`.
    BadColor(String),
    /// An acrylic key that sets nothing here.
    UnknownAcrylic(String),
    /// A known acrylic key whose value is outside [0, 1].
    BadAcrylic(String),
    /// A `glass` key that sets nothing: the key (`foo`) or a layer's key (`window.foo`).
    UnknownGlass(String),
    /// A known `glass` layer member that is not a number in [0, 1] or a colour, or a layer that is
    /// not an object: its path, such as `window.blackMaxOpacity` or `card`.
    BadGlass(String),
    /// The `glass.material` had a fault, so none of it applied: the path of the first fault, such
    /// as `material.macos.effect`.
    BadMaterial(String),
}

/// Why a file is no theme at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeError {
    /// The bytes are not JSON within the reader's limits.
    Json(JsonError),
    /// The top-level value is not an object.
    NotAnObject,
    /// A required field is absent.
    Missing(&'static str),
    /// A field has the wrong type (`colors: null` included).
    WrongType(&'static str),
    /// `name` is empty or only whitespace.
    EmptyName,
}

impl Theme {
    /// Reads a theme file over `base`: the file's colours and glass opacities replace the base's,
    /// the fields derived from them follow (below), everything else keeps the base value.
    ///
    /// Derived fields, each only when its source key was applied (so the preset's values are
    /// reproduced exactly by a file that sets the preset's sources): `scroll_thumb`, `scroll_thumb_hover` and
    /// `text_selection` (the accent at alpha 0x8C, 0xE6, 0x66), `diff_added_mark` and
    /// `diff_removed_mark` (the line fill at alpha 0x40), and the three `menu_text_*` colours (the
    /// matching `fg_*`).
    ///
    /// Errors name the envelope's fault: not JSON, not an object, `name`, `colors` or `acrylic`
    /// absent or of the wrong type, an empty `name`, `author` or `version` of the wrong type, a
    /// `glass` that is not an object. `author` may be null or absent; `version` is an exact
    /// integer or absent; `glass` is optional (the module comment has its schema).
    pub fn from_json(bytes: &[u8], base: &Theme) -> Result<ThemeFile, ThemeError> {
        let root = parse(bytes).map_err(ThemeError::Json)?;
        if !matches!(root, Value::Object(_)) {
            return Err(ThemeError::NotAnObject);
        }
        let name = field(&root, "name")?
            .as_str()
            .map_err(|_| ThemeError::WrongType("name"))?;
        if name.trim().is_empty() {
            return Err(ThemeError::EmptyName);
        }
        let author = match optional(&root, "author")? {
            None | Some(Value::Null) => None,
            Some(v) => Some(
                v.as_str()
                    .map_err(|_| ThemeError::WrongType("author"))?
                    .to_string(),
            ),
        };
        let version = match optional(&root, "version")? {
            None => 0,
            Some(v) => v.as_int().map_err(|_| ThemeError::WrongType("version"))?,
        };
        let colors = field(&root, "colors")?
            .string_map()
            .map_err(|_| ThemeError::WrongType("colors"))?;
        let acrylic = field(&root, "acrylic")?
            .number_map()
            .map_err(|_| ThemeError::WrongType("acrylic"))?;

        let mut theme = *base;
        let mut skipped = Vec::new();
        let mut applied: Vec<&str> = Vec::new();
        for (key, text) in colors {
            // A key is looked up once to find out if it is known, so the order of the checks is
            // the report: an unknown key is unknown whatever its value.
            let Some(slot) = color_slot(&mut theme, key) else {
                skipped.push(Skip::UnknownColor(key.to_string()));
                continue;
            };
            match parse_color(text) {
                Some(c) => {
                    slot.set(c);
                    applied.push(key);
                }
                None => skipped.push(Skip::BadColor(key.to_string())),
            }
        }
        for (key, value) in acrylic {
            let Some(slot) = acrylic_slot(&mut theme, key) else {
                skipped.push(Skip::UnknownAcrylic(key.to_string()));
                continue;
            };
            if (0.0..=1.0).contains(&value) {
                // The value is a finite f64 in [0, 1]; the cast keeps it in range.
                *slot = value as f32;
            } else {
                skipped.push(Skip::BadAcrylic(key.to_string()));
            }
        }
        let material = match optional(&root, "glass")? {
            None => None,
            Some(glass) => apply_glass(glass, &mut theme, &mut skipped)?,
        };
        derive(&mut theme, &applied);
        Ok(ThemeFile {
            theme,
            name: name.to_string(),
            author,
            version,
            material,
            skipped,
        })
    }
}

/// A required member: `Missing` when absent.
fn field<'v>(root: &'v Value, key: &'static str) -> Result<&'v Value, ThemeError> {
    root.get(key).map_err(|e| match e.kind {
        JsonErrorKind::Missing(_) => ThemeError::Missing(key),
        _ => ThemeError::NotAnObject,
    })
}

/// An optional member: `None` when absent.
fn optional<'v>(root: &'v Value, key: &'static str) -> Result<Option<&'v Value>, ThemeError> {
    match root.get(key) {
        Ok(v) => Ok(Some(v)),
        Err(JsonError {
            kind: JsonErrorKind::Missing(_),
            ..
        }) => Ok(None),
        Err(_) => Err(ThemeError::NotAnObject),
    }
}

/// `#RRGGBB` (opaque) or `#AARRGGBB`, hex digits in either case, nothing else: no space, no sign,
/// no short form, no name. Checked as bytes, so a multi-byte character is just not a digit.
fn parse_color(text: &str) -> Option<Rgba> {
    let digits = text.as_bytes().strip_prefix(b"#")?;
    if digits.len() != 6 && digits.len() != 8 {
        return None;
    }
    let mut v = [0u8; 8];
    for (slot, d) in v.iter_mut().zip(digits) {
        if !d.is_ascii_hexdigit() {
            return None;
        }
        *slot = char::from(*d).to_digit(16)? as u8;
    }
    let byte = |i: usize| v[i] << 4 | v[i + 1];
    Some(if digits.len() == 8 {
        [byte(2), byte(4), byte(6), byte(0)]
    } else {
        [byte(0), byte(2), byte(4), 0xFF]
    })
}

/// The glass opacity an acrylic key sets.
fn acrylic_slot<'a>(theme: &'a mut Theme, key: &str) -> Option<&'a mut f32> {
    let g = &mut theme.glass;
    Some(match key {
        "GlassTintDarkOpacity" => &mut g.window.tint_opacity,
        "GlassMaterialDarkOpacity" => &mut g.window.material_opacity,
        "GlassTintLightOpacity" => &mut g.panel.tint_opacity,
        "GlassMaterialLightOpacity" => &mut g.panel.material_opacity,
        "GlassMaterialDarkHeaderOpacity" => &mut g.card.material_opacity,
        _ => return None,
    })
}

/// Applies the `glass` section to `theme` and returns its material. A fault in a layer member is
/// a skip; a fault anywhere in the material is one skip and no material.
fn apply_glass(
    glass: &Value,
    theme: &mut Theme,
    skipped: &mut Vec<Skip>,
) -> Result<Option<Material>, ThemeError> {
    let members = glass
        .as_object()
        .map_err(|_| ThemeError::WrongType("glass"))?;
    let mut material = None;
    for (key, value) in members {
        let layer = match key.as_str() {
            "window" => &mut theme.glass.window,
            "panel" => &mut theme.glass.panel,
            "card" => &mut theme.glass.card,
            "material" => {
                match parse_material(value) {
                    Ok(m) => material = Some(m),
                    Err(path) => skipped.push(Skip::BadMaterial(path)),
                }
                continue;
            }
            _ => {
                skipped.push(Skip::UnknownGlass(key.clone()));
                continue;
            }
        };
        apply_layer(layer, key, value, skipped);
    }
    Ok(material)
}

/// Applies one layer object over `layer`; `name` is the layer's key, the prefix of every path.
fn apply_layer(layer: &mut GlassLayer, name: &str, value: &Value, skipped: &mut Vec<Skip>) {
    let Ok(members) = value.as_object() else {
        skipped.push(Skip::BadGlass(name.to_string()));
        return;
    };
    for (key, value) in members {
        let path = format!("{name}.{key}");
        let curve = &mut layer.curve;
        if key == "tint" {
            match value.as_str().ok().and_then(parse_color) {
                Some(c) => layer.tint = c,
                None => skipped.push(Skip::BadGlass(path)),
            }
            continue;
        }
        let known = matches!(
            key.as_str(),
            "tintOpacity"
                | "materialOpacity"
                | "midPoint"
                | "whiteMaxOpacity"
                | "midPointMaxOpacity"
                | "blackMaxOpacity"
        );
        if !known {
            skipped.push(Skip::UnknownGlass(path));
            continue;
        }
        let Some(n) = unit(value) else {
            skipped.push(Skip::BadGlass(path));
            continue;
        };
        match key.as_str() {
            // `n` is in [0, 1]; the cast to f32 keeps it there.
            "tintOpacity" => layer.tint_opacity = n as f32,
            "materialOpacity" => layer.material_opacity = n as f32,
            "midPoint" => curve.mid_point = n,
            "whiteMaxOpacity" => curve.white_max_opacity = n,
            "midPointMaxOpacity" => curve.mid_point_max_opacity = n,
            _ => curve.black_max_opacity = n,
        }
    }
}

/// A number in [0, 1], else `None` (a NaN fails the range check too).
fn unit(value: &Value) -> Option<f64> {
    value.as_f64().ok().filter(|n| (0.0..=1.0).contains(n))
}

/// The material, or the path of the first fault. The path is built from fixed names only (never
/// the file's text), so it stays short.
fn parse_material(value: &Value) -> Result<Material, String> {
    const P: &str = "material";
    if let Ok(name) = value.as_str() {
        return match name {
            "Vibrancy" => Ok(Material::Vibrancy),
            "VibrancyFollowsWindow" => Ok(Material::VibrancyFollowsWindow),
            "LiquidClear" => Ok(Material::LiquidClear),
            "LiquidRegular" => Ok(Material::LiquidRegular),
            "Opaque" => Ok(Material::Opaque),
            _ => Err(P.to_string()),
        };
    }
    let members = object(value, P)?;
    only_keys(members, &["macos", "windows"], P)?;
    let macos = member(members, "macos").map(parse_macos).transpose()?;
    let windows = member(members, "windows").map(parse_windows).transpose()?;
    Ok(Material::Custom(OsMaterial { macos, windows }))
}

fn parse_macos(value: &Value) -> Result<MacosMaterial, String> {
    const P: &str = "material.macos";
    let members = object(value, P)?;
    match required_text(members, "kind", P)? {
        "vibrancy" => {
            only_keys(members, &["kind", "effect", "state", "appearance"], P)?;
            let effect = required_text(members, "effect", P)?;
            let effect = VIBRANCY_EFFECTS
                .iter()
                .find(|(name, _)| *name == effect)
                .map(|(_, e)| *e)
                .ok_or_else(|| format!("{P}.effect"))?;
            let state = match required_text(members, "state", P)? {
                "Active" => MaterialState::Active,
                "Inactive" => MaterialState::Inactive,
                "FollowsWindow" => MaterialState::FollowsWindow,
                _ => return Err(format!("{P}.state")),
            };
            let appearance = match text(members, "appearance", P)? {
                None => None,
                Some("Dark") => Some(MaterialAppearance::Dark),
                Some("Light") => Some(MaterialAppearance::Light),
                Some(_) => return Err(format!("{P}.appearance")),
            };
            Ok(MacosMaterial::Vibrancy {
                effect,
                state,
                appearance,
            })
        }
        "liquid" => {
            only_keys(members, &["kind", "style", "tint", "opaque"], P)?;
            let style = match required_text(members, "style", P)? {
                "Clear" => LiquidStyle::Clear,
                "Regular" => LiquidStyle::Regular,
                _ => return Err(format!("{P}.style")),
            };
            Ok(MacosMaterial::Liquid {
                style,
                tint: colour(members, "tint", P)?,
                opaque: flag(members, "opaque", P)?,
            })
        }
        _ => Err(format!("{P}.kind")),
    }
}

fn parse_windows(value: &Value) -> Result<WindowsMaterial, String> {
    const P: &str = "material.windows";
    let members = object(value, P)?;
    match required_text(members, "kind", P)? {
        "acrylic" => {
            only_keys(members, &["kind", "tint", "dark"], P)?;
            Ok(WindowsMaterial::Acrylic {
                tint: colour(members, "tint", P)?,
                dark: flag(members, "dark", P)?,
            })
        }
        kind @ ("mica" | "tabbed") => {
            only_keys(members, &["kind", "dark"], P)?;
            let dark = flag(members, "dark", P)?;
            Ok(if kind == "mica" {
                WindowsMaterial::Mica { dark }
            } else {
                WindowsMaterial::Tabbed { dark }
            })
        }
        _ => Err(format!("{P}.kind")),
    }
}

/// The 19 effects by their Rust variant names.
const VIBRANCY_EFFECTS: [(&str, VibrancyEffect); 19] = [
    ("AppearanceBased", VibrancyEffect::AppearanceBased),
    ("Light", VibrancyEffect::Light),
    ("Dark", VibrancyEffect::Dark),
    ("Titlebar", VibrancyEffect::Titlebar),
    ("Selection", VibrancyEffect::Selection),
    ("Menu", VibrancyEffect::Menu),
    ("Popover", VibrancyEffect::Popover),
    ("Sidebar", VibrancyEffect::Sidebar),
    ("MediumLight", VibrancyEffect::MediumLight),
    ("UltraDark", VibrancyEffect::UltraDark),
    ("HeaderView", VibrancyEffect::HeaderView),
    ("Sheet", VibrancyEffect::Sheet),
    ("WindowBackground", VibrancyEffect::WindowBackground),
    ("HudWindow", VibrancyEffect::HudWindow),
    ("FullScreenUI", VibrancyEffect::FullScreenUI),
    ("Tooltip", VibrancyEffect::Tooltip),
    ("ContentBackground", VibrancyEffect::ContentBackground),
    (
        "UnderWindowBackground",
        VibrancyEffect::UnderWindowBackground,
    ),
    ("UnderPageBackground", VibrancyEffect::UnderPageBackground),
];

type Members = [(String, Value)];

fn object<'v>(value: &'v Value, path: &str) -> Result<&'v Members, String> {
    value.as_object().map_err(|_| path.to_string())
}

fn member<'v>(members: &'v Members, key: &str) -> Option<&'v Value> {
    members.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Refuses a key outside `allowed`: the fault is the parent's path, never the key's text.
fn only_keys(members: &Members, allowed: &[&str], path: &str) -> Result<(), String> {
    if members.iter().all(|(k, _)| allowed.contains(&k.as_str())) {
        Ok(())
    } else {
        Err(path.to_string())
    }
}

/// An optional string member; a member of another type is a fault.
fn text<'v>(members: &'v Members, key: &str, path: &str) -> Result<Option<&'v str>, String> {
    match member(members, key) {
        None => Ok(None),
        Some(v) => v.as_str().map(Some).map_err(|_| format!("{path}.{key}")),
    }
}

fn required_text<'v>(members: &'v Members, key: &str, path: &str) -> Result<&'v str, String> {
    text(members, key, path)?.ok_or_else(|| format!("{path}.{key}"))
}

/// An optional boolean member.
fn flag(members: &Members, key: &str, path: &str) -> Result<Option<bool>, String> {
    match member(members, key) {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(format!("{path}.{key}")),
    }
}

/// An optional colour member (`#RRGGBB` or `#AARRGGBB`).
fn colour(members: &Members, key: &str, path: &str) -> Result<Option<Rgba>, String> {
    match text(members, key, path)? {
        None => Ok(None),
        Some(t) => parse_color(t)
            .map(Some)
            .ok_or_else(|| format!("{path}.{key}")),
    }
}

/// The fields with no key of their own, from the applied sources (D4).
fn derive(t: &mut Theme, applied: &[&str]) {
    let has = |key: &str| applied.contains(&key);
    if has("AccentPrimaryColor") {
        t.scroll_thumb = with_alpha_byte(t.accent, 0x8C);
        t.scroll_thumb_hover = with_alpha_byte(t.accent, 0xE6);
        t.text_selection = with_alpha_byte(t.accent, 0x66);
    }
    if has("DiffAddedBackground") {
        t.diff_added_mark = with_alpha_byte(t.diff_added, 0x40);
    }
    if has("DiffRemovedBackground") {
        t.diff_removed_mark = with_alpha_byte(t.diff_removed, 0x40);
    }
    if has("ForegroundPrimary") {
        t.menu_text_primary = t.fg_primary;
    }
    if has("ForegroundTertiary") {
        t.menu_text_tertiary = t.fg_tertiary;
    }
    if has("ForegroundQuaternary") {
        t.menu_text_quaternary = t.fg_quaternary;
    }
}

#[cfg(test)]
mod tests {
    use super::parse_color;

    #[test]
    fn colours_are_aarrggbb_or_rrggbb() {
        assert_eq!(parse_color("#CCFFFFFF"), Some([0xFF, 0xFF, 0xFF, 0xCC]));
        assert_eq!(parse_color("#1A0E08"), Some([0x1A, 0x0E, 0x08, 0xFF]));
        assert_eq!(parse_color("#aabbccdd"), Some([0xBB, 0xCC, 0xDD, 0xAA]));
        for bad in [
            "",
            "#",
            "#FFF",
            "#FFFFF",
            "#FFFFFFF",
            "#FFFFFFFFF",
            "FFFFFF",
            "#GGGGGG",
            "#+F+F+F",
            "#FFé",
            "# FFFFF",
            "#FFFFFF ",
            " #FFFFFF",
            "#ＦＦＦＦＦＦ",
        ] {
            assert_eq!(parse_color(bad), None, "{bad:?}");
        }
    }
}
