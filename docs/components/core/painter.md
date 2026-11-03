---
id: core.painter
title: Immediate-mode painter
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/paint.rs
owner: orch/toolkit-v1
summary: Painter: ordered layers, fills, rings, strokes, acrylic, text, clip, transform and hit regions, building a plain-data Scene; holds the aggregate table of new primitives
---

## What it is

`Painter<H>`: immediate-mode layers (`LAYER_COUNT`), `fill`, `fill_radii`, `fill_over`, `ring`, `acrylic`, `custom`, `text`, `text_in`, `text_centered`, `measure`, `stroke`, clip (`set_clip`), transform (`set_xform`), and `hit`/`hovered` over an app-defined hit type `H`. Also `Rect`, `StrokeStyle`, `polyline_length`. The painter builds a `Scene` (`Painter.scene`, see `core.scene`) and shapes text through `&mut dyn TextSystem` (`Painter::new(s, window, text, hover, theme)`); it has no GPU, window or glyphon type in its signature.

## Needs

- Layers, primitives, clip and transform (R-core-1).
- Hit regions with an app hit type (R-core-2).

## Existing code

`crates/lune-ui-core/src/paint.rs:109` (`Painter`), `:14` (`Rect`), `:64` (`StrokeStyle`), `:431` (`polyline_length`); 507 lines; tests at `:440`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/paint.rs:14-342` (`Rect`, `StrokeStyle`, `Xform`, `Painter`, the primitive methods), `crates/lune-ui-core/src/icon.rs:1-25` (glyph spec), `crates/lune-ui-core/src/scene.rs:17-107` (what the painter records); every catalogue entry's `### Painter primitives` subsection (97 entries read by script; the `New:` lines are aggregated below).
Conflicts: none between sources. The painter is the moved, pixel-gated code, so its current methods are the "Existing" set; a primitive an entry needs and the painter lacks is a `New:` item, cited in the entry that needs it.

### Anatomy

- Existing primitives (the SF11 vocabulary, mapped to the methods that implement them): `fill` [R crates/lune-ui-core/src/paint.rs:215], `fill-radii` [R crates/lune-ui-core/src/paint.rs:219], `fill-over` [R crates/lune-ui-core/src/paint.rs:225], `ring` [R crates/lune-ui-core/src/paint.rs:231], `acrylic` [R crates/lune-ui-core/src/paint.rs:243], `backdrop` [R crates/lune-ui-core/src/paint.rs:189], `text` (with `text_in`, `text_centered`, `measure`) [R crates/lune-ui-core/src/paint.rs:274-311], `icon` (a glyph drawn through the text path) [R crates/lune-ui-core/src/icon.rs:22], `stroke`, `stroke-sweep` and `stroke-comet` (one tessellated polyline; the sweep and comet are `StrokeStyle.mode` 1 and 2, drawn by the shader) [R crates/lune-ui-core/src/paint.rs:64-72,329], [R crates/lune-ui-wgpu/src/shader.wgsl:106,150-157], `custom` [R crates/lune-ui-core/src/paint.rs:254], `clip-rect` (`set_clip`) [R crates/lune-ui-core/src/paint.rs:167], `xform` (`set_xform`: scale about an origin, a vertical offset and an alpha) [R crates/lune-ui-core/src/paint.rs:171,86-93].
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

- Existing scenes exercise the existing primitives: `controls` (fill, fill-radii, ring, text, icon), `acrylic` (acrylic), `blur-contrast` and `menu` (backdrop), `chrome` (stroke, stroke-sweep, stroke-comet, clip-rect), `custom-draw` (custom), `menu-opening` (xform).
- New primitives are exercised by the scenes of the entries that need them: image: `image-contain-wide` ([core.image]); rotate: `spinner-spinning` ([widget.spinner]); dash: `search-picker-link` ([widget.search-picker]); clip-rounded: `page-body-scroll` ([layout.page]) and `panel-card` ([layout.panel]); sector, path-fill and arc: `pie-chart-three` and `pie-chart-half` ([data.pie-chart]).

### Painter primitives

- Existing: fill, fill-radii, fill-over, ring, acrylic, backdrop, text, icon, stroke, stroke-sweep, stroke-comet, custom, clip-rect, xform.
- New: image, rotate, dash, clip-rounded, sector, path-fill, arc.

The aggregate of every `New:` item in the catalogue (the list C5 gives M5; R-core-10). "First milestone" is the earliest consumer; M5 adds the whole set so later milestones draw without painter work.

| primitive | needed by (ids) | first milestone | cite |
|---|---|---|---|
| image | core.image, widget.banner-empty-state, text.markdown-preview | M5 | [L src/Lunegit.Desktop/Views/FileViewerView.axaml:352], [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:445], [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:17] |
| rotate | widget.spinner, widget.banner-empty-state | M6 | [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:38-53] |
| dash | widget.search-picker | M6 | [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:30] |
| clip-rounded | layout.page, layout.panel | M7 | [L src/Lunegit.Desktop/App.axaml:769-770], [W docs/ui/components.md:249-262] |
| sector | data.pie-chart | M8 | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:113-137] |
| path-fill | data.pie-chart | M8 | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:140-158] |
| arc | data.pie-chart | M8 | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:132] |

Not needed by any entry (no row): `linear-gradient` and `shadow`. No Lunegit or Wane component read draws either (Wane's panel states "no shadow", [W docs/ui/components.md:249-262]); they are added only when a spec needs them. Of the other entries, 76 list `New: none`, 8 read `n/a` (no painter section applies) and 4 are external (no painter section); counted by script over the `### Painter primitives` subsections.

### Acceptance

- Existing tests (`crates/lune-ui-core/src/paint.rs`, module `tests` from :440): `custom_records_physical_clip_on_base`, `custom_is_cut_by_the_current_clip`, `custom_outside_the_window_records_nothing`, `custom_lands_on_the_current_layer`.
- The pixel gate: the 15 gallery scenes dumped before and after any painter change compare at zero delta (CLAUDE.md "Pixel gate").
- Done standard (D47) still needs, per primitive in the table: a recording-fake test that the right `Scene` item lands on the right layer under the current clip and `Xform`; a gallery scene from the table above; a pixel probe at scale 2 taken from the owner entry's Anatomy; and no wgpu type in a core signature (`scripts/check-boundaries.sh`).
- Also still needed for the existing set: a test per method for the `Xform` scale and alpha (only `custom` has tests today).

### Build

- Milestone: M5 (the seven new primitives, R-core-10) and M11 (done standard; the existing code is moved).
- Depends on: core.scene, core.theme, text.shaping, core.image.

## Open questions

- None on the seam: the painter depends on core types only (M2 steps 2 and 5). It lives in `lune-ui-core`, which has no dependencies (boundary rules 7 and 8).

## Notes for the planner

Moved in the behaviour-preserving extraction (R-ext-1); the M2 seam cuts keep the 15 gallery scenes at zero pixel delta.
