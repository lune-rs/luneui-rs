---
id: overlay.flyout
title: Tray flyout
area: overlay
status: planned
consumers: wane
source: —
owner: —
summary: Windows tray left-click surface: header, stats, route list, action rows
---

## What it is

Windows tray left-click surface: header, stats, route list, action rows. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Left click on the tray icon toggles it; an outside click or losing focus closes it; the action rows behave as menu items (Open Wane, Profile, Pause compression, Unload local model, Quit Wane in the danger tone).

## Needs

- Wane — Variants: none; parts: header (mark 16, wordmark 14 bold gold, state with status dot at the right), stats (`data.stat --compact` strip), routes (`data.kv-list`, ids mono, a state note such as "loaded"), separator, action rows (`overlay.menu` items). (R-overlay-3)
- Wane — States: open (menu motion); the state text follows the tray state (Serving, degraded, paused). (R-overlay-3)
- Wane — Sizes: 340 wide, padding 5, radius 9, 1 px `--popup-border`, fill `--menu-glass` with blur 30 / saturate 1.8; header padding 6 8; stats margin 8 0; routes padding 0 8. Windows: 12 px above the taskbar and 12 px from the right edge. (R-overlay-3)
- Wane — Behaviour: Left click on the tray icon toggles it; an outside click or losing focus closes it; the action rows behave as menu items (Open Wane, Profile, Pause compression, Unload local model, Quit Wane in the danger tone). (R-overlay-3)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: partial: the glass popup surface exists (popup_panel widgets/mod.rs:200; menu glass widgets/menu.rs:109); the stats block and the separate window are missing). Design source (not code to move): Luneterm crates/lune-ui/src/widgets/menu.rs:109-111 (backdrop, glass fill, ring); Luneium src/renderer/shell/components/Popup.css:2; docs/mockups/Tray.dc.html (content).

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: Needs a separate borderless, always-on-top, non-activating glass window anchored to the tray rect, with the in-app blur over the real desktop (platform work, section 5). The mock fixes its position; the real window must be placed from the tray icon's rect and kept on screen.

## Notes for the planner

Wane priority P2 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `overlay.flyout`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, tray.
