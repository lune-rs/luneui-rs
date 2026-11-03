---
id: core.icon
title: Codicon glyph constants
area: core
status: in-luneterm
consumers: luneterm, wane
source: crates/lune-ui/src/icon.rs
owner: —
summary: Codicon glyph constants and a glyph helper; the app supplies the icon font
---

## What it is

Constants such as `ADD`, `GEAR`, `ELLIPSIS`, `CHROME_CLOSE`, `CHECK`, `CHROME_MINIMIZE` and `glyph`.

## Needs

- Icon glyphs as constants; the font itself comes from the app (R-core-6, R-text-1).
- Wane — Variants: none; the size comes from `--icon-size` (set by the parent component) (R-core-6)
- Wane — States: none; the colour is `currentColor`, so an icon takes the state of its parent (hover, disabled, tone) (R-core-6)
- Wane — Sizes: 16 px box (`--icon-size`); 14 in the callout, 13 in menu rows, 12 for the checkbox check, the combo chevron, the dropdown check and the table sort arrow. 27 Codicons are used (`js/icons.js`). (R-core-6)
- Wane — Behaviour: Decorative (hidden from assistive tech). An icon-only button carries the label, not the icon. (R-core-6)

## Existing code

`crates/lune-ui/src/icon.rs:7-19` (constants), `:22` (`glyph`); 24 lines.

## Open questions

- The Codicon font licence is not in this repo's plans yet; whether the toolkit bundles the font is open (R-ext-4).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: exists icon.rs:7 (Codicon glyph constants; the app supplies the icon font); Wane priority P0. Native note: The mocks inline Codicon SVG path data. lune-ui draws font glyphs, and RznMono has no Codicon glyphs, so luneui-rs needs an icon source: bundle the Codicon font (as Luneterm bundles a Nerd Font today) or draw the SVG paths with the painter (section 5). Full spec: Wane `docs/ui/components.md`, section `core.icon`; mock: Wane `docs/ui/mocks/components.html`.
