//! Known answers for the bounded JSON reader on Lunegit's real theme and locale files (bundled
//! byte for byte under `assets/lunegit/`, see `licenses/lunegit-fixtures.txt`), and the mutation
//! tables: every prefix and single-byte deletion of the themes, the same for the first 4,096 bytes of
//! `en.json`, `en.json` cut at every 97th byte, and every fixture with a byte flipped at 97 places.
//! A mutation test passes by finishing: `parse` returns a `Result` for any bytes, so there is no
//! `catch_unwind`; the cases that must be errors say which.

use lune_ui::json::{JsonErrorKind as K, MAX_KEY, MAX_STRING, Value, parse};
use lune_ui::strings::{
    ArgFormat, Catalog, DisplayFormat, StringTable, TemplateAudit, audit_template, format_template,
};
use lune_ui::theme::{DARK, Skip, Theme, ThemeFile};
use std::fmt::Display;
use std::path::PathBuf;

const THEMES: [(&str, usize); 5] = [
    ("blue", 3760),
    ("dark-modern", 4535),
    ("green", 3761),
    ("orange", 3774),
    ("purple", 3774),
];

const LOCALES: [(&str, usize); 10] = [
    ("en", 67364),
    ("de", 75464),
    ("es", 74758),
    ("fr", 76537),
    ("ja", 84950),
    ("ko", 76858),
    ("ms", 71146),
    ("pt-BR", 73647),
    ("ru", 97801),
    ("zh-CN", 67166),
];

fn read(kind: &str, name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/lunegit")
        .join(kind)
        .join(format!("{name}.json"));
    match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => panic!("cannot read {}: {e}", path.display()),
    }
}

fn theme(name: &str) -> Vec<u8> {
    read("themes", name)
}

fn locale(name: &str) -> Vec<u8> {
    read("locale", name)
}

fn all_fixtures() -> Vec<(String, Vec<u8>)> {
    let themes = THEMES
        .iter()
        .map(|(n, _)| (format!("themes/{n}"), theme(n)));
    let locales = LOCALES
        .iter()
        .map(|(n, _)| (format!("locale/{n}"), locale(n)));
    themes.chain(locales).collect()
}

fn keys(v: &Value) -> Vec<&str> {
    v.as_object()
        .unwrap_or_default()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect()
}

// --- Known answers --------------------------------------------------------------------------

#[test]
fn the_fixtures_are_the_bytes_that_were_measured() {
    for (name, len) in THEMES {
        assert_eq!(theme(name).len(), len, "themes/{name}");
    }
    for (name, len) in LOCALES {
        assert_eq!(locale(name).len(), len, "locale/{name}");
    }
}

