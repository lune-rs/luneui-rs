---
id: core.input
title: Input model
area: core
status: done
consumers: any
source: crates/lune-ui-core/src/input.rs
owner: —
summary: Plain-data RawInput in, FrameOutput out; the windowing adapter translates both
---

## What it is

`RawInput` (logical size, scale, app-supplied time as `Duration`, focus, and the frame's `Event`s in arrival order), `Event` (pointer move, leave and button, scroll, key, text, IME), `PointerButton`, `ScrollDelta`, `Key`, `NamedKey`, `Modifiers`, `ImeEvent`; and `FrameOutput` (`CursorIcon`, IME area, `WindowCommand`s). `printable` is the one rule for when a key press also emits text. Coordinates are logical (physical divided by scale). `Event`, `NamedKey`, `CursorIcon` and `WindowCommand` are `#[non_exhaustive]`. Touch, gestures, file drops, theme and occlusion events are not modelled yet.

## Needs

- A windowing-free input seam (R-nfr-8); the toolkit never reads a clock (R-nfr-1).
- Cursor icons and window operations as data (R-platform-4, R-platform-5).
- An accessibility action as input, routed like a pointer or key event (R-infra-4).
- Inbound OS file-drop events (R-platform-5, D43).

## Existing code

`crates/lune-ui-core/src/input.rs:21` (`RawInput`), `:37` (`Event`), `:119` (`NamedKey`), `:282` (`FrameOutput`), `:305` (`WindowCommand`), `:321` (`CursorIcon`), `:405` (`printable`); 472 lines. The winit side is `platform.input`; the fake is `testing::ScriptedInput` (`core.testing`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/input.rs` (all 230 lines),
`crates/lune-ui-core/src/testing/input.rs:1-170`, `crates/lune-ui-winit/src/input.rs` (all), `crates/lune-ui-winit/tests/input.rs:95-130`;
Lunegit src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:10-30, src/Lunegit.Desktop/MainWindow.axaml.cs:91-113;
winit 0.30.13 `src/event.rs:178-192` in the local cargo registry (the file-drop events; not a pinned-repo source).
Conflict: none. Neither Lunegit nor Wane uses OS file drop or touch: Lunegit's kanban deliberately avoids OS
drag and drop for a pointer-capture gesture [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:18-24] and Wane mocks show none;
inbound OS file-drop events are in the toolkit by scope decision D43 alone.

### Anatomy

Moved, today (`crates/lune-ui-core/src/input.rs`):
- `RawInput { size, scale, time, focused, events }`: logical size, device pixels per logical pixel, app-supplied time as a `Duration`, window focus, events in arrival order [R crates/lune-ui-core/src/input.rs:16-27].
- `Event` (`#[non_exhaustive]`): `PointerMoved`, `PointerLeft`, `PointerButton { pos, button, pressed, modifiers }`, `Scroll { pos, delta, modifiers }`, `Key { key, pressed, repeat, modifiers }`, `Text`, `Ime`, `Focus`, `ModifiersChanged` [R crates/lune-ui-core/src/input.rs:29-65].
- `PointerButton` (primary, secondary, middle, back, forward, other) and `ScrollDelta::{Lines, Pixels}` (lines for a notched wheel, logical pixels for a touchpad) [R crates/lune-ui-core/src/input.rs:67-85].
- `Key::{Named, Character, Unidentified}`; `NamedKey` (`#[non_exhaustive]`) holds Enter, Tab, Space, Backspace, Delete, Insert, Escape, Home, End, PageUp, PageDown, the arrows and F1 to F12 [R crates/lune-ui-core/src/input.rs:87-128].
- `Modifiers { shift, control, alt, logo }`; `ImeEvent::{Enabled, Preedit { text, cursor }, Commit, Disabled}` with byte offsets as the platform gives them [R crates/lune-ui-core/src/input.rs:130-152], [D35].
- `FrameOutput { cursor, ime_area, commands }`; `WindowCommand` (`#[non_exhaustive]`: drag move, drag resize, minimize, toggle maximize, close, redraw); `CursorIcon` (the CSS names, `#[non_exhaustive]`); `printable` is the one rule for when a key press also emits `Text` [R crates/lune-ui-core/src/input.rs:154-230].

Additions (new variants on the `#[non_exhaustive]` enums, so no break) [D35]:
- M4: `Event::Access(AccessAction { target: WidgetId, action, value })`, routed by the `Ui` as the matching pointer or key input [infra.accessibility]. `FrameOutput` gains `repaint`, the semantic `tree`, `focused` and `clipboard` requests (the shapes are in [core.ui] and [platform.clipboard]); the existing three fields keep their types [R crates/lune-ui-core/src/input.rs:155-164].
- M10: file-drop events `Event::FileHover { pos, path }`, `Event::FileHoverCancelled` and `Event::FileDrop { pos, path }` (one event per file; `pos` is the last pointer position, because winit's events carry none) [D43], [derived: winit 0.30.13 `WindowEvent::HoveredFile(PathBuf)`, `HoveredFileCancelled`, `DroppedFile(PathBuf)` at `src/event.rs:180-192` in the cargo registry].
- M10: touch and gesture events stay out of v1 (no source needs them) [D35].
- Clipboard text arriving from the OS is an event the clipboard entry defines; `core.input` reserves the variant in M4, with the `clipboard` field of `FrameOutput` [platform.clipboard].
- Coordinates stay logical (physical divided by scale) and time stays app-supplied: the toolkit reads no clock [R crates/lune-ui-core/src/input.rs:5-6], [D3].

### States

n/a — plain data, not a component.

### Transitions

n/a — no animation.

### Input and focus

- Consumers: the `Ui` frame (`core.ui`) reads pointer, scroll, key, text, IME and focus events in order; `Event::Focus` and `RawInput.focused` are the window's focus, not a widget's [core.ui], [core.focus].
- Layer: n/a. Tab stop: n/a.
- A key press that types text is followed by a `Text` event; both are delivered, and the focused text widget takes `Text`, while shortcuts see `Key` [R crates/lune-ui-core/src/input.rs:57-60], [infra.shortcut-routing].
- Escape, Enter, Tab and the arrows are the keys Lunegit's own handlers use beyond letters (Escape closes a dialog, Up, Down and Enter drive a search list) and all exist as `NamedKey` [L src/Lunegit.Desktop/MainWindow.axaml.cs:93-97], [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml.cs:23-40], [R crates/lune-ui-core/src/input.rs:100-115].

### Retained state

- Transient: none (plain data; the adapter's `Translator` holds the pointer, modifiers and focus between frames, see `platform.input`).
- Persistent: none. App-owned: `time`, `size`, `scale`, the events.
- Exit retention: no.

### Accessibility

`AccessAction` is the one accessibility input; its fields and routing are in [infra.accessibility].

### Strings

None: no user-visible text.

### Gallery scenes

The 15 legacy scenes stay on the Painter API and pass hover directly to each scene [R apps/gallery/src/scenes.rs:90-94] [D59]. The Ui pages (`button`, `dialog`, `menu`, `ui`, `focus` and the rest) are driven by a scripted `RawInput` through `ScriptedInput`, so they exercise it. The file events (M10) have the `file-drop` page, whose scenes send them through the script's `Ev::FileHover`, `Ev::FileHoverCancelled` and `Ev::FileDrop`:

- `file-drop-hover`: a file is over the window with the pointer at (300, 200); the page lights its zone and shows the path it read from `ui.input()`, settled.
- `file-drop-dropped`: the same hover, then the drop; the path is listed, settled.

In the window the events are the OS's: drag a file over it.

### Painter primitives

n/a — no drawing.

### Acceptance

- Tests: `crates/lune-ui-winit/tests/input.rs` (`translator_and_scripted_input_agree`, `state_persists_and_events_drain`, `unmapped_keys_are_unidentified`, `other_mouse_button_keeps_its_number`, `every_cursor_icon_maps_to_the_same_named_winit_icon`); `printable` has a unit test in the file (`crates/lune-ui-core/src/input.rs::printable_drops_control_characters`: a bell drops to `None`, a tab is dropped inside text, empty is `None`, `é` stays); an `Access` event round trip through `ScriptedInput::access` is `crates/lune-ui-core/tests/ui_access.rs::default_clicks_a_checkbox_and_sets_changed` (the action reaches the widget and sets `changed`); file-drop events through `ScriptedInput` and the translator (M10: `crates/lune-ui-winit/tests/input.rs` `files::the_translator_and_the_script_agree_on_file_events`, `files::hover_cancel_and_drop_give_the_three_events_at_the_last_pointer_position`; the core side, `crates/lune-ui-core/tests/input.rs` `file_events_reach_the_app_and_change_no_widget_state` and `a_file_event_alone_asks_for_no_repaint`: the events reach `ui.input().events` and change no widget state and ask for no frame); a match over each `#[non_exhaustive]` enum in a downstream test compiles with a wildcard arm (`crates/lune-ui-core/tests/input.rs::non_exhaustive_seam_enums_match_with_a_wildcard_downstream`: `Event`, `NamedKey`, `WindowCommand`, `CursorIcon`).
- `ScriptedInput` gains `access(...)` and `file_hover(...)`, `file_hover_cancelled()`, `file_drop(...)` builders (M4, M10, D211); `Translator` and `ScriptedInput` agree on the new events in the same test style [R crates/lune-ui-core/src/testing/input.rs:16-30,161].
- No clock read: the boundary script stays green; `RawInput.time` is the only time [R crates/lune-ui-core/src/input.rs:5-6].

### Build

- Milestone: M4 (`Access`, the `FrameOutput` additions) and M10 (file-drop events).
- Depends on: core.ui, infra.accessibility, platform.clipboard.
- Done (D47, M11): tests `crates/lune-ui-core/src/input.rs::printable_drops_control_characters, crates/lune-ui-core/tests/input.rs::non_exhaustive_seam_enums_match_with_a_wildcard_downstream, crates/lune-ui-core/tests/input.rs::file_events_reach_the_app_and_change_no_widget_state, crates/lune-ui-core/tests/ui_access.rs::default_clicks_a_checkbox_and_sets_changed, crates/lune-ui-winit/tests/input.rs::translator_and_scripted_input_agree`; page `file-drop` (the Ui pages are driven by `ScriptedInput`); unverified here: real OS events (`platform.input`).

## Open questions

- Answered: lines per notch. `layout.scroll-area` converts a line to 48 logical px (`LINE`, `crates/lune-ui-core/src/widgets/scroll_area.rs:32`); no source states a value, so it is the toolkit's choice.
- Answered (D35, D103): IME offsets stay winit's byte offsets; the widget converts them to char offsets (`crates/lune-ui-core/tests/ime.rs::a_preedit_cursor_in_bytes_is_a_char_offset_in_the_widget`).
- Still open, a limit: NamedKey gaps: no ContextMenu, CapsLock or keypad keys; nothing in Lunegit's or Wane's specs needs one in the toolkit (the terminal's own keys stay in the terminal app, D8).

## Notes for the planner

Added in M2 step 6; M4's focus and click handling was its first consumer.
