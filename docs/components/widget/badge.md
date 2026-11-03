---
id: widget.badge
title: Badge
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: 18 px pill with a tone: neutral, ok, warn, err, data
---

## What it is

18 px pill with a tone: neutral, ok, warn, err, data. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static label (Live, upstream kind, verdict, risk level).

## Needs

- Wane — Variants: neutral (default); ok; warn; err; data (R-widget-4)
- Wane — States: none (not interactive) (R-widget-4)
- Wane — Sizes: Height 18, padding 0 6, radius 8 (`--radius-badge`), 1 px border, text 11/16 Medium, gap 6 for a leading dot. Tone recipe: fill tone at 1A, border tone at 44, text tone (data uses `--data-hi`); neutral: `--surface-overlay`, `--border-subtle`, secondary. (R-widget-4)
- Wane — Behaviour: Static label (Live, upstream kind, verdict, risk level). (R-widget-4)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Lunegit Desktop App/src/Lunegit.Desktop/Views/DiffView.axaml:12-16 (shape); DarkTheme.axaml:119-128 (tone recipe); Lunegit App.axaml:221-223.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: None; the 1A and 44 alpha fills are plain colours.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `widget.badge`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, local-models, overview, requests, savings, sessions, upstreams.
