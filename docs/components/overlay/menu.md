---
id: overlay.menu
title: Glass context menu
area: overlay
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/widgets/menu.rs
owner: —
summary: Glass context menu with open and close motion
---

## What it is

`menu` with `MenuEntry`, built from Luneium's ContextMenu.css and Popup.css; `menu_open_anim` and `menu_close_anim` drive its motion (`MENU_IN`, `MENU_OUT`).

## Needs

- Glass context menu (R-overlay-1).
- Open and close motion with the app passing time in (R-motion-1).

## Existing code

`crates/lune-ui/src/widgets/menu.rs:64` (`menu`), `:9` (`MenuEntry`), `:46` (`menu_open_anim`), `:52` (`menu_close_anim`); 192 lines; tests at `:163`.

## Open questions

- Submenus, keyboard navigation and separators are not in the inventory.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
