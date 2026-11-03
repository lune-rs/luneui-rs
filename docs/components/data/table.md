---
id: data.table
title: Data table
area: data
status: planned
consumers: wane
source: —
owner: —
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

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: Plain drawing; the cost is scale: the Requests log needs a virtualised list.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.table`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, sessions, upstreams.
