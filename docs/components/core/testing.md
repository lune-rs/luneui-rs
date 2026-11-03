---
id: core.testing
title: Seam fakes and contract suites
area: core
status: in-progress
consumers: any
source: crates/lune-ui-core/src/testing/mod.rs
owner: orch/toolkit-v1
summary: Public testing module: RecordingRenderer, FakeText, ScriptedInput, NullMaterial and the contract suites
---

## What it is

The fakes of the four seams, public so apps can test their UI code with them: `RecordingRenderer` (consumes a `Scene` in the documented draw order and records each draw call), `FakeText` with `text_system_contract` (the properties every `TextSystem` must have), `ScriptedInput` (builds `RawInput` frames: pointer, clicks, scroll, keys, IME, focus), and `NullMaterial` with `backdrop_contract`. The facade re-exports the module as `lune_ui::testing`.

## Needs

- Each seam gets a fake and a contract test (R-nfr-8).

## Existing code

`crates/lune-ui-core/src/testing/recording.rs:57` (`RecordingRenderer`), `crates/lune-ui-core/src/testing/text.rs:16` (`FakeText`), `:64` (`text_system_contract`), `crates/lune-ui-core/src/testing/input.rs:16` (`ScriptedInput`), `crates/lune-ui-core/src/testing/backdrop.rs:9` (`NullMaterial`), `:34` (`backdrop_contract`). The real adapters run the same suites: `crates/lune-ui-wgpu/tests/text.rs` (`TextCache`), `crates/lune-ui-winit/tests/input.rs` (`Translator` against `ScriptedInput`).

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-core/src/testing/mod.rs:1-21` (module and re-exports); the test files `crates/lune-ui-core/src/testing/backdrop.rs`, `recording.rs`, `text.rs` (test names).
Conflicts: none.

### Anatomy

- Backdrop: `NullMaterial`, `backdrop_contract` [R crates/lune-ui-core/src/testing/mod.rs:18].
- Input: `ScriptedInput` [R crates/lune-ui-core/src/testing/mod.rs:19].
- Recording: `DrawCall`, `RecordedFrame`, `RecordingRenderer` [R crates/lune-ui-core/src/testing/mod.rs:20].
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

- Existing tests (by path): `crates/lune-ui-core/src/testing/backdrop.rs` `null_material_meets_the_contract`, `contract_catches_broken_backdrops`; `crates/lune-ui-core/src/testing/recording.rs` `records_the_documented_draw_order`; `crates/lune-ui-core/src/testing/text.rs` `fake_text_meets_the_contract`, `fake_text_sizes_follow_its_formula`.
- Done standard still needs: a `ScriptedInput` contract test (the winit side is in `crates/lune-ui-winit/tests/input.rs`, `translator_and_scripted_input_agree`).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: core.painter, core.scene, text.shaping.

## Open questions

- None. A cargo feature was rejected (DECISIONS D36); the module is always compiled.

## Notes for the planner

Added across M2 steps 2 to 7. Later seams (clipboard, file dialogs) add their fake and contract here.
