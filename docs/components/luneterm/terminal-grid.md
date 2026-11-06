---
id: luneterm.terminal-grid
title: Terminal grid renderer
area: luneterm
status: external
consumers: luneterm
source: —
owner: —
summary: Terminal grid drawing stays in Luneterm (lune-termview) and plugs in through the CustomDraw hook
---

## What it is

Luneterm's `lune-termview` draws the terminal grid through `CustomDraw`. It stays in Luneterm (decision D8); the toolkit only provides the hook (`render.custom-draw`). The grid model, the glyph atlas and the passes (cell backgrounds, glyphs, decorations, cursor, selection, link hover) are Luneterm's.

## Needs

- The hook it builds on must stay stable (R-render-3).
- Not a toolkit deliverable (D8).

## Existing code

External: Luneterm `crates/lune-termview` (depends on lune-ui plus wgpu, cosmic-text, fontdb, etagere, zeno; not on lune-vt). It imports `lune_ui::custom::{CustomDraw, CustomFrame}`, `paint::Rect`, `color::Rgba` and `dump::{target, read_rgba, write_png}`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b, Luneterm 296cfdc. Read: Luneterm docs/architecture.md:52-70,686-691, crates/lune-termview/src/render.rs:1-30; this repo crates/lune-ui-wgpu/src/custom.rs:1-33,
crates/lune-ui-core/src/paint.rs:253-270, apps/gallery/src/scenes.rs:108-115, docs/DECISIONS.md D8. No conflicts: D8 overrides the Luneterm line that says termview moves with `lune-ui` [R docs/DECISIONS.md:17], [LT docs/architecture.md:690-691].

### Reusable parts

- `render.custom-draw`: the hook. The app records `Painter::custom(slot, rect)` in a layer and passes one `CustomDraw` implementation per slot to the frame renderer, which calls `prepare` once per frame and `render` per item with the scissor set to the item's clip, after the layer's under-text shapes and before its text [R crates/lune-ui-wgpu/src/custom.rs:1-8,22-33], [R crates/lune-ui-core/src/paint.rs:253-270], [LT docs/architecture.md:66-68].
- `core.painter`: `custom(slot, rect)` records a physical clip on the Base layer; custom items are not animated [R crates/lune-ui-core/src/paint.rs:253-270].
- `core.scene`: `CustomItem` (slot, rect, clip) is the plain-data record that crosses the seam [R crates/lune-ui-wgpu/src/custom.rs:9].
- `core.color`: `Rgba` for the terminal theme [LT crates/lune-termview/src/theme.rs:4].
- Gallery: the `custom-draw` scene exercises the hook at zero delta [R apps/gallery/src/scenes.rs:108-115].
- Luneterm's own: `TermView` (`impl CustomDraw`), the grid model, the atlas and the instanced passes [LT docs/architecture.md:52-68], [LT crates/lune-termview/src/render.rs:12].

### Build

- Milestone: external — stays in Luneterm
- Depends on: render.custom-draw, core.painter, core.scene, core.color

## Open questions

- None on the facade's exports. Checked at M11 in a scratch clone of Luneterm 296cfdc holding the snapshot of `2f066e9`: `cargo check --offline -p lune-termview --all-targets` passes with no source change, so `custom`, `paint::Rect`, `color::Rgba` and `dump` resolve (the run and the `glass-ui` migration map are in `docs/consuming.md`). Luneterm itself is not changed; it adopts the toolkit in its own M9 (D16).

## Notes for the planner

Listed so the catalogue shows the boundary. Do not build it here.
