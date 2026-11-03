---
id: data.stat
title: Stat cell
area: data
status: planned
consumers: wane
source: —
owner: —
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

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Wane `docs/ui/mocks/css/components.css` (data.stat); no lune-ui source. Value at most 22 px (A4).

## Spec

### Evidence

Pinned: Wane 69931d6, this repo fa2958b. Read: Wane `docs/ui/components.md:585-598`; `crates/lune-ui-core/src/theme.rs:55-84`
(`fg_tertiary`, `fg_primary`, `font_s` 11, `font_base` 13). Lunegit: no stat-cell source read.
Conflicts: none. The repo has no RznMono face token; the value font is an open question (Anatomy).

### Anatomy

- Cell: padding 12 16 [W docs/ui/components.md:585-598].
- Label: 11/16, `fg_tertiary` [W docs/ui/components.md:585-598], [R crates/lune-ui-core/src/theme.rs:58,82].
- Value: RznMono Medium 20/28, tabular figures, bright, ellipsis; at most 22 px [W docs/ui/components.md:585-598].
- Sub-line: 11/16; tone ok or warn [W docs/ui/components.md:585-598].
- Strip: cells share the width; 1 px hairline between cells [W docs/ui/components.md:585-598], [R crates/lune-ui-core/src/theme.rs:63] (`border_subtle`).
- Gold: `--gold` on the net-saved figure only [W docs/ui/components.md:585-598].
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
content, [L .claude/docs/animation-rules.md:148]). The duration and easing are a [D40] choice with no source; open question.

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

None: the label, value and sub-line are app text; the placeholder "—" and "No data" are the mock's strings, not in any source (Open questions).

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

- Behaviour (scripted input, injected time): a discrete value change animates once from the old to the new value; a continuous value does not animate. None exists.
- Pixel probes at scale 2: cell padding 24 px top and bottom, 32 px sides; hairline 2 px.
- Conformance: value change reaches the new value in 180 ms (fast) under Full, snaps under Off.
- Semantic node: `group` per cell; label, value and sub-line as `static-text`.

### Build

- Milestone: M8.
- Depends on: core.painter, core.theme, data.progress (meter), motion.transitions.

## Open questions

- The RznMono face: no repo token or font entry is in the sources read; the value font is open (fonts are an app or toolkit decision; the README says RznSans).
- The value's duration and easing: [D40] choice; no source; open.
- The placeholder strings "—" and "No data": mock text, not in Lunegit; open.
- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: needs tabular figures from RznMono; none otherwise.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.stat`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, local-models, overview, pricing, savings, sessions, tray.
