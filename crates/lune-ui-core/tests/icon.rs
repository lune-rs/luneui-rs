//! The 29 icon constants against the Codicons table.
//!
//! Source of the expected values: `@vscode/codicons@0.0.45`, `dist/codicon.csv` (535 rows,
//! `short_name,character,unicode`), read once at M11 and written below as literals; the csv is not
//! bundled. Each row is the constant, its name in that file and its code point. `GEAR` is
//! `settings-gear` there, the one constant whose Codicons name is not its own name with dashes.

use lune_ui_core::icon;

/// (constant, its name here, its Codicons name, its code point)
const TABLE: [(&str, &str, &str, char); 29] = [
    (icon::ADD, "ADD", "add", '\u{ea60}'),
    (icon::REMOVE, "REMOVE", "remove", '\u{eb3b}'),
    (icon::GEAR, "GEAR", "settings-gear", '\u{eb51}'),
    (icon::ELLIPSIS, "ELLIPSIS", "ellipsis", '\u{ea7c}'),
    (
        icon::CHROME_CLOSE,
        "CHROME_CLOSE",
        "chrome-close",
        '\u{eab8}',
    ),
    (icon::EDIT, "EDIT", "edit", '\u{ea73}'),
    (icon::COPY, "COPY", "copy", '\u{ebcc}'),
    (icon::INFO, "INFO", "info", '\u{ea74}'),
    (
        icon::CHEVRON_DOWN,
        "CHEVRON_DOWN",
        "chevron-down",
        '\u{eab4}',
    ),
    (icon::ARROW_LEFT, "ARROW_LEFT", "arrow-left", '\u{ea9b}'),
    (icon::CHECK, "CHECK", "check", '\u{eab2}'),
    (
        icon::CHROME_MINIMIZE,
        "CHROME_MINIMIZE",
        "chrome-minimize",
        '\u{eaba}',
    ),
    (
        icon::CHROME_MAXIMIZE,
        "CHROME_MAXIMIZE",
        "chrome-maximize",
        '\u{eab9}',
    ),
    (
        icon::CHROME_RESTORE,
        "CHROME_RESTORE",
        "chrome-restore",
        '\u{eabb}',
    ),
    (icon::SEARCH, "SEARCH", "search", '\u{ea6d}'),
    (icon::CALENDAR, "CALENDAR", "calendar", '\u{eab0}'),
    (icon::SYNC, "SYNC", "sync", '\u{ea77}'),
    (icon::STAR_FULL, "STAR_FULL", "star-full", '\u{eb59}'),
    (icon::STAR_EMPTY, "STAR_EMPTY", "star-empty", '\u{ea6a}'),
    (icon::CLOSE, "CLOSE", "close", '\u{ea76}'),
    (
        icon::CHEVRON_RIGHT,
        "CHEVRON_RIGHT",
        "chevron-right",
        '\u{eab6}',
    ),
    (icon::WARNING, "WARNING", "warning", '\u{ea6c}'),
    (icon::ERROR, "ERROR", "error", '\u{ea87}'),
    (icon::CLEAR_ALL, "CLEAR_ALL", "clear-all", '\u{eabf}'),
    (
        icon::CIRCLE_SLASH,
        "CIRCLE_SLASH",
        "circle-slash",
        '\u{eabd}',
    ),
    (icon::ARROW_UP, "ARROW_UP", "arrow-up", '\u{eaa1}'),
    (icon::ARROW_DOWN, "ARROW_DOWN", "arrow-down", '\u{ea9a}'),
    (icon::ACCOUNT, "ACCOUNT", "account", '\u{eb99}'),
    (icon::NOTE, "NOTE", "note", '\u{eb26}'),
];

#[test]
fn every_constant_is_its_codicons_code_point() {
    for (value, name, codicon, cp) in TABLE {
        assert_eq!(
            value,
            cp.to_string(),
            "{name} should be codicon `{codicon}` U+{:04X}",
            u32::from(cp)
        );
    }
}

#[test]
fn the_table_covers_distinct_glyphs_in_the_private_use_block() {
    let mut seen = std::collections::HashSet::new();
    for (_, name, _, cp) in TABLE {
        assert!(seen.insert(cp), "{name} repeats U+{:04X}", u32::from(cp));
        assert!(
            ('\u{ea60}'..='\u{ebff}').contains(&cp),
            "{name} U+{:04X} is outside the Codicons block",
            u32::from(cp)
        );
    }
    assert_eq!(seen.len(), 29);
}

#[test]
fn gear_is_settings_gear() {
    let row = TABLE.iter().find(|r| r.1 == "GEAR").expect("GEAR row");
    assert_eq!(row.2, "settings-gear");
    assert_eq!(icon::GEAR, "\u{eb51}");
}

#[test]
fn a_glyph_spec_uses_the_mono_role_at_its_size() {
    use lune_ui_core::text::{Spec, Style};
    let spec = icon::glyph(icon::ADD, 14.0, [1, 2, 3, 255]);
    assert_eq!(
        spec,
        Spec::plain(icon::ADD, Style::mono(14.0, 17.5, [1, 2, 3, 255]))
    );
}
