//! A bounded JSON reader. Bytes in, a value tree or a structured error out; no file system, no
//! clock, no third-party code. It reads files a user can edit (themes, locale tables), so every path
//! from bytes to a value is closed: any byte slice returns `Ok(Value)` or `Err(JsonError)`, never a
//! panic and never a partial tree.
//!
//! Bounds (constants below, in one place):
//! - the input is refused above [`MAX_INPUT`] before any parsing, and is validated as UTF-8 once;
//! - nesting is at most [`MAX_DEPTH`] containers deep, so the recursion is at most that many frames
//!   plus a constant;
//! - a decoded string is at most [`MAX_STRING`] bytes, a key at most [`MAX_KEY`];
//! - an object or array holds at most [`MAX_MEMBERS`] members, the whole tree at most
//!   [`MAX_VALUES`] values;
//! - work is linear in the input (duplicate-key checks are `n log n`), memory bounded by the input
//!   and the limits above.
//!
//! The grammar is RFC 8259's. A leading UTF-8 byte-order mark is skipped (offsets still count it).
//! Duplicate keys in one object are an error. A number is a finite `f64`; an integer literal whose
//! magnitude is above 2^53 is an inexact value ([`Number::is_exact_int`] is false and
//! [`Value::as_int`] refuses it); anything that overflows to infinity is an error, underflow to zero
//! is a value.
//!
//! Errors are structured, not sentences: the app writes the text. `offset` is a byte offset into the
//! input as given; `line` and `column` (1-based, columns in characters) are computed once, on error.
//! The typed readers on [`Value`] report no position (`offset`, `line` and `column` are all 0).

use std::collections::BTreeSet;

/// Largest input, in bytes. One more is [`JsonErrorKind::TooLarge`], before anything is parsed.
pub const MAX_INPUT: usize = 1_048_576;
/// Deepest nesting of arrays and objects. Scalars at the top are depth 0; 16 nested arrays are
/// allowed, 17 are [`JsonErrorKind::TooDeep`].
pub const MAX_DEPTH: usize = 16;
/// Longest decoded string, in bytes.
pub const MAX_STRING: usize = 4_096;
/// Longest decoded object key, in bytes.
pub const MAX_KEY: usize = 256;
/// Most members of one object, most elements of one array.
pub const MAX_MEMBERS: usize = 8_192;
/// Most values in the whole tree (every scalar, array and object counts one; keys do not).
pub const MAX_VALUES: usize = 65_536;

/// The largest integer an `f64` holds exactly.
const EXACT_INT_MAX: i64 = 1 << 53;

/// What went wrong. Reader errors carry a position; the last two come from the typed readers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonErrorKind {
    /// The input is longer than [`MAX_INPUT`].
    TooLarge,
    /// More than [`MAX_DEPTH`] containers deep.
    TooDeep,
    /// A decoded string longer than [`MAX_STRING`] bytes (offset: its opening quote).
    StringTooLong,
    /// A decoded key longer than [`MAX_KEY`] bytes (offset: its opening quote).
    KeyTooLong,
    /// A container with more than [`MAX_MEMBERS`] members (offset: the first one over).
    TooManyMembers,
    /// More than [`MAX_VALUES`] values in the tree (offset: the first one over).
    TooManyValues,
    /// The bytes are not UTF-8 (offset: the first bad byte).
    InvalidUtf8,
    /// The text breaks the grammar; the reason is a fixed phrase.
    Syntax(&'static str),
    /// A bad escape in a string, including a lone surrogate (offset: its backslash).
    InvalidEscape,
    /// Something follows the value.
    TrailingData,
    /// An object names a key twice (offset: the second one). The key is at most [`MAX_KEY`] bytes.
    DuplicateKey(String),
    /// A number that overflows `f64` (offset: its first character).
    NumberOutOfRange,
    /// A typed reader found another kind of value.
    WrongType,
    /// A typed reader found no member of this name.
    Missing(String),
}

