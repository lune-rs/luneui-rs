---
id: data.progress
title: Progress bar
area: data
status: planned
consumers: wane
source: —
owner: —
summary: 4 px track with an accent fill; warn and err only for meaning
---

## What it is

4 px track with an accent fill; warn and err only for meaning. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static. Indeterminate motion is not mocked.

## Needs

- Wane — Variants: default (accent); warn; err; with a head (name 13 left, RznMono value right) (R-data-2)
- Wane — States: a determinate fill only; the probe scene uses a pending row instead of an indeterminate bar. (R-data-2)
- Wane — Sizes: Track 4 high, radius 4, fill `--surface-overlay`; fill `--accent`; head 13/20 with 4 px below. (R-data-2)
- Wane — Behaviour: Static. Indeterminate motion is not mocked. (R-data-2)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Lunegit Desktop App/src/Lunegit.Desktop/App.axaml:253.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).

## Notes for the planner

Wane priority P1 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.progress`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, pricing, sessions.
