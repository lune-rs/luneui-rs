---
id: widget.stepper
title: Numeric stepper
area: widget
status: in-luneterm
consumers: luneterm, wane
source: crates/lune-ui/src/widgets/mod.rs
owner: —
summary: NumericUpDown: value with minus and plus buttons
---

## What it is

`stepper`: NumericUpDown with the value and minus and plus buttons.

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Wane — Variants: fill (container width, for a form column) (R-widget-1)
- Wane — States: rest; hover; focus-within (border `--accent`); disabled (opacity .4); button hover (`--surface-hover`). (R-widget-1)
- Wane — Sizes: 130 x 32 (`--control-w-stepper`), radius 6, 1 px `--border`; value RznMono 13 at left 9; minus and plus boxes 24 x 24, radius 4, top 3, right 31 and 3. (R-widget-1)
- Wane — Behaviour: Click the buttons, Up and Down keys, typed digits; clamping and step size are the owner's. Used for the port and the fee per request. (R-widget-1)

## Existing code

`crates/lune-ui/src/widgets/mod.rs:94`; the glass-ui spike calls it from `src/widgets.rs`. `widgets/mod.rs` has 203 lines and mostly no tests.

## Open questions

- Keyboard focus and key handling for this control are not in the inventory (R-widget-3).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists widgets/mod.rs:94; Wane priority P0. Full spec: Wane `docs/ui/components.md`, section `widget.stepper`; mock: Wane `docs/ui/mocks/components.html`.