/// A reader error: what, and where. See the module note for the position fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    /// What went wrong.
    pub kind: JsonErrorKind,
    /// Byte offset into the input as given (a leading byte-order mark counts).
    pub offset: usize,
    /// 1-based line (0 for a typed-reader error).
    pub line: usize,
    /// 1-based column in characters (0 for a typed-reader error).
    pub column: usize,
}

impl JsonError {
    fn typed(kind: JsonErrorKind) -> Self {
        Self {
            kind,
            offset: 0,
            line: 0,
            column: 0,
        }
    }

    /// Fills in line and column from the offset. Linear in the offset, called once per error.
    fn at(kind: JsonErrorKind, offset: usize, bytes: &[u8]) -> Self {
        let end = offset.min(bytes.len());
        let before = bytes.get(..end).unwrap_or_default();
        let mut line = 1;
        let mut column = 1;
        for &b in before {
            if b == b'\n' {
                line += 1;
                column = 1;
            } else if b & 0xC0 != 0x80 {
                // A continuation byte is part of the character before it.
                column += 1;
            }
        }
        Self {
            kind,
            offset,
            line,
            column,
        }
    }
}

/// A JSON number: a finite `f64` and whether it is an exact integer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number {
    value: f64,
    exact_int: bool,
}

impl Number {
    /// The value, always finite.
    pub fn value(self) -> f64 {
        self.value
    }

    /// True when the literal was an integer (no fraction, no exponent) of magnitude at most 2^53,
    /// so `value` holds it exactly.
    pub fn is_exact_int(self) -> bool {
        self.exact_int
    }
}

/// A parsed JSON value. Object members keep the file's order; keys are unique.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number.
    Number(Number),
    /// A string, decoded.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object, in member order.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The member named `key` of an object. `Missing(key)` when absent, `WrongType` when this is
    /// not an object.
    pub fn get(&self, key: &str) -> Result<&Value, JsonError> {
        match self {
            Value::Object(members) => members
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v)
                .ok_or_else(|| JsonError::typed(JsonErrorKind::Missing(key.to_string()))),
            _ => Err(JsonError::typed(JsonErrorKind::WrongType)),
        }
    }

    /// The string, or `WrongType`.
    pub fn as_str(&self) -> Result<&str, JsonError> {
        match self {
            Value::String(s) => Ok(s),
            _ => Err(JsonError::typed(JsonErrorKind::WrongType)),
        }
    }

    /// The number as `f64`, or `WrongType`.
    pub fn as_f64(&self) -> Result<f64, JsonError> {
        match self {
            Value::Number(n) => Ok(n.value),
            _ => Err(JsonError::typed(JsonErrorKind::WrongType)),
        }
    }

    /// The number as an integer. `WrongType` when this is not a number, or when it is not an exact
    /// integer (a fraction, an exponent, or a magnitude above 2^53).
    pub fn as_int(&self) -> Result<i64, JsonError> {
        match self {
            // The cast is exact: `exact_int` is set only for an integer literal of magnitude <= 2^53.
            Value::Number(n) if n.exact_int => Ok(n.value as i64),
            _ => Err(JsonError::typed(JsonErrorKind::WrongType)),
        }
    }

    /// The members of an object, in order, or `WrongType`.
    pub fn as_object(&self) -> Result<&[(String, Value)], JsonError> {
        match self {
            Value::Object(members) => Ok(members),
            _ => Err(JsonError::typed(JsonErrorKind::WrongType)),
        }
    }

    /// The elements of an array, or `WrongType`.
    pub fn as_array(&self) -> Result<&[Value], JsonError> {
        match self {
            Value::Array(items) => Ok(items),
            _ => Err(JsonError::typed(JsonErrorKind::WrongType)),
        }
    }

    /// An object whose every value is a string, as `(key, text)` pairs in order. `WrongType` for
    /// another kind of value or any non-string member.
    pub fn string_map(&self) -> Result<Vec<(&str, &str)>, JsonError> {
        self.as_object()?
            .iter()
            .map(|(k, v)| v.as_str().map(|s| (k.as_str(), s)))
            .collect()
    }

    /// An object whose every value is a number, as `(key, number)` pairs in order. `WrongType` for
    /// another kind of value or any non-number member.
    pub fn number_map(&self) -> Result<Vec<(&str, f64)>, JsonError> {
        self.as_object()?
            .iter()
            .map(|(k, v)| v.as_f64().map(|n| (k.as_str(), n)))
            .collect()
    }
}

