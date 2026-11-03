---
id: overlay.popup-panel
title: Popup panel surface
area: overlay
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/widgets/mod.rs
owner: orch/toolkit-v1
summary: Menu and dropdown surface (popup_panel)
---

## What it is

`popup_panel`: the glass surface menus and dropdowns draw on.

## Needs

- Popup surface shared by menu and dropdown (R-overlay-1).

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:200` (`popup_panel`).

## Open questions

- Tooltip and flyout reuse is open (R-overlay-3).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
