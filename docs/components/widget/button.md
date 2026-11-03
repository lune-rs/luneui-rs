---
id: widget.button
title: Button
area: widget
status: in-luneterm
consumers: luneterm, wane
source: crates/lune-ui/src/widgets/mod.rs
owner: —
summary: Button and its measured width
---

## What it is

`button` and `button_width` (its measured width).

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Wane — Variants: secondary (default); primary (accent fill); ghost (transparent, secondary text); icon (28 x 28); sm (24 high); fill (container width). A screen has at most one primary. (R-widget-1)
- Wane — States: rest; hover (`--surface-selected`; primary `--accent-hover`); active (`--surface-active`); focus (1 px `--text-secondary` outline, inset); disabled (opacity .4, inert). (R-widget-1)
- Wane — Sizes: Height 32 (`--control-h`), radius 6, padding 0 16, label 13/20, icon 16 with 8 gap; width = text + 32 (lune-ui). sm: height 24, radius 4, padding 0 12. Icon: 28 x 28, sm 24 x 24. (R-widget-1)
- Wane — Behaviour: Click, Enter and Space activate; Tab focus with the visible ring; hover fill in .12 s (`--dur-instant`). (R-widget-1)

## Existing code

`crates/lune-ui/src/widgets/mod.rs:146`. `widgets/mod.rs` has 203 lines and mostly no tests.

## Open questions

- Keyboard focus and key handling for this control are not in the inventory (R-widget-3).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists widgets/mod.rs:146 (primary, secondary); partial: ghost, icon, small and fill variants missing; Wane priority P0. Native note: Flat fills, no CSS-only effect; hover and focus are cheap in the painter. Full spec: Wane `docs/ui/components.md`, section `widget.button`; mock: Wane `docs/ui/mocks/components.html`.
