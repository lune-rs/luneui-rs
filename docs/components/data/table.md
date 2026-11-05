---
id: data.table
title: Data table
area: data
status: in-progress
consumers: wane
source: crates/lune-ui-core/src/widgets/table.rs
owner: toolkit-v1 M8
summary: 28 px rows with hairlines, numeric columns in mono, sortable header look, selectable rows
---

## What it is

28 px rows with hairlines, numeric columns in mono, sortable header look, selectable rows. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Click selects a row (master-detail); header click sorts (the look only is mocked); column resize, reorder and keyboard row navigation are requirements, not mocked. Long text ellipsizes.

## Needs

- Wane — Variants: cell tones: muted, warn, err, dim (no value or a reference figure), strong (the total); wrap (a label cell may take two lines); mono; num (RznMono tabular, right-aligned, header too); ellipsis. Controls inside: `widget.combo --sm`, `widget.button --sm`, `widget.toggle`, `widget.badge`. (R-data-2)
- Wane — States: row rest; hover (`--surface-hover`); selected (`--surface-selected`, bright text); sorted header (secondary text, 12 px arrow). (R-data-2)
- Wane — Sizes: Header 28 (11/16 Medium tertiary, hairline); rows 28 with 1 px `--border-subtle` hairlines, no zebra, no row radius; cell padding 0 8, first cell 16, last cell 16; body 13; wrap cells padding 4 vertical. (R-data-2)
- Wane — Behaviour: Click selects a row (master-detail); header click sorts (the look only is mocked); column resize, reorder and keyboard row navigation are requirements, not mocked. Long text ellipsizes. (R-data-2)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Wane `docs/ui/mocks/css/components.css` (data.table); no lune-ui source. Density from Luneterm glass-ui dump 06 row rhythm.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Wane `docs/ui/components.md:599-612`; `crates/lune-ui-core/src/theme.rs:55-84`
(`surface_hover`, `surface_selected`, `border_subtle`, `fg_*`, `font_*`); `grep -rn -i datagrid src/Lunegit.Desktop` at 33a1dd41: no hits (Lunegit has no data grid; its list rows are the side-panel lists, [L .claude/docs/animation-rules.md:145]).
Conflicts: none between sources. Wane's header 11/16 and body 13 are the only values; the repo has `font_s` 11 [R crates/lune-ui-core/src/theme.rs:82] and `font_base` 13 [R crates/lune-ui-core/src/theme.rs:83], which agree.

### Anatomy

- Header: 28 high; text 11, Medium, `fg_tertiary`; hairline below [W docs/ui/components.md:599-612]. Repo sizes: `font_s` 11 [R crates/lune-ui-core/src/theme.rs:82]; tertiary `fg_tertiary` #FFFFFF88 [R crates/lune-ui-core/src/theme.rs:58].
- Row: 28 high; hairline 1 px `border_subtle` #FFFFFF1A at the bottom [W docs/ui/components.md:599-612], [R crates/lune-ui-core/src/theme.rs:63]; no zebra; no row radius [W docs/ui/components.md:599-612].
- Cell: padding 0 8; first and last cell 16 [W docs/ui/components.md:599-612]; body text 13 [W docs/ui/components.md:599-612], [R crates/lune-ui-core/src/theme.rs:83].
- Numeric cell: RznMono tabular, right-aligned, header too [W docs/ui/components.md:599-612].
- Wrap cell: padding 4 vertical [W docs/ui/components.md:599-612].
- Sort arrow: 12 px in the sorted header [W docs/ui/components.md:599-612].

### States

| state | look | cite |
|---|---|---|
| row rest | no fill; text `fg_primary` | [W docs/ui/components.md:599-612], [R crates/lune-ui-core/src/theme.rs:55-56] |
| row hover | fill `surface_hover` #FFFFFF10 | [W docs/ui/components.md:599-612], [R crates/lune-ui-core/src/theme.rs:60] |
| row selected | fill `surface_selected` #FFFFFF18; `fg_bright` text ([D58]) | [W docs/ui/components.md:599-612], [R crates/lune-ui-core/src/theme.rs:61] |
| header sorted | secondary text; 12 px arrow | [W docs/ui/components.md:599-612], [R crates/lune-ui-core/src/theme.rs:57] |
| cell tone warn, err, dim, strong | the cell's colour; the values are not in the source | open question |
| focus-visible row | not in the source | open question |

### Transitions

n/a — no motion in Wane or Lunegit for a table; rows are a virtualized list and never animate per row
([L .claude/docs/animation-rules.md:145]). A selected-row change snaps.

### Input and focus

- Pointer: a click on a row selects it (master-detail) [W docs/ui/components.md:599-612]; a click on a header sorts (look only, the behaviour is a requirement) [W docs/ui/components.md:599-612]. Hover fills the row.
- Keyboard: arrow keys move the row; not mocked (requirement) [W docs/ui/components.md:599-612].
- Tab stop: the table is one stop; the controls inside are stops [core.focus].
- Layer: Base or Page, whatever the owner draws it on [derived: a container].
- Focus scope: none.
- Pointer capture while pressed: none in the sources.

### Retained state

- Transient: row hover.
- Persistent: the selection as a set of row keys; the sort key and direction; owned by the table; survives under `ui.keep_alive` [core.ui].
- App-owned: the rows, the column definitions, the cell values and tones [W docs/ui/components.md:599-612].
- Exit retention: no.

### Accessibility

- Role `table`, rows `row`, cells `cell`, headers `column-header` [infra.accessibility].
- Selected row: `selected` state; a sorted header carries the `sort` field (ascending, descending or none) [infra.accessibility], [D55].
- Live region: none.

### Strings

None: column titles and cell values are app text.

### Gallery scenes

- `table-rows` (new): five rows, one selected, one hovered, settled.
- `table-sorted` (new): a sorted header with the arrow, settled.
- `table-tones` (new): cells in muted, warn, err, dim and strong tones, settled.

### Painter primitives

- Existing: fill (row fills and hairlines), text, icon (sort arrow), clip-rect (ellipsis clip). [R crates/lune-ui-core/src/paint.rs:215,274,167]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): a click selects one row; a header click reports the sort request; arrow keys move the selection (requirement). None exists.
- Pixel probes at scale 2: row 56 px; header 56 px; hairline 2 px; cell padding 16 px; first and last cell 32 px.
- Conformance: none (no motion).
- Semantic node: `table` with `row`, `cell` and `column-header`; selected row `selected`.

### Build

- Milestone: M8.
- Depends on: layout.virtualized-list, core.painter, core.focus, core.ui, infra.accessibility, widget.combo, widget.button, widget.toggle, widget.badge.

## Open questions

- Tone colours for warn, err, dim and strong cells: no values in the sources; open.
- Keyboard row navigation: a requirement, not mocked; open.
- Column resize and reorder: requirements, not mocked; open.
- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: plain drawing; the cost is scale: the Requests log needs a virtualised list.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.table`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, sessions, upstreams.
