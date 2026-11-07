---
id: core.testing
title: Seam fakes and contract suites
area: core
status: done
consumers: any
source: crates/lune-ui-core/src/testing/mod.rs
owner: —
summary: Public testing module: RecordingRenderer, FakeText, ScriptedInput, NullMaterial and the contract suites
---

## What it is

`hit_regions(&UiStore)` lists the last frame's hit regions (`HitRegion`: id, rect, layer, tag, enabled) in draw order (D266). The fakes of the four seams, public so apps can test their UI code with them: `RecordingRenderer` (consumes a `Scene` in the documented draw order and records each draw call), `FakeText` with `text_system_contract` (the properties every `TextSystem` must have), `ScriptedInput` (builds `RawInput` frames: pointer, clicks, scroll, keys, IME, focus), and `NullMaterial` with `backdrop_contract`. The facade re-exports the module as `lune_ui::testing`.

## Needs

- Each seam gets a fake and a contract test (R-nfr-8).

## Existing code

`crates/lune-ui-core/src/testing/recording.rs:70` (`RecordingRenderer`), `text.rs:27` (`FakeText`), `:321` (`text_system_contract`), `input.rs:18` (`ScriptedInput`), `:240` (`scripted_input_contract`), `backdrop.rs:9` (`NullMaterial`), `:34` (`backdrop_contract`), `images.rs:11` (`FakeImages`), `:105` (`image_store_contract`), `ui.rs:38` (`UiHarness`), `disabled_violations`. The real adapters run the same suites: `crates/lune-ui-wgpu/tests/text.rs` (`TextCache`), `crates/lune-ui-winit/tests/input.rs` (`Translator` against `ScriptedInput`).

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-core/src/testing/mod.rs:1-21` (module and re-exports); the test files `crates/lune-ui-core/src/testing/backdrop.rs`, `recording.rs`, `text.rs` (test names).
Conflicts: none.

### Anatomy

- Backdrop: `NullMaterial`, `backdrop_contract` [R crates/lune-ui-core/src/testing/mod.rs:18].
- Input: `ScriptedInput` [R crates/lune-ui-core/src/testing/mod.rs:19].
- Recording: `DrawCall`, `RecordedFrame`, `RecordingRenderer`, and `flattened` (a scene with its overlays folded into the fixed layers, to compare with the Painter API) [R crates/lune-ui-core/src/testing/mod.rs:20].
- Text: `FakeText`, `text_system_contract` [R crates/lune-ui-core/src/testing/mod.rs:21].

### States

n/a — test fixtures; no state of their own.

### Transitions

n/a — no motion.

### Input and focus

n/a — `ScriptedInput` feeds input into the app under test; it has no focus of its own.

### Retained state

n/a — fixtures reset per test.

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

n/a — the fixtures are headless; the gallery draws the same scenes on the real GPU (`apps/gallery/src/scenes.rs:82-180`).

### Painter primitives

- Existing: none.
- Note: The recording renderer records the painter's calls)..
- New: none.

### Acceptance

- Each seam's fake meets its contract and the contract catches a broken one (all in `crates/lune-ui-core/src/testing/`): backdrop `null_material_meets_the_contract`, `contract_catches_broken_backdrops`; input `input.rs::scripted_input_meets_its_contract` (I1 to I5: time passes as given, events in order, each once, button events carry the position current at that point, printable text follows a key) and `contract_catches_broken_scripted_inputs` (a fake that repeats events fails I3, one that advances the clock fails I1, one that reorders fails I2 or I5); images `the_fake_meets_the_contract`, `the_contract_catches_a_store_that_accepts_bad_data`; text `fake_text_meets_the_contract`, `fake_text_sizes_follow_its_formula` and the `the_contract_catches_*` cases; the recording renderer `records_the_documented_draw_order`; the harness `ui.rs::disabled_violations_names_the_bad_nodes_under_the_root_only`.
- The real adapters run the same suites: `crates/lune-ui-wgpu/tests/text.rs::text_cache_with_fonts_meets_the_contract` (`TextCache`), `crates/lune-ui-wgpu/tests/images.rs::the_gpu_store_meets_the_contract` (`Gpu` as `ImageStore`), `crates/lune-ui-winit/tests/input.rs::translator_and_scripted_input_agree` (`Translator` against `ScriptedInput`). The backdrop contract has no real adapter run here (`platform.material`).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: core.painter, core.scene, text.shaping.
- Done (D47, M11): tests `crates/lune-ui-core/src/testing/input.rs::scripted_input_meets_its_contract, crates/lune-ui-core/src/testing/input.rs::contract_catches_broken_scripted_inputs, crates/lune-ui-core/src/testing/backdrop.rs::contract_catches_broken_backdrops, crates/lune-ui-core/src/testing/images.rs::the_contract_catches_a_store_that_accepts_bad_data, crates/lune-ui-core/src/testing/text.rs::fake_text_meets_the_contract, crates/lune-ui-core/src/testing/recording.rs::records_the_documented_draw_order, crates/lune-ui-core/src/testing/ui.rs::disabled_violations_names_the_bad_nodes_under_the_root_only, crates/lune-ui-winit/tests/input.rs::translator_and_scripted_input_agree`; page n/a (headless fixtures); unverified here: the backdrop contract against a real OS window (`platform.material`).

## Open questions

- None. A cargo feature was rejected (DECISIONS D36); the module is always compiled.

## Notes for the planner

Added across M2 steps 2 to 7. Later seams (clipboard, file dialogs) add their fake and contract here.
