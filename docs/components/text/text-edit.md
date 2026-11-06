---
id: text.text-edit
title: Text editing engine
area: text
status: in-progress
consumers: any
source: crates/lune-ui-core/src/text_edit.rs
owner: orch/toolkit-v1
summary: Caret, selection, word and line navigation, undo and redo, clipboard through FrameOutput and preedit for single- and multi-line text
---

## What it is

The editing behaviour under every editable text: a caret and a selection over a buffer, navigation by
grapheme, word and line, delete, undo and redo, cut, copy and paste through `FrameOutput`, and the
preedit text an input method composes (`text.ime`). `widget.text-field` is the single-line widget on
it and `text.code-editor-view` the multi-line one. Today `field` only draws a placeholder, the text and
a caret after the text (`crates/lune-ui-core/src/widgets/mod.rs:120-144`); there is no caret movement,
selection or clipboard. Lunegit gets all of it from Avalonia's `TextBox`; the toolkit has to own it.

## Needs

- Text selection, caret movement and clipboard in text fields (R-text-5).
- IME composition integrated with the edit state (R-text-4).
- Edit state with undo history as persistent state that survives keep-alive (R-core-8).
- Caret and hit queries on the text system (`text.shaping`, `TextSystem`, C2).

## Existing code

Since M9 (D179, D180) the engine is generic over the text it edits: `TextEdit` reads through the `EditText` trait (`char_len`, `fingerprint`, `window`, `slice`) and changes through `EditBuffer` (`replace`), implemented for `str` and `String` (the whole text is the window, so every rule is as before) and for `CodeDoc`, whose window is a line, so an edit costs O(log lines) instead of the whole text. `EditLayout` gained the provided `lines_near`, and `TextEdit::takes(&cmd)` says what `apply(cmd)` would consume, for a widget that must take a key and act on none (a disabled scope, D177). The M9 code view (`text.code-editor-view`) is its second host. A word motion on a `CodeDoc` stops at its window's edge (Q16). Built in M5 (D87, D88, D92, D94, D95, D103): `crates/lune-ui-core/src/text_edit.rs` (`TextEdit`, `EditCommand`, `SpecLayout`) drives `field` and `text_area` (`widgets/field.rs`, `widgets/text_area.rs`). Before it, the only editing code was `field` (`crates/lune-ui-core/src/widgets/mod.rs:120-144`): placeholder, text, a 1.5 px caret after the text. Input events exist
(`crates/lune-ui-core/src/input.rs:32-67`), including `Text`, `Key`, `Ime`; `TextSystem` has `prepare` and `measure` only (`crates/lune-ui-core/src/text.rs:110-118`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: this repo crates/lune-ui-core/src/widgets/mod.rs:45-69,120-144, crates/lune-ui-core/src/input.rs:28-67,86-165, crates/lune-ui-core/src/text.rs:100-118, crates/lune-ui-winit/src/input.rs:145-165;
Lunegit src/Lunegit.Desktop/Controls/FilterBar.axaml:8-44, Views/Settings/GitSettingsView.axaml:60-84, Views/CrashReportWindow.axaml:70-78, Views/SplitDiffView.axaml.cs:253-275; Wane docs/ui/components.md:347-360,709-729.
Lunegit has no editing code of its own for text boxes: it styles Avalonia's `TextBox` and sets a caret brush (grep for `PasswordChar`, `UndoLimit` and `SelectionBrush` over `**/*.axaml` finds none), so the key and undo behaviour below is the platform text-editing convention, marked `[derived]`, not read from Lunegit.
Conflicts: (1) caret colour: Lunegit uses `ForegroundTertiary` on the filter and pickers and `ForegroundPrimary` in the settings views [L src/Lunegit.Desktop/Controls/FilterBar.axaml:26], [L src/Lunegit.Desktop/Views/Settings/GitSettingsView.axaml:70]; the moved `field` draws `fg_primary` [R crates/lune-ui-core/src/widgets/mod.rs:140].
The moved code wins (pixel-gated). (2) corner radius: Lunegit's filter 4 [L src/Lunegit.Desktop/Controls/FilterBar.axaml:8], the moved box 6 [R crates/lune-ui-core/src/theme.rs:85], Wane 6 [W docs/ui/components.md:352]; the moved code wins.

### Anatomy

- Buffer and offsets: the app owns the string; the engine keeps the caret and the selection anchor as char offsets and clamps both to the text, on a grapheme boundary, every frame, so no input or external change can leave them out of range. This is the totality rule Lunegit's drag maths states for its own inputs [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:16-20].
- Caret: a 1.5 px wide rect, as tall as the text line (16 px when the field is empty), `fg_primary`, drawn 1 px after the text end; vertically centred in a single-line field [R crates/lune-ui-core/src/widgets/mod.rs:137-141].
- Single line: text clipped to the box inset 6 px, drawn 10 px from the left edge [R crates/lune-ui-core/src/widgets/mod.rs:130-135]; it scrolls horizontally to keep the caret in view [derived: a clipped line must follow the caret; no source states the margin].
- Multi-line: Lunegit's read-only multi-line text uses no wrap and accepts Return [L src/Lunegit.Desktop/Views/CrashReportWindow.axaml:73-76]; wrapping and the vertical scroll are `layout.scroll-area`.
- Selection: a range between anchor and head; its fill is a `Theme` value that does not exist yet (Open questions).
- Placeholder: `fg_tertiary`, shown only while the text is empty [R crates/lune-ui-core/src/widgets/mod.rs:131-133].
- Password: a fixed mask of 16 dots whatever the length, so it does not leak the key length, and the value cannot be copied [W docs/ui/components.md:350,353].
- Read-only: the caret and selection work, edits do not; select and copy only [W docs/ui/components.md:661], [L src/Lunegit.Desktop/Views/CrashReportWindow.axaml:74].
- Clear button for search-style fields: a 18 x 18 button with a 12 px close icon, shown only while the text is non-empty [L src/Lunegit.Desktop/Controls/FilterBar.axaml:13-20].
- IME area: the field reports the rect being edited in `FrameOutput.ime_area`; `None` when no text field has focus, and the adapter disables the input method [R crates/lune-ui-core/src/input.rs:156-163].
- Caret and hit queries are asked of the text system: caret rect for an offset, offset for a point. They are added to `TextSystem` in M5 (C2); today the trait has `prepare` and `measure` only [R crates/lune-ui-core/src/text.rs:110-118].
- Offsets from the platform: IME offsets arrive as byte offsets of the string and are converted to char offsets at the boundary [R crates/lune-ui-core/src/input.rs:141-153], [D35].

### States

| state | look | cite |
|---|---|---|
| unfocused, empty | placeholder in `fg_tertiary`, no caret | [R crates/lune-ui-core/src/widgets/mod.rs:131-133] |
| unfocused, text | text in `fg_primary`, no caret | [R crates/lune-ui-core/src/widgets/mod.rs:134-136] |
| focused | caret after the text; the box ring turns `accent` | [R crates/lune-ui-core/src/widgets/mod.rs:137-141,50-69] |
| selecting | selection fill over the range; the caret at the head | [derived: the platform convention; fill value open] |
| composing | preedit text with an underline at the caret (see `text.ime`) | [text.ime] |
| read-only | as focused, edits ignored | [W docs/ui/components.md:661] |
| password | fixed 16 dots | [W docs/ui/components.md:350,353] |
| disabled | opacity 0.4, inert | [W docs/ui/components.md:351], [L src/Lunegit.Desktop/App.axaml:482] |
| hover (box) | box fill `surface_hover` when not focused | [R crates/lune-ui-core/src/widgets/mod.rs:129,50-60] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| focus gained or lost | box ring colour `border` to `accent` | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |
| hover | box fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102] |

