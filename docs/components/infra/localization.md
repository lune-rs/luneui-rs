---
id: infra.localization
title: Localization
area: infra
status: planned
consumers: any
source: Lunegit: src/Lunegit.Infrastructure/Localization/en.json
owner: —
summary: One string table of flat dotted keys in core, luneui. keys with English defaults, positional formatting; locale JSON loaded in M10
---

## What it is

One string table in core that maps flat dotted keys to text. Toolkit keys are prefixed `luneui.`
(for example `luneui.dialog.cancel`) so they never collide with an app's keys; apps put their own
keys in the same table. `ui.tr(key)` looks up the active locale, then the English default compiled
into core, then returns the key itself, and never panics. `ui.tr_fmt(key, &args)` fills positional
`{0}` to `{n}`. M4 builds the table and the two calls; M10 only loads locale JSON in Lunegit's
`en.json` shape into this same table (no second string mechanism). Lunegit's own service is the model:
a flat key to string map, an English baseline under every language, the key as the last resort.

## Needs

- Localization of the toolkit's own text and an app's text through one table (R-infra-1).
- A bounded JSON reader behind locale loading, fail closed (R-infra-6, D44).
- Fonts and shaping for CJK text come from the app's fonts (R-text-1, D5).
- No clock, file or environment access in the library: the app reads files and passes bytes (R-nfr-1).

## Existing code

