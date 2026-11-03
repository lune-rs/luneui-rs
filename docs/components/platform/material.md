---
id: platform.material
title: OS glass materials
area: platform
status: in-progress
consumers: luneterm
source: crates/lune-ui/src/material.rs
owner: orch/toolkit-v1
summary: Material enum and apply: macOS NSVisualEffect and Liquid Glass, Windows acrylic and mica, opaque elsewhere
---

## What it is

`Material` and `apply`, dispatching to `#[cfg]` submodules: `material/macos.rs` (NSVisualEffectView, NSGlassEffectView), `material/windows.rs` (DWM acrylic, mica, tabbed via `window-vibrancy`), `material/other.rs` (glass returns `Err`).

## Needs

- macOS glass (R-platform-1); Windows glass (R-platform-2); opaque fallback elsewhere (R-platform-3).
- The only `unsafe` blocks (3, in `material/macos.rs`) stay in this module, each with a `// SAFETY:` comment (R-nfr-3).

## Existing code

`crates/lune-ui/src/material.rs:23` (`Material`), `:47` (`apply`); `material/macos.rs:63`, `:65`, `:75` (`unsafe`); 49 + 77 + 23 + 11 lines. No tests.

## Open questions

- Windows glass is unverified on hardware by this inventory; Luneterm's go/no-go gate covers it (Luneterm decision 3).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
