---
id: widget.checkbox
title: Checkbox
area: widget
status: in-luneterm
consumers: luneterm, wane
source: crates/lune-ui/src/widgets/mod.rs
owner: —
summary: Checkbox
---

## What it is

`checkbox`.

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Wane — States: unchecked; checked (accent fill, 12 px check); hover (`--surface-hover` fill); focus (1 px outline, offset 2); disabled (opacity .4). (R-widget-1)
- Wane — Sizes: 16 x 16, radius 3 (`--radius-check`), 1 px `--text-secondary` ring; label 13 secondary, gap 8; min height 24. (R-widget-1)
- Wane — Behaviour: Click and Space toggle. Used for first-run import rows and the Local models filters only (D7). (R-widget-1)

## Existing code

`crates/lune-ui/src/widgets/mod.rs:176`; the glass-ui spike calls it from `src/widgets.rs`. `widgets/mod.rs` has 203 lines and mostly no tests.

## Open questions

- Keyboard focus and key handling for this control are not in the inventory (R-widget-3).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists widgets/mod.rs:176; Wane priority P1. Full spec: Wane `docs/ui/components.md`, section `widget.checkbox`; mock: Wane `docs/ui/mocks/components.html`.
