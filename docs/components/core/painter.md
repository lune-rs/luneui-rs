---
id: core.painter
title: Immediate-mode painter
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui/src/paint.rs
owner: orch/toolkit-v1
summary: Painter: ordered layers, fills, rings, strokes, acrylic, text, clip, transform and hit regions
---

## What it is

`Painter<H>`: immediate-mode layers (`LAYER_COUNT`), `fill`, `fill_radii`, `fill_over`, `ring`, `acrylic`, `custom`, `text`, `text_in`, `text_centered`, `measure`, `stroke`, clip (`set_clip`), transform (`set_xform`), and `hit`/`hovered` over an app-defined hit type `H`. Also `Rect`, `StrokeStyle`, `polyline_length`.

## Needs

- Layers, primitives, clip and transform (R-core-1).
- Hit regions with an app hit type (R-core-2).

## Existing code

`crates/lune-ui/src/paint.rs:136` (`Painter`), `:11` (`Rect`), `:90` (`StrokeStyle`), `:444` (`polyline_length`); 519 lines; tests at `:453`.

## Open questions

- Whether `Painter` stays tied to glyphon and wgpu types is the extraction plan's seam question (`CLAUDE.md` architecture rules).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
