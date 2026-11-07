---
id: render.dump
title: Offscreen frame dump
area: render
status: done
consumers: luneterm
source: crates/lune-ui-wgpu/src/dump.rs
owner: —
summary: Offscreen target, GPU readback and PNG writer for frame goldens
---

## What it is

`target`, `read_rgba` and `write_png`: render offscreen, read back, write a PNG, with no window or screen capture.

## Needs

- Frame dump for goldens and the pixel gate (R-render-4, R-ext-2).

## Existing code

`crates/lune-ui-wgpu/src/dump.rs:9` (`target`), `:31` (`read_rgba`), `:104` (`write_png`); 148 lines plus the PNG round-trip test; tests at `:115`. The compare tool is `tools/dump-compare`; the command is `scripts/pixel-gate.sh`.

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

- Every scene `gallery --list` prints (421 names) is dumped by the pixel gate; the first 13 legacy ids (`controls`, `menu`, `menu-opening`, `custom-draw`, `dropdown`, `dropdown-flipped`, `dialog-confirm`, `dialog-message`, `dialog-body`, `glass-off`, `blur-contrast`, `chrome`, `acrylic`) were the set when this entry was written [R apps/gallery/src/scenes.rs:82-180].

### Painter primitives

n/a — the dump reads the GPU target; it draws nothing.

### Acceptance

- Tests (`crates/lune-ui-wgpu/src/dump.rs`, module `tests`): `unpad_swizzles_bgra_and_keeps_alpha`, `unpad_passes_rgba_through`, `unpad_rejects_other_formats_and_short_buffers`, `write_png_round_trips_the_pixels_and_names_a_failure` (a 2 by 2 PNG decodes to the bytes written; an uncreatable path is an error naming `create`). `target` and `read_rgba` are the readback every GPU test in `crates/lune-ui-wgpu/tests/` uses.
- The dump-and-compare command exists and runs: `scripts/pixel-gate.sh` (CLAUDE.md, Commands, "Pixel gate") dumps the scenes with `gallery --dump` and compares them with `tools/dump-compare`; M11's pre-audit run: 413 files identical against 55cfaaf.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.frame, render.gpu.
- Done (D47, M11): tests `crates/lune-ui-wgpu/src/dump.rs::unpad_swizzles_bgra_and_keeps_alpha, crates/lune-ui-wgpu/src/dump.rs::unpad_passes_rgba_through, crates/lune-ui-wgpu/src/dump.rs::unpad_rejects_other_formats_and_short_buffers, crates/lune-ui-wgpu/src/dump.rs::write_png_round_trips_the_pixels_and_names_a_failure`; page n/a (a tool; the gate dumps every scene through it); unverified here: none.

## Open questions

- Answered: the scene list is `apps/gallery` (`--list`, `--dump`) and the compare tool is `tools/dump-compare`; both live in this repo, driven by `scripts/pixel-gate.sh`.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
