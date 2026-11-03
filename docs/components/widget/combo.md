---
id: widget.combo
title: Combo box
area: widget
status: in-luneterm
consumers: luneterm, wane
source: crates/lune-ui/src/widgets/mod.rs
owner: —
summary: ComboBox: value plus chevron
---

## What it is

`combo`: ComboBox showing the value plus a chevron; opens `overlay.dropdown`.

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Wane — Variants: fill; mono (the value is an id); wide (300 px, a column of id combos); sm (24 high); short (112 px, a duration in a table column) (R-widget-1)
- Wane — States: rest; hover; open and focus (border `--accent`); disabled (opacity .4). (R-widget-1)
- Wane — Sizes: 230 x 32 (`--control-w-combo`), radius 6, 1 px `--border`, fill `--surface-input`, padding 0 30 0 9; value 13 with ellipsis; chevron-down 12 px in a 20 px slot at right 7. (R-widget-1)
- Wane — Behaviour: Click opens the dropdown; Enter, Space and Down open it from the keyboard; Esc closes; the chosen value replaces the label. (R-widget-1)

## Existing code

`crates/lune-ui/src/widgets/mod.rs:71`; the glass-ui spike calls it from `src/widgets.rs`. `widgets/mod.rs` has 203 lines and mostly no tests.

## Open questions

- Keyboard focus and key handling for this control are not in the inventory (R-widget-3).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists widgets/mod.rs:71; Wane priority P0. Full spec: Wane `docs/ui/components.md`, section `widget.combo`; mock: Wane `docs/ui/mocks/components.html`.
