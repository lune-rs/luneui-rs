---
id: platform.input
title: winit input translation and output
area: platform
status: in-progress
consumers: any
source: crates/lune-ui-winit/src/input.rs
owner: orch/toolkit-v1
summary: Translator turns winit events into RawInput; OutputState applies FrameOutput to a window
---

## What it is

`Translator` (one per window: `window_event` or the per-event methods, then `take(time)` to get a `RawInput`) divides physical positions by the scale factor and ignores touch, gestures, file drops, theme and occlusion. `OutputState` (one per window) applies a `FrameOutput` and touches the cursor and the IME only when they changed; `apply` returns true when the frame asked to close. `cursor_icon` maps the core `CursorIcon` to winit's. The app owns the event loop and calls these from it.

## Needs

- Input and output as data between winit and the toolkit (R-nfr-8, R-platform-5 for cursors).

## Existing code

`crates/lune-ui-winit/src/input.rs:26` (`Translator`), `:171` (`take`), `:235` (`cursor_icon`), `:281` (`OutputState`), `:293` (`apply`); 334 lines. Tests: `crates/lune-ui-winit/tests/input.rs` (translator and `ScriptedInput` agree; every cursor icon maps to the same named winit icon).

## Open questions

- Unmapped named keys become `Unidentified`; M5 extends the mapping. `OutputState::apply` against a live window is not run by the gates.

## Notes for the planner

Added in M2 step 6. The core model is `core.input`.