/// Parses `bytes` as one JSON value, within the limits in this module.
pub fn parse(bytes: &[u8]) -> Result<Value, JsonError> {
    if bytes.len() > MAX_INPUT {
        return Err(JsonError::at(JsonErrorKind::TooLarge, MAX_INPUT, bytes));
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            return Err(JsonError::at(
                JsonErrorKind::InvalidUtf8,
                e.valid_up_to(),
                bytes,
            ));
        }
    };
    let mut p = Parser {
        text,
        bytes,
        pos: if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            3
        } else {
            0
        },
        values: 0,
    };
    p.run()
        .map_err(|(kind, offset)| JsonError::at(kind, offset, bytes))
}

/// A failure before its line and column are known: the kind and the byte offset.
type Fail = (JsonErrorKind, usize);

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    values: usize,
}

impl Parser<'_> {
    fn run(&mut self) -> Result<Value, Fail> {
        self.skip_ws();
        let v = self.value(0)?;
        self.skip_ws();
        if self.pos < self.bytes.len() {
            return Err((JsonErrorKind::TrailingData, self.pos));
        }
        Ok(v)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn syntax(&self, what: &'static str) -> Fail {
        (JsonErrorKind::Syntax(what), self.pos)
    }

    /// One value at the current position. `depth` is the number of containers around it.
    fn value(&mut self, depth: usize) -> Result<Value, Fail> {
        self.values += 1;
        if self.values > MAX_VALUES {
            return Err((JsonErrorKind::TooManyValues, self.pos));
        }
        match self.peek() {
            None => Err(self.syntax("expected a value")),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Value::String(
                self.string(MAX_STRING, JsonErrorKind::StringTooLong)?,
            )),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.syntax("expected a value")),
        }
    }

    fn literal(&mut self, word: &'static str, v: Value) -> Result<Value, Fail> {
        let end = self.pos + word.len();
        if self.bytes.get(self.pos..end) == Some(word.as_bytes()) {
            self.pos = end;
            Ok(v)
        } else {
            Err(self.syntax("invalid literal"))
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, Fail> {
        if depth >= MAX_DEPTH {
            return Err((JsonErrorKind::TooDeep, self.pos));
        }
        self.pos += 1; // '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_ws();
            if items.len() >= MAX_MEMBERS {
                return Err((JsonErrorKind::TooManyMembers, self.pos));
            }
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.syntax("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, Fail> {
        if depth >= MAX_DEPTH {
            return Err((JsonErrorKind::TooDeep, self.pos));
        }
        self.pos += 1; // '{'
        let mut members: Vec<(String, Value)> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.skip_ws();
            if members.len() >= MAX_MEMBERS {
                return Err((JsonErrorKind::TooManyMembers, self.pos));
            }
            if self.peek() != Some(b'"') {
                return Err(self.syntax("expected a string key"));
            }
            let key_at = self.pos;
            let key = self.string(MAX_KEY, JsonErrorKind::KeyTooLong)?;
            if !seen.insert(key.clone()) {
                return Err((JsonErrorKind::DuplicateKey(key), key_at));
            }
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(self.syntax("expected ':'"));
            }
            self.pos += 1;
            self.skip_ws();
            let v = self.value(depth + 1)?;
            members.push((key, v));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(self.syntax("expected ',' or '}'")),
            }
        }
    }

    /// A string at the current `"`, decoded, at most `limit` bytes; `too_long` is the error kind
    /// when it exceeds that.
    fn string(&mut self, limit: usize, too_long: JsonErrorKind) -> Result<String, Fail> {
        let start = self.pos;
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            // Copy the run up to the next quote, backslash or control byte. Those are ASCII, so the
            // run ends on a character boundary of the (validated) text.
            let run_start = self.pos;
            while let Some(b) = self.peek() {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            let run = self
                .text
                .get(run_start..self.pos)
                .ok_or((JsonErrorKind::InvalidUtf8, run_start))?;
            if out.len() + run.len() > limit {
                return Err((too_long, start));
            }
            out.push_str(run);
            match self.peek() {
                None => return Err(self.syntax("unterminated string")),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    let c = self.escape()?;
                    if out.len() + c.len_utf8() > limit {
                        return Err((too_long, start));
                    }
                    out.push(c);
                }
                Some(_) => return Err(self.syntax("control character in string")),
            }
        }
    }

    /// One escape at the current backslash.
    fn escape(&mut self) -> Result<char, Fail> {
        let at = self.pos;
        let bad = (JsonErrorKind::InvalidEscape, at);
        self.pos += 1;
        let c = match self.peek() {
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            Some(b'/') => '/',
            Some(b'b') => '\u{8}',
            Some(b'f') => '\u{c}',
            Some(b'n') => '\n',
            Some(b'r') => '\r',
            Some(b't') => '\t',
            Some(b'u') => {
                self.pos += 1;
                let first = self.hex4().ok_or_else(|| bad.clone())?;
                return match first {
                    0xD800..=0xDBFF => {
                        // A high surrogate must be followed by `\u` and a low surrogate.
                        if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u") {
                            return Err(bad);
                        }
                        self.pos += 2;
                        let second = self.hex4().ok_or_else(|| bad.clone())?;
                        if !(0xDC00..=0xDFFF).contains(&second) {
                            return Err(bad);
                        }
                        let code = 0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00);
                        char::from_u32(code).ok_or(bad)
                    }
                    0xDC00..=0xDFFF => Err(bad),
                    _ => char::from_u32(first).ok_or(bad),
                };
            }
            _ => return Err(bad),
        };
        self.pos += 1;
        Ok(c)
    }

    /// Four hex digits at the current position, consumed.
    fn hex4(&mut self) -> Option<u32> {
        let digits = self.bytes.get(self.pos..self.pos + 4)?;
        let mut v = 0u32;
        for &d in digits {
            v = v * 16 + char::from(d).to_digit(16)?;
        }
        self.pos += 4;
        Some(v)
    }

    fn digits(&mut self) -> usize {
        let from = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        self.pos - from
    }

    fn number(&mut self) -> Result<Value, Fail> {
        let start = self.pos;
        let mut integer = true;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(self.syntax("leading zero in number"));
                }
            }
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(self.syntax("expected a digit")),
        }
        if self.peek() == Some(b'.') {
            integer = false;
            self.pos += 1;
            if self.digits() == 0 {
                return Err(self.syntax("expected a digit after '.'"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            integer = false;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if self.digits() == 0 {
                return Err(self.syntax("expected a digit in the exponent"));
            }
        }
        let token = self
            .text
            .get(start..self.pos)
            .ok_or((JsonErrorKind::InvalidUtf8, start))?;
        let value: f64 = token
            .parse()
            .map_err(|_| (JsonErrorKind::Syntax("invalid number"), start))?;
        if !value.is_finite() {
            return Err((JsonErrorKind::NumberOutOfRange, start));
        }
        let exact_int = integer
            && token
                .parse::<i64>()
                .is_ok_and(|i| (-EXACT_INT_MAX..=EXACT_INT_MAX).contains(&i));
        Ok(Value::Number(Number { value, exact_int }))
    }
}
