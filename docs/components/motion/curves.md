---
id: motion.curves
title: Easing curves and menu motion
area: motion
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/widgets/menu.rs
owner: —
summary: cubic_bezier easing and the menu open and close animation values
---

## What it is

`cubic_bezier` and `MenuAnim` with `MENU_IN` and `MENU_OUT`; the app passes the time in.

## Needs

- Easing and time-in animation values (R-motion-1, R-nfr-1).

## Existing code

`crates/lune-ui/src/widgets/menu.rs:28` (`cubic_bezier`), `:25` (`MenuAnim`), `:21-22` (`MENU_IN`, `MENU_OUT`).

## Open questions

- The curve function lives in `widgets/menu.rs`; whether it moves to a `motion` module is for the planner.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it. General animation tiers are `motion.animation-tiers`.
