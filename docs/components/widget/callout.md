---
id: widget.callout
title: Callout
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Inline note with an icon, text and an optional action; neutral, data, warn, err
---

## What it is

Inline note with an icon, text and an optional action; neutral, data, warn, err. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static text with an optional action button.

## Needs

- Wane — Variants: neutral (default); data; warn; err. Parts: icon, body, title (500), figure (`__figure`, the one number a neutral callout is about: mono, bright, never gold), action (a button at the right). (R-widget-4)
- Wane — States: none; not dismissible in the mocks. Used for the budget alert, the disconnected and proxy-offline notices, the probe note and the tier preview. (R-widget-4)
- Wane — Sizes: Padding 8 10, radius 6, 1 px border; icon 14 in a 20 px slot, gap 8; text 13/20; neutral fill `--surface-input`, border `--border-subtle`; tone fills use the badge recipe (tone at 1A, border at 44, tone-coloured icon). No full-width bar, no accent stripe. (R-widget-4)
- Wane — Behaviour: Static text with an optional action button. (R-widget-4)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Lunegit Desktop App/src/Lunegit.Desktop/Controls/IndexingBanner.axaml:8-24 (1 px tone border, radius 6, 14 px icon).

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `widget.callout`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, overview, pricing, upstreams.
