---
id: render.dump
title: Offscreen frame dump
area: render
status: in-progress
consumers: luneterm
source: crates/lune-ui-wgpu/src/dump.rs
owner: orch/toolkit-v1
summary: Offscreen target, GPU readback and PNG writer for frame goldens
---

## What it is

`target`, `read_rgba` and `write_png`: render offscreen, read back, write a PNG, with no window or screen capture.

## Needs

- Frame dump for goldens and the pixel gate (R-render-4, R-ext-2).

## Existing code

`crates/lune-ui-wgpu/src/dump.rs:9` (`target`), `:31` (`read_rgba`), `:102` (`write_png`); 146 lines; tests at `:113`.

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-wgpu/src/dump.rs:1-146` (module doc 1-3; `target` 9, `read_rgba` 31, `write_png` 102; tests).
Conflicts: none.

### Anatomy

- Offscreen render target [R crates/lune-ui-wgpu/src/dump.rs:9].
- GPU readback through a mapped buffer; no window or screen capture, so dumps work on a hidden window and compare byte for byte [R crates/lune-ui-wgpu/src/dump.rs:1-3,31].
- PNG writer [R crates/lune-ui-wgpu/src/dump.rs:102].
- Row padding removed and BGRA swizzled to RGBA by `unpad_to_rgba` [R crates/lune-ui-wgpu/src/dump.rs:72]; the test names are below.

### States

n/a — no states.

### Transitions

n/a — no motion.

### Input and focus

n/a — no input.

### Retained state

n/a — each dump is one read-back.

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

- The 14 gallery scene ids (`controls`, `menu`, `menu-opening`, `custom-draw`, `dropdown`, `dropdown-flipped`, `dialog-confirm`, `dialog-message`, `dialog-body`, `glass-off`, `unfocused`, `blur-contrast`, `chrome`, `acrylic`) are the pixel-gate set [R apps/gallery/src/scenes.rs:82-180].

### Painter primitives

n/a — the dump reads the GPU target; it draws nothing.

### Acceptance

- Existing tests (by path, `crates/lune-ui-wgpu/src/dump.rs`): `unpad_swizzles_bgra_and_keeps_alpha`, `unpad_passes_rgba_through`, `unpad_rejects_other_formats_and_short_buffers`.
- Done standard still needs: the pixel gate's dump-and-compare command (CLAUDE.md, Commands, "Pixel gate": the exact command is set by the extraction plan and is not yet runnable).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.frame, render.gpu.

## Open questions

- The scene list and the compare tool live in the glass-ui spike, not in lune-ui; where they go is the extraction plan's call.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
