---
id: widget.toggle
title: Toggle switch
area: widget
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/mod.rs
owner: orch/toolkit-v1
summary: Fluent ToggleSwitch: 40x20 track, accent fill when on
---

## What it is

`toggle`: Fluent ToggleSwitch look, 40 by 20 track, accent fill when on.

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Wane — Variants: none; with or without a text label (a row title usually labels it) (R-widget-1)
- Wane — States: off; on (accent fill); hover (track `--surface-hover`, knob 14); focus (1 px outline, offset 2); disabled (opacity .4). (R-widget-1)
- Wane — Sizes: Track 40 x 20, radius 10 (`--radius-toggle`), 1 px `--text-secondary` border when off; knob 12 (14 on hover) at 3 px; on: fill `--accent`, knob `--on-accent`; label 13 secondary, gap 12; min height 24. Transition .18 s (`--dur-fast`). (R-widget-1)
- Wane — Behaviour: Click and Space toggle; used for every on/off setting (rule enabled, pipeline stages, free wins, security options, restore on quit). A checkbox is for picking items from a list. (R-widget-1)

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:20`; the glass-ui spike calls it from `src/widgets.rs`. `widgets/mod.rs` has 203 lines and mostly no tests.

## Open questions

- Keyboard focus and key handling for this control are not in the inventory (R-widget-3).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists widgets/mod.rs:20; Wane priority P0. Native note: None beyond the knob animation. Full spec: Wane `docs/ui/components.md`, section `widget.toggle`; mock: Wane `docs/ui/mocks/components.html`.
