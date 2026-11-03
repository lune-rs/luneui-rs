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

## Open questions

- None. A cargo feature was rejected (DECISIONS D36); the module is always compiled.

## Notes for the planner

Added across M2 steps 2 to 7. Later seams (clipboard, file dialogs) add their fake and contract here.
