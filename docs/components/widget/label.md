---
id: widget.label
title: Label
area: widget
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/widgets/mod.rs
owner: —
summary: UI-font text at the theme base size
---

## What it is

`label`: UI-font text at the theme base size.

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).

## Existing code

`crates/lune-ui/src/widgets/mod.rs:15`; the glass-ui spike calls it from `src/widgets.rs`. `widgets/mod.rs` has 203 lines and mostly no tests.

## Open questions

- Keyboard focus and key handling for this control are not in the inventory (R-widget-3).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
