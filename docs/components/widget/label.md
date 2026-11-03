---
id: widget.label
title: Label
area: widget
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui/src/widgets/mod.rs
owner: orch/toolkit-v1
summary: UI-font text at the theme base size
---

## What it is

`label`: UI-font text at the theme base size.

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Wane — Variants: secondary; small (11/16 tertiary); section (14 Medium); title (18/24 Medium bright); mono (RznMono); strong (500, bright: a name that heads a block) (R-widget-1)
- Wane — States: none (static text) (R-widget-1)
- Wane — Sizes: Base 13/20 400 primary (`--font-base`); small 11 (`--font-s`); section 14 (`--font-m`); title 18 (`--font-title`). RznSans for UI text, RznMono for ids, paths, money and numbers. (R-widget-1)
- Wane — Behaviour: Selectable text for ids and paths (copy) is a requirement, not mocked. (R-widget-1)

## Existing code

`crates/lune-ui/src/widgets/mod.rs:15`; the glass-ui spike calls it from `src/widgets.rs`. `widgets/mod.rs` has 203 lines and mostly no tests.

## Open questions

- Keyboard focus and key handling for this control are not in the inventory (R-widget-3).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists widgets/mod.rs:15; Wane priority P0. Native note: Text shaping through `text.shaping` with RznSans and RznMono registered by the app; tabular figures need the font's `tnum` feature in the shaper. Full spec: Wane `docs/ui/components.md`, section `widget.label`; mock: Wane `docs/ui/mocks/components.html`.
