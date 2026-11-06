//! The string table: lookup, insert rules, placeholder formatting and the defaults.

use std::fmt::Display;
use std::path::Path;

use lune_ui_core::strings::{
    ArgFormat, DEFAULTS, DisplayFormat, StringError, StringTable, default_text, format_template,
};

fn fmt(template: &str, args: &[&dyn Display]) -> String {
    format_template(template, args, &DisplayFormat)
}

/// Groups the integer args' digits in threes and records the spec it was handed.
struct Grouping {
    seen: std::cell::RefCell<Vec<String>>,
}

impl ArgFormat for Grouping {
    fn format(&self, _index: usize, arg: &dyn Display, spec: &str, out: &mut String) {
        self.seen.borrow_mut().push(spec.to_owned());
        let text = arg.to_string();
        let n = text.len();
        for (i, c) in text.chars().enumerate() {
            if i > 0 && (n - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
    }
}

#[test]
fn real_world_string_with_escaped_braces() {
    let t = "Drop stash@{{{0}}} '{1}'? This cannot be undone via the UI.";
    assert_eq!(
        fmt(t, &[&"3", &"wip"]),
        "Drop stash@{3} 'wip'? This cannot be undone via the UI."
    );
}

#[test]
fn missing_extra_and_escapes() {
    assert_eq!(fmt("{0} and {1}", &[&"x"]), "x and {1}");
    assert_eq!(fmt("{0}", &[&"a", &"b"]), "a");
    assert_eq!(fmt("{{}}", &[]), "{}");
    assert_eq!(fmt("a{", &[]), "a{");
    assert_eq!(fmt("a}b", &[]), "a}b");
    assert_eq!(fmt("{x}{-1}{ 0}{", &[&"a"]), "{x}{-1}{ 0}{");
    assert_eq!(fmt("{0:N0", &[&"a"]), "{0:N0");
    assert_eq!(
        fmt("{99999999999999999999999}", &[&"a"]),
        "{99999999999999999999999}"
    );
    assert_eq!(fmt("{1:N0}", &[&"a"]), "{1:N0}");
    assert_eq!(fmt("é{0}é", &[&1]), "é1é");
}

#[test]
fn spec_goes_to_the_hook() {
    assert_eq!(fmt("{0:N0}", &[&"1234"]), "1234");
    let g = Grouping {
        seen: Default::default(),
    };
    assert_eq!(
        format_template("{0:N0} / {0}", &[&"1234"], &g),
        "1,234 / 1,234"
    );
    assert_eq!(*g.seen.borrow(), vec!["N0".to_owned(), String::new()]);
}

#[test]
fn totality_short_strings() {
    let alphabet = ['{', '}', '0', '1', ':', 'a', 'é'];
    let mut count = 0usize;
    for len in 0..=6u32 {
        let total = alphabet.len().pow(len);
        for mut n in 0..total {
            let mut s = String::new();
            for _ in 0..len {
                s.push(alphabet[n % alphabet.len()]);
                n /= alphabet.len();
            }
            let _ = fmt(&s, &[&"x"]);
            count += 1;
        }
    }
    assert!(count > 100_000);
}

#[test]
fn table_lookup_and_fallbacks() {
    let mut t = StringTable::default();
    assert_eq!(t.tr("luneui.dialog.cancel"), "Cancel");
    assert_eq!(t.tr("app.hello"), "app.hello");
    assert_eq!(t.get("app.hello"), None);
    t.insert("app.hello", "Hi {0}").expect("test setup");
    t.insert("luneui.dialog.cancel", "Batal")
        .expect("test setup");
    assert_eq!(t.tr("app.hello"), "Hi {0}");
    assert_eq!(t.tr("luneui.dialog.cancel"), "Batal");
    assert_eq!(t.tr_fmt("app.hello", &[&"Ada"], &DisplayFormat), "Hi Ada");
    assert_eq!(default_text("luneui.dialog.cancel"), Some("Cancel"));
    assert_eq!(default_text("app.hello"), None);
}

#[test]
fn insert_rules() {
    let mut t = StringTable::default();
    assert_eq!(t.insert("", "x"), Err(StringError::EmptyKey));
    assert_eq!(
        t.insert("luneui.nope.nothing", "x"),
        Err(StringError::UnknownToolkitKey(
            "luneui.nope.nothing".to_owned()
        ))
    );
    assert!(t.insert("luneui.", "x").is_err());
    assert!(t.insert("luneuix.a", "x").is_ok());
    assert!(t.insert("luneui", "x").is_ok());
    assert!(t.get("luneui.nope.nothing").is_none());
}

#[test]
fn never_panics_on_odd_keys() {
    let t = StringTable::default();
    assert_eq!(t.tr(""), "");
    let long = "k".repeat(10_000);
    assert_eq!(t.tr(&long), long);
    assert_eq!(t.tr_fmt(&long, &[], &DisplayFormat), long);
}

fn is_key(key: &str) -> bool {
    let mut parts = key.split('.');
    parts.next() == Some("luneui") && {
        let rest: Vec<&str> = parts.collect();
        rest.len() >= 2
            && rest.iter().all(|p| {
                !p.is_empty()
                    && p.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            })
    }
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).expect("test setup") {
        let p = e.expect("test setup").path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn defaults_cover_every_key_literal_in_src() {
    for (k, v) in DEFAULTS {
        assert!(is_key(k), "default key `{k}` is malformed");
        assert!(!v.is_empty(), "default for `{k}` is empty");
    }
    assert!(is_key("luneui.dialog.cancel"));
    assert!(!is_key("luneui.a") && !is_key("luneui.A.b") && !is_key("luneui..b"));

    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(files.len() > 10, "scan found only {} files", files.len());
    let needle = "\"luneui.";
    let mut literals = 0;
    for f in files {
        let text = std::fs::read_to_string(&f).expect("test setup");
        let mut rest = text.as_str();
        while let Some(at) = rest.find(needle) {
            let tail = &rest[at + 1..];
            let end = tail.find('"').expect("unterminated literal");
            let key = &tail[..end];
            literals += 1;
            assert!(
                is_key(key),
                "{}: `{key}` is not a well-formed key",
                f.display()
            );
            assert!(
                default_text(key).is_some_and(|v| !v.is_empty()),
                "{}: `{key}` has no default",
                f.display()
            );
            rest = &tail[end + 1..];
        }
    }
    assert!(
        literals >= 1,
        "the scan found no key literal (instrument check)"
    );
}

/// The keys M7's widgets draw, with their English text.
#[test]
fn the_m7_defaults_hold_their_text() {
    let expected = [
        ("luneui.menu.name", "Menu"),
        ("luneui.toast-notifications.clear-all", "Clear all"),
        ("luneui.toast-notifications.history-header", "NOTIFICATIONS"),
        ("luneui.toast-notifications.dismiss", "Dismiss"),
        ("luneui.command-palette.show-more", "Show more results"),
        (
            "luneui.command-palette.footer-right",
            "Enter open    Esc close",
        ),
        (
            "luneui.command-palette.destructive-tooltip",
            "Destructive action \u{2014} requires confirmation",
        ),
        (
            "luneui.settings-page.restart-message",
            "Some changes require a restart to take effect.",
        ),
        ("luneui.settings-page.restart-action", "Restart now"),
    ];
    for (key, text) in expected {
        assert_eq!(default_text(key), Some(text), "{key}");
    }
}

/// D27: a table switched between two frames changes the text drawn on the second frame and
/// nothing else: the same fills, the same one text item, a different key.
#[test]
fn switching_the_table_between_frames_changes_the_shaped_text_and_nothing_else() {
    use lune_ui_core::paint::Rect;
    use lune_ui_core::testing::UiHarness;
    use lune_ui_core::widgets::button::ButtonKind;
    let mut h = UiHarness::new((200.0, 100.0));
    let draw = |h: &mut UiHarness| {
        let (scene, _) = h.frame(|ui| {
            let label = ui.tr("luneui.dialog.cancel").to_owned();
            ui.button(
                "b",
                Rect::new(10.0, 10.0, 120.0, 32.0),
                &label,
                ButtonKind::Secondary,
            );
        });
        scene
    };
    let english = draw(&mut h);
    h.strings
        .insert("luneui.dialog.cancel", "Abbrechen")
        .expect("a default key");
    h.at_ms(1000);
    let german = draw(&mut h);
    let (a, b) = (&english.layers[0], &german.layers[0]);
    assert_eq!((a.texts.len(), b.texts.len()), (1, 1));
    assert_ne!(a.texts[0].key, b.texts[0].key, "the text changed");
    assert_eq!(a.geometry, b.geometry, "the shapes did not");
}
