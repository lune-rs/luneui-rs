//! Language files (`Catalog`), the string-table overlay and the template scanner behind
//! `format_template` and `audit_template`. The known answers on the real files are in the gallery's
//! file tests.

use std::fmt::Display;

use lune_ui_core::json::{JsonErrorKind, MAX_INPUT};
use lune_ui_core::strings::{
    ArgFormat, Catalog, CatalogError, DEFAULTS, DisplayFormat, StringError, StringTable,
    TemplateAudit, audit_template, format_template,
};

fn load(json: &str) -> Catalog {
    match Catalog::from_json(json.as_bytes()) {
        Ok(c) => c,
        Err(e) => panic!("expected a catalog for {json:?}, got {e:?}"),
    }
}

fn error(json: &str) -> CatalogError {
    match Catalog::from_json(json.as_bytes()) {
        Ok(c) => panic!("expected an error for {json:?}, got {:?}", c.code),
        Err(e) => e,
    }
}

fn file(strings: &str) -> String {
    format!(r#"{{"code":"xx","displayName":"Test","version":3,"strings":{{{strings}}}}}"#)
}

fn fmt(template: &str, args: &[&dyn Display]) -> String {
    format_template(template, args, &DisplayFormat)
}

fn audit(template: &str) -> (usize, usize, usize) {
    let a = audit_template(template);
    (a.plain, a.spec, a.malformed)
}

// --- Catalog --------------------------------------------------------------------------------

#[test]
fn a_catalog_carries_its_header_and_its_strings() {
    let c = load(
        r#"{"code":"de","displayName":"Deutsch","version":57,"strings":{"app.hello":"Hallo"}}"#,
    );
    assert_eq!(
        (c.code.as_str(), c.display_name.as_str(), c.version),
        ("de", "Deutsch", 57)
    );
    assert_eq!(c.table().get("app.hello"), Some("Hallo"));
    assert_eq!(
        c.table().iter().collect::<Vec<_>>(),
        [("app.hello", "Hallo")]
    );
    let table = c.into_table();
    assert_eq!(table.tr("app.hello"), "Hallo");
}

#[test]
fn version_is_optional_and_unknown_top_level_keys_are_ignored() {
    let c = load(r#"{"code":"x","displayName":"X","extra":[1,2],"strings":{}}"#);
    assert_eq!(c.version, 0);
    assert_eq!(c.table().iter().count(), 0);
}

#[test]
fn each_entry_m10_error_is_an_error() {
    // Non-object `strings`.
    assert_eq!(
        error(r#"{"code":"x","displayName":"X","strings":[]}"#),
        CatalogError::WrongType("strings")
    );
    assert_eq!(
        error(r#"{"code":"x","displayName":"X","strings":"text"}"#),
        CatalogError::WrongType("strings")
    );
    assert_eq!(
        error(r#"{"code":"x","displayName":"X","strings":null}"#),
        CatalogError::WrongType("strings")
    );
    // A string map holds strings only.
    assert_eq!(
        error(&file(r#""a.b":1"#)),
        CatalogError::WrongType("strings")
    );
    // A missing `code`, and the other required fields.
    assert_eq!(
        error(r#"{"displayName":"X","strings":{}}"#),
        CatalogError::Missing("code")
    );
    assert_eq!(
        error(r#"{"code":"x","strings":{}}"#),
        CatalogError::Missing("displayName")
    );
    assert_eq!(
        error(r#"{"code":"x","displayName":"X"}"#),
        CatalogError::Missing("strings")
    );
    // Wrong types and blanks.
    assert_eq!(
        error(r#"{"code":1,"displayName":"X","strings":{}}"#),
        CatalogError::WrongType("code")
    );
    assert_eq!(
        error(r#"{"code":"","displayName":"X","strings":{}}"#),
        CatalogError::Empty("code")
    );
    assert_eq!(
        error(r#"{"code":"x","displayName":" \t","strings":{}}"#),
        CatalogError::Empty("displayName")
    );
    assert_eq!(
        error(r#"{"code":"x","displayName":"X","version":"1","strings":{}}"#),
        CatalogError::WrongType("version")
    );
    assert_eq!(
        error(r#"{"code":"x","displayName":"X","version":1.5,"strings":{}}"#),
        CatalogError::WrongType("version")
    );
    assert_eq!(error("[]"), CatalogError::NotAnObject);
    assert_eq!(error("3"), CatalogError::NotAnObject);
}

#[test]
fn a_depth_bomb_an_oversize_input_and_invalid_utf8_are_the_readers_errors() {
    let bomb = format!(
        r#"{{"code":"x","displayName":"X","strings":{}{}}}"#,
        "[".repeat(5000),
        "]".repeat(5000)
    );
    assert!(matches!(
        Catalog::from_json(bomb.as_bytes()),
        Err(CatalogError::Json(e)) if e.kind == JsonErrorKind::TooDeep
    ));
    let big = vec![b' '; MAX_INPUT + 1];
    assert!(matches!(
        Catalog::from_json(&big),
        Err(CatalogError::Json(e)) if e.kind == JsonErrorKind::TooLarge
    ));
    assert!(matches!(
        Catalog::from_json(b"{\"code\":\"x\xff\"}"),
        Err(CatalogError::Json(e)) if e.kind == JsonErrorKind::InvalidUtf8
    ));
    assert!(matches!(
        Catalog::from_json(b""),
        Err(CatalogError::Json(_))
    ));
    assert!(matches!(
        Catalog::from_json(br#"{"code":"x","displayName":"X","strings":{"a":"1","a":"2"}}"#),
        Err(CatalogError::Json(e)) if e.kind == JsonErrorKind::DuplicateKey("a".to_string())
    ));
}

#[test]
fn a_toolkit_key_that_is_a_default_loads_and_an_unknown_one_fails_the_whole_file() {
    let (key, english) = DEFAULTS[0];
    let c = load(&file(&format!(r#""{key}":"Abbrechen","app.other":"x""#)));
    assert_eq!(c.table().tr(key), "Abbrechen");
    assert_eq!(english, "Cancel");
    // Unknown `luneui.` key: the load fails, nothing is kept.
    assert_eq!(
        error(&file(r#""app.ok":"fine","luneui.no-such-key":"x""#)),
        CatalogError::Key(StringError::UnknownToolkitKey(
            "luneui.no-such-key".to_string()
        ))
    );
    // `luneui` itself without a dot is not in the namespace.
    assert!(Catalog::from_json(file(r#""luneui":"x","luneuix.a":"y""#).as_bytes()).is_ok());
    // An empty key.
    assert_eq!(
        error(&file(r#""":"x""#)),
        CatalogError::Key(StringError::EmptyKey)
    );
}

#[test]
fn a_key_a_catalog_omits_falls_back_through_tr_to_english() {
    let c = load(&file(r#""app.only-here":"Hier""#));
    // A toolkit key the catalog leaves out: the English default.
    assert_eq!(c.table().tr("luneui.dialog.cancel"), "Cancel");
    // An app key it leaves out: the key, until the app's base table supplies it.
    assert_eq!(c.table().tr("app.base-only"), "app.base-only");
    let mut base = StringTable::default();
    assert!(base.insert("app.base-only", "Base").is_ok());
    assert!(base.insert("app.only-here", "Here").is_ok());
    base.overlay(c.table());
    assert_eq!(
        base.tr("app.base-only"),
        "Base",
        "the omitted key keeps the base text"
    );
    assert_eq!(base.tr("app.only-here"), "Hier", "the catalog's key wins");
}

#[test]
fn overlay_lets_the_other_tables_keys_win_and_leaves_it_unchanged() {
    let mut a = StringTable::default();
    let mut b = StringTable::default();
    for (k, v) in [("one", "a1"), ("two", "a2")] {
        assert!(a.insert(k, v).is_ok());
    }
    for (k, v) in [("two", "b2"), ("three", "b3")] {
        assert!(b.insert(k, v).is_ok());
    }
    a.overlay(&b);
    assert_eq!(
        a.iter().collect::<Vec<_>>(),
        [("one", "a1"), ("three", "b3"), ("two", "b2")]
    );
    assert_eq!(b.iter().count(), 2, "the argument is not changed");
    // Overlaying in the other order gives the other winner.
    let mut c = StringTable::default();
    assert!(c.insert("two", "c2").is_ok());
    c.overlay(&a);
    c.overlay(&c.clone());
    assert_eq!(c.get("two"), Some("b2"));
    let (mut x, y) = (StringTable::default(), StringTable::default());
    x.overlay(&y);
    assert_eq!(x.iter().count(), 0);
}

// --- Template scanner -----------------------------------------------------------------------

#[test]
fn format_known_answers_are_unchanged() {
    assert_eq!(
        fmt(
            "Drop stash@{{{0}}} '{1}'? This cannot be undone via the UI.",
            &[&"3", &"wip"]
        ),
        "Drop stash@{3} 'wip'? This cannot be undone via the UI."
    );
    assert_eq!(fmt("{0} and {1}", &[&"x"]), "x and {1}");
    assert_eq!(fmt("{0}", &[&"a", &"b", &"c"]), "a");
    assert_eq!(fmt("{{ and }}", &[]), "{ and }");
    assert_eq!(fmt("a { b", &[&"x"]), "a { b");
    assert_eq!(fmt("{0:N0}", &[&"1234"]), "1234");
    assert_eq!(fmt("{0}{0}{0}", &[&"ab"]), "ababab");
}

#[test]
fn the_spec_reaches_the_hook_unchanged() {
    struct Spy;
    impl ArgFormat for Spy {
        fn format(&self, index: usize, arg: &dyn Display, spec: &str, out: &mut String) {
            out.push_str(&format!("[{index}|{arg}|{spec}]"));
        }
    }
    let r = format_template("{0:d MMM yyyy} {1} {0:}", &[&"x", &"y"], &Spy);
    assert_eq!(r, "[0|x|d MMM yyyy] [1|y|] [0|x|]");
}

#[test]
fn hostile_templates_return_and_keep_what_they_cannot_read() {
    let big: &dyn Display = &"A";
    // An index past usize is not a placeholder: kept literally.
    assert_eq!(
        fmt("{99999999999999999999}", &[big]),
        "{99999999999999999999}"
    );
    assert_eq!(
        fmt("{99999999999999999999:x}", &[big]),
        "{99999999999999999999:x}"
    );
    assert_eq!(
        fmt("{18446744073709551615}", &[big]),
        "{18446744073709551615}"
    );
    assert_eq!(
        fmt("{18446744073709551616}", &[big]),
        "{18446744073709551616}"
    );
    assert_eq!(fmt("{0:", &[big]), "{0:");
    assert_eq!(fmt("{0:abc", &[big]), "{0:abc");
    assert_eq!(fmt("{", &[big]), "{");
    assert_eq!(fmt("}", &[big]), "}");
    assert_eq!(fmt("{{{", &[big]), "{{");
    assert_eq!(fmt("}}}", &[big]), "}}");
    assert_eq!(
        fmt("{0:{}}", &[big]),
        "A}",
        "the spec runs to the first closing brace"
    );
    assert_eq!(fmt("{0 }", &[big]), "{0 }");
    assert_eq!(fmt("{-1}", &[big]), "{-1}");
    assert_eq!(fmt("{ 0}", &[big]), "{ 0}");
    assert_eq!(
        fmt("{\u{661}}", &[big]),
        "{\u{661}}",
        "an Arabic-Indic digit is no digit"
    );
    assert_eq!(
        fmt("{\u{e9}0}", &[big]),
        "{\u{e9}0}",
        "a multi-byte character right after the brace"
    );
    assert_eq!(fmt("{\u{1f600}}", &[big]), "{\u{1f600}}");
    assert_eq!(fmt("\u{1f600}{0}\u{1f600}", &[big]), "\u{1f600}A\u{1f600}");
    assert_eq!(fmt("", &[big]), "");
    assert_eq!(fmt("{1}", &[]), "{1}");
}

#[test]
fn output_is_bounded_by_the_template_and_the_arguments() {
    // 4,096 placeholders of one 4,096-byte argument: exactly 4,096 copies, nothing more.
    let arg = "x".repeat(4096);
    let template = "{0}".repeat(4096);
    let out = fmt(&template, &[&arg.as_str()]);
    assert_eq!(out.len(), 4096 * 4096);
    // Escapes shrink: 100 `{{`, then 100 `}` (50 pairs), then 100 `{` (50 pairs).
    let t = "{{".repeat(100) + &"}".repeat(100) + &"{".repeat(100);
    assert_eq!(fmt(&t, &[]).len(), 100 + 50 + 50);
    // A template of unmatched `{0:` finishes (linear, not quadratic): 4,096 of them.
    let t = "{0:".repeat(4096);
    assert_eq!(fmt(&t, &[&"a"]), t);
    // And a long run of digits after a brace, repeated.
    let t = format!("{{{}", "9".repeat(4000)).repeat(4);
    assert_eq!(fmt(&t, &[&"a"]), t);
}

#[test]
fn the_audit_counts_what_format_reads() {
    assert_eq!(audit("no placeholders"), (0, 0, 0));
    assert_eq!(audit("{0} and {1}"), (2, 0, 0));
    assert_eq!(audit("{0:N0} of {1}"), (1, 1, 0));
    assert_eq!(audit("{0:}"), (0, 1, 0), "an empty spec is still a spec");
    assert_eq!(
        audit("stash@{{{0}}} '{1}'"),
        (2, 0, 0),
        "escapes are not placeholders"
    );
    assert_eq!(audit("{{ }}"), (0, 0, 0));
    // Malformed: an unmatched `{`, an unmatched `}`, a non-digit index, an overflowing index.
    assert_eq!(audit("{"), (0, 0, 1));
    assert_eq!(audit("}"), (0, 0, 1));
    assert_eq!(audit("{a}"), (0, 0, 2), "a stray open and a stray close");
    assert_eq!(audit("{99999999999999999999}"), (0, 0, 2));
    assert_eq!(audit("{0:abc"), (0, 0, 1));
    assert_eq!(audit("{{{"), (0, 0, 1));
    assert_eq!(audit("}}}"), (0, 0, 1));
    assert_eq!(audit("{0:{}}"), (0, 1, 1));
    assert_eq!(audit(""), (0, 0, 0));
    assert_eq!(
        audit_template("{0}"),
        TemplateAudit {
            plain: 1,
            spec: 0,
            malformed: 0
        }
    );
}

#[test]
fn the_audit_and_format_agree_on_every_short_template() {
    // Over every string of 6 characters from a brace-heavy alphabet: a template with no malformed
    // piece and N placeholders, formatted with an argument for every index, shows an argument; and
    // formatting never fails for the rest.
    let alphabet = ['{', '}', '0', '1', ':', 'x'];
    // Indices up to 1111 can occur in a six-character template; all have an argument.
    let args: Vec<&dyn Display> = (0..1200).map(|_| &"<a>" as &dyn Display).collect();
    let mut count = 0u32;
    let mut stack = vec![String::new()];
    while let Some(t) = stack.pop() {
        let a = audit_template(&t);
        let out = format_template(&t, &args, &DisplayFormat);
        if a.malformed == 0 && a.plain + a.spec > 0 {
            assert!(out.contains('<'), "{t:?} -> {out:?}");
        }
        if a.plain + a.spec == 0 {
            assert!(!out.contains('<'), "{t:?} -> {out:?}");
        }
        count += 1;
        if t.chars().count() < 6 {
            for c in alphabet {
                stack.push(format!("{t}{c}"));
            }
        }
    }
    assert_eq!(count, (0..=6).map(|n| 6u32.pow(n)).sum::<u32>());
}
