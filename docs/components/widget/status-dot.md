---
id: widget.status-dot
title: Status dot
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: 8 px tone dot: ok, warn, err, data, paused
---

## What it is

8 px tone dot: ok, warn, err, data, paused. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static. The dot never stands alone: a word beside it (state, name) carries the meaning, so colour is not the only channel.

## Needs

- Wane — Variants: ok; warn; err; data; paused (neutral) (R-widget-4)
- Wane — States: none; no glow, ring or pulse. Paused also means "not checked" (upstream health while the proxy is stopped) and "ended" (Sessions). (R-widget-4)
- Wane — Sizes: 8 px circle (`--radius-round`); the tray icon's dot is a separate part of `platform.tray-icon`. (R-widget-4)
- Wane — Behaviour: Static. The dot never stands alone: a word beside it (state, name) carries the meaning, so colour is not the only channel. (R-widget-4)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Step 1 footer dot (`docs/ui/mocks/css/components.css` widget.status-dot); tones from `docs/ui/mocks/css/tokens.css`.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `widget.status-dot`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, savings, sessions, settings, tray, upstreams.
