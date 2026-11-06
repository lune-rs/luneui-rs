---
id: core.painter
title: Immediate-mode painter
area: core
status: done
consumers: luneterm
source: crates/lune-ui-core/src/paint.rs, crates/lune-ui-core/src/paint/path.rs
owner: —
summary: Painter: ordered layers, fills, rings, strokes, acrylic, text, clip, transform and hit regions, building a plain-data Scene; holds the aggregate table of new primitives
---

## What it is

`Painter<H>`: immediate-mode layers (`LAYER_COUNT`), `fill`, `fill_radii`, `fill_over`, `ring`, `acrylic`, `custom`, `text`, `text_in`, `text_centered`, `measure`, `stroke`, clip (`set_clip`), transform (`set_xform`), and `hit`/`hovered` over an app-defined hit type `H`. Also `Rect`, `StrokeStyle`, `polyline_length`. The painter builds a `Scene` (`Painter.scene`, see `core.scene`) and shapes text through `&mut dyn TextSystem` (`Painter::new(s, window, text, hover, theme)`); it has no GPU, window or glyphon type in its signature.

## Needs

- Layers, primitives, clip and transform (R-core-1).
- Hit regions with an app hit type (R-core-2).

## Existing code

`crates/lune-ui-core/src/paint.rs:216` (`Painter`), `:19` (`Rect`), `:86` (`StrokeStyle`), `:113` (`Xform`), `:1287` (`polyline_length`); 1,378 lines, with the path geometry in `paint/path.rs`; unit tests at `:1297`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/paint.rs:14-342` (`Rect`, `StrokeStyle`, `Xform`, `Painter`, the primitive methods), `crates/lune-ui-core/src/icon.rs:1-25` (glyph spec), `crates/lune-ui-core/src/scene.rs:17-107` (what the painter records); every catalogue entry's `### Painter primitives` subsection (97 entries read by script; the `New:` lines are aggregated below).
Conflicts: none between sources. The painter is the moved, pixel-gated code, so its current methods are the "Existing" set; a primitive an entry needs and the painter lacks is a `New:` item, cited in the entry that needs it.

### Anatomy

