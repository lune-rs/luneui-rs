---
id: render.surface
title: Window surface
area: render
status: in-progress
consumers: any
source: crates/lune-ui-wgpu/src/surface.rs
owner: orch/toolkit-v1
summary: One window's swapchain on the shared Gpu: acquire, present, resize; no winit dependency
---

## What it is

`WindowSurface` wraps a `wgpu::Surface` configured for a `Gpu` (`new`, `resize`, `acquire`, `present`, `format`, `size`, `alpha_mode`). `acquire` returns `Acquired::Frame`, `Skip` or `Redraw` instead of calling a windowing function; the app asks its window for a redraw and calls `pre_present_notify` before `present`. The surface comes from any raw-window-handle window (`Instance::create_surface`), so `lune-ui-wgpu` does not depend on winit. Per window: one `WindowSurface` and one `FrameRenderer` (`render.frame`) on the one `Gpu`.

## Needs

- Several windows on one GPU, and headless targets (C2; R-nfr-8).

## Existing code

`crates/lune-ui-wgpu/src/surface.rs:10` (`Acquired`), `:21` (`WindowSurface`), `:37` (`new`), `:97` (`resize`), `:106` (`acquire`), `:125` (`present`); 128 lines. Window mode is compile- and review-checked only in M2; the headless path is tested in `crates/lune-ui-wgpu/tests/targets.rs`.

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-wgpu/src/surface.rs:1-128` (module doc 1-4; `Acquired` 10, `WindowSurface` 21, `new` 37, `format` 83, `size` 88, `alpha_mode` 93, `resize` 97, `acquire` 106, `present` 125).
Conflicts: none.

### Anatomy

- A window's swapchain: the surface, its configuration, acquire and present [R crates/lune-ui-wgpu/src/surface.rs:1-4].
- Windowing-free: the app creates the `wgpu::Surface` from any window handle and asks for a redraw when `acquire` says so [R crates/lune-ui-wgpu/src/surface.rs:2-4].
- Resize takes a width and height [R crates/lune-ui-wgpu/src/surface.rs:97].

### States

| state | look | cite |
|---|---|---|
| acquired | a frame to draw | [R crates/lune-ui-wgpu/src/surface.rs:10,106] |
| needs redraw | `acquire` says so | [R crates/lune-ui-wgpu/src/surface.rs:2-4] |

### Transitions

n/a — no motion.

### Input and focus

n/a — no input.

### Retained state

- Persistent: the surface and its configuration [R crates/lune-ui-wgpu/src/surface.rs:21].
- App-owned: the window handle.

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

n/a — the gallery's scenes render offscreen through dumps; the window swapchain has no scene of its own.

### Painter primitives

n/a — no drawing.

### Acceptance

- Existing tests: none in `crates/lune-ui-wgpu/src/surface.rs`.
- Done standard still needs: a resize-and-acquire test on a hidden window, and the pixel gate on a presented frame (open: the platform run is not in this milestone).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.gpu, platform.window.

## Open questions

- Window-mode presentation is not exercised by an automated test (no visible windows in the gates).

## Notes for the planner

Added in M2 step 3.
