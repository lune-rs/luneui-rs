---
id: render.surface
title: Window surface
area: render
status: in-progress
consumers: any
source: crates/lune-ui-wgpu/src/surface.rs
owner: consumer's window run
summary: One window's swapchain on the shared Gpu: acquire, present, resize; no winit dependency
---

## What it is

`WindowSurface` wraps a `wgpu::Surface` configured for a `Gpu` (`new`, `resize`, `acquire`, `present`, `format`, `size`, `alpha_mode`). `acquire` returns `Acquired::Frame`, `Skip` or `Redraw` instead of calling a windowing function; the app asks its window for a redraw and calls `pre_present_notify` before `present`. The surface comes from any raw-window-handle window (`Instance::create_surface`), so `lune-ui-wgpu` does not depend on winit. Per window: one `WindowSurface` and one `FrameRenderer` (`render.frame`) on the one `Gpu`.

## Needs

- Several windows on one GPU, and headless targets (C2; R-nfr-8).

## Existing code

`crates/lune-ui-wgpu/src/surface.rs:10` (`Acquired`), `:21` (`WindowSurface`), `:37` (`new`), `:99` (`resize`), `:108` (`acquire`), `:127` (`present`); 130 lines. The headless path is tested in `crates/lune-ui-wgpu/tests/targets.rs`; the window path is run by the platform smoke (`gallery --smoke-platform`, a stage that resizes the hidden window's surface and acquires from it, M11).

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

- Done standard still needs: a resize-and-acquire test on a hidden window, and the pixel gate on a presented frame (open: the platform run is not in this milestone).
- Resize and acquire on a hidden window: `apps/gallery/src/smoke.rs` `resize_stage` (`gallery --smoke-platform`, needs a logged-in GUI session): resizes the surface to a size that is not its own, checks `size()`, acquires (a frame, a skip or a redraw are answers; an error or a panic fails the smoke), resizes back and checks. Its last run in this environment (debug, logged-in macOS session): exit 0, surface resized (2080, 1320) -> (2144, 1352) and back, acquire answered skip (the window is hidden). It is not a `cargo test`, so no test cites it.
- The headless target path, which shares the format and size handling: `crates/lune-ui-wgpu/tests/targets.rs::one_renderer_across_two_sizes_matches_a_fresh_renderer_at_each`.
- Not done here: the pixel gate on a presented frame.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.gpu, platform.window.

## Open questions

- Window-mode presentation is not exercised by an automated test (no visible windows in the gates).
- Class B (D234): this entry's own Acceptance says "the pixel gate on a presented frame (open: the platform run is not in this milestone)". A presented frame is what the OS composites to the screen; nothing here can read it back (reading it would need a `COPY_SRC` surface configuration for every app, D234). So the entry stays `in-progress` and `owner:` names the run that closes it: a consumer's window run that presents a frame and compares it.

## Notes for the planner

Added in M2 step 3.
