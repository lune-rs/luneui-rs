//! The bounded JSON reader: every limit with the exact boundary on both sides, every fail-closed
//! error with its offset, the typed readers, and a bounded-work walk. The known answers on the real
//! theme and locale files are in the gallery's file tests.

use lune_ui_core::json::{
    JsonError, JsonErrorKind as K, MAX_DEPTH, MAX_INPUT, MAX_KEY, MAX_MEMBERS, MAX_STRING,
    MAX_VALUES, Value, parse,
};

fn ok(s: &str) -> Value {
    match parse(s.as_bytes()) {
        Ok(v) => v,
        Err(e) => panic!("expected a value for {s:?}, got {e:?}"),
    }
}

fn err(bytes: &[u8]) -> JsonError {
    match parse(bytes) {
        Ok(v) => panic!("expected an error for {bytes:?}, got {v:?}"),
        Err(e) => e,
    }
}

/// The error of a text, as (kind, offset).
fn fail(s: &str) -> (K, usize) {
    let e = err(s.as_bytes());
    (e.kind, e.offset)
}

fn nested(n: usize) -> String {
    format!("{}{}", "[".repeat(n), "]".repeat(n))
}

fn string_of(n: usize) -> String {
    format!("\"{}\"", "a".repeat(n))
}

fn array_of(n: usize) -> String {
    format!("[{}]", vec!["0"; n].join(","))
}

fn object_of(n: usize) -> String {
    let members: Vec<String> = (0..n).map(|i| format!("\"k{i}\":0")).collect();
    format!("{{{}}}", members.join(","))
}

/// A top-level array of arrays holding exactly `total` values in the whole tree (the outer array
/// and every inner one count), each container within `MAX_MEMBERS`.
fn values_total(total: usize) -> String {
    let mut remaining = total - 1;
    let mut inner = Vec::new();
    while remaining > 0 {
        let len = (remaining - 1).min(MAX_MEMBERS);
        inner.push(array_of(len));
        remaining -= 1 + len;
    }
    format!("[{}]", inner.join(","))
}

// --- Limits ---------------------------------------------------------------------------------

#[test]
fn input_at_the_limit_parses_and_one_over_is_too_large() {
    // 255 strings of 4,096 bytes, padded with spaces to the exact size.
    let item = string_of(MAX_STRING);
    let body = vec![item; 255].join(",");
    let mut text = format!("[{body}]");
    assert!(text.len() < MAX_INPUT);
    text.push_str(&" ".repeat(MAX_INPUT - text.len()));
    assert_eq!(text.len(), MAX_INPUT);
    assert!(parse(text.as_bytes()).is_ok());
    text.push(' ');
    let e = err(text.as_bytes());
    assert_eq!(e.kind, K::TooLarge);
    assert_eq!(e.offset, MAX_INPUT);
}

#[test]
fn size_is_checked_before_anything_else() {
    // Not UTF-8 and not JSON, but too large: the size error comes first.
    let e = err(&vec![0xFF; MAX_INPUT + 1]);
    assert_eq!(e.kind, K::TooLarge);
}

#[test]
fn depth_16_parses_and_17_is_too_deep() {
    assert!(parse(nested(MAX_DEPTH).as_bytes()).is_ok());
    let e = err(nested(MAX_DEPTH + 1).as_bytes());
    assert_eq!(e.kind, K::TooDeep);
    assert_eq!(e.offset, MAX_DEPTH, "the 17th bracket");
    // Objects count the same.
    let mut obj = String::new();
    for _ in 0..MAX_DEPTH {
        obj.push_str("{\"a\":");
    }
    obj.push('0');
    obj.push_str(&"}".repeat(MAX_DEPTH));
    assert!(parse(obj.as_bytes()).is_ok());
    let deeper = format!("[{obj}]");
    assert_eq!(err(deeper.as_bytes()).kind, K::TooDeep);
    // The top-level scalar is depth 0.
    assert!(parse(b"0").is_ok());
}

#[test]
fn a_megabyte_of_open_brackets_is_too_deep_not_a_stack_overflow() {
    let open = "[".repeat(MAX_INPUT);
    let e = err(open.as_bytes());
    assert_eq!(e.kind, K::TooDeep);
    assert_eq!(e.offset, MAX_DEPTH);
    let open_obj = "{\"a\":".repeat(MAX_INPUT / 5);
    assert_eq!(err(open_obj.as_bytes()).kind, K::TooDeep);
}

