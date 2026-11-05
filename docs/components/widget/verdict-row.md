---
id: widget.verdict-row
title: Verdict check row
area: widget
status: in-progress
consumers: wane
source: —
owner: toolkit-v1 M6
summary: Verdict row: mark OK, WARN or NO in its tone, then name and text; three-column or stacked; static
---

## What it is

`verdict-row`: check row with a mark (OK, WARN or NO) in the tone, a name and a text. Three columns by default, or stacked for a narrow column. Static; the tone is on the mark only [W docs/ui/components.md:517-521].

## Needs

- Wane controls the toolkit lacks (R-widget-4).

## Existing code

`crates/lune-ui-core/src/widgets/verdict_row.rs` (M6): `Verdict`, `Ui::verdict_row`. Gallery page `apps/gallery/src/pages/verdict_row.rs`; tests `crates/lune-ui-core/tests/widget_verdict_row.rs`. Wane is the only source for this entry.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:515-527,686`; `docs/ui/mocks/css/tokens.css:13,14,49,52,55,125`; Lunegit `src/Lunegit.Desktop/Themes/DarkTheme.axaml:115-117`.
Conflict: the mark tones. Wane's `--ok` #6fd39a and `--warn` #f2a65a against Lunegit's tones for the same meanings; Lunegit's win, the tokens `ok` #73C991, `warn` #E2B93D, `err` #E06C75 of [D58], as in `widget.badge`.

### Anatomy

- Rows min 24 high; grid 40 / 92 / 1fr [W docs/ui/components.md:520].
- Mark: RznMono 11 Medium [W docs/ui/components.md:520]; name 13 tertiary (stacked: primary, as `.c-verdict--stacked .c-verdict__name` in the mock) [W docs/ui/components.md:520], [W docs/ui/mocks/css/tokens.css:14]; text 13/20 primary [W docs/ui/components.md:520], [W docs/ui/mocks/css/tokens.css:12,125].
- Stacked (`--stacked`): grid 40 / 1fr, padding 4 0; mark and name on line 1; text 11 secondary on a 16 px line below, wrapping (row 90 px at scale 2 for one line); hairlines between rows (the widget draws one under every stacked row: it cannot know the last row, [D130]); for a column about 214 px wide [W docs/ui/components.md:518,686].

### States

| state | look | cite |
|---|---|---|
| OK | mark in `ok` #73C991 | [W docs/ui/components.md:519], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:116], [D58] |
| WARN | mark in `warn` #E2B93D | [W docs/ui/components.md:519], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:115], [D58] |
| NO | mark in `err` #E06C75 | [W docs/ui/components.md:519], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:117], [D58] |

Static: no hover, focus or disabled look [W docs/ui/components.md:521].

### Transitions

n/a — static [W docs/ui/components.md:521].

### Input and focus

n/a — static [W docs/ui/components.md:521].

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: the verdict, the name, the text (passed every frame).
- Exit retention: no.

### Accessibility

- Role `row` with `cell`s; name = the name; value = the mark word; the mark's tone is not the only channel, the word carries it [infra.accessibility].

### Strings

The mark words are the toolkit strings `luneui.verdict-row.ok` (OK), `luneui.verdict-row.warn` (WARN) and `luneui.verdict-row.no` (NO), read through `ui.tr` so an app's table can translate them [D129]. The name and the text are app data.

### Gallery scenes

- `verdict-row-states`: OK, WARN and NO in three columns, then the same three stacked; settled.

### Painter primitives

- Existing: text
- New: none

### Acceptance

- Pixel probes at scale 2: row min 48 px; grid columns 80 px and 184 px; stacked column 428 px.
- Conformance: n/a — no transition.
- Semantic node: role row; name equals the name passed.

### Build

- Milestone: M6
- Depends on: core.painter, text.shaping

## Open questions

- None. (The mark words' source, once unsettled, is answered by [D129]: toolkit strings.)

## Notes for the planner

Wane P1. Used by the local-models quant cards in the mocks.
