---
id: layout.panel
title: Panel (group box)
area: layout
status: planned
consumers: wane
source: —
owner: —
summary: Outline-only group box with title, meta, body and action footer
---

## What it is

Outline-only group box with title, meta, body and action footer. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static container. Not collapsible in the mocks.

## Needs

- Wane — Variants: `--flush` (table or list edge to edge, hairline under the header); `--strip` (row of stat cells with hairline dividers); `--card` (a panel inside a panel, 12 padding, children stacked, footer button pinned to the bottom). (R-layout-1)
- Wane — States: unavailable body: one tertiary line, "Unavailable while the proxy is stopped." Panels in a row share a bottom edge (layout rule). (R-layout-1)
- Wane — Sizes: Radius 8, 1 px `--border-subtle`, no fill, no shadow; header padding 12 16, title 14/20 Medium, meta 11/16 tertiary at the right; body padding 0 16 16 (`__body--rows`: 0 4 8); footer padding 12 16, top hairline, note 11/16 tertiary at the left, secondary then primary button at the right. (R-layout-1)
- Wane — Behaviour: Static container. Not collapsible in the mocks. (R-layout-1)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Luneterm spikes/glass-ui/src/settings_view.rs:280 (draw_group), :282-345; glass-ui dump 06.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: Outline only, so nothing to approximate; the panel must clip children to the radius.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `layout.panel`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, savings, sessions, settings, tray, upstreams.
