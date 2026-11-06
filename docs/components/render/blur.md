---
id: render.blur
title: Backdrop blur pass
area: render
status: done
consumers: luneterm
source: crates/lune-ui-wgpu/src/blur.rs, crates/lune-ui-wgpu/src/blur.wgsl
owner: —
summary: Backdrop blur pass with its WGSL shader
---

## What it is

`Blur` (public: `scene_view`, `dump`; the rest is crate-private, driven by `FrameRenderer`). It reads the core `BackdropRequest` that the painter records. The private uniform struct is `BackdropUniform`, so no name clashes with the `Backdrop` trait in `lune-ui-core`.

## Needs

- Backdrop blur (R-render-2).

## Existing code

`crates/lune-ui-wgpu/src/blur.rs:190` (`Blur`), `:317` (`scene_view`), `:385` (`dump`), `:20` (`BackdropUniform`); 407 lines; `blur.wgsl` 95 lines. No unit tests in the file; the integration tests in `crates/lune-ui-wgpu/tests/` (Acceptance) and the gallery's blur scenes cover it.

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-wgpu/src/blur.rs:1-2,187-196,315-318,383-386` (module doc, `Blur` struct, `scene_view`, `dump`);
`crates/lune-ui-core/src/paint.rs:189` (`backdrop`, the painter call that requests the blur).
Conflicts: none. The menu glass in Wane is `blur(30px) saturate(1.8)` [W docs/ui/components.md:543-556]; the repo's blur takes its strength and saturation from the backdrop request, or from the frame's `FrameParams` when the request says 0 (M7, [D138]).

### Anatomy

- Offscreen scene texture, then a copy, a horizontal and a vertical blur pass, each with its own parameter buffer [R crates/lune-ui-wgpu/src/blur.rs:190-196].
- The scene texture view is exposed for the frame pass [R crates/lune-ui-wgpu/src/blur.rs:315].
- Backdrop request: a rect and a radius [R crates/lune-ui-core/src/paint.rs:189], plus (M7, [D138]) the `layer` it sits under, its own `sigma` and its own `saturation` (0 means the frame's `FrameParams` values), set by `Painter::backdrop_with`. The offscreen scene holds the layers below `Scene::split()`: `SCENE_LAYERS`, or the request's layer when higher, so a glass surface blurs everything under it. The frame renders that scene, copies it to the target, draws the blur over it and then the layers from the split up; the renderer and the recording fake read the same `Scene::split`.
- One blur per frame: of two requests the lower layer keeps it, and on one layer the last wins. A glass menu over a dialog whose scrim is blurred therefore has no blur of its own.

### States

n/a — the blur has no states; it is on or not per backdrop request.

### Transitions

n/a — no motion.

### Input and focus

n/a — no input.

### Retained state

- Transient: the offscreen scene and blur textures, rebuilt on a size change.
- Persistent: the per-target `Blur` value (textures, buffers, bind groups) [R crates/lune-ui-wgpu/src/blur.rs:187-190].
- App-owned: none.

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

- `blur-contrast` (existing): the blur against high-contrast content, settled. [R apps/gallery/src/scenes.rs:166-171]
- `menu` (existing): the menu's backdrop over the controls scene. [R apps/gallery/src/scenes.rs:95-101]

### Painter primitives

- Existing: backdrop. [R crates/lune-ui-core/src/paint.rs:189]
- New: none.

### Acceptance

- Pixel probes on a headless device (`crates/lune-ui-wgpu/tests/primitives.rs`): `a_glass_panel_on_raised_blurs_the_modal_layer_under_it` (inside the glass the black/white edge is blurred, outside it stays sharp), `a_popup_backdrop_leaves_the_modal_layer_sharp_and_on_top` (the split: layers from the request's up are not blurred), `a_backdrop_with_its_own_sigma_and_saturation_differs_from_the_frames` (sigma 4 and saturation 1 by value equal the frame's look; sigma 8 reaches further; saturation 2 differs).
- The scene texture follows the target's size: `crates/lune-ui-wgpu/tests/targets.rs::one_renderer_across_two_sizes_matches_a_fresh_renderer_at_each` (one `FrameRenderer` drawn at A, B, A, A equals a fresh renderer at each size, pixel for pixel).
- One blur per frame and the split: `crates/lune-ui-core/tests/painter.rs::of_two_backdrops_the_lower_layer_keeps_it_in_either_draw_order`, `on_one_layer_the_last_backdrop_wins`, `the_recording_renderer_puts_the_backdrop_at_the_split`.
- The pixel gate on `blur-contrast` and `menu` at zero delta: `scripts/pixel-gate.sh` (the orchestrator's gate; M11's pre-audit run, base 55cfaaf, held 413 of 413 identical).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.frame, render.gpu, core.scene.
- Done (D47, M11): tests `crates/lune-ui-wgpu/tests/primitives.rs::a_glass_panel_on_raised_blurs_the_modal_layer_under_it, crates/lune-ui-wgpu/tests/primitives.rs::a_popup_backdrop_leaves_the_modal_layer_sharp_and_on_top, crates/lune-ui-wgpu/tests/primitives.rs::a_backdrop_with_its_own_sigma_and_saturation_differs_from_the_frames, crates/lune-ui-wgpu/tests/targets.rs::one_renderer_across_two_sizes_matches_a_fresh_renderer_at_each, crates/lune-ui-core/tests/painter.rs::of_two_backdrops_the_lower_layer_keeps_it_in_either_draw_order, crates/lune-ui-core/tests/painter.rs::the_recording_renderer_puts_the_backdrop_at_the_split`; page n/a (a renderer pass, drawn by the scenes `blur-contrast` and `menu`); unverified here: none (the tests skip without a GPU adapter; with `LUNEUI_REQUIRE_GPU=1` they fail instead).

## Open questions

- None: the old name clash is gone (the struct is private `BackdropUniform`; the trait is `lune-ui-core`'s `Backdrop`, implemented by `WindowBackdrop` and `NullMaterial`).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
