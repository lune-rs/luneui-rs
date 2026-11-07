//! Loading a theme from a JSON file: bytes in, a new [`Theme`] and a list of what was skipped out.
//! The file is untrusted. Parsing goes through the bounded reader ([`crate::json`]); a colour
//! string is checked byte by byte; a number outside its range is skipped. Nothing here panics and no
//! value outside its type's range enters a [`Theme`].
//!
//! The file is `{ name, author?, version?, colors: {key: "#AARRGGBB"}, acrylic: {key: number} }`.
//! It overlays a base theme: a key the file omits keeps the base value, so an empty `colors`
//! returns the base. Which colour key sets which field is `COLOR_KEYS` in the parent module; the
//! acrylic keys are below. The app reads the bytes and swaps the new theme in at a frame boundary
//! (the toolkit reads no file and watches none).

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
    /// absent or of the wrong type, an empty `name`, `author` or `version` of the wrong type.
    /// `author` may be null or absent; `version` is an exact integer or absent.
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
        derive(&mut theme, &applied);
        Ok(ThemeFile {
            theme,
            name: name.to_string(),
            author,
            version,
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
