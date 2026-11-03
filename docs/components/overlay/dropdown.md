---
id: overlay.dropdown
title: Drop-down list
area: overlay
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/widgets/dropdown.rs
owner: —
summary: Combo drop-down option list that flips above its anchor near the window bottom
---

## What it is

`dropdown`: an option list that flips above its `anchor` near the window bottom.

## Needs

- Drop-down list that stays on screen (R-overlay-1).

## Existing code

`crates/lune-ui/src/widgets/dropdown.rs:9` (`dropdown`); 54 lines.

## Open questions

- Scrolling for long option lists is not in the inventory (R-layout-2).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
