//! Theme files: the overlay over a base theme, the colour syntax, the skips, the envelope errors,
//! the derived tokens, the acrylic level and the hard cut a theme swap makes of colour transitions.
//! The known answers on the real files are in the gallery's file tests.

use lune_ui_core::color::Rgba;
use lune_ui_core::input::Repaint;
use lune_ui_core::motion::{Kind, Motion};
use lune_ui_core::paint::Rect;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::{AcrylicLevel, COLOR_KEYS, DARK, Skip, Theme, ThemeError, ThemeFile};

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
    want.accent_hover = [0x81, 0xC0, 0xFF, 0xFF];
    want.scroll_thumb = [0x71, 0xB0, 0xF7, 0x8C];
    want.scroll_thumb_hover = [0x71, 0xB0, 0xF7, 0xE6];
    want.text_selection = [0x71, 0xB0, 0xF7, 0x66];
    assert_eq!(f.theme, want);
    // Saturating, per channel.
    let f = load(&with_colors(r##""AccentPrimaryColor":"#F0FFF8EF""##));
    assert_eq!(f.theme.accent_hover, [0xFF; 4]);
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
fn the_key_table_has_50_distinct_keys_each_of_which_resolves() {
    assert_eq!(COLOR_KEYS.len(), 50);
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
        &mut base.accent_hover,
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
                        panel: [0.1, 0.1, 0.12, 1.0],
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
