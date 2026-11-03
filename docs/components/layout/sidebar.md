---
id: layout.sidebar
title: Sidebar
area: layout
status: planned
consumers: wane
source: —
owner: —
summary: 208 px column on the window glass: nav list (or wizard steps) and a status footer
---

## What it is

208 px column on the window glass: nav list (or wizard steps) and a status footer. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Nav items navigate; the footer is static text. The wizard sidebar has no nav and no links (Back and Continue only).

## Needs

- Wane — Variants: main (nav items); wizard (`widget.steps` fills the nav slot, `role=navigation`, label "Setup steps") (R-layout-1)
- Wane — States: footer ok (ok dot, "Proxy on 127.0.0.1:4646"); footer error (err dot, "Proxy stopped", no address; the page notice carries the detail). (R-layout-1)
- Wane — Sizes: 208 wide (`--sidebar-w`), no card or fill of its own (it sits on the window glass); nav padding 8, items 2 apart; footer padding 12 16, top hairline `--border-subtle`, text 11/16 tertiary, addresses and `~/.wane` in RznMono; 10 px bottom margin (`--gutter`). (R-layout-1)
- Wane — Behaviour: Nav items navigate; the footer is static text. The wizard sidebar has no nav and no links (Back and Continue only). (R-layout-1)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Luneterm spikes/glass-ui/src/settings_view.rs:209 (nav column, 190 wide there); `docs/ui/mocks/css/components.css` (layout.sidebar block).

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: No blur of its own: it shows the window material. Must collapse or hide below a minimum window width (not mocked; the layout responds to width only).

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `layout.sidebar`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, savings, sessions, settings, upstreams.
