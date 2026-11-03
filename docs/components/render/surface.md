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

## Open questions

- Window-mode presentation is not exercised by an automated test (no visible windows in the gates).

## Notes for the planner

Added in M2 step 3.
