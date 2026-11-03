---
id: render.blur
title: Backdrop blur pass
area: render
status: in-progress
consumers: luneterm
source: crates/lune-ui-wgpu/src/blur.rs
owner: orch/toolkit-v1
summary: Backdrop blur pass with its WGSL shader
---

## What it is

`Blur` (public: `scene_view`, `dump`; the rest is crate-private, driven by `FrameRenderer`). It reads the core `BackdropRequest` that the painter records. The private uniform struct is `BackdropUniform`, so no name clashes with the `Backdrop` trait in `lune-ui-core`.

## Needs

- Backdrop blur (R-render-2).

## Existing code

`crates/lune-ui-wgpu/src/blur.rs:190` (`Blur`), `:315` (`scene_view`), `:383` (`dump`), `:20` (`BackdropUniform`); 405 lines; `blur.wgsl` 95 lines. No tests of its own; covered by the gallery's blur scenes.

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-wgpu/src/blur.rs:1-2,187-196,315-318,383-386` (module doc, `Blur` struct, `scene_view`, `dump`);
`crates/lune-ui-core/src/paint.rs:189` (`backdrop`, the painter call that requests the blur).
Conflicts: none. The menu glass in Wane is `blur(30px) saturate(1.8)` [W docs/ui/components.md:543-556]; the repo's blur takes its strength from the backdrop request, and the saturate has no parameter (Open questions).

### Anatomy

- Offscreen scene texture, then a copy, a horizontal and a vertical blur pass, each with its own parameter buffer [R crates/lune-ui-wgpu/src/blur.rs:190-196].
- The scene texture view is exposed for the frame pass [R crates/lune-ui-wgpu/src/blur.rs:315].
- Backdrop request: a rect and a radius [R crates/lune-ui-core/src/paint.rs:189].

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

- Existing tests: none in `crates/lune-ui-wgpu/src/blur.rs`.
- Done standard still needs: a pixel gate on `blur-contrast` and `menu` at zero delta, and a test that the scene texture is rebuilt on a size change.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.frame, render.gpu, core.scene.

## Open questions

- None: the old name clash is gone (the struct is private `BackdropUniform`; the trait is `lune-ui-core`'s `Backdrop`, implemented by `WindowBackdrop` and `NullMaterial`).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
