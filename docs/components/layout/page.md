---
id: layout.page
title: Page card (header, scroll body, footer)
area: layout
status: planned
consumers: wane
source: —
owner: —
summary: The card that holds a screen: header with title and actions, scrolling body, optional pinned footer
---

## What it is

The card that holds a screen: header with title and actions, scrolling body, optional pinned footer. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: The body scrolls vertically only (`widget.scrollbar`); the layout responds to width, never to height (`?full=1` grows the window to the content). The footer stays pinned.

## Needs

- Wane — Variants: with footer (`c-page__footer`, the wizard) (R-layout-1)
- Wane — States: inactive window: the card background becomes the solid inactive colour. A disabled primary in the footer keeps its place. (R-layout-1)
- Wane — Sizes: Radius 8 (`--radius-panel`), 1 px `--border-subtle`, fill `--glass-card` (rgba 50 50 50 .808, Lunegit's main-content acrylic: black tint 0.8, material 0.7); margin 0 10 10 6; header padding 16, title 18/24 Medium bright, subtitle 13/20 tertiary, actions right with 8 gap; body padding 0 16 16; footer padding 12 16, top hairline, secondary action left, primary group right. (R-layout-1)
- Wane — Behaviour: The body scrolls vertically only (`widget.scrollbar`); the layout responds to width, never to height (`?full=1` grows the window to the content). The footer stays pinned. (R-layout-1)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: partial: the CARD glass layer exists (theme.rs:101, CARD_CORNER_RADIUS theme.rs:111); header, scroll body and footer are missing). Design source (not code to move): Lunegit Desktop App/src/Lunegit.Desktop/Themes/DarkTheme.axaml:226-233 (GlassTintLightOpacity 0.8, GlassMaterialLightOpacity 0.7) used at Views/RepositoryContentView.axaml:47-58 (content acrylic); Luneterm docs/glass-styling.md section 4 (card layer, radius 8); Luneterm crates/lune-ui/src/theme.rs:101; Luneterm spikes/glass-ui dump 06.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: CSS `--glass-card` flattens Lunegit's content acrylic (black tint 0.8, material 0.7; derivation in `mocks/css/tokens.css`), not lune-ui's denser CARD layer (rgba 37 37 37 .89); luneui-rs paints the real layer (`core.acrylic`) and should take these Lunegit levels as its CARD material. The card needs scroll clipping with the radius.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `layout.page`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, savings, sessions, settings, upstreams.
