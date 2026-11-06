---
id: infra.json
title: Bounded JSON reader
area: infra
status: in-progress
consumers: any
source: crates/lune-ui-core/src/json.rs
owner: —
summary: Small JSON reader in core with input, depth and string limits, fail-closed errors and no panics; backs theme and locale loading
---

## What it is

A small JSON reader in `lune-ui-core` with no third-party dependency (D12, hard rule 4): a parsed
value tree (objects, arrays, strings, numbers, booleans, null), hard limits on input size, nesting
depth, string and key length and member count, errors that say what and where, and no panic on any
input. It backs `infra.theme-hot-reload` (theme files) and `infra.localization` (locale files). It
never touches the file system or the clock: the app reads the bytes and passes them in.
Lunegit reads the same two kinds of file with `System.Text.Json` and sets no limit at all.

## Needs

- Bounded JSON reader with limits, fail-closed errors and no panics (R-infra-6, D44).
- Theme values loadable from JSON (R-infra-3) and locale tables (R-infra-1).
- Security review at M10 (D12): the reader parses files a user can edit.

## Existing code

`crates/lune-ui-core/src/json.rs` (M10 step 1), tests `crates/lune-ui-core/tests/json.rs` and `apps/gallery/tests/lunegit_files.rs`. Lunegit's loaders: `src/Lunegit.Infrastructure/Theming/JsonThemeRegistry.cs:75-76,138-139` and
`src/Lunegit.Infrastructure/Localization/JsonLanguageRegistry.cs:78-79,144-145`, `LocalizationService.cs:104-130` (the embedded English file).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Infrastructure/Theming/JsonThemeRegistry.cs:1-147, Theming/ThemeJsonContext.cs:1-15, src/Lunegit.Core/Theme/ThemeFile.cs:1-21,
src/Lunegit.Infrastructure/Localization/LocalizationService.cs:1-130, Localization/LanguageJsonContext.cs:1-15, Localization/JsonLanguageRegistry.cs:60-90, src/Lunegit.Core/Localization/LanguageFile.cs:1-25,
src/Lunegit.Desktop/SampleThemes/dark-modern.json:1-40 and blue.json:60-100, src/Lunegit.Infrastructure/Localization/en.json:1-10.
Measurements of the real files (instrument: `python3 -I` with `json.load`; checked first on `en.json`, which gives 1019 keys, the count the plan's explorer reported): en.json is 67,364 bytes, 1019 strings, longest string 362 characters, longest key 59 characters, nesting depth 2, no byte-order mark;
the five sample themes are 3,760 to 4,535 bytes (`wc -c`), nesting depth 2, at most 102 colour keys and 8 acrylic keys, colours as 9-character `#AARRGGBB` (440 of them) or 7-character `#RRGGBB` (2).
Lunegit sets no limit: `File.ReadAllText` then `JsonSerializer.Deserialize` with the source-generated context and default options (`grep -n 'MaxDepth\|JsonSerializerOptions\|FileInfo'` over both folders finds nothing) [L src/Lunegit.Infrastructure/Theming/JsonThemeRegistry.cs:75-76], [L src/Lunegit.Infrastructure/Localization/JsonLanguageRegistry.cs:78-79].
No conflicts among sources; Wane has no JSON need.

### Anatomy

- Input: a byte slice the app supplies; the reader refuses anything that is not valid UTF-8 (a leading byte-order mark is skipped; none of the real files has one). No path, no I/O, no environment: hard rule 2 [R CLAUDE.md:67].
- Output: an immutable value tree. Object members keep their order; numbers are finite `f64` with an exact-integer flag; no NaN, no infinities (JSON cannot spell them) [derived: the JSON grammar].
- Limits, proposed from the real files with headroom, to be reviewed at M10 (Open questions):

| limit | value | derivation |
|---|---|---|
| input size | 1,048,576 bytes | [derived: en.json is 67,364 bytes; 15 times that] |
| nesting depth | 16 | [derived: the real files nest 2 levels; 8 times that] |
| string length | 4,096 bytes | [derived: the longest real string is 362 characters; 11 times that] |
| key length | 256 bytes | [derived: the longest real key is 59 characters; 4 times that] |
| members per object, elements per array | 8,192 | [derived: en.json has 1019 members; 8 times that] |

- Errors (fail closed: the first error ends the read and returns no partial tree): `TooLarge`, `TooDeep`, `StringTooLong`, `KeyTooLong`, `TooManyMembers`, `InvalidUtf8 { offset }`, `Syntax { offset, what }`, `InvalidEscape { offset }`, `TrailingData { offset }`, `DuplicateKey { key }`, `NumberOutOfRange { offset }`. Each carries a byte offset (and a line and column derived from it) so the app can say where; Lunegit's messages name the file and give no position [L src/Lunegit.Infrastructure/Theming/JsonThemeRegistry.cs:84,108].
- Duplicate keys in one object are an error, so a theme cannot say two things for one token [derived: fail-closed; Lunegit's behaviour for a duplicate key in `Dictionary` deserialization is not read].
- No panics: no `unwrap`, no unchecked index, no recursion deeper than the depth limit, no allocation sized from an unvalidated length; `unwrap_used` is a workspace lint [R CLAUDE.md:84].
- Typed readers over the tree for what the two file kinds need: a string-to-string map, a string-to-number map, a string field, an integer field; each returns a typed error, never a default, for a wrong type [L src/Lunegit.Core/Theme/ThemeFile.cs:10-21], [L src/Lunegit.Core/Localization/LanguageFile.cs:10-20].
- The envelope Lunegit uses, kept so existing files load: a theme is `{ name, author, version, colors: {key: "#AARRGGBB"}, acrylic: {key: number} }` and a locale is `{ code, displayName, version, strings: {key: text} }`, camel-case keys [L src/Lunegit.Infrastructure/Theming/ThemeJsonContext.cs:11-14], [L src/Lunegit.Infrastructure/Localization/en.json:2-5].
- Unknown keys in a theme or a locale are not a reader error: the reader returns the whole tree; the theme and locale loaders decide (`infra.theme-hot-reload`, `infra.localization`).
- Fixtures for M10, copied from Lunegit's real files, not hand-written: the five themes `src/Lunegit.Desktop/SampleThemes/blue.json`, `dark-modern.json`, `green.json`, `orange.json`, `purple.json` and `src/Lunegit.Infrastructure/Localization/en.json` [L src/Lunegit.Desktop/SampleThemes/dark-modern.json:1-6], [L src/Lunegit.Infrastructure/Localization/en.json:1-5]. Negative fixtures are made from them by mutation: truncated at every 97th byte, nesting 17 deep, a 4,097-byte string, an invalid UTF-8 byte, a trailing comma, a lone surrogate escape, a number with 400 digits, a duplicated key.

### States

n/a — the reader is a pure function: input bytes in, a tree or an error out; it has no look and no interaction state.

### Transitions

n/a — no look to animate.

### Input and focus

n/a — no pointer, keys, focus or layer.

### Retained state

- Transient: none. Persistent: none. App-owned: the bytes, the parsed tree and the choice to keep it.
- Exit retention: no. The reader is stateless and reentrant [derived: a pure function].

### Accessibility

n/a — no node: no widget.

### Strings

None: the error texts are the app's (the reader gives structured errors, not sentences).

### Gallery scenes

n/a — the reader draws nothing; it is exercised through the theme loader, which the `theme-*` scenes of `infra.theme-hot-reload` render (M10).

### Painter primitives

- Existing: none.
- New: none.

### Acceptance

- Known answers: the five sample themes and `en.json` parse; `en.json` yields 1019 members under `strings`, and each theme yields the object with 5 keys (`name`, `author`, `version`, `colors`, `acrylic`) [L src/Lunegit.Desktop/SampleThemes/blue.json:60-100].
- Limits: an input of 1,048,577 bytes returns `TooLarge` before parsing; 17 nested arrays return `TooDeep`; a 4,097-byte string returns `StringTooLong`; a 257-byte key returns `KeyTooLong`; an object with 8,193 members returns `TooManyMembers`.
- Fail closed: each negative fixture returns an error and no partial tree; `{"a":1,"a":2}` returns `DuplicateKey`; `[1,]`, `{"a":}`, a lone `\ud800`, bytes `ff fe` and `1e999` each return the matching error with the right offset.
- No panics: every prefix of every fixture, every single-byte deletion and every byte flipped at 97 positions returns a `Result` (a table test over the mutations; no `catch_unwind`).
- Bounded work: parse time is linear in the input size; a 1,048,576-byte input of 16-deep arrays finishes without stack growth beyond 16 frames [derived: the depth limit].
- Pixel probes and conformance: n/a (no drawing, no transitions).
- Semantic-node test: n/a.
- Security review (D12): the reader is parsing user-editable files; M10's review (the orchestrator's, after step 11) checks the limits and the error paths. The tests above are done (M10 step 1, `crates/lune-ui-core/tests/json.rs`, `apps/gallery/tests/lunegit_files.rs`).

### Build

- Milestone: M10
- Depends on: infra.localization, infra.theme-hot-reload

## Open questions

None open. Answered in M10 step 1 (D204):

- The limit values stay as proposed, in one place (`json::MAX_*`), plus `MAX_VALUES` 65,536 for the whole tree. M10's security review still confirms them.
- An integer above 2^53 is an inexact value: `Number::is_exact_int` is false and `Value::as_int` refuses it; `as_f64` still answers.
- A leading byte-order mark is skipped, and still counts in offsets and in the size limit.

## Notes for the planner

New in M3 (PD9). Lunegit's silent tolerance (a malformed theme is skipped with a warning toast, an unparsable colour is skipped, an unknown acrylic key is skipped) is the loaders' policy, not the reader's: the reader is strict, the loaders choose what to forgive [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:358-369,381-382]. The security review at M10 is a hard requirement (D12); this entry is the contract it reviews.