#[test]
fn every_theme_parses_to_an_object_of_five_keys() {
    for (name, _) in THEMES {
        let v = parse(&theme(name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(
            keys(&v),
            ["name", "author", "version", "colors", "acrylic"],
            "{name}"
        );
        assert!(v.get("name").and_then(Value::as_str).is_ok(), "{name}");
        assert!(v.get("version").and_then(Value::as_int).is_ok(), "{name}");
        let colors = v.get("colors").and_then(Value::string_map);
        assert!(
            colors
                .as_ref()
                .is_ok_and(|c| !c.is_empty() && c.len() <= 102),
            "{name}"
        );
        assert!(
            v.get("acrylic").and_then(Value::number_map).is_ok(),
            "{name}"
        );
    }
}

#[test]
fn en_json_yields_1019_strings() {
    let v = parse(&locale("en")).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(keys(&v), ["code", "displayName", "version", "strings"]);
    let strings = v.get("strings").and_then(Value::string_map);
    assert_eq!(strings.map(|s| s.len()), Ok(1019));
    assert_eq!(v.get("code").and_then(Value::as_str), Ok("en"));
}

#[test]
fn every_language_file_parses_to_the_locale_envelope() {
    for (name, _) in LOCALES {
        let v = parse(&locale(name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(
            keys(&v),
            ["code", "displayName", "version", "strings"],
            "{name}"
        );
        assert!(v.get("code").and_then(Value::as_str).is_ok(), "{name}");
        assert!(
            v.get("displayName").and_then(Value::as_str).is_ok(),
            "{name}"
        );
        assert!(v.get("version").and_then(Value::as_int).is_ok(), "{name}");
        assert!(
            v.get("strings")
                .and_then(Value::string_map)
                .is_ok_and(|s| !s.is_empty()),
            "{name}"
        );
    }
}

#[test]
fn no_real_file_comes_near_a_limit() {
    // The limits are proposals derived from these files; this keeps the headroom visible.
    for (name, bytes) in all_fixtures() {
        let v = parse(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        fn walk(v: &Value, longest_str: &mut usize, longest_key: &mut usize) {
            match v {
                Value::String(s) => *longest_str = (*longest_str).max(s.len()),
                Value::Array(a) => a.iter().for_each(|x| walk(x, longest_str, longest_key)),
                Value::Object(o) => {
                    for (k, x) in o {
                        *longest_key = (*longest_key).max(k.len());
                        walk(x, longest_str, longest_key);
                    }
                }
                _ => {}
            }
        }
        let (mut s, mut k) = (0, 0);
        walk(&v, &mut s, &mut k);
        assert!(s * 2 <= MAX_STRING, "{name}: longest string {s}");
        assert!(k * 2 <= MAX_KEY, "{name}: longest key {k}");
    }
}

// --- Themes ---------------------------------------------------------------------------------

fn load_theme(name: &str) -> ThemeFile {
    match Theme::from_json(&theme(name), &DARK) {
        Ok(f) => f,
        Err(e) => panic!("{name}: {e:?}"),
    }
}

#[test]
fn all_five_theme_files_load_and_skip_no_colour_and_no_opacity_as_bad() {
    for (name, _) in THEMES {
        let f = load_theme(name);
        let bad: Vec<&Skip> = f
            .skipped
            .iter()
            .filter(|s| matches!(s, Skip::BadColor(_) | Skip::BadAcrylic(_)))
            .collect();
        assert!(bad.is_empty(), "{name}: {bad:?}");
        assert!(!f.name.is_empty(), "{name}");
        assert_ne!(f.theme, DARK, "{name} changes the look");
        // About fifty colour keys have no field; the rest are known.
        let unknown = f
            .skipped
            .iter()
            .filter(|s| matches!(s, Skip::UnknownColor(_)))
            .count();
        assert!(
            (20..=70).contains(&unknown),
            "{name}: {unknown} unknown colour keys"
        );
    }
}

#[test]
fn the_entrys_known_answers() {
    // dark-modern.json: "ForegroundPrimary": "#FFCCCCCC".
    assert_eq!(
        load_theme("dark-modern").theme.fg_primary,
        [0xCC, 0xCC, 0xCC, 0xFF]
    );
    // blue.json: "AccentPrimaryColor": "#FF71B0F7".
    assert_eq!(load_theme("blue").theme.accent, [0x71, 0xB0, 0xF7, 0xFF]);
    // And its derived scrollbar thumb follows the accent.
    assert_eq!(
        load_theme("blue").theme.scroll_thumb,
        [0x71, 0xB0, 0xF7, 0x8C]
    );
    // ARGB to RGBA: a theme that says "#CCFFFFFF" gets white at alpha 0xCC.
    let f = Theme::from_json(
        br##"{"name":"t","colors":{"ForegroundPrimary":"#CCFFFFFF"},"acrylic":{}}"##,
        &DARK,
    );
    assert_eq!(f.map(|f| f.theme.fg_primary), Ok([0xFF, 0xFF, 0xFF, 0xCC]));
}

#[test]
fn orange_tint_is_rrggbb_so_it_is_opaque() {
    // orange.json: "GlassTintColor": "#1A0E08".
    let g = load_theme("orange").theme.glass;
    let tint = [0x1A, 0x0E, 0x08, 0xFF];
    assert_eq!(
        (g.window.tint, g.panel.tint, g.card.tint),
        (tint, tint, tint)
    );
}

#[test]
fn dark_modern_material_opacity_reaches_the_window_layer() {
    // dark-modern.json: "GlassMaterialDarkOpacity": 0.98.
    let g = load_theme("dark-modern").theme.glass;
    assert_eq!(g.window.material_opacity, 0.98);
    assert_eq!(g.window.tint_opacity, 1.0);
}

// --- Locale catalogs ------------------------------------------------------------------------

fn load_locale(name: &str) -> Catalog {
    match Catalog::from_json(&locale(name)) {
        Ok(c) => c,
        Err(e) => panic!("{name}: {e:?}"),
    }
}

#[test]
fn en_json_gives_1019_strings() {
    let c = load_locale("en");
    assert_eq!(
        (c.code.as_str(), c.display_name.as_str()),
        ("en", "English (US)")
    );
    assert_eq!(c.table().iter().count(), 1019);
    assert_eq!(c.table().get("common.cancel"), Some("Cancel"));
}

#[test]
fn the_nine_languages_load_with_1019_strings_each() {
    let en = load_locale("en");
    for (name, _) in LOCALES.iter().filter(|(n, _)| *n != "en") {
        let c = load_locale(name);
        assert_eq!(c.code, *name, "{name}");
        assert_eq!(c.table().iter().count(), 1019, "{name}");
        // The same keys as English.
        let same = c
            .table()
            .iter()
            .map(|(k, _)| k)
            .eq(en.table().iter().map(|(k, _)| k));
        assert!(same, "{name}: keys differ from en.json");
    }
}

/// `(keys with a placeholder, keys with only plain ones, keys with a spec, malformed)` over a table.
fn audit_table(t: &StringTable) -> (usize, usize, usize, usize) {
    let (mut any, mut plain, mut spec, mut malformed) = (0, 0, 0, 0);
    for (_, text) in t.iter() {
        let TemplateAudit {
            plain: p,
            spec: s,
            malformed: m,
        } = audit_template(text);
        any += usize::from(p + s > 0);
        plain += usize::from(p > 0 && s == 0);
        spec += usize::from(s > 0);
        malformed += m;
    }
    (any, plain, spec, malformed)
}

#[test]
fn the_placeholder_audit_over_en_json() {
    let c = load_locale("en");
    let (any, plain, spec, malformed) = audit_table(c.table());
    assert_eq!((any, plain, spec), (119, 111, 8));
    assert_eq!(malformed, 0, "a form format_template does not parse");
}

#[test]
fn no_language_file_has_a_malformed_template() {
    for (name, _) in LOCALES {
        let (_, _, _, malformed) = audit_table(load_locale(name).table());
        assert_eq!(malformed, 0, "{name}");
    }
}

#[test]
fn the_entrys_tr_fmt_known_answers_through_the_real_strings() {
    let table = load_locale("en").into_table();
    let args: [&dyn Display; 2] = [&"3", &"wip"];
    assert_eq!(
        table.tr_fmt("dialog.confirm.dropStash.message", &args, &DisplayFormat),
        "Drop stash@{3} 'wip'? This cannot be undone via the UI."
    );
    // `{0:N0}`: the default hook ignores the spec; an app hook gets it unchanged.
    let key = "toast.git.fsmonitorPromptDetail";
    let plain = table.tr_fmt(key, &[&"1234"], &DisplayFormat);
    assert!(
        plain.starts_with("This repo has 1234 tracked files."),
        "{plain}"
    );
    struct Group;
    impl ArgFormat for Group {
        fn format(&self, _i: usize, arg: &dyn Display, spec: &str, out: &mut String) {
            assert_eq!(spec, "N0", "the spec text reaches the hook unchanged");
            let digits = arg.to_string();
            for (n, c) in digits.chars().enumerate() {
                if n > 0 && (digits.len() - n) % 3 == 0 {
                    out.push(',');
                }
                out.push(c);
            }
        }
    }
    let grouped = format_template(table.tr(key), &[&"1234"], &Group);
    assert!(
        grouped.starts_with("This repo has 1,234 tracked files."),
        "{grouped}"
    );
}

#[test]
fn a_language_overlaid_on_english_falls_back_for_a_key_it_lacks() {
    let mut base = load_locale("en").into_table();
    let mut de = StringTable::default();
    assert!(de.insert("common.cancel", "Abbrechen").is_ok());
    base.overlay(&de);
    assert_eq!(base.tr("common.cancel"), "Abbrechen");
    assert_eq!(base.tr("common.reload"), "Reload");
}

// --- Negative fixtures made from the real files ---------------------------------------------

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap_or_default()
}

fn kind_of(bytes: &[u8]) -> K {
    match parse(bytes) {
        Ok(_) => panic!("expected an error"),
        Err(e) => e.kind,
    }
}

#[test]
fn a_theme_nested_past_the_depth_limit_is_too_deep() {
    let t = text(&theme("dark-modern"));
    // The theme is 2 deep; 14 arrays around it make 16 and 15 make 17.
    let around = |n: usize| format!("{}{t}{}", "[".repeat(n), "]".repeat(n));
    assert!(parse(around(14).as_bytes()).is_ok());
    assert_eq!(kind_of(around(15).as_bytes()), K::TooDeep);
}

#[test]
fn a_theme_with_a_4097_byte_string_is_refused() {
    let t = text(&theme("blue"));
    let long = format!("\"name\": \"{}\"", "x".repeat(MAX_STRING + 1));
    let bad = t.replacen("\"name\": \"Blue\"", &long, 1);
    assert!(bad != t, "the replacement landed");
    assert_eq!(kind_of(bad.as_bytes()), K::StringTooLong);
    let ok = t.replacen(
        "\"name\": \"Blue\"",
        &format!("\"name\": \"{}\"", "x".repeat(MAX_STRING)),
        1,
    );
    assert!(parse(ok.as_bytes()).is_ok());
}

#[test]
fn a_theme_with_an_invalid_utf8_byte_is_refused() {
    let mut b = theme("green");
    b.insert(40, 0xFF);
    let e = parse(&b).err().map(|e| (e.kind, e.offset));
    assert_eq!(e, Some((K::InvalidUtf8, 40)));
}

#[test]
fn a_theme_with_a_trailing_comma_is_refused() {
    let t = text(&theme("orange"));
    let at = t.rfind('}').unwrap_or(0);
    let (head, tail) = t.split_at(at);
    let bad = format!("{},{tail}", head.trim_end());
    assert!(
        matches!(kind_of(bad.as_bytes()), K::Syntax(_)),
        "inner object closes before the last brace"
    );
}

#[test]
fn a_theme_with_a_lone_surrogate_is_refused() {
    let t = text(&theme("purple"));
    let bad = t.replacen("\"Purple\"", "\"\\ud800\"", 1);
    assert!(bad != t, "the replacement landed");
    assert_eq!(kind_of(bad.as_bytes()), K::InvalidEscape);
}

#[test]
fn a_theme_with_a_400_digit_number_is_refused() {
    let t = text(&theme("dark-modern"));
    let bad = t.replacen(
        "\"version\": 2",
        &format!("\"version\": {}", "7".repeat(400)),
        1,
    );
    assert!(bad != t, "the replacement landed");
    assert_eq!(kind_of(bad.as_bytes()), K::NumberOutOfRange);
}

#[test]
fn a_theme_with_a_duplicated_key_is_refused() {
    let t = text(&theme("blue"));
    let bad = t.replacen("\"author\"", "\"name\": \"again\",\n  \"author\"", 1);
    assert!(bad != t, "the replacement landed");
    assert_eq!(kind_of(bad.as_bytes()), K::DuplicateKey("name".to_string()));
}

// --- Mutation tables ------------------------------------------------------------------------

/// Runs `parse` on every prefix of `bytes`; the result of a truncated file is a `Result`, and the
/// whole file (and the file without its final newline) are the only ones that are `Ok`.
fn every_prefix(bytes: &[u8]) -> usize {
    (0..=bytes.len())
        .filter(|&n| parse(&bytes[..n]).is_ok())
        .count()
}

fn every_deletion(bytes: &[u8]) -> usize {
    let mut ok = 0;
    let mut cut = Vec::with_capacity(bytes.len());
    for i in 0..bytes.len() {
        cut.clear();
        cut.extend_from_slice(&bytes[..i]);
        cut.extend_from_slice(&bytes[i + 1..]);
        ok += usize::from(parse(&cut).is_ok());
    }
    ok
}

#[test]
fn every_prefix_of_every_theme_returns_a_result() {
    for (name, _) in THEMES {
        let b = theme(name);
        // Only the whole file and the file without its final newline are documents (a prefix of an
        // object never closes it).
        assert_eq!(every_prefix(&b), 2, "{name}");
    }
}

#[test]
fn every_single_byte_deletion_of_every_theme_returns_a_result() {
    for (name, _) in THEMES {
        let b = theme(name);
        // Deleting a whitespace byte leaves a valid theme; most others do not. Only finishing counts.
        let ok = every_deletion(&b);
        assert!(
            ok < b.len(),
            "{name}: {ok} of {} deletions still parse",
            b.len()
        );
    }
}

#[test]
fn the_first_4096_bytes_of_en_json_survive_every_prefix_and_deletion() {
    let b = locale("en");
    let head = &b[..4096];
    assert_eq!(every_prefix(head), 0, "a cut object never parses");
    assert_eq!(
        every_deletion(head),
        0,
        "the head has no closing brace, so no deletion parses"
    );
}

#[test]
fn en_json_cut_at_every_97th_byte_returns_an_error() {
    let b = locale("en");
    let mut cuts = 0;
    for n in (0..b.len()).step_by(97) {
        assert!(parse(&b[..n]).is_err(), "cut at {n}");
        cuts += 1;
    }
    assert!(cuts > 600, "{cuts}");
}

#[test]
fn every_fixture_with_a_byte_flipped_at_97_positions_returns_a_result() {
    for (name, bytes) in all_fixtures() {
        for mask in [0xFFu8, 0x01, 0x20] {
            for k in 0..97 {
                let at = k * bytes.len() / 97;
                let mut b = bytes.clone();
                b[at] ^= mask;
                let _ = parse(&b);
            }
        }
        // A flip to a UTF-8 lead byte, a quote and a backslash at the same positions.
        for repl in [0xC3u8, b'"', b'\\', b'{', b']'] {
            for k in 0..97 {
                let at = k * bytes.len() / 97;
                let mut b = bytes.clone();
                b[at] = repl;
                let _ = parse(&b);
            }
        }
        assert!(parse(&bytes).is_ok(), "{name}: the original still parses");
    }
}
