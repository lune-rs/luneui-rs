---
id: core.painter
title: Immediate-mode painter
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/paint.rs
owner: orch/toolkit-v1
summary: Painter: ordered layers, fills, rings, strokes, acrylic, text, clip, transform and hit regions, building a plain-data Scene
---

## What it is

`Painter<H>`: immediate-mode layers (`LAYER_COUNT`), `fill`, `fill_radii`, `fill_over`, `ring`, `acrylic`, `custom`, `text`, `text_in`, `text_centered`, `measure`, `stroke`, clip (`set_clip`), transform (`set_xform`), and `hit`/`hovered` over an app-defined hit type `H`. Also `Rect`, `StrokeStyle`, `polyline_length`. The painter builds a `Scene` (`Painter.scene`, see `core.scene`) and shapes text through `&mut dyn TextSystem` (`Painter::new(s, window, text, hover, theme)`); it has no GPU, window or glyphon type in its signature.

## Needs

- Layers, primitives, clip and transform (R-core-1).
- Hit regions with an app hit type (R-core-2).

## Existing code

`crates/lune-ui-core/src/paint.rs:109` (`Painter`), `:14` (`Rect`), `:64` (`StrokeStyle`), `:431` (`polyline_length`); 507 lines; tests at `:440`.

## Open questions

- None on the seam: the painter depends on core types only (M2 steps 2 and 5). It lives in `lune-ui-core`, which has no dependencies (boundary rules 7 and 8).

## Notes for the planner

Moved in the behaviour-preserving extraction (R-ext-1); the M2 seam cuts keep the 15 gallery scenes at zero pixel delta.
