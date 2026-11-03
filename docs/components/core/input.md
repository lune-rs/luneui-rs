---
id: core.input
title: Input model
area: core
status: in-progress
consumers: any
source: crates/lune-ui-core/src/input.rs
owner: orch/toolkit-v1
summary: Plain-data RawInput in, FrameOutput out; the windowing adapter translates both
---

## What it is

`RawInput` (logical size, scale, app-supplied time as `Duration`, focus, and the frame's `Event`s in arrival order), `Event` (pointer move, leave and button, scroll, key, text, IME), `PointerButton`, `ScrollDelta`, `Key`, `NamedKey`, `Modifiers`, `ImeEvent`; and `FrameOutput` (`CursorIcon`, IME area, `WindowCommand`s). `printable` is the one rule for when a key press also emits text. Coordinates are logical (physical divided by scale). `Event`, `NamedKey`, `CursorIcon` and `WindowCommand` are `#[non_exhaustive]`. Touch, gestures, file drops, theme and occlusion events are not modelled yet.

## Needs

- A windowing-free input seam (R-nfr-8); the toolkit never reads a clock (R-nfr-1).
- Cursor icons and window operations as data (R-platform-4, R-platform-5).

## Existing code

`crates/lune-ui-core/src/input.rs:16` (`RawInput`), `:32` (`Event`), `:156` (`FrameOutput`), `:169` (`WindowCommand`), `:185` (`CursorIcon`), `:227` (`printable`); 230 lines. The winit side is `platform.input`; the fake is `testing::ScriptedInput` (`core.testing`).

## Open questions

- Touch and gestures (M10) and clipboard, file drop and theme events (M5) extend the `#[non_exhaustive]` enums. Whether IME offsets stay winit's byte offsets is for M5 (DECISIONS D35).

## Notes for the planner

Added in M2 step 6. No widget consumes `RawInput` yet; M4's focus and click handling is the first consumer.
