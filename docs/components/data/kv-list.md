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

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Wane `docs/ui/mocks/css/components.css` (data.kv-list); no lune-ui source.

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
| value selected for copy | the text selection look; not in the source | open question |

### Transitions

n/a — static (Wane: "Static"); a value change follows the app's data without motion.

### Input and focus

- Pointer: values accept selection for copy [W docs/ui/components.md:613-626]; the rows take no click.
- Keyboard: copy of the selected text (the platform's copy); no key handling in the source.
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

- Role `group` for the list; each pair is a `group` with a `static-text` key and value [infra.accessibility].
- Values are selectable text: name = key, value = the text [infra.accessibility].
- Live region: none.

### Strings

None: keys and values are app text.

### Gallery scenes

- `kv-list-default` (new): six pairs with the longest key sizing the column, settled.
- `kv-list-stacked` (new): stacked two-column variant, settled.
- `kv-list-wrap` (new): a wrapped value over two lines, settled.

### Painter primitives

- Existing: text, clip-rect (the ellipsis). [R crates/lune-ui-core/src/paint.rs:274,167]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): a value is selectable and copies its text; rows take no clicks. None exists.
- Pixel probes at scale 2: row 48 px; key-value gap 32 px; key text at 26 px font size; the key column equals the longest key's measured width.
- Conformance: none (no motion).
- Semantic node: `group` per pair; key and value `static-text`.

### Build

- Milestone: M6.
- Depends on: core.painter, core.theme, infra.accessibility.

## Open questions

- Selection look for copy: not in any source; open.
- Whether a copy button is part of the component: not in the mock text; open.
- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: column sizing needs a measure pass (`measure` on the painter) for the longest key.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.kv-list`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, requests, routes, sessions, settings, tray, upstreams.