#[test]
fn string_4096_parses_and_4097_is_too_long() {
    let v = ok(&string_of(MAX_STRING));
    assert_eq!(v.as_str().map(str::len), Ok(MAX_STRING));
    let e = err(string_of(MAX_STRING + 1).as_bytes());
    assert_eq!(e.kind, K::StringTooLong);
    assert_eq!(e.offset, 0, "the opening quote");
    // In an array the offset is still the string's own quote.
    assert_eq!(
        fail(&format!("[1,{}]", string_of(MAX_STRING + 1))),
        (K::StringTooLong, 3)
    );
}

#[test]
fn string_limit_counts_decoded_bytes() {
    // Escapes: 6 input bytes per decoded byte, so 4,096 of them pass and 4,097 do not.
    let esc = |n: usize| format!("\"{}\"", "\\u0041".repeat(n));
    assert_eq!(ok(&esc(MAX_STRING)).as_str().map(str::len), Ok(MAX_STRING));
    assert_eq!(err(esc(MAX_STRING + 1).as_bytes()).kind, K::StringTooLong);
    // Multi-byte characters count their bytes: 1,365 x 3 + 1 = 4,096.
    let euro = format!("\"{}a\"", "\u{20ac}".repeat(1365));
    assert_eq!(ok(&euro).as_str().map(str::len), Ok(MAX_STRING));
    let over = format!("\"{}\"", "\u{20ac}".repeat(1366));
    assert_eq!(err(over.as_bytes()).kind, K::StringTooLong);
    // A pair decodes to one 4-byte character.
    assert_eq!(ok("\"\\ud83d\\ude00\"").as_str().map(str::len), Ok(4));
}

#[test]
fn key_256_parses_and_257_is_too_long() {
    let with_key = |n: usize| format!("{{\"{}\":1}}", "k".repeat(n));
    assert!(parse(with_key(MAX_KEY).as_bytes()).is_ok());
    let e = err(with_key(MAX_KEY + 1).as_bytes());
    assert_eq!(e.kind, K::KeyTooLong);
    assert_eq!(e.offset, 1, "the key's opening quote");
}

#[test]
fn object_8192_members_parse_and_8193_is_too_many() {
    assert_eq!(
        ok(&object_of(MAX_MEMBERS)).as_object().map(<[_]>::len),
        Ok(MAX_MEMBERS)
    );
    let text = object_of(MAX_MEMBERS + 1);
    let e = err(text.as_bytes());
    assert_eq!(e.kind, K::TooManyMembers);
    // The offset is the 8,193rd key's opening quote.
    let at = text
        .find(&format!("\"k{MAX_MEMBERS}\""))
        .unwrap_or(usize::MAX);
    assert_eq!(e.offset, at);
}

#[test]
fn array_8192_elements_parse_and_8193_is_too_many() {
    assert_eq!(
        ok(&array_of(MAX_MEMBERS)).as_array().map(<[_]>::len),
        Ok(MAX_MEMBERS)
    );
    let text = array_of(MAX_MEMBERS + 1);
    let e = err(text.as_bytes());
    assert_eq!(e.kind, K::TooManyMembers);
    assert_eq!(
        e.offset,
        text.len() - 2,
        "the last element, before the bracket"
    );
}

#[test]
fn values_65536_parse_and_65537_is_too_many() {
    fn count(v: &Value) -> usize {
        match v {
            Value::Array(a) => 1 + a.iter().map(count).sum::<usize>(),
            Value::Object(o) => 1 + o.iter().map(|(_, v)| count(v)).sum::<usize>(),
            _ => 1,
        }
    }
    let v = ok(&values_total(MAX_VALUES));
    assert_eq!(count(&v), MAX_VALUES);
    let e = err(values_total(MAX_VALUES + 1).as_bytes());
    assert_eq!(e.kind, K::TooManyValues);
}

#[test]
fn many_small_values_in_one_megabyte_stop_at_the_value_limit() {
    // 1 MiB of `[],` would build about 350,000 nodes: the value limit ends it early.
    let inner = format!("[{}[]]", "[],".repeat(MAX_MEMBERS - 1));
    let text = format!("[{}]", vec![inner; 40].join(","));
    assert!(text.len() <= MAX_INPUT);
    let e = err(text.as_bytes());
    assert_eq!(e.kind, K::TooManyValues, "{e:?}");
}

