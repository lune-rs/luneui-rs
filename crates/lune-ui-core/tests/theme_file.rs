//! Theme files: the overlay over a base theme, the colour syntax, the skips, the envelope errors,
//! the derived tokens, the acrylic level and the hard cut a theme swap makes of colour transitions.
//! The known answers on the real files are in the gallery's file tests.

use lune_ui_core::GlassLayer;
use lune_ui_core::backdrop::{
    LiquidStyle, MacosMaterial, Material, MaterialAppearance, MaterialState, OsMaterial,
    VibrancyEffect, WindowsMaterial,
};
use lune_ui_core::color::Rgba;
use lune_ui_core::input::Repaint;
use lune_ui_core::motion::{Kind, Motion};
use lune_ui_core::paint::Rect;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::{
    AcrylicLevel, COLOR_KEYS, DARK, Skip, Theme, ThemeError, ThemeFile, near_black,
};

fn load(json: &str) -> ThemeFile {
    match Theme::from_json(json.as_bytes(), &DARK) {
        Ok(f) => f,
        Err(e) => panic!("expected a theme for {json:?}, got {e:?}"),
    }
}

fn with_colors(colors: &str) -> String {
    format!(r#"{{"name":"t","colors":{{{colors}}},"acrylic":{{}}}}"#)
}

fn with_acrylic(acrylic: &str) -> String {
    format!(r#"{{"name":"t","colors":{{}},"acrylic":{{{acrylic}}}}}"#)
}

fn error(json: &str) -> ThemeError {
    match Theme::from_json(json.as_bytes(), &DARK) {
        Ok(f) => panic!(
            "expected an error for {json:?}, got a theme named {:?}",
            f.name
        ),
        Err(e) => e,
    }
}

fn hex(c: Rgba) -> String {
    format!("#{:02X}{:02X}{:02X}{:02X}", c[3], c[0], c[1], c[2])
}

// --- Overlay --------------------------------------------------------------------------------

#[test]
fn one_key_changes_one_field_and_nothing_else() {
    let f = load(&with_colors(r##""SurfaceBorder":"#80102030""##));
    let mut want = DARK;
    want.border = [0x10, 0x20, 0x30, 0x80];
    assert_eq!(f.theme, want);
    assert!(f.skipped.is_empty());
    assert_eq!((f.name.as_str(), f.author, f.version), ("t", None, 0));
}

#[test]
fn the_accent_brings_its_derived_tokens() {
    let f = load(&with_colors(r##""AccentPrimaryColor":"#FF71B0F7""##));
    let mut want = DARK;
    want.accent = [0x71, 0xB0, 0xF7, 0xFF];
    want.scroll_thumb = [0x71, 0xB0, 0xF7, 0x8C];
    want.scroll_thumb_hover = [0x71, 0xB0, 0xF7, 0xE6];
    want.text_selection = [0x71, 0xB0, 0xF7, 0x66];
    assert_eq!(f.theme, want);
}

#[test]
fn each_derived_token_follows_only_its_own_source() {
    let t = |key: &str, v: &str| load(&with_colors(&format!(r#""{key}":"{v}""#))).theme;
    let f = t("DiffAddedBackground", "#2681B88B");
    assert_eq!(f.diff_added, [0x81, 0xB8, 0x8B, 0x26]);
    assert_eq!(f.diff_added_mark, [0x81, 0xB8, 0x8B, 0x40]);
    assert_eq!(f.diff_removed_mark, DARK.diff_removed_mark);
    let f = t("DiffRemovedBackground", "#26C74E39");
    assert_eq!(f.diff_removed_mark, [0xC7, 0x4E, 0x39, 0x40]);
    assert_eq!(f.diff_added_mark, DARK.diff_added_mark);
    let f = t("ForegroundPrimary", "#FFCCCCCC");
    assert_eq!(f.menu_text_primary, [0xCC, 0xCC, 0xCC, 0xFF]);
    assert_eq!(f.menu_text_tertiary, DARK.menu_text_tertiary);
    let f = t("ForegroundTertiary", "#FF9D9D9D");
    assert_eq!(f.menu_text_tertiary, [0x9D, 0x9D, 0x9D, 0xFF]);
    assert_eq!(f.menu_text_primary, DARK.menu_text_primary);
    let f = t("ForegroundQuaternary", "#FF868686");
    assert_eq!(f.menu_text_quaternary, [0x86, 0x86, 0x86, 0xFF]);
    // A source that is skipped derives nothing.
    let f = t("DiffAddedBackground", "#GGGGGG");
    assert_eq!(f.diff_added_mark, DARK.diff_added_mark);
    let f = t("AccentPrimaryColor", "nope");
    assert_eq!(f.scroll_thumb, DARK.scroll_thumb);
}

#[test]
fn an_empty_file_returns_the_base_unchanged() {
    let f = load(r#"{"name":"t","colors":{},"acrylic":{}}"#);
    assert_eq!(f.theme, DARK);
    // Whatever the base is.
    let mut base = DARK;
    base.accent = [1, 2, 3, 4];
    base.scroll_thumb = [5, 6, 7, 8];
    let f = Theme::from_json(br#"{"name":"t","colors":{},"acrylic":{}}"#, &base);
    assert_eq!(f.map(|f| f.theme), Ok(base));
}

#[test]
fn the_key_table_has_47_distinct_keys_each_of_which_resolves() {
    assert_eq!(COLOR_KEYS.len(), 47);
    let mut seen = std::collections::BTreeSet::new();
    for k in COLOR_KEYS {
        assert!(seen.insert(*k), "{k} twice");
        assert!(DARK.color_key(k).is_some(), "{k}");
    }
    assert_eq!(DARK.color_key("NoSuchKey"), None);
    assert_eq!(
        DARK.color_key("foregroundprimary"),
        None,
        "keys are case-sensitive"
    );
}

#[test]
fn the_preset_written_through_the_table_loads_back_equal() {
    // Every key at the preset's value, the five glass opacities too; the base is the preset with
    // every table field and every derived field changed, so only the file can restore them.
    let colors: Vec<String> = COLOR_KEYS
        .iter()
        .map(|k| format!(r#""{k}":"{}""#, hex(DARK.color_key(k).unwrap_or_default())))
        .collect();
    let g = DARK.glass;
    let acrylic = format!(
        r#""GlassTintDarkOpacity":{},"GlassMaterialDarkOpacity":{},"GlassTintLightOpacity":{},"GlassMaterialLightOpacity":{},"GlassMaterialDarkHeaderOpacity":{}"#,
        g.window.tint_opacity,
        g.window.material_opacity,
        g.panel.tint_opacity,
        g.panel.material_opacity,
        g.card.material_opacity
    );
    let json = format!(
        r#"{{"name":"dark","colors":{{{}}},"acrylic":{{{acrylic}}}}}"#,
        colors.join(",")
    );

    let junk: Rgba = [9, 8, 7, 6];
    let mut base = DARK;
    for k in COLOR_KEYS {
        // Overwrite through the loader itself, so the fields the table names are the ones changed.
        let f = Theme::from_json(
            with_colors(&format!(r##""{k}":"#09080706""##)).as_bytes(),
            &base,
        );
        base = f.map(|f| f.theme).unwrap_or(base);
    }
    for field in [
        &mut base.scroll_thumb,
        &mut base.scroll_thumb_hover,
        &mut base.text_selection,
        &mut base.menu_text_primary,
        &mut base.menu_text_tertiary,
        &mut base.menu_text_quaternary,
        &mut base.diff_added_mark,
        &mut base.diff_removed_mark,
    ] {
        *field = junk;
    }
    base.glass.window.tint_opacity = 0.123;
    base.glass.window.material_opacity = 0.123;
    base.glass.panel.tint_opacity = 0.123;
    base.glass.panel.material_opacity = 0.123;
    base.glass.card.material_opacity = 0.123;
    assert_ne!(base, DARK, "the perturbed base differs");

    let f = Theme::from_json(json.as_bytes(), &base);
    let Ok(f) = f else {
        panic!("the preset file must load: {f:?}")
    };
    assert!(f.skipped.is_empty(), "{:?}", f.skipped);
    assert_eq!(f.theme, DARK);
}

// --- Glass ----------------------------------------------------------------------------------

#[test]
fn the_tint_key_sets_all_three_layers_and_the_opacity_keys_one_each() {
    let json = r##"{"name":"t","colors":{"GlassTintColor":"#1A0E08"},"acrylic":{
        "GlassTintDarkOpacity":0.1,"GlassMaterialDarkOpacity":0.2,
        "GlassTintLightOpacity":0.3,"GlassMaterialLightOpacity":0.4,
        "GlassMaterialDarkHeaderOpacity":0.5}}"##;
    let g = load(json).theme.glass;
    let tint = [0x1A, 0x0E, 0x08, 0xFF];
    assert_eq!(
        (g.window.tint, g.panel.tint, g.card.tint),
        (tint, tint, tint)
    );
    assert_eq!(
        (g.window.tint_opacity, g.window.material_opacity),
        (0.1, 0.2)
    );
    assert_eq!((g.panel.tint_opacity, g.panel.material_opacity), (0.3, 0.4));
    assert_eq!(g.card.material_opacity, 0.5);
    assert_eq!(
        g.card.tint_opacity, DARK.glass.card.tint_opacity,
        "no key sets it"
    );
}

// --- Colour syntax --------------------------------------------------------------------------

#[test]
fn argb_goes_to_rgba_and_rgb_is_opaque() {
    let f = load(&with_colors(
        r##""SurfaceInput":"#CCFFFFFF","SurfaceHover":"#1A0E08""##,
    ));
    assert_eq!(f.theme.surface_input, [0xFF, 0xFF, 0xFF, 0xCC]);
    assert_eq!(f.theme.surface_hover, [0x1A, 0x0E, 0x08, 0xFF]);
    let f = load(&with_colors(r##""SurfaceInput":"#aabbccdd""##));
    assert_eq!(f.theme.surface_input, [0xBB, 0xCC, 0xDD, 0xAA]);
}

#[test]
fn bad_colours_are_skipped_by_key_and_change_nothing() {
    let long = format!("#{}", "F".repeat(4095));
    for bad in [
        "#ZZZZZZ",
        "#+F+F+F",
        "#FFé",
        "#FFF",
        "",
        "#",
        "FFFFFF",
        "#FFFFFFF",
        "#FFFFFFFFF",
        "# FFFFF",
        "#FFFFFF ",
        " #FFFFFF",
        "#-1-1-1-1",
        "red",
        "#ＦＦＦＦＦＦ",
        &long,
    ] {
        let f = load(&with_colors(&format!(r#""SurfaceInput":"{bad}""#)));
        assert_eq!(
            f.skipped,
            [Skip::BadColor("SurfaceInput".to_string())],
            "{bad:?}"
        );
        assert_eq!(f.theme, DARK, "{bad:?}");
    }
}

#[test]
fn each_skip_kind_is_reported_in_file_order() {
    let json = r##"{"name":"t",
        "colors":{"Nonsense":"#FFFFFF","SurfaceInput":"zzz","SurfaceHover":"#FF000000"},
        "acrylic":{"GlassMaterialSolidOpacity":0.5,"GlassMaterialDarkOpacity":1.5,"GlassTintDarkOpacity":-0.1,"GlassTintLightOpacity":0}}"##;
    let f = load(json);
    assert_eq!(
        f.skipped,
        [
            Skip::UnknownColor("Nonsense".to_string()),
            Skip::BadColor("SurfaceInput".to_string()),
            Skip::UnknownAcrylic("GlassMaterialSolidOpacity".to_string()),
            Skip::BadAcrylic("GlassMaterialDarkOpacity".to_string()),
            Skip::BadAcrylic("GlassTintDarkOpacity".to_string()),
        ]
    );
    assert_eq!(
        f.theme.surface_hover,
        [0, 0, 0, 0xFF],
        "the good key still applies"
    );
    assert_eq!(f.theme.glass.panel.tint_opacity, 0.0, "0 is in range");
    assert_eq!(
        f.theme.glass.window.material_opacity,
        DARK.glass.window.material_opacity
    );
    // An unknown key is unknown whatever its value says.
    let f = load(&with_colors(r#""Nonsense":"bad""#));
    assert_eq!(f.skipped, [Skip::UnknownColor("Nonsense".to_string())]);
    // The ends of the range are in.
    let f = load(&with_acrylic(
        r#""GlassTintDarkOpacity":1,"GlassMaterialDarkOpacity":0"#,
    ));
    assert!(f.skipped.is_empty());
    assert_eq!(f.theme.glass.window.material_opacity, 0.0);
}

#[test]
fn the_window_fallback_key_is_gone() {
    // An inactive window stays glass (R13): no widget has a solid fill by window state, so
    // `WindowFallbackBackground` is an unknown key.
    let f = load(&with_colors(r##""WindowFallbackBackground":"#FF333333""##));
    assert_eq!(
        f.skipped,
        [Skip::UnknownColor("WindowFallbackBackground".to_string())]
    );
    assert_eq!(f.theme, DARK);
}

#[test]
fn the_toast_fill_and_border_keys_are_gone() {
    // The toast card is the glass surface (D8): `menu_glass` and `menu_popup_border` draw it.
    let f = load(&with_colors(
        r##""ToastBackground":"#E6181818","ToastBorder":"#33FFFFFF""##,
    ));
    assert_eq!(
        f.skipped,
        [
            Skip::UnknownColor("ToastBackground".to_string()),
            Skip::UnknownColor("ToastBorder".to_string())
        ]
    );
    assert_eq!(f.theme, DARK);
}

#[test]
fn a_dropdown_fill_key_is_gone_and_the_glass_tint_has_a_key() {
    // `BackdropDark` fed the opaque list panel (D8); every list is glass now, so it is unknown.
    let f = load(&with_colors(r##""BackdropDark":"#F0000000""##));
    assert_eq!(f.skipped, [Skip::UnknownColor("BackdropDark".to_string())]);
    assert_eq!(f.theme, DARK);
    let f = load(&with_colors(r##""MenuGlassBackground":"#80102030""##));
    assert!(f.skipped.is_empty(), "{:?}", f.skipped);
    assert_eq!(f.theme.menu_glass, [0x10, 0x20, 0x30, 0x80]);
}

// --- Envelope -------------------------------------------------------------------------------

#[test]
fn the_envelope_is_checked() {
    assert!(matches!(error("not json"), ThemeError::Json(_)));
    assert!(matches!(error(""), ThemeError::Json(_)));
    assert_eq!(error("[]"), ThemeError::NotAnObject);
    assert_eq!(error("\"x\""), ThemeError::NotAnObject);
    assert_eq!(
        error(r#"{"colors":{},"acrylic":{}}"#),
        ThemeError::Missing("name")
    );
    assert_eq!(
        error(r#"{"name":"t","acrylic":{}}"#),
        ThemeError::Missing("colors")
    );
    assert_eq!(
        error(r#"{"name":"t","colors":{}}"#),
        ThemeError::Missing("acrylic")
    );
    assert_eq!(
        error(r#"{"name":3,"colors":{},"acrylic":{}}"#),
        ThemeError::WrongType("name")
    );
    assert_eq!(
        error(r#"{"name":null,"colors":{},"acrylic":{}}"#),
        ThemeError::WrongType("name")
    );
    assert_eq!(
        error(r#"{"name":"","colors":{},"acrylic":{}}"#),
        ThemeError::EmptyName
    );
    assert_eq!(
        error(r#"{"name":" \t","colors":{},"acrylic":{}}"#),
        ThemeError::EmptyName
    );
    assert_eq!(
        error(r#"{"name":"t","colors":null,"acrylic":{}}"#),
        ThemeError::WrongType("colors")
    );
    assert_eq!(
        error(r#"{"name":"t","colors":[],"acrylic":{}}"#),
        ThemeError::WrongType("colors")
    );
    assert_eq!(
        error(r#"{"name":"t","colors":{"a":1},"acrylic":{}}"#),
        ThemeError::WrongType("colors")
    );
    assert_eq!(
        error(r#"{"name":"t","colors":{},"acrylic":null}"#),
        ThemeError::WrongType("acrylic")
    );
    assert_eq!(
        error(r#"{"name":"t","colors":{},"acrylic":{"a":"x"}}"#),
        ThemeError::WrongType("acrylic")
    );
    assert_eq!(
        error(r#"{"name":"t","author":3,"colors":{},"acrylic":{}}"#),
        ThemeError::WrongType("author")
    );
    assert_eq!(
        error(r#"{"name":"t","version":1.5,"colors":{},"acrylic":{}}"#),
        ThemeError::WrongType("version")
    );
    assert_eq!(
        error(r#"{"name":"t","version":"2","colors":{},"acrylic":{}}"#),
        ThemeError::WrongType("version")
    );
    assert_eq!(
        error(r#"{"name":"t","version":9007199254740993,"colors":{},"acrylic":{}}"#),
        ThemeError::WrongType("version"),
        "an inexact integer is no version"
    );
    // The reader's errors come through whole.
    let ThemeError::Json(e) = error(r#"{"name":"t","name":"u","colors":{},"acrylic":{}}"#) else {
        panic!("expected the reader's error")
    };
    assert_eq!(
        e.kind,
        lune_ui_core::json::JsonErrorKind::DuplicateKey("name".to_string())
    );
}

#[test]
fn optional_fields_and_unknown_top_level_keys() {
    let f = load(
        r#"{"name":"Blue","author":"me","version":7,"extra":{"a":[1]},"colors":{},"acrylic":{}}"#,
    );
    assert_eq!(
        (f.name.as_str(), f.author.as_deref(), f.version),
        ("Blue", Some("me"), 7)
    );
    let f = load(r#"{"name":"t","author":null,"colors":{},"acrylic":{}}"#);
    assert_eq!(f.author, None);
    // Case-sensitive top-level names: "Name" is no name.
    assert_eq!(
        error(r#"{"Name":"t","colors":{},"acrylic":{}}"#),
        ThemeError::Missing("name")
    );
    // Key names are case-sensitive too.
    let f = load(&with_colors(r##""foregroundprimary":"#FFFFFFFF""##));
    assert_eq!(
        f.skipped,
        [Skip::UnknownColor("foregroundprimary".to_string())]
    );
}

#[test]
fn nothing_panics_on_a_damaged_file() {
    let json = br##"{"name":"t","author":"a","version":1,"colors":{"AccentPrimaryColor":"#FF71B0F7","GlassTintColor":"#1A0E08"},"acrylic":{"GlassTintDarkOpacity":0.5}}"##;
    for n in 0..json.len() {
        let _ = Theme::from_json(&json[..n], &DARK);
        let mut cut = json.to_vec();
        cut.remove(n);
        let _ = Theme::from_json(&cut, &DARK);
        let mut flip = json.to_vec();
        flip[n] ^= 0xFF;
        let _ = Theme::from_json(&flip, &DARK);
    }
}

// --- Glass section --------------------------------------------------------------------------

fn with_glass(glass: &str) -> String {
    format!(r#"{{"name":"t","colors":{{}},"acrylic":{{}},"glass":{glass}}}"#)
}

fn colour_text(c: Rgba) -> String {
    hex(c).replacen('#', "\"#", 1) + "\""
}

/// A layer as the file writes it, from the layer's own fields.
fn layer_json(l: &GlassLayer) -> String {
    format!(
        r#"{{"tint":{},"tintOpacity":{},"materialOpacity":{},"midPoint":{},"whiteMaxOpacity":{},"midPointMaxOpacity":{},"blackMaxOpacity":{}}}"#,
        colour_text(l.tint),
        l.tint_opacity,
        l.material_opacity,
        l.curve.mid_point,
        l.curve.white_max_opacity,
        l.curve.mid_point_max_opacity,
        l.curve.black_max_opacity,
    )
}

fn effect_name(e: VibrancyEffect) -> &'static str {
    // Exhaustive: a new effect does not compile until it has a name here (and in the loader).
    match e {
        VibrancyEffect::AppearanceBased => "AppearanceBased",
        VibrancyEffect::Light => "Light",
        VibrancyEffect::Dark => "Dark",
        VibrancyEffect::Titlebar => "Titlebar",
        VibrancyEffect::Selection => "Selection",
        VibrancyEffect::Menu => "Menu",
        VibrancyEffect::Popover => "Popover",
        VibrancyEffect::Sidebar => "Sidebar",
        VibrancyEffect::MediumLight => "MediumLight",
        VibrancyEffect::UltraDark => "UltraDark",
        VibrancyEffect::HeaderView => "HeaderView",
        VibrancyEffect::Sheet => "Sheet",
        VibrancyEffect::WindowBackground => "WindowBackground",
        VibrancyEffect::HudWindow => "HudWindow",
        VibrancyEffect::FullScreenUI => "FullScreenUI",
        VibrancyEffect::Tooltip => "Tooltip",
        VibrancyEffect::ContentBackground => "ContentBackground",
        VibrancyEffect::UnderWindowBackground => "UnderWindowBackground",
        VibrancyEffect::UnderPageBackground => "UnderPageBackground",
    }
}

const ALL_EFFECTS: [VibrancyEffect; 19] = [
    VibrancyEffect::AppearanceBased,
    VibrancyEffect::Light,
    VibrancyEffect::Dark,
    VibrancyEffect::Titlebar,
    VibrancyEffect::Selection,
    VibrancyEffect::Menu,
    VibrancyEffect::Popover,
    VibrancyEffect::Sidebar,
    VibrancyEffect::MediumLight,
    VibrancyEffect::UltraDark,
    VibrancyEffect::HeaderView,
    VibrancyEffect::Sheet,
    VibrancyEffect::WindowBackground,
    VibrancyEffect::HudWindow,
    VibrancyEffect::FullScreenUI,
    VibrancyEffect::Tooltip,
    VibrancyEffect::ContentBackground,
    VibrancyEffect::UnderWindowBackground,
    VibrancyEffect::UnderPageBackground,
];

/// The near-black material as the file writes it, from the constant's own fields.
fn near_black_material_json() -> String {
    let Material::Custom(OsMaterial {
        macos:
            Some(MacosMaterial::Vibrancy {
                effect,
                state,
                appearance: None,
            }),
        windows:
            Some(WindowsMaterial::Acrylic {
                tint: Some(tint),
                dark: Some(dark),
            }),
    }) = near_black::MATERIAL
    else {
        panic!("near_black::MATERIAL changed shape: update this helper");
    };
    format!(
        r#"{{"macos":{{"kind":"vibrancy","effect":"{effect:?}","state":"{state:?}"}},"windows":{{"kind":"acrylic","tint":{},"dark":{dark}}}}}"#,
        colour_text(tint),
    )
}

#[test]
fn a_file_built_from_the_near_black_constants_loads_as_near_black() {
    // Retuning a constant needs no change here: the file is written from the constants.
    let glass = format!(
        r#"{{"window":{},"panel":{},"card":{},"material":{}}}"#,
        layer_json(&near_black::WINDOW),
        layer_json(&near_black::PANEL),
        layer_json(&near_black::CARD),
        near_black_material_json(),
    );
    let f = load(&with_glass(&glass));
    assert_eq!(f.skipped, []);
    assert_eq!(f.theme, near_black::THEME);
    assert_eq!(f.material, Some(near_black::MATERIAL));
}

#[test]
fn a_file_without_glass_has_no_material_and_changes_no_glass() {
    let f = load(&with_colors(""));
    assert_eq!(f.material, None);
    assert_eq!(f.theme, DARK);
    assert_eq!(f.skipped, []);
    let empty = load(&with_glass("{}"));
    assert_eq!(
        (empty.theme, empty.material, empty.skipped),
        (DARK, None, vec![])
    );
}

#[test]
fn each_layer_key_sets_exactly_its_field() {
    type Set = fn(&mut GlassLayer, f64);
    let keys: [(&str, Set); 6] = [
        ("tintOpacity", |l, n| l.tint_opacity = n as f32),
        ("materialOpacity", |l, n| l.material_opacity = n as f32),
        ("midPoint", |l, n| l.curve.mid_point = n),
        ("whiteMaxOpacity", |l, n| l.curve.white_max_opacity = n),
        ("midPointMaxOpacity", |l, n| {
            l.curve.mid_point_max_opacity = n
        }),
        ("blackMaxOpacity", |l, n| l.curve.black_max_opacity = n),
    ];
    for (key, set) in keys {
        for layer in ["window", "panel", "card"] {
            let f = load(&with_glass(&format!(r#"{{"{layer}":{{"{key}":0.33}}}}"#)));
            let mut want = DARK;
            let target = match layer {
                "window" => &mut want.glass.window,
                "panel" => &mut want.glass.panel,
                _ => &mut want.glass.card,
            };
            set(target, 0.33);
            assert_eq!(f.theme, want, "{layer}.{key}");
            assert_eq!(f.skipped, [], "{layer}.{key}");
        }
    }
    let f = load(&with_glass(r##"{"card":{"tint":"#102030"}}"##));
    let mut want = DARK;
    want.glass.card.tint = [0x10, 0x20, 0x30, 0xFF];
    assert_eq!(f.theme, want);
    let f = load(&with_glass(r##"{"card":{"tint":"#80102030"}}"##));
    want.glass.card.tint = [0x10, 0x20, 0x30, 0x80];
    assert_eq!(f.theme, want);
}

#[test]
fn glass_wins_over_the_acrylic_keys_it_overlaps() {
    let json = r#"{"name":"t","colors":{},"acrylic":{"GlassMaterialDarkOpacity":0.2,"GlassTintDarkOpacity":0.3},"glass":{"window":{"materialOpacity":0.9}}}"#;
    let f = load(json);
    assert_eq!(f.theme.glass.window.material_opacity, 0.9);
    assert_eq!(f.theme.glass.window.tint_opacity, 0.3);
}

#[test]
fn layer_numbers_accept_zero_and_one_and_skip_outside_the_range() {
    for ok in ["0.0", "1.0", "0", "1"] {
        let f = load(&with_glass(&format!(
            r#"{{"panel":{{"blackMaxOpacity":{ok}}}}}"#
        )));
        assert_eq!(f.skipped, [], "{ok}");
    }
    for bad in [
        "1.0001", "-0.1", "2", "-1e-9", "1e300", r#""0.5""#, "null", "true", "[]",
    ] {
        let f = load(&with_glass(&format!(
            r#"{{"panel":{{"blackMaxOpacity":{bad},"midPoint":0.25}}}}"#
        )));
        assert_eq!(
            f.skipped,
            [Skip::BadGlass("panel.blackMaxOpacity".to_string())],
            "{bad}"
        );
        // The field is untouched and the sibling member still applies.
        assert_eq!(
            f.theme.glass.panel.curve.black_max_opacity, DARK.glass.panel.curve.black_max_opacity,
            "{bad}"
        );
        assert_eq!(f.theme.glass.panel.curve.mid_point, 0.25, "{bad}");
    }
}

#[test]
fn bad_layer_values_and_unknown_keys_are_skipped_by_path() {
    let f = load(&with_glass(
        r##"{"window":{"tint":"red","foo":1,"tintOpacity":0.5},"card":3,"panel":null,"nope":{}}"##,
    ));
    assert_eq!(
        f.skipped,
        [
            Skip::BadGlass("window.tint".to_string()),
            Skip::UnknownGlass("window.foo".to_string()),
            Skip::BadGlass("card".to_string()),
            Skip::BadGlass("panel".to_string()),
            Skip::UnknownGlass("nope".to_string()),
        ]
    );
    assert_eq!(f.theme.glass.window.tint, DARK.glass.window.tint);
    assert_eq!(f.theme.glass.window.tint_opacity, 0.5);
    assert_eq!(f.material, None);
}

#[test]
fn glass_of_the_wrong_type_is_an_error() {
    for bad in ["[]", "1", "null", r#""x""#, "true"] {
        assert_eq!(
            error(&with_glass(bad)),
            ThemeError::WrongType("glass"),
            "{bad}"
        );
    }
}

#[test]
fn a_curve_midpoint_at_either_end_draws_without_a_panic() {
    for mid in ["0.0", "1.0"] {
        for tint in ["#000000", "#FFFFFF", "#808080", "#FF0000", "#7F7F00"] {
            let f = load(&with_glass(&format!(
                r#"{{"window":{{"tint":"{tint}","midPoint":{mid}}},"panel":{{"midPoint":{mid}}}}}"#
            )));
            for l in [
                f.theme.glass.window,
                f.theme.glass.panel,
                f.theme.glass.card,
            ] {
                let p = l.premultiplied();
                assert!(p.iter().all(|v| v.is_finite()), "{mid} {tint}: {p:?}");
            }
        }
    }
}

#[test]
fn a_preset_material_string_loads_to_that_preset() {
    for (name, want) in [
        ("Vibrancy", Material::Vibrancy),
        ("VibrancyFollowsWindow", Material::VibrancyFollowsWindow),
        ("LiquidClear", Material::LiquidClear),
        ("LiquidRegular", Material::LiquidRegular),
        ("Opaque", Material::Opaque),
    ] {
        let f = load(&with_glass(&format!(r#"{{"material":"{name}"}}"#)));
        assert_eq!(f.material, Some(want), "{name}");
        assert_eq!(f.skipped, [], "{name}");
        // The Theme is untouched: the material is not a drawing value.
        assert_eq!(f.theme, DARK, "{name}");
    }
}

#[test]
fn every_vibrancy_effect_loads_by_its_name() {
    for e in ALL_EFFECTS {
        let f = load(&with_glass(&format!(
            r#"{{"material":{{"macos":{{"kind":"vibrancy","effect":"{}","state":"Inactive","appearance":"Light"}}}}}}"#,
            effect_name(e)
        )));
        assert_eq!(
            f.material,
            Some(Material::Custom(OsMaterial {
                macos: Some(MacosMaterial::Vibrancy {
                    effect: e,
                    state: MaterialState::Inactive,
                    appearance: Some(MaterialAppearance::Light),
                }),
                windows: None,
            })),
            "{e:?}"
        );
    }
}

#[test]
fn the_other_material_forms_load() {
    let f = load(&with_glass(
        r##"{"material":{"macos":{"kind":"liquid","style":"Regular","tint":"#80102030","opaque":true},"windows":{"kind":"mica","dark":false}}}"##,
    ));
    assert_eq!(
        f.material,
        Some(Material::Custom(OsMaterial {
            macos: Some(MacosMaterial::Liquid {
                style: LiquidStyle::Regular,
                tint: Some([0x10, 0x20, 0x30, 0x80]),
                opaque: Some(true),
            }),
            windows: Some(WindowsMaterial::Mica { dark: Some(false) }),
        }))
    );
    for (kind, want) in [
        ("tabbed", WindowsMaterial::Tabbed { dark: None }),
        (
            "acrylic",
            WindowsMaterial::Acrylic {
                tint: None,
                dark: None,
            },
        ),
    ] {
        let f = load(&with_glass(&format!(
            r#"{{"material":{{"windows":{{"kind":"{kind}"}}}}}}"#
        )));
        assert_eq!(
            f.material,
            Some(Material::Custom(OsMaterial {
                macos: None,
                windows: Some(want)
            }))
        );
    }
    let f = load(&with_glass(r#"{"material":{}}"#));
    assert_eq!(
        f.material,
        Some(Material::Custom(OsMaterial {
            macos: None,
            windows: None
        }))
    );
}

#[test]
fn a_fault_in_the_material_leaves_no_material_and_one_skip() {
    let cases = [
        (
            r#"{"macos":{"kind":"vibrancy","effect":"Nope","state":"Active"}}"#,
            "material.macos.effect",
        ),
        (
            r#"{"macos":{"kind":"vibrancy","effect":"UltraDark"}}"#,
            "material.macos.state",
        ),
        (
            r#"{"macos":{"kind":"vibrancy","effect":"UltraDark","state":"Active","appearance":"Blue"}}"#,
            "material.macos.appearance",
        ),
        (r#"{"macos":{"effect":"UltraDark"}}"#, "material.macos.kind"),
        (r#"{"macos":{"kind":"glass"}}"#, "material.macos.kind"),
        (r#"{"macos":{"kind":"liquid"}}"#, "material.macos.style"),
        (
            r#"{"macos":{"kind":"liquid","style":"Clear","tint":"nope"}}"#,
            "material.macos.tint",
        ),
        (
            r#"{"macos":{"kind":"liquid","style":"Clear","opaque":"yes"}}"#,
            "material.macos.opaque",
        ),
        // A key of the other kind, and a key of no kind.
        (
            r#"{"macos":{"kind":"liquid","style":"Clear","effect":"Light"}}"#,
            "material.macos",
        ),
        (
            r##"{"windows":{"kind":"mica","tint":"#000000"}}"##,
            "material.windows",
        ),
        (
            r#"{"windows":{"kind":"acrylic","dark":1}}"#,
            "material.windows.dark",
        ),
        (r#"{"windows":{"kind":"x"}}"#, "material.windows.kind"),
        (r#"{"windows":3}"#, "material.windows"),
        (r#"{"linux":{}}"#, "material"),
        (r#""Vibrancyy""#, "material"),
        (r#""vibrancy""#, "material"),
        ("3", "material"),
        ("null", "material"),
        ("[]", "material"),
    ];
    for (material, path) in cases {
        // A valid first half proves nothing of the material applies when its second half is bad.
        let f = load(&with_glass(&format!(
            r#"{{"material":{material},"window":{{"midPoint":0.25}}}}"#
        )));
        assert_eq!(f.material, None, "{material}");
        assert_eq!(
            f.skipped,
            [Skip::BadMaterial(path.to_string())],
            "{material}"
        );
        // The layers still apply.
        assert_eq!(f.theme.glass.window.curve.mid_point, 0.25, "{material}");
    }
}

#[test]
fn a_hostile_glass_section_cannot_panic_or_escape_its_range() {
    let json = br##"{"name":"t","colors":{},"acrylic":{},"glass":{"window":{"tint":"#80102030","tintOpacity":0.5,"midPoint":0.0},"material":{"macos":{"kind":"vibrancy","effect":"UltraDark","state":"Active"},"windows":{"kind":"acrylic","tint":"#A0000000","dark":true}}}}"##;
    for n in 0..json.len() {
        let _ = Theme::from_json(&json[..n], &DARK);
        let mut cut = json.to_vec();
        cut.remove(n);
        let _ = Theme::from_json(&cut, &DARK);
        let mut flip = json.to_vec();
        flip[n] ^= 0xFF;
        let _ = Theme::from_json(&flip, &DARK);
    }
}

// --- Acrylic level --------------------------------------------------------------------------

fn material(t: &Theme) -> [f32; 3] {
    [
        t.glass.window.material_opacity,
        t.glass.panel.material_opacity,
        t.glass.card.material_opacity,
    ]
}

fn near(a: [f32; 3], b: [f32; 3]) {
    for (x, y) in a.into_iter().zip(b) {
        assert!((x - y).abs() < 1e-6, "{a:?} vs {b:?}");
    }
}

#[test]
fn acrylic_levels_scale_the_material_opacities_only() {
    // The preset has 0.7, 0.7 and 0.8.
    assert_eq!(material(&DARK), [0.7, 0.7, 0.8]);
    assert_eq!(
        DARK.with_acrylic(AcrylicLevel::Default),
        DARK,
        "Default returns the input"
    );
    near(
        material(&DARK.with_acrylic(AcrylicLevel::LessOpaque)),
        [0.35, 0.35, 0.4],
    );
    near(
        material(&DARK.with_acrylic(AcrylicLevel::MoreOpaque)),
        [0.85, 0.85, 0.9],
    );
    near(
        material(&DARK.with_acrylic(AcrylicLevel::Opaque)),
        [1.0, 1.0, 1.0],
    );
    // Tints and every other field stay.
    let more = DARK.with_acrylic(AcrylicLevel::MoreOpaque);
    assert_eq!(more.glass.window.tint, DARK.glass.window.tint);
    assert_eq!(more.glass.panel.tint_opacity, DARK.glass.panel.tint_opacity);
    let mut same = more;
    same.glass = DARK.glass;
    assert_eq!(same, DARK);
    // Out-of-range inputs (a base built by hand) end up in [0, 1].
    let mut odd = DARK;
    odd.glass.window.material_opacity = 7.0;
    odd.glass.panel.material_opacity = -3.0;
    for level in [
        AcrylicLevel::LessOpaque,
        AcrylicLevel::MoreOpaque,
        AcrylicLevel::Opaque,
    ] {
        for m in material(&odd.with_acrylic(level)) {
            assert!((0.0..=1.0).contains(&m), "{level:?} {m}");
        }
    }
    assert_eq!(AcrylicLevel::default(), AcrylicLevel::Default);
}

// --- The swap is a hard cut -----------------------------------------------------------------

fn other_theme() -> Theme {
    load(&with_colors(
        r##""AccentPrimaryColor":"#FF71B0F7","ForegroundPrimary":"#FFCCCCCC","SurfaceHover":"#40FF0000""##,
    ))
    .theme
}

/// A toggle drawn at 0 ms and 1000 ms (settled) under `a`, then once at 1016 ms under `b`.
fn swap_frame(a: Theme, b: Theme, on: bool) -> (lune_ui_core::scene::Scene, Repaint) {
    let rect = Rect::new(20.0, 20.0, 40.0, 24.0);
    let mut state = on;
    let mut h = UiHarness::new((200.0, 100.0));
    h.theme = a;
    h.at_ms(0);
    h.frame(|ui| {
        ui.toggle("t", rect, &mut state, "Notifications");
    });
    h.at_ms(1000);
    let (_, out) = h.frame(|ui| {
        ui.toggle("t", rect, &mut state, "Notifications");
    });
    assert_eq!(out.repaint, Repaint::Idle, "settled under the first theme");
    h.theme = b;
    h.at_ms(1016);
    let (scene, out) = h.frame(|ui| {
        ui.toggle("t", rect, &mut state, "Notifications");
    });
    (scene, out.repaint)
}

fn solo(b: Theme, on: bool) -> lune_ui_core::scene::Scene {
    let rect = Rect::new(20.0, 20.0, 40.0, 24.0);
    let mut state = on;
    let mut h = UiHarness::new((200.0, 100.0));
    h.theme = b;
    h.at_ms(1016);
    h.frame(|ui| {
        ui.toggle("t", rect, &mut state, "Notifications");
    })
    .0
}

#[test]
fn a_toggle_drawn_under_theme_a_then_b_equals_a_solo_frame_under_b() {
    // Mutant: drop the `theme_cut` branch in `animate_color` and the colours fade from A to B, so
    // the scene differs from the solo frame and this fails (checked by hand when written).
    for on in [false, true] {
        let b = other_theme();
        let (scene, repaint) = swap_frame(DARK, b, on);
        assert_eq!(scene, solo(b, on), "on = {on}");
        assert_ne!(
            repaint,
            Repaint::Now,
            "nothing animates because of the swap, on = {on}"
        );
        assert_ne!(scene, solo(DARK, on), "the swap changed something");
    }
}

#[test]
fn a_swap_snaps_a_colour_that_is_mid_transition_and_one_whose_target_did_not_move() {
    let m = Motion::of(Kind::Hover);
    let (c1, c2, c3): (Rgba, Rgba, Rgba) = ([10, 20, 30, 255], [200, 100, 50, 255], [1, 2, 3, 4]);
    let mut h = UiHarness::new((100.0, 100.0));
    let id = lune_ui_core::ui::WidgetId::ROOT.with("c");
    let run = |h: &mut UiHarness, ms: u64, target: Rgba| {
        h.at_ms(ms);
        let mut v = [0; 4];
        let (_, out) = h.frame(|ui| v = ui.animate_color(id, target, &m));
        (v, out.repaint)
    };
    assert_eq!(run(&mut h, 0, c1).0, c1);
    // The target moves under the same theme: a fade starts, and a little later it is between.
    assert_eq!(run(&mut h, 20, c2).1, Repaint::Now);
    let (mid, repaint) = run(&mut h, 60, c2);
    assert_eq!(repaint, Repaint::Now);
    assert!(mid != c1 && mid != c2, "{mid:?} is mid-fade");
    // The theme changes with the target unmoved: the fade in flight snaps to the target.
    h.theme = other_theme();
    let (v, repaint) = run(&mut h, 70, c2);
    assert_eq!(v, c2, "snapped to the target");
    assert_ne!(repaint, Repaint::Now);
    // The theme changes and the target moves too: no fade, the target at once.
    h.theme = DARK;
    let (v, repaint) = run(&mut h, 80, c3);
    assert_eq!(v, c3);
    assert_ne!(repaint, Repaint::Now);
    // Same theme again: fades resume (the cut is for the swap frame only).
    assert_eq!(run(&mut h, 90, c1).1, Repaint::Now);
    let (v, repaint) = run(&mut h, 130, c1);
    assert_eq!(repaint, Repaint::Now);
    assert!(v != c1 && v != c3, "{v:?} is mid-fade again");
}

#[test]
fn a_float_transition_is_not_cut_by_a_swap() {
    use lune_ui_core::ui::WidgetId;
    let m = Motion::of(Kind::Hover);
    let id = WidgetId::ROOT.with("f");
    let mut h = UiHarness::new((100.0, 100.0));
    let run = |h: &mut UiHarness, ms: u64, target: f32| {
        h.at_ms(ms);
        let mut v = 0.0;
        h.frame(|ui| v = ui.animate_f32(id, target, &m));
        v
    };
    run(&mut h, 0, 0.0);
    run(&mut h, 20, 1.0);
    h.theme = other_theme();
    let v = run(&mut h, 30, 1.0);
    assert!(v > 0.0 && v < 1.0, "{v}: still moving across the swap");
}

#[test]
fn the_first_frame_needs_no_snap() {
    // A first frame under any theme draws that theme's colours at once and rests.
    let b = other_theme();
    let rect = Rect::new(20.0, 20.0, 40.0, 24.0);
    let mut on = true;
    let mut h = UiHarness::new((200.0, 100.0));
    h.theme = b;
    let (_, out) = h.frame(|ui| {
        ui.toggle("t", rect, &mut on, "Notifications");
    });
    assert_eq!(out.repaint, Repaint::Idle);
}

// A theme swap and a closing overlay. The exit of an overlay replays what its content recorded,
// colours already resolved, so a swap must not let one fade out in the old theme (D209).

mod closing {
    use super::*;
    use lune_ui_core::paint::LayerId;
    use lune_ui_core::scene::Scene;
    use lune_ui_core::ui::Ui;
    use lune_ui_core::widgets::dialog::{Cancel, Dialog, DialogGlass};
    use lune_ui_core::widgets::label::LabelRole;
    use lune_ui_core::widgets::popover::PopoverOptions;

    #[derive(Clone, Copy, Debug)]
    enum Kind {
        Popover,
        Dialog,
    }

    const TOGGLE: Rect = Rect::new(20.0, 20.0, 40.0, 24.0);

    /// A page (a toggle) and, while `open`, the overlay.
    fn frame(h: &mut UiHarness, ms: u64, kind: Kind, open: bool) -> (Scene, Repaint) {
        let mut on = true;
        h.at_ms(ms);
        let (scene, out) = h.frame(|ui: &mut Ui<'_>| {
            ui.painter().on(LayerId::Page);
            ui.toggle("t", TOGGLE, &mut on, "Notifications");
            if !open {
                return;
            }
            match kind {
                Kind::Popover => {
                    let o = PopoverOptions::new(Rect::new(20.0, 60.0, 80.0, 24.0), (140.0, 80.0));
                    ui.popover("pop", &o, |ui, rect| {
                        ui.label("l", (rect.x + 8.0, rect.y + 8.0), "Hello", LabelRole::Base);
                    });
                }
                Kind::Dialog => {
                    let d = Dialog {
                        title: "Discard",
                        message: "Unsaved changes.",
                        extra_h: 0.0,
                        confirm: "Discard",
                        cancel: Cancel::Default,
                    };
                    let glass = DialogGlass {
                        fill: [0.2, 0.2, 0.2, 0.62],
                        noise: true,
                    };
                    ui.dialog("dialog", &d, &glass, |_, _, _, _| {});
                }
            }
        });
        (scene, out.repaint)
    }

    /// The overlay opens under theme `a` and settles, stops being called at 1100 ms, and the theme
    /// becomes `b` at `swap_ms` (1100: the frame the overlay starts to close; 1116: the next one).
    /// Returns the frame at `swap_ms` and what a frame a second later reports.
    fn run(kind: Kind, a: Theme, b: Theme, swap_ms: u64) -> (Scene, Repaint, Repaint) {
        let mut h = UiHarness::new((300.0, 200.0));
        h.theme = a;
        frame(&mut h, 0, kind, true);
        frame(&mut h, 600, kind, true);
        frame(&mut h, 1000, kind, true);
        if swap_ms > 1100 {
            frame(&mut h, 1100, kind, false);
        }
        h.theme = b;
        let (scene, repaint) = frame(&mut h, swap_ms, kind, false);
        let (_, later) = frame(&mut h, swap_ms + 1000, kind, false);
        (scene, repaint, later)
    }

    /// The page alone under `b` at `ms`, a fresh window.
    fn solo(kind: Kind, b: Theme, ms: u64) -> Scene {
        let mut h = UiHarness::new((300.0, 200.0));
        h.theme = b;
        frame(&mut h, ms, kind, false).0
    }

    #[test]
    fn a_closing_overlay_is_gone_in_the_frame_of_a_swap() {
        // Mutant: drop the `theme_cut` retain in `settle_overlays` and the exit replays in the old
        // theme's colours, so the scene differs from the page alone and this fails.
        for kind in [Kind::Popover, Kind::Dialog] {
            for swap_ms in [1100, 1116] {
                let b = other_theme();
                let (scene, _, later) = run(kind, DARK, b, swap_ms);
                assert_eq!(
                    scene,
                    solo(kind, b, swap_ms),
                    "{kind:?} swapped at {swap_ms}"
                );
                // Nothing of the overlay is left to move: the page goes idle.
                assert_eq!(later, Repaint::Idle, "{kind:?} at {swap_ms}");
            }
        }
    }

    #[test]
    fn the_same_frame_without_a_swap_still_shows_the_exit() {
        // The control: with no swap the overlay is still fading out at those times, so the scene
        // is not the page alone and the frame asks for another.
        for kind in [Kind::Popover, Kind::Dialog] {
            let (scene, repaint, later) = run(kind, DARK, DARK, 1116);
            assert_ne!(scene, solo(kind, DARK, 1116), "{kind:?}");
            assert_eq!(repaint, Repaint::Now, "{kind:?}");
            assert_eq!(later, Repaint::Idle, "{kind:?}");
        }
    }
}
