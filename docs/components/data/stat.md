---
id: data.stat
title: Stat cell
area: data
status: in-progress
consumers: wane
source: —
owner: toolkit-v1 M6
summary: A labelled figure in a strip: label, mono value, sub-line, optional meter
---

## What it is

A labelled figure in a strip: label, mono value, sub-line, optional meter. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static. Not a card: a strip (`c-panel--strip`) of cells.

## Needs

- Wane — Variants: default; gold (`--gold`, the net-saved figure only, D6); text (value in sans 14, for a word); compact (flyout: padding 6 8, value 14/20); meter (`__meter`: a `data.progress` closes the cell). (R-data-2)
- Wane — States: no data: value "—", sub-line "No data" (no gold on a placeholder); sub-line tone ok or warn for a good or bad delta. (R-data-2)
- Wane — Sizes: Padding 12 16; label 11/16 tertiary; value RznMono Medium 20/28 tabular, bright, ellipsis; sub-line 11/16; strip cells share the width with a 1 px hairline between them. (R-data-2)
- Wane — Behaviour: Static. Not a card: a strip (`c-panel--strip`) of cells. (R-data-2)

## Existing code

`crates/lune-ui-core/src/widgets/stat.rs` (M6): `StatOptions`, `Ui::stat`, `Ui::stat_strip`. Gallery page `apps/gallery/src/pages/stat.rs`; tests `crates/lune-ui-core/tests/widget_stat.rs`. Design source (not code moved): Wane `docs/ui/mocks/css/components.css` (data.stat); no lune-ui source. Value at most 22 px (A4).

## Spec

### Evidence

Pinned: Wane 69931d6, this repo fa2958b. Read: Wane `docs/ui/components.md:585-598`; `crates/lune-ui-core/src/theme.rs:55-84`
(`fg_tertiary`, `fg_primary`, `font_s` 11, `font_base` 13). Lunegit: no stat-cell source read.
Conflicts: none. The repo has no RznMono face token; the value font is an open question (Anatomy).

### Anatomy

- Cell: padding 12 16 [W docs/ui/components.md:585-598].
- Label: 11/16, `fg_tertiary` [W docs/ui/components.md:585-598], [R crates/lune-ui-core/src/theme.rs:58,82].
- Value: RznMono Medium 20/28, tabular figures, `fg_bright` (#FFFFFFDD, [D58]), ellipsis; at most 22 px [W docs/ui/components.md:585-598].
- Sub-line: 11/16; tone `ok` or `warn` [W docs/ui/components.md:585-598], the tokens of [D58] (#73C991, #E2B93D, [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:116,115]).
- Strip: cells share the width; 1 px hairline between cells [W docs/ui/components.md:585-598], [R crates/lune-ui-core/src/theme.rs:63] (`border_subtle`).
- Gold: `gold` (#E9C46A, [W docs/ui/mocks/css/tokens.css:48], [D58]) on the net-saved figure only [W docs/ui/components.md:585-598].
- Compact (flyout): padding 6 8; value 14/20 [W docs/ui/components.md:585-598].
- Meter: a `data.progress` closes the cell [W docs/ui/components.md:585-598].

### States

| state | look | cite |
|---|---|---|
| value | label, value, sub-line as above | [W docs/ui/components.md:585-598] |
| no data | value "—", sub-line "No data"; no gold | [W docs/ui/components.md:585-598] |
| delta ok | sub-line ok tone | [W docs/ui/components.md:585-598] |
| delta warn | sub-line warn tone | [W docs/ui/components.md:585-598] |
| gold | the net-saved figure only | [W docs/ui/components.md:585-598] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| value changes on a discrete update | value | value | fast | linear | keep | [D40], [motion] |

The value animates on a discrete change only. Never animated: a continuously updating value (high-frequency
content, [L .claude/docs/animation-rules.md:148]). The duration and easing are a [D40] choice with no source, settled by [D126]: the `value` kind at `fast`, linear, kept under Reduced (its 120 ms) and snapped under Off. The tween runs on a number the app passes with its formatter (`StatOptions.number`); at rest the drawn text is exactly the app's `value`; a text-only value and a `live` one snap.

### Input and focus

- Pointer: none; the cell is static [W docs/ui/components.md:585-598].
- Keyboard: none.
- Tab stop: no.
- Layer: whatever layer the owner draws it on [derived: a container].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: the value transition progress.
- Persistent: none.
- App-owned: label, value, sub-line text and tone, the gold flag, the meter fraction [W docs/ui/components.md:585-598].
- Exit retention: no.

### Accessibility

- Cell: role `group` named by the label; the value is `static-text`; the sub-line is `static-text` [infra.accessibility].
- No data: the value is "—" and the sub-line "No data" as text [W docs/ui/components.md:585-598].
- Live region: none.

### Strings

The label, value and sub-line are app text. The placeholder value "—" is a literal; the sub-line "No data" is the toolkit string `luneui.stat.no-data` [D126].

### Gallery scenes

- `stat-default` (new): a strip of three cells with ok and warn sub-lines, settled.
- `stat-gold` (new): one gold cell, settled.
- `stat-no-data` (new): the no-data placeholder, settled.
- `stat-meter` (new): a cell closed by a progress meter, settled.
- `stat-compact` (new): the compact variant in the flyout width, settled.
- `stat-value-mid` (new): a value change at t = 90 ms (half of fast, 180 ms) [L src/Lunegit.Desktop/App.axaml:152].

### Painter primitives

- Existing: fill (hairline), text, clip-rect (the value ellipsis). [R crates/lune-ui-core/src/paint.rs:215,274,167]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): a discrete value change animates once from the old to the new value; a continuous value does not animate. Tests `widget_stat.rs` (`a_change_of_number_animates_once_from_the_old_to_the_new`, `a_live_or_text_only_value_never_puts_the_frame_in_flight`).
- Pixel probes at scale 2: cell padding 24 px top and bottom, 32 px sides; hairline 2 px.
- Conformance: value change reaches the new value in 180 ms (fast) under Full, snaps under Off.
- Semantic node: `group` per cell; label, value and sub-line as `static-text`.

### Build

- Milestone: M6.
- Depends on: core.painter, core.theme, data.progress (meter), motion.transitions.

## Open questions

- None. The value font is `Font::Mono`, which the app supplies (RznMono or other) [D129]; the duration, easing and placeholder strings are settled by [D126], which also makes this a toolkit component (M7's flyout needs it). Native rendering of tabular figures is the app's face.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.stat`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, local-models, overview, pricing, savings, sessions, tray.
