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

The present mode and the frame latency are `SurfaceOptions` (`with_options`, `options`, `set_options`; `new` keeps the default, `Fifo` with a latency of 2, 0.4.0's). `Fifo` makes `acquire` wait for the display when every drawable is queued (D299). `set_options` and `resize` run between frames, never while a texture from `acquire` is alive.

## Needs

- Several windows on one GPU, and headless targets (C2; R-nfr-8).
- A window surface set so the main thread does not wait in acquire when no frame is due (R-nfr-9).

## Existing code

`crates/lune-ui-wgpu/src/surface.rs:12` (`Acquired`), `:30` (`SurfaceOptions`), `:63` (`WindowSurface`), `:82` (`new`), `:92` (`with_options`), `:143` (`options`), `:154` (`set_options`), `:181` (`resize`), `:193` (`acquire`), `:217` (`present`); 248 lines. The headless path is tested in `crates/lune-ui-wgpu/tests/targets.rs`; the window path is run by the platform smoke (`gallery --smoke-platform`, a stage that resizes the hidden window's surface and acquires from it, M11).

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-wgpu/src/surface.rs:1-128` (module doc 1-4; `Acquired` 10, `WindowSurface` 21, `new` 37, `format` 83, `size` 88, `alpha_mode` 93, `resize` 97, `acquire` 106, `present` 125).
Conflicts: none.

### Anatomy

- A window's swapchain: the surface, its configuration, acquire and present [R crates/lune-ui-wgpu/src/surface.rs:1-4].
- Windowing-free: the app creates the `wgpu::Surface` from any window handle and asks for a redraw when `acquire` says so [R crates/lune-ui-wgpu/src/surface.rs:2-4].
- Resize takes a width and height [R crates/lune-ui-wgpu/src/surface.rs:97].
- Present mode and frame latency: `SurfaceOptions { present_mode, frame_latency }`, default `Fifo`, 2; a mode the surface does not list falls back to `Fifo`; `AutoVsync` and `AutoNoVsync` go to wgpu, and `AutoNoVsync` is the first of `Immediate`, `Mailbox`, `Fifo` the surface lists (D299) [R crates/lune-ui-wgpu/src/surface.rs:30-40, 50-60].
- Blocking: under `Fifo`, `acquire` waits for the display when every drawable is queued. Measured on macOS/Metal only (Apple M1 Pro, one 60 Hz display): with `Repaint::paced` and `AutoNoVsync` (`Immediate` there) a run of frames did not wait in `acquire`. DX12, Vulkan and GL are unmeasured; tearing is unchecked on every platform (D299).
- No configure while an acquired texture is alive: `resize` and `set_options` say so, and `acquire` drops a suboptimal texture before it reconfigures (D11). At 0.4.0 `acquire` reconfigured with the texture alive, which wgpu rejects with a panic; only Vulkan reports suboptimal, so the fix is checked by reading only.

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
- Frame pacing on the real renderer (D299; macOS/Metal only; Apple M1 Pro, one 60 Hz display; `cargo run --release -p gallery -- --pace-probe`, a logged-in session, the window left visible and untouched). Reading at 9250138: exit 0, first run, `load1=5.20`. Phase 1 (0.4.0, `Now` every frame, `Fifo`): scheduled 60.0/s (180 frames), acquire p50 16.21 p95 16.38 p99 16.87 max 16.88 ms, busy 1000 ms/s. Phase 2 (busy cadence, `Fifo`): scheduled 30.0/s (90 frames), acquire p50 0.04 p95 0.06 p99 0.07 max 0.07 ms, busy 14 ms/s. Phase 3 (paced, `Fifo`): scheduled 60.0/s (180 frames), acquire p50 0.03 p95 0.05 p99 0.07 max 0.08 ms, busy 24 ms/s. Phase 4 (paced, `AutoNoVsync`): scheduled 60.0/s (180 frames), acquire p50 0.03 p95 0.04 p99 0.05 max 0.06 ms, busy 28 ms/s. Verdict: held. Phase 4's `AutoNoVsync` resolved to `Immediate` on Metal (the planner's listing; the probe does not print it, as `options()` was not read back). Frames were all scheduled (no `os` frames, no `Skip` or `Redraw`).
- The headless target path, which shares the format and size handling: `crates/lune-ui-wgpu/tests/targets.rs::one_renderer_across_two_sizes_matches_a_fresh_renderer_at_each`.
- Not done here: the pixel gate on a presented frame.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.gpu, platform.window.

## Open questions

- Window-mode presentation is not exercised by an automated test (no visible windows in the gates).
- Frame pacing (D298, D299): `paced` with `AutoNoVsync` is measured on macOS/Metal only; DX12, Vulkan and GL are unmeasured; tearing under `AutoNoVsync` is unchecked on every platform; `acquire`'s suboptimal path (D11) is fixed by reading and never run (Metal never reports suboptimal).
- Class B (D234): this entry's own Acceptance says "the pixel gate on a presented frame (open: the platform run is not in this milestone)". A presented frame is what the OS composites to the screen; nothing here can read it back (reading it would need a `COPY_SRC` surface configuration for every app, D234). So the entry stays `in-progress` and `owner:` names the run that closes it: a consumer's window run that presents a frame and compares it.

## Notes for the planner

Added in M2 step 3.
