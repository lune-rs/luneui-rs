---
id: core.scene
title: Display list
area: core
status: done
consumers: any
source: crates/lune-ui-core/src/scene.rs
owner: —
summary: Plain-data Scene the painter builds and a renderer consumes; the draw-order contract
---

## What it is

`Scene` (six fixed `Layer`s, an `OverlayLayer` per overlay and an optional `BackdropRequest`), `LayerGeometry` (acrylic, under, over shapes and strokes), `Shape`, `StrokeVertex`, `TextItem`, `CustomItem`, `LayerId`, `LAYER_COUNT`, `SCENE_LAYERS` and `FrameParams`. Std only, in physical pixels; the records mirror the GPU's field by field and the wgpu adapter copies them into its own Pod structs. This is the renderer seam: the data, not a trait. The draw-order contract is in the module doc: layers back to front in `Scene::in_order`, each fixed layer followed by the overlays on it in open order ([D256]); in a layer acrylic, under shapes, custom items, text, over shapes, strokes; layers under `SCENE_LAYERS` render offscreen, which a backdrop request blurs.

## Needs

- A display list a renderer can consume without the painter (R-render-1, R-nfr-8).
- A fake renderer for tests of painting code (R-nfr-8).

## Existing code

`crates/lune-ui-core/src/scene.rs:179` (`Scene`), `:44` (`Shape`), `:86` (`LayerGeometry`), `:109` (`CustomItem`), `:136` (`ImageItem`), `:165` (`BackdropRequest`), `:197` (`FrameParams`); 206 lines, the draw-order contract in its header. Consumers: `crates/lune-ui-wgpu/src/gpu.rs:554` (`upload`), `crates/lune-ui-core/src/testing/recording.rs` (the fake).

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-core/src/scene.rs:17-116` (layer id, counts, shape, stroke, layer geometry, text and custom items, layer, backdrop request, scene, frame params);
`crates/lune-ui-core/src/testing/recording.rs` (the draw-order test); `apps/gallery/src/scenes.rs:369` (the menu on `LayerId::Popup`), `:131-136` (dialogs on the Modal layer through the `dialog` widget, `crates/lune-ui-core/src/widgets/dialog.rs:50`).
Conflicts: none.

### Anatomy

- Layers in draw order: `Base` 0, `Page` 1, `Popup` 2, `Modal` 3, `Raised` 4, `Toast` 5 [R crates/lune-ui-core/src/scene.rs `LayerId`]. `Raised` holds an overlay opened above an active modal; the `Ui` chooses it at open ([D137]). `scene.layers[id as usize]` is what was drawn on `id` outside any overlay; `Scene::merged(id)` folds in the overlays on it, for tests and inspection ([D256]).
- `LAYER_COUNT` 6 [R crates/lune-ui-core/src/scene.rs].
- `SCENE_LAYERS` 2: the layers under a popup draw to an offscreen scene so a glass menu can blur them. `Scene::split()` is a place in `Scene::in_order`: the fixed layer `SCENE_LAYERS`, or the backdrop request's own layer when that is later, so a glass surface blurs everything drawn before it; `BackdropRequest` carries `layer`, `tag` (its overlay, 0 for none), `sigma` and `saturation` (0 means the frame's). One backdrop per frame: of two, the lower layer keeps it ([D138]).
- Items per layer: `Shape` (31), `StrokeVertex` (43), `LayerGeometry` (55), `TextItem` (63), `CustomItem` (74), `Layer` (82), `BackdropRequest` (91), `Scene` (99), `FrameParams` (107) [R crates/lune-ui-core/src/scene.rs:31-107].

### States

| state | look | cite |
|---|---|---|
| layer on (Base, Page, Popup, Modal, Raised, Toast) | items land on the current layer | [R crates/lune-ui-core/src/paint.rs:162] |

### Transitions

n/a — the scene is rebuilt each frame; no motion.

### Input and focus

n/a — the scene holds no input; hit regions belong to the painter (`hit`).

### Retained state

- Transient: the scene is per frame; nothing survives a frame.
- Persistent: none; the app passes the painter each frame.

### Accessibility

n/a — no UI node.

### Strings

n/a — text items carry strings from the caller.

### Gallery scenes

- `menu` (existing): the menu on `Popup`, scenes.rs:95-101,366-392.
- `dialog-confirm` (existing): the dialog on `Modal`, scenes.rs:130-136,470-481.
- `custom-draw` (existing): a custom item on its layer, scenes.rs:109-115.

### Painter primitives

- Existing: none.
- Note: The painter writes into it.
- New: none.

### Acceptance

- The draw order: `crates/lune-ui-core/src/testing/recording.rs::records_the_documented_draw_order` (acrylic, under, path fills, customs, images, text, over, strokes inside a layer; the backdrop at the split).
- Items land on the layer set by `Painter::on`, for every kind, under the clip and `Xform`: `crates/lune-ui-core/tests/painter.rs::fills_rings_and_backdrops_take_the_layer_clip_scale_and_alpha_of_the_xform`, `strokes_fans_and_polygons_take_the_layer_clip_scale_and_alpha_of_the_xform`, `text_takes_the_layer_clip_scale_and_alpha_of_the_xform`, `image_lands_on_the_current_layer_under_the_clip_and_xform`, `text_rotated_lands_on_the_current_layer_under_the_clip_and_xform` (each asserts the other layers stay empty), and the paint.rs unit test `custom_lands_on_the_current_layer` for customs.
- The split: `painter.rs::the_recording_renderer_puts_the_backdrop_at_the_split`, `of_two_backdrops_the_lower_layer_keeps_it_in_either_draw_order`.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: core.painter, render.frame.
- Done (D47, M11): tests `crates/lune-ui-core/src/testing/recording.rs::records_the_documented_draw_order, crates/lune-ui-core/tests/painter.rs::fills_rings_and_backdrops_take_the_layer_clip_scale_and_alpha_of_the_xform, crates/lune-ui-core/tests/painter.rs::image_lands_on_the_current_layer_under_the_clip_and_xform, crates/lune-ui-core/tests/painter.rs::the_recording_renderer_puts_the_backdrop_at_the_split`; page n/a (plain data; drawn by the legacy scenes `menu`, `dialog-confirm`, `custom-draw`); unverified here: none.

## Open questions

- Layers added after M2, each at zero pixel delta for the scenes that existed: `Toast` above `Modal` by M4 ([D54], `LAYER_COUNT` 5), and `Raised` between `Modal` and `Toast` by M7 ([D137], `LAYER_COUNT` 6, `Toast` is 5). `SCENE_LAYERS` stays 2.
- A second renderer (a software one, say) is the real test of the data shape; none exists. No renderer trait on purpose (DECISIONS D25).

## Notes for the planner

Added in M2 step 2. Zero pixel delta against M1 on the 15 gallery scenes. `core.testing` holds the fake.
