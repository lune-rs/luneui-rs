---
id: data.kv-list
title: Key-value list
area: data
status: planned
consumers: wane
source: —
owner: —
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

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: Column sizing needs a measure pass (`measure` on the painter) for the longest key.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.kv-list`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, requests, routes, sessions, settings, tray, upstreams.