#[test]
fn sixteen_deep_arrays_filling_a_megabyte_finish() {
    // A top-level array of 15-deep arrays is 16 deep; the member and value limits end it, and the
    // call returns a `Result` without growing the stack past the depth limit.
    let leaf = nested(MAX_DEPTH - 1);
    let n = (MAX_INPUT - 2) / (leaf.len() + 1);
    let text = format!("[{}]", vec![leaf; n].join(","));
    assert!(text.len() <= MAX_INPUT);
    let r = parse(text.as_bytes());
    assert!(matches!(
        r,
        Err(JsonError {
            kind: K::TooManyMembers | K::TooManyValues,
            ..
        })
    ));
}

// --- Fail closed ----------------------------------------------------------------------------

#[test]
fn syntax_errors_carry_their_offset() {
    assert_eq!(fail("[1,]"), (K::Syntax("expected a value"), 3));
    assert_eq!(fail("{\"a\":}"), (K::Syntax("expected a value"), 5));
    assert_eq!(fail("[1 2]"), (K::Syntax("expected ',' or ']'"), 3));
    assert_eq!(fail("{\"a\" 1}"), (K::Syntax("expected ':'"), 5));
    assert_eq!(fail("{1:2}"), (K::Syntax("expected a string key"), 1));
    assert_eq!(fail("{\"a\":1,}"), (K::Syntax("expected a string key"), 7));
    assert_eq!(
        fail("{\"a\":1 \"b\":2}"),
        (K::Syntax("expected ',' or '}'"), 7)
    );
    assert_eq!(fail("[1"), (K::Syntax("expected ',' or ']'"), 2));
    assert_eq!(fail("{\"a\":1"), (K::Syntax("expected ',' or '}'"), 6));
    assert_eq!(fail("["), (K::Syntax("expected a value"), 1));
    assert_eq!(fail(""), (K::Syntax("expected a value"), 0));
    assert_eq!(fail("  \n "), (K::Syntax("expected a value"), 4));
    assert_eq!(fail("tru"), (K::Syntax("invalid literal"), 0));
    assert_eq!(fail("nul"), (K::Syntax("invalid literal"), 0));
    assert_eq!(fail("[truex]"), (K::Syntax("expected ',' or ']'"), 5));
    assert_eq!(fail("\"abc"), (K::Syntax("unterminated string"), 4));
    assert_eq!(fail("'a'"), (K::Syntax("expected a value"), 0));
}

#[test]
fn a_raw_control_character_in_a_string_is_a_syntax_error() {
    assert_eq!(
        fail("\"a\nb\""),
        (K::Syntax("control character in string"), 2)
    );
    assert_eq!(
        fail("\"a\tb\""),
        (K::Syntax("control character in string"), 2)
    );
    assert_eq!(
        fail("\"\u{1}\""),
        (K::Syntax("control character in string"), 1)
    );
    // Escaped, they are fine; so is a raw DEL.
    assert_eq!(ok("\"a\\nb\"").as_str(), Ok("a\nb"));
    assert_eq!(ok("\"\u{7f}\"").as_str(), Ok("\u{7f}"));
}

#[test]
fn trailing_data_is_an_error() {
    assert_eq!(fail("1 2"), (K::TrailingData, 2));
    assert_eq!(fail("[]x"), (K::TrailingData, 2));
    assert_eq!(fail("{} {}"), (K::TrailingData, 3));
    assert_eq!(fail("null,"), (K::TrailingData, 4));
    assert!(parse(b" [] \r\n\t ").is_ok(), "whitespace after is fine");
}

