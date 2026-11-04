---
id: data.kv-list
title: Key-value list
area: data
status: in-progress
consumers: wane
source: —
owner: toolkit-v1 M6
summary: Aligned key and value pairs: facts, paths, ids, figures
---

## What it is

Aligned key and value pairs: facts, paths, ids, figures. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static; values are selectable for copy (ids, paths).

## Needs

- Wane — Variants: default (key column sized by the longest key); stacked (two columns, key 11 over a 13 mono value); mono and wrap values. (R-data-2)
- Wane — Sizes: Grid with a max-content key column and 16 px gap; rows 24; key 13 tertiary, value 13 primary with ellipsis; wrap value 20 line height with 2 px padding; stacked key 11/16. (R-data-2)
- Wane — Behaviour: Static; values are selectable for copy (ids, paths). (R-data-2)

## Existing code

`crates/lune-ui-core/src/widgets/kv_list.rs` (`KvOptions`, `Ui::kv_list`) and `crates/lune-ui-core/src/widgets/selectable.rs` (`Ui::selectable_text`), M6. Gallery page `apps/gallery/src/pages/kv_list.rs`; tests `crates/lune-ui-core/tests/widget_kv_list.rs`. Design source (not code moved): Wane `docs/ui/mocks/css/components.css` (data.kv-list); no lune-ui source.

## Spec

### Evidence

Pinned: Wane 69931d6, this repo fa2958b. Read: Wane `docs/ui/components.md:613-626`; `crates/lune-ui-core/src/theme.rs:55-84` (`fg_secondary`, `fg_tertiary`, `font_s`, `font_base`);.
Conflicts: none. Lunegit has no kv-list source read; not used for values.

### Anatomy

- Grid: key column max-content; 16 px gap between columns [W docs/ui/components.md:613-626].
- Row: 24 high [W docs/ui/components.md:613-626].
- Key: 13, `fg_tertiary` [W docs/ui/components.md:613-626], [R crates/lune-ui-core/src/theme.rs:58,83].
- Value: 13, `fg_primary`, ellipsis when long [W docs/ui/components.md:613-626], [R crates/lune-ui-core/src/theme.rs:56].
- Wrap value: 20 line height with 2 px padding [W docs/ui/components.md:613-626].
- Stacked: key 11 over value 13 mono, two columns [W docs/ui/components.md:613-626], [R crates/lune-ui-core/src/theme.rs:82].

### States

| state | look | cite |
|---|---|---|
| rest (the only interaction state) | key and value as above | [W docs/ui/components.md:613-626] |
| value selected for copy | `text_selection` fill under the text, while the value has focus [D125] | [D125] |

### Transitions

n/a — static (Wane: "Static"); a value change follows the app's data without motion.

### Input and focus

- Pointer: values accept selection for copy [W docs/ui/components.md:613-626]; a press focuses the value and places the caret, a drag, a double press and Shift extend it. The rows, the keys and the space beside a value take no input: the hit region is the drawn text. An elided value selects as a whole: any press selects all of it and Copy writes all of the full text [D125].
- Keyboard: the platform's copy and select-all on the focused value; the caret keys move the selection of a value shown in full; an elided value ignores them, so a partial selection of it cannot exist.
- Tab stop: no; the values are not controls [derived: static text, W docs/ui/components.md:613-626].
- Layer: whatever layer the owner draws it on [derived: a container].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: the text selection (while a value is selected).
- Persistent: none.
- App-owned: the keys, the values, the column mode (default, stacked, mono, wrap) [W docs/ui/components.md:613-626].
- Exit retention: no.

### Accessibility

- Each pair is a `group` named by its key with a `static-text` key and a `static-text` value [infra.accessibility]; the list itself has no node of its own.
- Values are selectable text: name = key, value = the full text, with the selection [infra.accessibility].
- Live region: none.

### Strings

None: keys and values are app text.

### Gallery scenes

- `kv-list-default` (new): six pairs with the longest key sizing the column, settled.
- `kv-list-stacked` (new): stacked two-column variant, settled.
- `kv-list-wrap` (new): a wrapped value over two lines, settled.
- `kv-list-selected` (new): the commit value selected whole, focused by a press, settled.

### Painter primitives

- Existing: text, clip-rect (the ellipsis). [R crates/lune-ui-core/src/paint.rs:274,167]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): a value is selectable and copies its text; rows take no clicks. Tests `widget_kv_list.rs` (a drag then Copy writes the substring; a press on an elided value then Copy writes the whole; Tab skips every value; a press outside a value does nothing).
- Pixel probes at scale 2: row 48 px; key-value gap 32 px; key text at 26 px font size; the key column equals the longest key's measured width.
- Conformance: none (no motion).
- Semantic node: `group` per pair; key and value `static-text`.

### Build

- Milestone: M6.
- Depends on: core.painter, core.theme, infra.accessibility.

## Open questions

- None. The selection look (`text_selection`), no copy button and a toolkit component are settled by [D125]. Column sizing measures the longest key with `Painter::measure`.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.kv-list`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, requests, routes, sessions, settings, tray, upstreams.
