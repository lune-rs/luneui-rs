---
id: overlay.menu
title: Glass context menu
area: overlay
status: in-luneterm
consumers: luneterm, wane
source: crates/lune-ui/src/widgets/menu.rs
owner: —
summary: Glass context menu with open and close motion
---

## What it is

`menu` with `MenuEntry`, built from Luneium's ContextMenu.css and Popup.css; `menu_open_anim` and `menu_close_anim` drive its motion (`MENU_IN`, `MENU_OUT`).

## Needs

- Glass context menu (R-overlay-1).
- Open and close motion with the app passing time in (R-motion-1).
- Wane — Variants: plain; with a two-line header (`__header`: name with a status dot, status line); danger item (`c-menu__item--danger`, `--err` text; Quit Wane); chevron hints (submenu); shortcut hints (11 px quaternary) (R-overlay-1)
- Wane — States: item rest; hover (`--popup-item-hover`); pressed (`--popup-item-pressed`); disabled (`--menu-disabled`); menu opening and closing (motion below). (R-overlay-1)
- Wane — Sizes: Min width 190, padding 5, radius 9 (`--radius-menu`), 1 px `--popup-border-soft` (white at .08; user, 2026-11-04), fill `--menu-glass` (rgb 44 44 44 / .616); items 28 high, radius 4, padding 0 8, label 12 (`--font-menu`), icon slot 14 (13 px glyph), hint 11; separator slot 11 high with a 1 px line. (R-overlay-1)
- Wane — Behaviour: Opens at the pointer; click or Enter picks an item; Esc or an outside click closes. Motion: open .18 s ease-out from 4 px lower at 97%, close .12 s ease-in to 2 px lower at 98%, origin top-left (menu.rs:21-22). Arrow keys and shortcut routing are requirements, not mocked. (R-overlay-1)

## Existing code

`crates/lune-ui/src/widgets/menu.rs:64` (`menu`), `:9` (`MenuEntry`), `:46` (`menu_open_anim`), `:52` (`menu_close_anim`); 192 lines; tests at `:163`.

## Open questions

- Submenus, keyboard navigation and separators are not in the inventory.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists widgets/menu.rs:64 (the macOS variant adds a two-line header and chevron hints); Wane priority P0. Native note: `backdrop-filter: blur(30px) saturate(1.8)` approximates lune-ui's backdrop pass (`render.blur`); luneui-rs draws the real blur. On macOS the tray menu may be the native NSMenu instead (D8 chose the glass surface, so the stats header fits). Full spec: Wane `docs/ui/components.md`, section `overlay.menu`; mock: Wane `docs/ui/mocks/components.html`.