None in this repo. Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, 33a1dd41), paths relative to its root:
- `src/Lunegit.Infrastructure/Localization/en.json` (1019 strings, the baseline)
- `src/Lunegit.Infrastructure/Localization/LocalizationService.cs` (`Get`, `Format`, `SetLanguage`)
- `src/Lunegit.Infrastructure/Localization/JsonLanguageRegistry.cs` (scans the languages folder, validates files)
- `src/Lunegit.Core/Localization/LanguageFile.cs` (the file shape)
- `src/Lunegit.Desktop/Localization/LocalizationSource.cs` (hot swap through bindings), `TrExtension.cs`
- `src/Lunegit.Desktop/SampleLanguages/` (de, es, fr, ja, ko, ms, pt-BR, ru, zh-CN)

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Infrastructure/Localization/en.json:1-14,60-70,
(grep of every key with `{`), src/Lunegit.Infrastructure/Localization/LocalizationService.cs:1-130,
src/Lunegit.Infrastructure/Localization/JsonLanguageRegistry.cs:1-120, src/Lunegit.Core/Localization/LanguageFile.cs:1-21,
src/Lunegit.Desktop/Localization/LocalizationSource.cs:1-101; `crates/lune-ui-core/src/text.rs` (cache keyed by content, D27).
Instrument: a Python `json` parse of en.json at 33a1dd41 with the regexes `\{\d+\}` and `\{\d+:[^{}]*\}`: 1019 strings, 111 keys with a plain
`{n}` placeholder and 8 keys with a format specifier (119 keys in all, the survey's figure), 3 keys with `{{` or `}}`; run on a copy of the
real file.
Conflict: Lunegit's `Format` hands the whole template back unchanged when `string.Format` throws, which a missing
argument does [L src/Lunegit.Infrastructure/Localization/LocalizationService.cs:58-70]; C4 fills what it can and leaves a
missing argument's placeholder in place. The entry follows C4 (the contract M4 builds, and it never panics); the two differ only when an
argument is missing. Lunegit's `{0:N0}` and date formats are not plain `{n}`: see Open questions.

### Anatomy

- Table: flat dotted keys to text, like Lunegit's `strings` map (`common.cancel` is "Cancel") [L src/Lunegit.Infrastructure/Localization/en.json:5-8], [L src/Lunegit.Core/Localization/LanguageFile.cs:20].
- Toolkit keys start with `luneui.`; every other key is the app's; the prefix keeps the two apart [derived: C4 string rules].
- English defaults for every `luneui.` key are compiled into core as a static table; no file is read [derived: C4], [D2]. Lunegit's equivalent is the embedded `en.json`, "the canonical key registry", overlaid first under every language [L src/Lunegit.Infrastructure/Localization/LocalizationService.cs:16-18,104-130].
- `ui.tr(key)`: the active locale's value, else the English default, else the key; no panic for any key, including an empty one [derived: C4], [L src/Lunegit.Infrastructure/Localization/LocalizationService.cs:47-56]. For an app's own keys the app's English text is the locale table it loads when no other locale is active [derived: core holds English defaults for `luneui.` keys only].
- `ui.tr_fmt(key, &args)`: positional `{0}` to `{n}` over the `tr` result; `{{` and `}}` are literal braces; a missing argument leaves its placeholder; extra arguments are ignored [derived: C4 string rules; Lunegit's `string.Format` subset, `src/Lunegit.Infrastructure/Localization/LocalizationService.cs:58-70`]. Parsing runs left to right, so `stash@{{{0}}}` yields `stash@{`, argument 0, `}` [L src/Lunegit.Infrastructure/Localization/en.json:593].
- No plural rules in v1: Lunegit has none either; it uses a separate key (`toast.storage.cacheClearedPlural`) or a "(s)" suffix, and apps do the same [L src/Lunegit.Infrastructure/Localization/en.json:446,491,522].
- Active locale: a table the app holds and passes in; switching it takes effect on the next frame. Lunegit needs observers, a language epoch and a binding per key to push a change [L src/Lunegit.Desktop/Localization/LocalizationSource.cs:13-21,36]; an immediate-mode frame re-reads every key, and the shaped-text cache is keyed by content, so changed text re-shapes [D27].
- Catalog JSON (M10): `{ "code", "displayName", "version", "strings": { key: value } }` [L src/Lunegit.Infrastructure/Localization/en.json:1-5]. Required: `code`, `displayName`, `strings`; `version` is an integer for app-managed refresh and the loader ignores it [L src/Lunegit.Core/Localization/LanguageFile.cs:7-8,12-20], [L src/Lunegit.Infrastructure/Localization/JsonLanguageRegistry.cs:83-91].
- `Catalog::from_json(&[u8]) -> Result<Catalog, CatalogError>` reads through the bounded JSON reader (size, depth and string-length limits, fail-closed, no panics) and returns an error for any malformed or missing required field [D44], [L src/Lunegit.Infrastructure/Localization/JsonLanguageRegistry.cs:83-91,111-115].
- A catalog overlays the base table, so a key a translation omits falls back to English [L src/Lunegit.Infrastructure/Localization/LocalizationService.cs:90-97]. A duplicate language code is rejected by the app's list; the built-in English wins over a file claiming `en` [L src/Lunegit.Infrastructure/Localization/JsonLanguageRegistry.cs:93-103].
- App-side, not in the toolkit: scanning a languages folder, showing a warning for a skipped file, choosing the language [L src/Lunegit.Infrastructure/Localization/JsonLanguageRegistry.cs:42-47,63,88-90], and formatting numbers and dates (the app passes finished strings as arguments) [derived: no source needs more].
- Sample languages for known-answer fixtures: de, es, fr, ja, ko, ms, pt-BR, ru, zh-CN [L src/Lunegit.Desktop/SampleLanguages/de.json:1].

### States

n/a — a lookup, not a component.

### Transitions

n/a — no animation; a language switch is applied at once on the next frame.

### Input and focus

n/a — no input.

### Retained state

- Transient: none. Persistent: none.
- App-owned: the active locale table and the base table, passed into the frame; the app owns language choice and persistence [L src/Lunegit.Infrastructure/Localization/LocalizationService.cs:72-102].
- Exit retention: no.

### Accessibility

n/a — no widget. Accessible names resolve through this table [infra.accessibility].

### Strings

None: this entry defines the table, not keys. Each widget's luneui.<slug>.<name> keys and English defaults live in that widget's entry; the defaults test collects them all.

### Gallery scenes

- `dialog-confirm`, `dialog-message` (existing): once their button labels come from `ui.tr`, they must stay at zero delta in English.
- `locale-pseudo`: the confirm dialog and a menu drawn with a test locale that wraps every default in brackets and lengthens it by 30 percent, settled; it shows every toolkit string was routed through the table and the layout survives longer text.

### Painter primitives

n/a — no drawing.

### Acceptance

- Defaults test: every `luneui.` key referenced by toolkit source has a non-empty English default, and every default key matches `luneui.[a-z0-9-]+(\.[a-z0-9-]+)+` [derived: C4 defaults test].
- Fallback chain: with a locale lacking key K, `tr(K)` returns the English default; with neither, the key; `tr("")` and a 10,000-character key return without panic.
- `tr_fmt` known answers (copied from Lunegit's real strings): `Drop stash@{{{0}}} '{1}'? This cannot be undone via the UI.` with `["3", "wip"]` gives `Drop stash@{3} 'wip'? This cannot be undone via the UI.` [L src/Lunegit.Infrastructure/Localization/en.json:593]; `"{0} and {1}"` with one argument gives `x and {1}`; extra arguments are ignored; `{{` and `}}` give `{` and `}`; an unmatched `{` is kept literally and never panics [derived: C4].
- Switching the locale between two frames changes the shaped text on the second frame and nothing else (no stale cache) [D27].
- M10: `Catalog::from_json` loads a copy of Lunegit's real `en.json` (1019 strings) and each sample language file; non-object `strings`, a missing `code`, a depth bomb, an oversize input and invalid UTF-8 each return an error and never panic [D44], [L src/Lunegit.Core/Localization/LanguageFile.cs:12-20].
- M10: a placeholder audit over the real `en.json` reports the 119 placeholder-bearing keys, 111 plain and 8 with a format specifier (see Open questions); it fails on any form C4 does not format until the roadmap is revised.
- No file, clock or environment access in library source (boundary rules stay green; R-nfr-1).

### Build

- Milestone: M4 (table, `tr`, `tr_fmt`, defaults, tests) and M10 (`Catalog::from_json`, fixtures, audit).
- Depends on: core.ui, text.shaping, infra.json.

## Open questions

- Eight `en.json` keys use .NET format specifiers that plain `{n}` does not cover: `{0:N0}` (a thousands-grouped count, `toast.git.fsmonitorPromptDetail`) and date and time forms `{0:t}`, `{0:ddd}`, `{0:d MMM}`, `{0:d MMM yyyy}`, `{0:ddd d MMM yyyy}` (seven `aiSessions.time.*` keys) [L src/Lunegit.Infrastructure/Localization/en.json:396,1000-1006]. C6 says any other form extends C4 through a roadmap revision. Options: the toolkit stays plain `{n}` and the app pre-formats numbers and dates into the argument (then the Lunegit keys change to plain `{n}` when ported); or C4 gains a small specifier set. Until decided, a placeholder with a specifier is left literal by `tr_fmt`.
- An app key that starts with `luneui.` or a catalog that defines one: C4 reserves the prefix but does not say whether the table rejects it or ignores it. M4 decides; rejecting with an error at load is the fail-closed choice.

## Notes for the planner

C4's string rules are the M4 contract; this entry adds Lunegit's real behaviour as evidence and the one
gap above. Do not copy Lunegit's binding machinery (`LocalizationSource`, language epoch): it exists
because retained-mode bindings do not re-read keys.
