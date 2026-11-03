---
id: platform.material
title: OS glass materials
area: platform
status: in-progress
consumers: luneterm
source: crates/lune-ui-platform/src/material.rs
owner: orch/toolkit-v1
summary: Backdrop seam in core (Material, Backdrop, NullMaterial); OS glass behind it: macOS NSVisualEffect and Liquid Glass, Windows acrylic and mica, opaque elsewhere
---

## What it is

Core: `Material`, `BackdropKind`, `GlassLayer` and the `Backdrop` trait (`best_supported`, `apply`); fake `testing::NullMaterial` and `testing::backdrop_contract`. Platform crate: `apply(&winit::window::Window, Material)` and `WindowBackdrop`, which implements `Backdrop` over it, dispatching to `#[cfg]` submodules: `material/macos.rs` (NSVisualEffectView, NSGlassEffectView), `material/windows.rs` (DWM acrylic, mica, tabbed via `window-vibrancy`), `material/other.rs` (glass returns `Err`).

## Needs

- macOS glass (R-platform-1); Windows glass (R-platform-2); opaque fallback elsewhere (R-platform-3).
- The only `unsafe` blocks (3, in `lune-ui-platform/src/material/macos.rs`) stay in this crate under `src/material/`, each with a `// SAFETY:` comment (R-nfr-3; boundary rule 6).

## Existing code

`crates/lune-ui-core/src/backdrop.rs:27` (`Material`), `:49` (`Backdrop`); `crates/lune-ui-platform/src/material.rs:34` (`apply`), `:40` (`WindowBackdrop`); `crates/lune-ui-platform/src/material/macos.rs:67`, `:69`, `:79` (`unsafe`); 50 + 81 + 27 + 15 lines. The contract test runs on the fake in `crates/lune-ui-core/src/testing/backdrop.rs`; OS glass itself is untested here.

## Open questions

- Windows glass is unverified on hardware by this inventory; Luneterm's go/no-go gate covers it (Luneterm decision 3).

## Notes for the planner

The seam landed in M2 step 7. `apply` still takes a winit `Window` (DECISIONS D31: a `HasWindowHandle` form needs a new SAFETY argument).
