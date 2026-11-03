---
id: platform.tray-icon
title: Tray icon with status dot
area: platform
status: planned
consumers: wane
source: —
owner: —
summary: 16 px monochrome glyph with a status dot: green serving, amber degraded, grey paused
---

## What it is

16 px monochrome glyph with a status dot: green serving, amber degraded, grey paused. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Left click opens the flyout (Windows) or the menu (macOS); right click opens the menu on Windows. The tooltip names the state in words, the second channel for colour-vision (section 8). Not mocked: animation, badge counts.

## Needs

- Wane — Variants: `--2x` (the same icon at 32 px for high-DPI trays) (R-platform-6)
- Wane — States: serving (`__dot--ok`), degraded (`__dot--warn`), paused (`__dot--paused`). The three differ only by the dot colour; the glyph stays monochrome. (R-platform-6)
- Wane — Sizes: 16 px glyph; 6 px dot at right -2 / bottom -2 with a 1.5 px cut-out ring in the surface colour; `--2x`: 32 px glyph, 12 px dot, 3 px ring. The glyph is the provisional crescent (Q1), not a Codicon. (R-platform-6)
- Wane — Behaviour: Left click opens the flyout (Windows) or the menu (macOS); right click opens the menu on Windows. The tooltip names the state in words, the second channel for colour-vision (section 8). Not mocked: animation, badge counts. (R-platform-6)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): docs/PLAN.md:511-512 (tray icon colours); `docs/ui/mocks/tray.html?surface=icon-states`; provisional crescent from docs/mockups/Tray.dc.html:44.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: The OS draws the tray slot. luneui-rs (or the platform layer) must supply the icon as OS bitmaps at the tray's sizes, rendered from the same vector with the dot composited in, and a template (monochrome) image on macOS. The ring colour in the mock is the taskbar or menu-bar colour; a real icon cannot know it, so the dot needs a ring that works on light and dark trays.

## Notes for the planner

Wane priority P2 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `platform.tray-icon`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, tray.
