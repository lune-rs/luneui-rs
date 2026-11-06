---
id: render.gpu
title: Shared GPU
area: render
status: done
consumers: luneterm
source: crates/lune-ui-wgpu/src/gpu.rs
owner: —
summary: One shared Gpu (device, queue, noise texture, per-format pipelines and glyph atlas) for any number of targets
---

## What it is

`Gpu` (`new`, `upload`, `end_frame`), `Globals`, `Uploaded`, `instance`. One `Gpu` serves every target: it owns the device, the queue, the noise texture and, per colour format, the pipelines and the glyph atlas; each target (`render.frame`, `render.surface`) owns everything a draw writes. `Gpu::new` needs no window (headless works); `Gpu::end_frame` ages the text cache once per app frame. `Shape`, `StrokeVertex` and `LayerGeometry` are core plain data (`core.scene`); the adapter copies them into private Pod structs. The noise texture decodes `assets/acrylic-noise.png` through `include_bytes!`.

## Needs

- wgpu renderer for the painter's layers (R-render-1).
- Bundled noise texture with licence (R-render-5, R-ext-4).

## Existing code

`crates/lune-ui-wgpu/src/gpu.rs:108` (`Gpu`), `:213` (`Gpu::new`), `:201` (`instance`), `:336` (`is_lost`), `:505` (`end_frame`), `:554` (`upload`), `:739` (noise texture); 852 lines; shaders `src/shader.wgsl` (181 lines), `image.wgsl`, `blur.wgsl`. Two-target contract tests in `crates/lune-ui-wgpu/tests/targets.rs`.

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-wgpu/src/gpu.rs:1-2,15,90,121,131,331,340` (module doc, `Globals`, `Gpu`, `Uploaded`, `instance`, `end_frame`, `upload`); the test `gpu_records_are_bit_exact` (`crates/lune-ui-wgpu/src/gpu.rs`).
Conflicts: none.

### Anatomy

- Device and queue; the noise texture; per colour format: shape and stroke pipelines, blur pipelines and one glyph atlas [R crates/lune-ui-wgpu/src/gpu.rs:1-2].
- Instance creation takes an optional display handle [R crates/lune-ui-wgpu/src/gpu.rs:131].
- Scene upload: `upload(scene)` returns the uploaded records [R crates/lune-ui-wgpu/src/gpu.rs:340].
- End of frame: `end_frame(text)` [R crates/lune-ui-wgpu/src/gpu.rs:331].

### States

n/a — a device has no UI states; a lost device is reported by `Gpu::is_lost` (D8), which is not a UI state.

### Transitions

n/a — no motion.

### Input and focus

n/a — no input.

### Retained state

- Persistent: device, queue, pipelines, noise texture, glyph atlas [R crates/lune-ui-wgpu/src/gpu.rs:90].
- Transient: the uploaded buffers per frame [R crates/lune-ui-wgpu/src/gpu.rs:121].

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

- `controls` (existing) and every other scene use the device [R apps/gallery/src/scenes.rs:82-180].

### Painter primitives

n/a — the GPU draws the primitives the scene records.

### Acceptance

- The records the shaders read: `crates/lune-ui-wgpu/src/gpu.rs::gpu_records_are_bit_exact`.
- Many targets on one device: `crates/lune-ui-wgpu/tests/targets.rs` (`two_targets_on_one_gpu_match_each_alone`, `second_format_on_one_gpu_matches_a_fresh_gpu`).
- Device loss (D8: report, do not recover): `crates/lune-ui-wgpu/tests/device_lost.rs::a_new_device_is_not_lost` and `a_destroyed_device_is_lost_a_draw_on_it_never_succeeds_and_a_rebuild_draws` (a destroyed device reports lost after a poll and stays lost; a draw on it never reports success; a new `Gpu` and renderers draw). The doc on `Gpu::is_lost` lists what the app rebuilds: `Gpu`, text, renderers, surfaces, images.
- The pixel gate at zero delta: the orchestrator's gate (M11's pre-audit run held 413 of 413).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.frame, text.shaping.
- Done (D47, M11): tests `crates/lune-ui-wgpu/src/gpu.rs::gpu_records_are_bit_exact, crates/lune-ui-wgpu/tests/targets.rs::two_targets_on_one_gpu_match_each_alone, crates/lune-ui-wgpu/tests/targets.rs::second_format_on_one_gpu_matches_a_fresh_gpu, crates/lune-ui-wgpu/tests/device_lost.rs::a_new_device_is_not_lost, crates/lune-ui-wgpu/tests/device_lost.rs::a_destroyed_device_is_lost_a_draw_on_it_never_succeeds_and_a_rebuild_draws`; page n/a (the device under every scene); unverified here: loss of a real GPU (a driver reset, an unplugged adapter), which the tests stand in for with `Device::destroy`.

## Open questions

- Answered: the `.wgsl` files are compiled in with `include_str!` (`crates/lune-ui-wgpu/src/gpu.rs:281,291`, `blur.rs:64`) and the noise texture with `include_bytes!` (`gpu.rs:743`).
- Answered: `crates/lune-ui-wgpu/assets/acrylic-noise.png` is Avalonia's acrylic noise texture, with its licence in `licenses/avalonia-LICENSE` and its row in `THIRD_PARTY_NOTICES.md:8`.

## Notes for the planner

The one-GPU, many-targets shape landed in M2 steps 3 and 4 (DECISIONS D28, D30).