#[test]
fn escapes() {
    assert_eq!(
        ok(r#""\"\\\/\b\f\n\r\t""#).as_str(),
        Ok("\"\\/\u{8}\u{c}\n\r\t")
    );
    assert_eq!(ok(r#""é€""#).as_str(), Ok("\u{e9}\u{20ac}"));
    assert_eq!(ok(r#""\u0000""#).as_str(), Ok("\0"));
    // A surrogate pair is one character; the same character raw is the same string.
    assert_eq!(ok(r#""😀""#).as_str(), Ok("\u{1f600}"));
    assert_eq!(ok("\"\u{1f600}\"").as_str(), Ok("\u{1f600}"));
}

#[test]
fn bad_escapes_are_invalid_escape_at_the_backslash() {
    assert_eq!(
        fail(r#""\ud800""#),
        (K::InvalidEscape, 1),
        "lone high surrogate"
    );
    assert_eq!(
        fail(r#""\udc00""#),
        (K::InvalidEscape, 1),
        "lone low surrogate"
    );
    assert_eq!(
        fail(r#""ab\ud800A""#),
        (K::InvalidEscape, 3),
        "high then not low"
    );
    assert_eq!(
        fail(r#""\ud800x""#),
        (K::InvalidEscape, 1),
        "high then a plain character"
    );
    assert_eq!(fail(r#""\x""#), (K::InvalidEscape, 1));
    assert_eq!(fail(r#""\u12""#), (K::InvalidEscape, 1), "short hex");
    assert_eq!(fail(r#""\uZZZZ""#), (K::InvalidEscape, 1));
    assert_eq!(
        fail(r#""\u+123""#),
        (K::InvalidEscape, 1),
        "a sign is no hex digit"
    );
    assert_eq!(fail("\"\\"), (K::InvalidEscape, 1), "backslash at the end");
    // Keys use the same decoder.
    assert_eq!(fail(r#"{"\ud800":1}"#), (K::InvalidEscape, 2));
}

#[test]
fn invalid_utf8_is_refused_at_the_first_bad_byte() {
    let e = err(&[0xFF, 0xFE]);
    assert_eq!((e.kind, e.offset), (K::InvalidUtf8, 0));
    let e = err(b"[\"a\xc3\"]");
    assert_eq!(
        (e.kind, e.offset),
        (K::InvalidUtf8, 3),
        "a truncated sequence"
    );
    let e = err(b"\"\xed\xa0\x80\"");
    assert_eq!(
        (e.kind, e.offset),
        (K::InvalidUtf8, 1),
        "an encoded surrogate"
    );
    let e = err(b"\"\xc0\xaf\"");
    assert_eq!(e.kind, K::InvalidUtf8, "an overlong form");
    // UTF-16 text, as a file saved by some editors, is no JSON here.
    assert_eq!(err(&[0xFF, 0xFE, b'[', 0, b']', 0]).kind, K::InvalidUtf8);
}

#[test]
fn a_leading_byte_order_mark_is_skipped_and_still_counts_in_offsets() {
    let mut with = vec![0xEF, 0xBB, 0xBF];
    with.extend_from_slice(b"[1]");
    assert!(parse(&with).is_ok());
    let mut bad = vec![0xEF, 0xBB, 0xBF];
    bad.extend_from_slice(b"[1,]");
    let e = err(&bad);
    assert_eq!((e.kind, e.offset), (K::Syntax("expected a value"), 6));
    assert_eq!(e.column, 5, "the mark is one character");
    // Only a leading mark is skipped.
    assert_eq!(
        err("[\u{feff}1]".as_bytes()).kind,
        K::Syntax("expected a value")
    );
    // A mark alone is an empty document.
    assert_eq!(err(&[0xEF, 0xBB, 0xBF]).offset, 3);
    // The mark counts toward the size limit.
    let mut big = vec![0xEF, 0xBB, 0xBF];
    big.extend_from_slice(b"[]");
    big.extend(std::iter::repeat_n(b' ', MAX_INPUT - big.len() + 1));
    assert_eq!(err(&big).kind, K::TooLarge);
}

#[test]
fn duplicate_keys_are_an_error_at_the_second() {
    let e = err(br#"{"a":1,"a":2}"#);
    assert_eq!(e.kind, K::DuplicateKey("a".to_string()));
    assert_eq!(e.offset, 7);
    // The same key spelled by an escape is the same key.
    assert_eq!(
        fail(r#"{"a":1,"a":2}"#),
        (K::DuplicateKey("a".to_string()), 7)
    );
    // Per object: the same key in two objects, or nested, is fine.
    assert!(parse(br#"{"a":{"a":1},"b":{"a":2}}"#).is_ok());
    assert_eq!(
        fail(r#"{"a":1,"b":{"x":1,"x":2}}"#),
        (K::DuplicateKey("x".to_string()), 18)
    );
}

#[test]
fn numbers_follow_the_json_grammar() {
    for good in [
        "0", "-0", "7", "-12", "1.5", "0.25", "1e3", "1E+3", "2e-3", "-0.5e1", "10",
    ] {
        assert!(parse(good.as_bytes()).is_ok(), "{good}");
    }
    assert_eq!(fail("01"), (K::Syntax("leading zero in number"), 1));
    assert_eq!(fail("-01"), (K::Syntax("leading zero in number"), 2));
    assert_eq!(fail("+1"), (K::Syntax("expected a value"), 0));
    assert_eq!(fail(".5"), (K::Syntax("expected a value"), 0));
    assert_eq!(fail("1."), (K::Syntax("expected a digit after '.'"), 2));
    assert_eq!(fail("-"), (K::Syntax("expected a digit"), 1));
    assert_eq!(fail("-a"), (K::Syntax("expected a digit"), 1));
    assert_eq!(
        fail("1e"),
        (K::Syntax("expected a digit in the exponent"), 2)
    );
    assert_eq!(
        fail("1e+"),
        (K::Syntax("expected a digit in the exponent"), 3)
    );
    assert_eq!(fail("0x10"), (K::TrailingData, 1));
    assert_eq!(fail("NaN"), (K::Syntax("expected a value"), 0));
    assert_eq!(fail("Infinity"), (K::Syntax("expected a value"), 0));
    assert_eq!(fail("-Infinity"), (K::Syntax("expected a digit"), 1));
}

#[test]
fn a_number_past_f64_is_out_of_range_and_underflow_is_a_value() {
    assert_eq!(fail("1e999"), (K::NumberOutOfRange, 0));
    assert_eq!(fail("-1e999"), (K::NumberOutOfRange, 0));
    assert_eq!(fail("[0,1e309]"), (K::NumberOutOfRange, 3));
    assert_eq!(fail(&"9".repeat(400)), (K::NumberOutOfRange, 0));
    assert_eq!(
        fail(&format!("[{}]", "9".repeat(400))),
        (K::NumberOutOfRange, 1)
    );
    assert_eq!(ok("1e-999").as_f64(), Ok(0.0));
    assert_eq!(ok("[1e308]").as_array().map(<[_]>::len), Ok(1));
    // 309 digits is still finite when small enough.
    let big = format!("1{}", "0".repeat(300));
    assert!(ok(&big).as_f64().is_ok_and(f64::is_finite));
}

#[test]
fn integers_above_2_pow_53_are_inexact_and_as_int_refuses_them() {
    assert_eq!(ok("9007199254740992").as_int(), Ok(9_007_199_254_740_992));
    assert_eq!(ok("-9007199254740992").as_int(), Ok(-9_007_199_254_740_992));
    for big in [
        "9007199254740993",
        "-9007199254740993",
        "18446744073709551616",
    ] {
        let v = ok(big);
        let Value::Number(n) = &v else {
            panic!("not a number")
        };
        assert!(!n.is_exact_int(), "{big}");
        assert!(n.value().is_finite());
        assert_eq!(v.as_int().map_err(|e| e.kind), Err(K::WrongType), "{big}");
        assert!(v.as_f64().is_ok());
    }
}

// --- Typed readers --------------------------------------------------------------------------

#[test]
fn typed_readers_answer_or_say_wrong_type() {
    let v = ok(
        r#"{"n":3,"f":1.5,"s":"x","e":1e2,"o":{"a":"b","c":"d"},"m":{"p":1,"q":2.5},"l":[1,2]}"#,
    );
    assert_eq!(v.get("n").and_then(Value::as_int), Ok(3));
    assert_eq!(v.get("f").and_then(Value::as_f64), Ok(1.5));
    assert_eq!(v.get("s").and_then(Value::as_str), Ok("x"));
    assert_eq!(
        v.get("o").and_then(Value::string_map),
        Ok(vec![("a", "b"), ("c", "d")])
    );
    assert_eq!(
        v.get("m").and_then(Value::number_map),
        Ok(vec![("p", 1.0), ("q", 2.5)])
    );
    assert_eq!(v.get("l").and_then(Value::as_array).map(<[_]>::len), Ok(2));

    let kind = |r: Result<_, JsonError>| r.map(|_: ()| ()).map_err(|e| e.kind);
    // A wrong type is an error, never a default.
    assert_eq!(
        kind(v.get("n").and_then(Value::as_str).map(drop)),
        Err(K::WrongType)
    );
    assert_eq!(
        kind(v.get("s").and_then(Value::as_f64).map(drop)),
        Err(K::WrongType)
    );
    assert_eq!(
        kind(v.get("f").and_then(Value::as_int).map(drop)),
        Err(K::WrongType)
    );
    assert_eq!(
        kind(v.get("e").and_then(Value::as_int).map(drop)),
        Err(K::WrongType)
    );
    assert_eq!(
        kind(v.get("m").and_then(Value::string_map).map(drop)),
        Err(K::WrongType)
    );
    assert_eq!(
        kind(v.get("o").and_then(Value::number_map).map(drop)),
        Err(K::WrongType)
    );
    assert_eq!(
        kind(v.get("n").and_then(Value::string_map).map(drop)),
        Err(K::WrongType)
    );
    assert_eq!(
        kind(v.get("n").and_then(Value::as_object).map(drop)),
        Err(K::WrongType)
    );
    assert_eq!(
        kind(v.get("n").and_then(Value::as_array).map(drop)),
        Err(K::WrongType)
    );
    // `get` on a non-object, and a missing member.
    assert_eq!(kind(ok("[1]").get("a").map(drop)), Err(K::WrongType));
    assert_eq!(
        kind(v.get("zz").map(drop)),
        Err(K::Missing("zz".to_string()))
    );
    // Typed errors name no position.
    let e = v.get("zz").err();
    assert_eq!(e.map(|e| (e.offset, e.line, e.column)), Some((0, 0, 0)));
}

#[test]
fn objects_keep_member_order() {
    let v = ok(r#"{"z":1,"a":2,"m":3}"#);
    let keys: Vec<&str> = v
        .as_object()
        .unwrap_or_default()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    assert_eq!(keys, ["z", "a", "m"]);
    assert_eq!(v.number_map(), Ok(vec![("z", 1.0), ("a", 2.0), ("m", 3.0)]));
}

#[test]
fn scalars_at_the_top_level() {
    assert_eq!(ok("null"), Value::Null);
    assert_eq!(ok(" true "), Value::Bool(true));
    assert_eq!(ok("false"), Value::Bool(false));
    assert_eq!(ok("\"x\"").as_str(), Ok("x"));
    assert_eq!(ok("[]"), Value::Array(vec![]));
    assert_eq!(ok("{}"), Value::Object(vec![]));
}

// --- Line and column ------------------------------------------------------------------------

#[test]
fn line_and_column_are_one_based_and_count_characters() {
    let e = err(b"[\n  1,\n  ]");
    assert_eq!((e.offset, e.line, e.column), (9, 3, 3));
    let e = err("[\"\u{e9}\",]".as_bytes());
    assert_eq!(
        (e.offset, e.line, e.column),
        (6, 1, 6),
        "the accented letter is one column"
    );
    let e = err(b"{\"a\":1,\"a\":2}");
    assert_eq!((e.line, e.column), (1, 8));
    // CRLF: the line break is the newline.
    let e = err(b"[\r\n1,\r\n]");
    assert_eq!((e.line, e.column), (3, 1));
    // An error at the end of the input.
    let e = err(b"\"abc");
    assert_eq!((e.offset, e.line, e.column), (4, 1, 5));
}

// --- Bounded work ---------------------------------------------------------------------------

#[test]
fn a_pseudo_random_walk_over_json_like_bytes_always_returns_a_result() {
    // xorshift; the alphabet leans on the characters that open and close things.
    const ALPHA: &[u8] = b"[]{}\",:0123456789.eE+-\\ntrufalsb\xff\xc3\xa9\n \t/uD8dc";
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let mut seen_ok = 0;
    for _ in 0..4000 {
        let len = (next() % 64) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| ALPHA[(next() % ALPHA.len() as u64) as usize])
            .collect();
        if parse(&bytes).is_ok() {
            seen_ok += 1;
        }
    }
    assert!(
        seen_ok > 0,
        "the alphabet reaches some valid inputs, so the walk is not all rejects"
    );
}

#[test]
fn many_keys_in_one_object_are_not_checked_quadratically() {
    // 8,192 distinct keys of ~105 bytes: a linear scan per key would be ~33 million compares of
    // long shared prefixes; the set keeps this fast. The call just has to finish and be correct.
    let prefix = "p".repeat(100);
    let members: Vec<String> = (0..MAX_MEMBERS)
        .map(|i| format!("\"{prefix}{i:05}\":0"))
        .collect();
    let text = format!("{{{}}}", members.join(","));
    assert!(text.len() <= MAX_INPUT);
    assert_eq!(ok(&text).as_object().map(<[_]>::len), Ok(MAX_MEMBERS));
    let dup = format!(
        "{{{},\"{prefix}00000\":1}}",
        members[..MAX_MEMBERS - 1].join(",")
    );
    assert!(matches!(err(dup.as_bytes()).kind, K::DuplicateKey(_)));
}
