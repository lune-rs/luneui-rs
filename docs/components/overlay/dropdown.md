---
id: overlay.dropdown
title: Drop-down list
area: overlay
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui/src/widgets/dropdown.rs
owner: orch/toolkit-v1
summary: Combo drop-down option list that flips above its anchor near the window bottom
---

## What it is

`dropdown`: an option list that flips above its `anchor` near the window bottom.

## Needs

- Drop-down list that stays on screen (R-overlay-1).
- Wane — States: item rest; hover (`--surface-hover`); selected (`--surface-selected`, 12 px check at the right); disabled (tertiary text, inert). (R-overlay-1)
- Wane — Sizes: Panel padding 4, radius 9, 1 px `--popup-border-soft` (white at .08; user, 2026-11-04), the same glass as `overlay.menu` (`--menu-glass`, blur 30; user, 2026-11-03: not lune-ui's opaque `dropdown_bg`); rows 32 high, radius 4, padding 0 6 0 9; 4 to 5 px below the anchor, same width; no shadow. (R-overlay-1)
- Wane — Behaviour: Opens under the combo and flips above it near the window bottom; a click outside or Esc closes it; Up, Down, Enter choose. Not mocked: type-ahead, scrolling a long list. (R-overlay-1)

## Existing code

`crates/lune-ui/src/widgets/dropdown.rs:9` (`dropdown`); 54 lines.

## Open questions

- Scrolling for long option lists is not in the inventory (R-layout-2).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: partial: widgets/dropdown.rs:9 draws an opaque `dropdown_bg` panel; the menu glass (theme.rs `menu_glass`) is missing on it; Wane priority P0. Native note: Drawn on the modal or popup layer, above scrolling content; the mock's `position: absolute` must become a popup layer with window-edge flipping. Full spec: Wane `docs/ui/components.md`, section `overlay.dropdown`; mock: Wane `docs/ui/mocks/components.html`.
