---
id: text.ime
title: IME composition input
area: text
status: in-progress
consumers: luneterm, lunegit, wane
source: crates/lune-ui-core/src/widgets/field.rs
owner: orch/toolkit-v1
summary: IME composition in text widgets: preedit drawn inline at the caret, commit, candidate-window area, byte offsets from the platform
---

## What it is

Input-method composition for every editable text. The platform sends `Ime` events (enabled, preedit
with a cursor range, commit, disabled); the toolkit already carries them as `ImeEvent` and tells the
window where text is being edited through `FrameOutput.ime_area`. What is missing is the widget half:
`text.text-edit` keeps the preedit, draws it inline at the caret with an underline, replaces it on commit,
and reports the caret rect so the OS can place its candidate window. It is a toolkit responsibility, not
the app's: every text widget needs it (decided by R-text-4 resolution, M3).

## Needs

- IME composition input (R-text-4).
- Text field behaviour that composition must integrate with (R-text-5): the edit state is `text.text-edit`.
- Wane — IME composition is a requirement of the text field (R-text-4).

## Existing code

Built in M5 (D103): `field` and `text_area` consume the events (`crates/lune-ui-core/src/widgets/field.rs`, `text_edit.rs`) and report `ime_area`. The input side existed first: events and output, with nothing consuming them: `ImeEvent` (`crates/lune-ui-core/src/input.rs:141-153`), `FrameOutput.ime_area` (`:156-163`), the winit translation
(`crates/lune-ui-winit/src/input.rs:149-161`) and the output side that enables the input method and sets the cursor area (`crates/lune-ui-winit/src/input.rs:278-312`); the scripted builder
`ScriptedInput::ime` (`crates/lune-ui-core/src/testing/input.rs:148`); a translation test (`crates/lune-ui-winit/tests/input.rs:42-48`). No widget sets `ime_area`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: this repo crates/lune-ui-core/src/input.rs:28-67,141-165, crates/lune-ui-winit/src/input.rs:145-165,278-312, crates/lune-ui-winit/tests/input.rs:36-60,
crates/lune-ui-core/src/testing/input.rs:148-153, crates/lune-ui-core/src/widgets/mod.rs:120-144; Lunegit: `grep -rln 'TextInputMethod\|InputMethod\|Preedit\|Composition' src --include='*.cs'` (instrument: grep, counted 2026-11-04) finds one file,
src/Lunegit.Tests/Terminal/TerminalKeyRouterTests.cs, whose only hit is an AltGr composition test [L src/Lunegit.Tests/Terminal/TerminalKeyRouterTests.cs:217]; Lunegit never handles composition itself, Avalonia's `TextBox` does; Wane docs/ui/components.md:353,729;
Luneterm 296cfdc: `git grep -i '\bIME\b' 296cfdc -- docs crates` finds nothing.
So no Lunegit, Wane or Luneterm source states IME behaviour or a look; the behaviour below is the platform input-method contract and is marked `[derived]`. No conflicts among sources.

### Anatomy