- Existing primitives (the SF11 vocabulary, mapped to the methods that implement them): `fill` [R crates/lune-ui-core/src/paint.rs:215], `fill-radii` [R crates/lune-ui-core/src/paint.rs:219], `fill-over` [R crates/lune-ui-core/src/paint.rs:225], `ring` [R crates/lune-ui-core/src/paint.rs:231], `acrylic` [R crates/lune-ui-core/src/paint.rs:243], `backdrop` [R crates/lune-ui-core/src/paint.rs:189], `text` (with `text_in`, `text_centered`, `measure`) [R crates/lune-ui-core/src/paint.rs:274-311], `icon` (a glyph drawn through the text path) [R crates/lune-ui-core/src/icon.rs:22], `stroke`, `stroke-sweep` and `stroke-comet` (one tessellated polyline; `Painter::stroke_sweep` and `Painter::stroke_comet` are `StrokeStyle.mode` 1 and 2, drawn by the shader; a style's lengths and centre are logical px of the drawing space, and `stroke` turns them into the shader's units through the transform stack, so a recorded `Op::Stroke` replays right in another painter's space) [R crates/lune-ui-core/src/paint.rs:64-72,329], [R crates/lune-ui-wgpu/src/shader.wgsl:106,150-157], `custom` [R crates/lune-ui-core/src/paint.rs:254], `clip-rect` (`set_clip`) [R crates/lune-ui-core/src/paint.rs:167], `xform` (`set_xform`: scale about an origin, a vertical offset and an alpha) [R crates/lune-ui-core/src/paint.rs:171,86-93].
- Order: layers draw back to front; inside a layer the draw-order contract is in [core.scene]. Every primitive records on the layer set by `on` [R crates/lune-ui-core/src/paint.rs:162].
- Every primitive method records plain data into the `Scene`; none touches a GPU, window or text-shaping type [core.scene].
- Rule for a new primitive: it is a method on `Painter` that records into `Scene`, with a recording-fake test and a gallery scene; the wgpu adapter draws it; the pixel gate stays at zero delta for every existing scene [D47].

### States

n/a — the painter is a recorder; component states are in each component's entry.

### Transitions

n/a — the painter animates nothing; a transition is a changed argument between frames (`Xform` carries the popup scale and fade) [motion].

### Input and focus

n/a — the painter has hit regions (`hit`, `hovered`) but no input handling; the layered hit test is [core.ui].

### Retained state

- Transient: the painter and its scene are rebuilt each frame.
- Persistent: none.
- App-owned: the theme, the window rect, the scale and the hover value passed to `Painter::new`.
- Exit retention: no.

### Accessibility

n/a — the painter draws pixels; nodes are emitted by widgets through `ui.node` [infra.accessibility].

### Strings

None: the painter draws text it is given.

### Gallery scenes

- Existing scenes exercise the existing primitives: `controls` (fill, fill-radii, ring, text, icon), `acrylic` (acrylic), `blur-contrast` and `menu` (backdrop), `chrome` (stroke, clip-rect), `tab-strip-busy-sweep` (stroke-sweep), `tab-strip-busy-comet` (stroke-comet), `custom-draw` (custom), `menu-opening` (xform).
- M5 built the seven new primitives and shows each on the gallery's `primitives` and `image` pages: `primitive-arc`, `primitive-sector`, `primitive-path-fill`, `primitive-dash`, `primitive-clip-rounded`, `primitive-rotate` and the seven `image-*` scenes (D105).
- New primitives are exercised by the scenes of the entries that need them: image: `image-contain-wide` ([core.image]); rotate: `spinner-spinning` ([widget.spinner]); dash: `search-picker-link` ([widget.search-picker]); clip-rounded: `page-body-scroll` ([layout.page]) and `panel-card` ([layout.panel]); sector: `pie-chart-three` and `pie-chart-half` ([data.pie-chart]); path-fill and arc: no M8 entry uses them (the pie is `sector` fans, the lane dot a closed stroke, D172), so only the `primitives` page and the tab strip (`fill_path`) show them.

### Painter primitives

- Existing: fill, fill-radii, fill-over, ring, acrylic, backdrop, text, icon, stroke, stroke-sweep, stroke-comet, custom, clip-rect, xform.
- New: image, rotate, dash, clip-rounded, sector, path-fill, arc.

The aggregate of every `New:` item in the catalogue (the list C5 gives M5; R-core-10). "First milestone" is the earliest consumer; M5 adds the whole set so later milestones draw without painter work.

| primitive | needed by (ids) | first milestone | cite |
|---|---|---|---|
| image | core.image, widget.banner-empty-state, text.markdown-preview | M5 | [L src/Lunegit.Desktop/Views/FileViewerView.axaml:352], [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:445], [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:17] |
| rotate | widget.spinner, widget.banner-empty-state | M6 (done: `Painter::text_rotated` turns the spinner's glyph, scene `spinner-spinning`; the banner's busy glyph is the spinner) | [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:38-53] |
| dash | widget.search-picker | M6 (done: `Painter::dash` draws the link trigger's underline, scene `search-picker-link`) | [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:30] |
| clip-rounded | layout.page, layout.panel | M5 (done: `Painter::push_clip_rounded`, scene `primitive-clip-rounded`; M7 uses it in `Ui::page` and `Ui::panel`, scenes `page-body-scroll` and `panel-card`) | [L src/Lunegit.Desktop/App.axaml:769-770], [W docs/ui/components.md:249-262] |
| sector | data.pie-chart | M8 (done: `Ui::pie_chart` draws a fan per slice and the top-half highlight with `Painter::sector`, scenes `pie-chart-three` and `pie-chart-half`) | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:113-137] |
| path-fill | data.pie-chart | M8 (not used by the pie: a fan through `sector` is a whole disc at a sweep of 2π, so no path fill is needed; the primitive stays built, M7's tab strip tints with `Painter::fill_path`) | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:140-158] |
| arc | data.pie-chart | M8 (not used by the pie: `sector` covers it; the lane graph's dot first used `Painter::arc` and is a closed `Painter::stroke` since D172, because `arc` flattens on the centre radius) | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:132] |

Not needed by any entry (no row): `linear-gradient` and `shadow`. No Lunegit or Wane component read draws either (Wane's panel states "no shadow", [W docs/ui/components.md:249-262]); they are added only when a spec needs them. Of the other entries, 78 list `New: none` (two more since the page and the panel took the M5 clip-rounded), 8 read `n/a` (no painter section applies) and 4 are external (no painter section); counted by script over the `### Painter primitives` subsections.

### Acceptance

- Existing tests (`crates/lune-ui-core/src/paint.rs`, module `tests` from :1297): `custom_records_physical_clip_on_base`, `custom_is_cut_by_the_current_clip`, `custom_outside_the_window_records_nothing`, `custom_lands_on_the_current_layer`.
- The pixel gate: the 15 gallery scenes dumped before and after any painter change compare at zero delta (CLAUDE.md "Pixel gate").
- Recording (the right `Scene` item on the right layer, under the clip and the `Xform`'s scale and alpha), per primitive: `image` and `text_rotated` (`crates/lune-ui-core/tests/painter.rs`: `image_lands_on_the_current_layer_under_the_clip_and_xform`, `text_rotated_lands_on_the_current_layer_under_the_clip_and_xform`); `dash` (`dash_respects_layer_clip_scale_and_alpha`); `clip-rounded` (`a_rounded_clip_travels_on_shapes_strokes_and_fills`, `the_radius_is_limited_to_half_the_short_side_and_scales_with_the_transform`); `sector`, `path-fill` (`strokes_fans_and_polygons_take_the_layer_clip_scale_and_alpha_of_the_xform`); `arc` (`an_arc_is_clipped_and_lands_on_its_layer`, `an_arc_radius_takes_the_xform_scale`).
- The existing set under an `Xform`: `fill`, `fill_radii`, `fill_over`, `ring`, `backdrop` (`fills_rings_and_backdrops_take_the_layer_clip_scale_and_alpha_of_the_xform`), `text` (`text_takes_the_layer_clip_scale_and_alpha_of_the_xform`), `stroke` (the strokes test above), `acrylic` (`acrylic_opacity_comes_from_the_stack`), `fill` once more (`single_entry_uses_the_one_transform_formulas`, `nested_known_answer`). The ring's width follows the device scale only, not the `Xform`'s; no test pins that.
- Pixel probes at scale 2 (`crates/lune-ui-wgpu/tests`): `dash`, `sector`, `path-fill`, `clip-rounded`, `arc` in `primitives.rs`; `image` and `text_rotated` in `images.rs` (`four_exact_quadrants_at_an_integer_ratio`, `rotated_text_is_drawn_through_buffer_draw`).
- Gallery scenes: the table above; the `primitives` and `image` pages hold the seven.
- No wgpu type in a core signature: `scripts/check-boundaries.sh` (in the gate).
- The pixel gate: the gallery scenes dumped before and after any painter change compare at zero delta (CLAUDE.md "Pixel gate").

### Build

- Milestone: M5 (the seven new primitives, R-core-10) and M11 (done standard; the existing code is moved). M5 built the seven primitives (steps 4 and 5; the page is `primitives`); M8 put `sector` to use (the pie chart) and used no new primitive. M6 put the rotate and dash rows to use (the spinner and the search picker's link trigger) and added the two stroke helpers `Painter::stroke_sweep` and `Painter::stroke_comet` over `StrokeStyle.mode`, which the tab strip's busy cue draws (D122).
- Depends on: core.scene, core.theme, text.shaping, core.image.
- Done (D47, M11): tests `crates/lune-ui-core/tests/painter.rs::image_lands_on_the_current_layer_under_the_clip_and_xform, crates/lune-ui-core/tests/painter.rs::text_rotated_lands_on_the_current_layer_under_the_clip_and_xform, crates/lune-ui-core/tests/painter.rs::fills_rings_and_backdrops_take_the_layer_clip_scale_and_alpha_of_the_xform, crates/lune-ui-core/tests/painter.rs::strokes_fans_and_polygons_take_the_layer_clip_scale_and_alpha_of_the_xform, crates/lune-ui-core/tests/painter.rs::text_takes_the_layer_clip_scale_and_alpha_of_the_xform, crates/lune-ui-core/tests/painter.rs::an_arc_radius_takes_the_xform_scale, crates/lune-ui-core/tests/paint_vector.rs::dash_respects_layer_clip_scale_and_alpha, crates/lune-ui-core/tests/paint_vector.rs::an_arc_is_clipped_and_lands_on_its_layer, crates/lune-ui-core/tests/paint_vector.rs::a_rounded_clip_travels_on_shapes_strokes_and_fills, crates/lune-ui-wgpu/tests/primitives.rs::an_arc_is_a_stroked_ring_piece_from_its_start_turning_clockwise_at_scale_2, crates/lune-ui-wgpu/tests/primitives.rs::a_sector_has_an_antialiased_edge_and_a_solid_inside, crates/lune-ui-wgpu/tests/primitives.rs::a_dashed_line_has_its_runs_and_gaps, crates/lune-ui-wgpu/tests/images.rs::rotated_text_is_drawn_through_buffer_draw`; page `primitives`; unverified here: none.

## Open questions

- None on the seam: the painter depends on core types only (M2 steps 2 and 5). The seven M5 primitives are built (D105); `linear-gradient` and `shadow` are not, because no entry needs them. It lives in `lune-ui-core`, which has no dependencies (boundary rules 7 and 8).

## Notes for the planner

Moved in the behaviour-preserving extraction (R-ext-1); the M2 seam cuts keep the 15 gallery scenes at zero pixel delta.
