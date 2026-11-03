---
id: widget.text-field
title: Text field
area: widget
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/widgets/mod.rs
owner: —
summary: TextBox with placeholder and a caret when focused
---

## What it is

`field`: TextBox with a placeholder and a caret when focused.

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Text selection, caret movement and clipboard (R-text-5).

## Existing code

`crates/lune-ui/src/widgets/mod.rs:120`; the glass-ui spike calls it from `src/widgets.rs`. `widgets/mod.rs` has 203 lines and mostly no tests.

## Open questions

- Keyboard focus and key handling for this control are not in the inventory (R-widget-3).
- Text selection, caret movement and clipboard behaviour are unverified (R-text-5); IME is not covered (R-text-4).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