- Events: `Enabled`, `Preedit { text, cursor: Option<(usize, usize)> }`, `Commit(text)`, `Disabled`; a preedit's `cursor` is the selection inside `text`, `None` hides the cursor, and an empty `text` clears the preedit [R crates/lune-ui-core/src/input.rs:141-153].
- Offsets: the platform's byte offsets, converted to char offsets inside the widget [R crates/lune-ui-core/src/input.rs:141], [D35]. Example from the translation test: preedit "か" with cursor (0, 3) is the whole 3-byte character [R crates/lune-ui-winit/tests/input.rs:42-47].
- Preedit draws inline at the caret: the committed text before the caret, then the preedit, then the committed text after it shifts right [derived: the inline composition model every input method expects].
- The preedit is marked with an underline across its range; the clause selected by `cursor` is drawn emphasised; with `None` no cursor is drawn [derived: the platform convention; the look values are open (Open questions)].
- Candidate window: the OS places it from `FrameOutput.ime_area`, the rect of the caret (or of the preedit's selected clause) in logical pixels; `None` when no text widget has focus, which disables the input method [R crates/lune-ui-core/src/input.rs:156-163], [R crates/lune-ui-winit/src/input.rs:296-310].
- Adapter behaviour: when the area first becomes `Some` the adapter allows IME and sets the cursor area; every change moves it; `None` disables IME [R crates/lune-ui-winit/src/input.rs:296-312].
- Commit: replaces the preedit (and any selection) with the text, as one undo step in `text.text-edit` [derived: a composed word is one edit].
- Disabled or focus lost: the preedit is dropped and nothing is committed [derived: the platform contract; `Disabled` ends composition, [R crates/lune-ui-core/src/input.rs:152]].

### States

| state | look | cite |
|---|---|---|
| idle | no preedit; `ime_area` is `None` while no text widget has focus | [R crates/lune-ui-core/src/input.rs:158-161] |
| enabled | a text widget has focus; `ime_area` is the caret rect; nothing extra drawn | [R crates/lune-ui-winit/src/input.rs:296-306] |
| composing | preedit inline with an underline and the selected clause emphasised | [derived: the platform convention] |
| composing, no cursor | preedit with the underline and no cursor (`cursor` is `None`) | [R crates/lune-ui-core/src/input.rs:143-147] |
| committed | the text sits in the buffer; the preedit is gone; the caret is after it | [derived: the platform contract] |
| disabled | preedit dropped; `ime_area` back to `None` | [R crates/lune-ui-core/src/input.rs:152,158-161] |

Hover, pressed and focus-visible belong to the host widget. A disabled or read-only widget never enables IME [derived: nothing can be inserted].

### Transitions

n/a — preedit changes are high-frequency content and never animate [L .claude/docs/animation-rules.md:148]; the underline and the clause emphasis appear and change with the events, without a tween. The candidate window is the OS's.

### Input and focus

- Pointer: a press inside the field while composing: the platform commits or cancels; not stated by any source (Open questions).
- Keys: while a preedit exists the engine does not consume Enter, arrows, Backspace or Escape: they belong to the input method; typed characters arrive as `Ime` events, not `Text` [derived: the platform IME contract; the key and text events are separate, [R crates/lune-ui-core/src/input.rs:51-60]].
- Shortcuts: app and toolkit shortcuts are not run for keys the input method consumes [derived: same].
- Tab stop: the host's. Layer: the host's. No focus scope.

### Retained state

- Transient: the preedit text, its cursor range, the reported `ime_area`; dropped when the host loses focus (and so after one unused frame) [core.ui].
- Persistent: none: a composition does not survive a hidden tab; the committed text goes through the host's undo history [text.text-edit].
- App-owned: nothing.
- Exit retention: no.

### Accessibility

n/a — the preedit has no node of its own. The host's `text-input` value excludes the uncommitted preedit until it is committed [derived: assistive technology reads the committed value]; `role-gap: composition range` (the role list has no composition state).

### Strings

None: no text of its own.

### Gallery scenes

- `ime-preedit`: field with "lune" and a Japanese preedit "かな" at the end, cursor (0, 3) bytes (the first character selected), settled.
- `ime-preedit-no-cursor`: the same preedit with `cursor` `None`, settled.
- `ime-preedit-middle`: the caret between "lu" and "ne" with a preedit, text after it shifted right, settled.
- `ime-committed`: after the commit "かな", the caret after it, settled.
- `ime-multiline`: a preedit on line 2 of a multi-line text, settled.
- `ime-area`: the reported `ime_area` outline drawn by the gallery over a focused field, settled.

### Painter primitives

- Existing: fill (the underline as a thin rect), fill-over (the emphasised clause), text, clip-rect.
- New: none.

### Acceptance

- Scripted events (`ScriptedInput::ime`): Enabled, Preedit "か" (0, 3), Commit "か", Disabled gives the text "か" with the caret at char 1, no preedit afterwards, and `ime_area` `None` after the focus leaves [R crates/lune-ui-core/src/testing/input.rs:148], [R crates/lune-ui-winit/tests/input.rs:42-48].
- Offsets: in "héllo" a preedit cursor (3, 3) in bytes is char 2; an offset inside a multi-byte character or past the end is clamped and never panics [D35].
- Clearing: a Preedit with empty text clears the preedit and draws no underline [R crates/lune-ui-core/src/input.rs:143-145].
- `ime_area`: with a focused field it is `Some` and lies inside the field's rect, and moves with the caret as the text grows; with no focused text widget it is `None` [R crates/lune-ui-core/src/input.rs:158-161].
- Keys: while a preedit exists, Enter and arrow keys do not move the caret or submit.
- Retention: the preedit is gone after the host loses focus.
- Pixel probes at scale 2: an underline row under exactly the preedit range and nowhere else (thickness is open); the text after the caret is shifted right by the preedit's measured width.
- Conformance: n/a, no transitions.
- Semantic-node test: the host's value excludes the preedit until commit.
- Real typing with an input method is unverified here (no VM, no hands-on run); Luneterm's M9 run covers it [D15].

### Build

- Milestone: M5. Built in M5 (steps 13 and 15; the page is `ime`, 6 scenes; the gallery bundles no CJK font, so the catalogue's "かな" preedit draws as boxes, D90). Open for `done` (D47): M11 re-checks the whole bar on every entry; real typing with an input method (R1) and bidi caret placement are unverified here, and so is the Windows and Linux run time.
- Depends on: text.text-edit, core.input, platform.input, core.ui, infra.accessibility

## Open questions

- Answered (D103: 1 px `fg_primary`, the clause 2 px over `text_selection`): the look of the underline and of the selected clause (thickness, colour): no source states them; M5 takes them from the theme.
- Answered (D103, ignored): what a pointer press inside the field does to a composition (commit or cancel): platform behaviour, not stated by any source read.
- Still open: `role-gap: composition range`: SF8 has no field for a composition range.
- Answered (D103, it scrolls with the text): whether the preedit shows on a different layer than the committed text when the field scrolls horizontally: M5 decides with the scroll rule in `text.text-edit`.

## Notes for the planner

Renamed from the Luneterm-area IME entry (D10, D42). The earlier open question "toolkit or app responsibility" is answered: the toolkit (R-text-4). Not verifiable here without an input method; see the last Acceptance line.