Typing, caret movement and selection changes never animate: they are high-frequency content [L .claude/docs/animation-rules.md:148]. Caret blink is an open question (below): Lunegit's rules allow no continuous loop outside the duration scale except the documented terminal outline [L .claude/docs/animation-rules.md:172], and the moved caret is solid.

### Input and focus

- Wheel and swipe: a text area registers no scroll sense while its text fits, so the delta reaches the scroll area around it; one that overflows keeps it (D176).
- Pointer: press places the caret at the hit offset; drag selects with pointer capture until release; Shift-press extends; double-click selects a word, triple-click the line [derived: the platform convention]; the cursor icon over a field is `Text` [R crates/lune-ui-core/src/input.rs:185-199].
- Keys, with the shortcut modifier being logo on macOS and control elsewhere (the app passes which; the toolkit reads no OS, hard rule 2) [R crates/lune-ui-core/src/input.rs:130-139]: Left and Right move by grapheme; with the word modifier by word; Home and End go to the line ends, with the shortcut modifier to the text ends; Shift extends the selection; Up, Down, PageUp and PageDown in multi-line text; Backspace and Delete remove the selection or one grapheme, with the word modifier a word; select-all, undo, redo, cut, copy and paste on the shortcut modifier plus A, Z, Shift-Z or Y, X, C, V [derived: the platform text-editing convention, as Avalonia's `TextBox` implements it for Lunegit]. The listed behaviours are Wane's set: caret, selection, clipboard (cut, copy, paste), Esc, Tab [W docs/ui/components.md:353].
- Enter: in a single-line field reports `submitted`; in a multi-line text inserts a line break [derived: `AcceptsReturn` marks multi-line text in Lunegit, CrashReportWindow.axaml:75].
- Tab moves focus on, it does not insert a tab, in a single-line field [W docs/ui/components.md:353], [core.focus]. Esc: Wane lists it without a meaning (Open questions).
- Text typed arrives as `Event::Text` after the `Key` press that produced it [R crates/lune-ui-core/src/input.rs:57-58]; while a preedit exists the engine does not consume Enter, arrows or Esc: they belong to the input method [derived: platform IME contract; see `text.ime`].
- Tab stop: yes. Layer: the layer of the field. No focus scope. A press elsewhere takes focus away [core.focus].

### Retained state

- Transient: hover, the ring transition, the preedit text and its cursor (dropped when focus is lost) [core.ui].
- Persistent: caret and selection anchor (char offsets), the undo and redo history, the horizontal scroll offset; owned by the edit widget, kept under `ui.keep_alive` (a hidden tab keeps its caret and undo history for 600 undrawn frames) [core.ui].
- App-owned: the text itself, passed every frame as `field` takes it today [R crates/lune-ui-core/src/widgets/mod.rs:120-127]; read-only and password flags; the max length. When the app passes text that differs from the engine's last result (an external edit), the engine clamps the offsets and clears the undo history, so a history never replays against a different text [derived: an edit log is only valid against the text it was made on].
- Exit retention: no.
- Undo: typed characters coalesce into one step until a pause, a caret move or a different kind of edit [derived: the platform convention]; the stack depth is bounded (Open questions).

### Accessibility

- Role `text-input` for one line, `multiline-text` for several; name = the field's label the app passes; value = the text and the selection as char offsets, withheld for a password; states focusable, focused, disabled, read-only, required, invalid; actions focus and set-value [infra.accessibility].
- A password field exposes no value and no selection.

### Strings

None: the engine has no text of its own. A cut, copy and paste context menu, if shipped, takes its labels from `overlay.menu` strings (Open questions).

### Gallery scenes

- `controls` (existing): empty and focused fields at scales 1 and 2.
- `dialog-body` (existing): the focused rename field in a dialog.
- `text-edit-caret`: field with "lune" and the caret after the text, settled.
- `text-edit-selection`: "lune term" with "ne te" selected, settled.
- `text-edit-scrolled`: text wider than the box with the caret at the end, scrolled to it, settled.
- `text-edit-multiline`: three lines, caret on line 2, settled.
- `text-edit-readonly`: read-only multi-line text with a selection, settled.
- `text-edit-password`: password field with the fixed mask, settled.
- `text-edit-disabled`: disabled field with text, settled.
- `text-edit-error`: invalid field (ring in the error colour), settled.
- `text-edit-focus-ring`: focus gained, t = 90 ms (half of `fast`).
- `text-edit-hover`: pointer enters the box, t = 60 ms (half of `instant`).

### Painter primitives

- Existing: fill (selection), fill-over (caret), fill-radii, ring, text, clip-rect, xform (scroll offset).
- New: none (caret and hit queries are on the text system, not the painter).

### Acceptance

- Behaviour (scripted keys and pointer, no clock): typing "abc" puts the caret at 3; Left twice gives 1; Shift-Right selects 1 to 2; select-all then typing replaces the text; Backspace at 0 does nothing; Delete at the end does nothing.
- Graphemes: a Backspace after "e" plus a combining accent removes both as one grapheme; one Backspace after an emoji with a skin-tone modifier removes the whole cluster [derived: grapheme editing convention].
- Offsets: a preedit cursor of bytes (3, 3) in "héllo" is char offset 2 ("h" is 1 byte, "é" is 2) [R crates/lune-ui-core/src/input.rs:141-153], [D35]; any caret, anchor or preedit offset past the end or inside a character is clamped and never panics.
- Undo: typing "a", "b", "c" then undo gives "" in one step; redo restores "abc"; an external text change clears the history and clamps the caret.
- Clipboard: Cut and Copy put the selection into `FrameOutput`'s clipboard request and Paste inserts the text the adapter supplies; a password field copies nothing [W docs/ui/components.md:353], [platform.clipboard].
- Retention: a hidden edit under `ui.keep_alive` keeps caret, selection and undo history for 600 undrawn frames; outside any scope it is gone after one frame [core.ui].
- Pixel probes at scale 2: field 64 device px high, radius 12, ring 2 px (`accent` when focused, `border` otherwise); caret 3 px wide and the text line tall; placeholder colour `fg_tertiary` [R crates/lune-ui-core/src/widgets/mod.rs:50-69,137-141].
- Conformance: the ring and hover transitions pass `motion::conformance`; Reduced keeps them, Off snaps [motion].
- Semantic node: role `text-input`, value and selection follow the text; an injected `AccessAction` set-value replaces the text and reports `changed`.

### Build

- Insert limit: `EditFlags::max_insert` (bytes; `None` is no limit). The text field and the text area set `MAX_SHAPED_INSERT` (1,048,576) because they shape all their text; an insert over it is refused whole, never cut, by paste, input-method commit and assistive set-value alike (D232, D233). The code editor sets none: it draws only the lines in view.
- Milestone: M5. Built in M5 (steps 11 to 13 and 15; the page is `text-edit`, 10 new scenes). The clipboard is the OS's since M10 (`platform.clipboard`, D213 to D215; D107's in-process string is gone). Open for `done` (D47): M11 re-checks the whole bar on every entry; real typing with an input method (R1) and bidi caret placement are unverified here, and so is the Windows and Linux run time.
- Depends on: core.ui, core.focus, core.input, text.shaping, platform.clipboard, text.ime, infra.accessibility, motion.transitions

## Open questions

- Answered (D103: `text_selection`, no blink): selection fill colour and caret blink: no source states them (Lunegit leaves both to Avalonia, Wane does not mock them); M5 chooses a `Theme` value for the selection and decides on blink under the motion rules.
- Answered (D103, not consumed): Esc in a field: Wane lists it [W docs/ui/components.md:353] but not what it does (blur, revert or clear); M5 decides.
- Answered (D103, 200 and 500 ms): undo depth and the coalescing pause: no source; M5 decides.
- Answered (D103, none until M7's menu upgrades): a cut, copy and paste context menu on right press: Lunegit's text boxes use Avalonia's default, not read; M5 decides with `overlay.menu`.
- Answered (D92, a `Keymap` value): whether the word-motion flavour (end of word on macOS, start of the next word on Windows) is a flag the app passes or derived from the shortcut modifier flavour.

## Notes for the planner

New in M3 (PD9). The code editor view (M9) reuses this engine with a document model the app supplies (D45). The existing `field` keeps its signature and look (pixel-gated) while M5 gives it the engine.
