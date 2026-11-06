---
id: platform.clipboard
title: Clipboard
area: platform
status: in-progress
consumers: any
source: crates/lune-ui-core/src/input.rs
owner: orch/toolkit-v1
summary: Text clipboard through FrameOutput requests and input events; types in M4, fake in M5, platform adapter in M10
---

## What it is

Reading and writing the OS clipboard as UTF-8 text. The toolkit core never touches the OS: a widget
that wants to copy or cut puts a write request in `FrameOutput`; one that wants to paste puts a read
request there and receives the text as an input event on a later frame; the windowing adapter does
the OS call. Lunegit uses text only (24 `SetTextAsync` call sites, one `TryGetTextAsync`), all
asynchronous, all reached through one helper. Files, images and rich text are not used and not in v1.

## Needs

- OS clipboard (R-platform-5, narrowed in M3: clipboard is in scope; types M4, fake M5, adapter M10).
- Cut, copy and paste for the editing engine (R-text-5): `text.text-edit`.
- Copy buttons and copy-selected-rows (code block, diff rows, detail fields) (R-text-3).
- Third-party crates only for platform layers (hard rule 4); a new dependency needs a DECISIONS row and a size note (hard rule 5).

## Existing code

`crates/lune-ui-winit/src/clipboard.rs` (M10 step 5: `Clipboard`, `MAX_PASTE_BYTES`, the `arboard` backend); tests `crates/lune-ui-winit/tests/clipboard.rs`. The core side (`ClipboardRequest`, the reply events) is `crates/lune-ui-core/src/input.rs`.
Lunegit: `src/Lunegit.Desktop/ViewModels/SidePanels/ClipboardHelper.cs:1-24`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/ViewModels/SidePanels/ClipboardHelper.cs:1-24, Controls/Terminal/TerminalView.cs:1195-1205,1374-1386,1785-1795, Views/SplitDiffView.axaml.cs:253-275,
Views/CommitDetailView.axaml.cs:29-51, Views/MarkdownPreviewView.axaml.cs:386-402; a grep over `src/Lunegit.Desktop` (instrument: `grep -rn SetTextAsync --include='*.cs' | wc -l`, 2026-11-04) gives 24 write sites and `grep -rn 'GetTextAsync\|TryGetTextAsync'` one read site, [L src/Lunegit.Desktop/Controls/Terminal/TerminalView.cs:1791];
Wane docs/ui/components.md:353,713; this repo crates/lune-ui-core/src/input.rs:28-67,154-164.
Lunegit's reads and writes are text only; no `SetDataObject`, no file or image format was found. No conflicts among sources.

### Anatomy

- Data: one UTF-8 string per request. No formats, no files, no images, no rich text in v1 (none found in Lunegit; Wane's needs are text copies and a paste into fields) [L src/Lunegit.Desktop/ViewModels/SidePanels/ClipboardHelper.cs:9-15], [W docs/ui/components.md:713].
- Requests, in `FrameOutput`: `Write(text)` for copy and cut; `Read` for paste. At most one write per frame survives: the last one wins [derived: the clipboard holds one value]. A `Read` is answered at most once.
- Replies, in `RawInput`: an `Event` carrying the pasted text (`ClipboardText(String)`), or a failure event when the clipboard holds no text or cannot be read. A reply arrives on a later frame, never the same one: Lunegit's calls are `await`ed [L src/Lunegit.Desktop/Controls/Terminal/TerminalView.cs:1203,1385,1791].
- Who decides when: the widget, from its key and pointer handling; the adapter never sees a shortcut. Lunegit's terminal pastes on Cmd+V (macOS) or Ctrl+Shift+V (elsewhere, not plain Ctrl+V) and on a right press when the program is not tracking the mouse; that is terminal policy and stays in the app [L src/Lunegit.Desktop/Controls/Terminal/TerminalView.cs:1199-1203,1376-1385].
- A missing clipboard is silent: Lunegit's helper returns without error when there is no main window [L src/Lunegit.Desktop/ViewModels/SidePanels/ClipboardHelper.cs:11-12,17-23]. The toolkit reports a failed read as an event and ignores a failed write (the adapter may log).
- Secrets: a password field never emits a write [W docs/ui/components.md:353], [text.text-edit].
- Untrusted input: pasted text comes from outside the app. The widget that receives it applies its own bounds (`text.text-edit`'s max length); the adapter caps the size it forwards (Open questions) [derived: hard rule on parsing untrusted input; no Lunegit cap read].
- Platform adapter: in `lune-ui-winit` or `lune-ui-platform`, behind `#[cfg(target_os)]` modules; the core holds only the request and event types (M4) and the fake (M5; `testing::ScriptedInput` gains a clipboard event, a fake adapter records writes) [R crates/lune-ui-core/src/input.rs:28-67], [D36].

### States

n/a — no look; a copy or paste is an event with a result, not a displayed state. The visible feedback is the widget's (`text.code-block` check mark).

### Transitions

n/a — no look.

### Input and focus

n/a — it has no pointer sense, key handling or focus; the keys that trigger it belong to the widgets (`text.text-edit`, `text.code-block`, `text.diff-view`) [text.text-edit].

### Retained state

- Transient: the pending read request (until answered). Persistent: none. App-owned: nothing.
- Exit retention: no.

### Accessibility

n/a — no node; a copy's confirmation is announced by the widget as a polite `status` (see `text.code-block`).

### Strings

None: no text of its own.

### Gallery scenes

- No scene of its own (no pixels). The write path is exercised by the scenes `code-block-copied` and `text-edit-selection`, defined in `text.code-block` and `text.text-edit`; the adapter is exercised by tests.

### Painter primitives

- Existing: none.
- New: none.

### Acceptance

- Core (fake adapter, scripted frames): a copy key on a selection puts one `Write` with exactly the selected text in `FrameOutput`; two copies in one frame keep the last; a cut also edits the text; paste: a `Read` in frame N and a `ClipboardText("x")` event in frame N+1 inserts "x" at the caret; a failure event inserts nothing; a password field emits no `Write` [text.text-edit].
- Multi-byte text round trips unchanged (a string with an accented letter, a CJK character and an emoji).
- Adapter (M10, done, `crates/lune-ui-winit/tests/clipboard.rs`, on a fake backend; the gate never touches the OS clipboard): `write_then_read_returns_the_text_with_multi_byte_characters_intact`, `two_writes_in_a_frame_keep_the_last`, `a_read_reply_lands_in_the_next_take_exactly_once`, `a_failed_read_gives_the_event`, `a_read_over_the_cap_is_refused_whole`, `a_write_over_the_cap_never_reaches_the_backend`, `ten_million_bytes_pass_whole`, `the_empty_string_round_trips`; and one `#[ignore]`d `real_round_trip` on macOS, run by hand (it skips unless the clipboard holds plain text only, writes, reads, compares and restores). Originally: a write then a read returns the same text; reading with no text on the clipboard returns the failure event; a 10,000,000-byte write is either forwarded whole or refused, never truncated silently [derived: the cap is open]. Windows and Linux are compile-checked only [D15].
- A one-line field refuses a paste (or an assistive set-value) over 1 MiB whole, whatever the adapter's cap (D232; `a_one_line_editor_refuses_an_insert_over_the_cap_whole`, `set_value_over_the_one_line_cap_is_refused_whole_and_clipboard_text_the_same`): the adapter's 32 MiB is the code view's, which draws only the lines in view.
- No panics on any text, including embedded NUL and lone surrogate-like sequences (the type is `String`, so only valid UTF-8 reaches it).
- Pixel probes, conformance and semantic node: n/a.

### Build

- Milestone: M4 (`ClipboardRequest` and the reply `Event` variants, with `core.ui`'s `FrameOutput.clipboard` and `core.input`'s reserved variant); M5 (the fake adapter and `ScriptedInput` clipboard event, with `text.text-edit`, whose acceptance tests need them); M10 (the platform adapter, per the roadmap's M10 row). Until M10 the code views of M9 (copy buttons) run on the fake.
- Depends on: core.ui, core.input, platform.input

## Open questions

None open. Answered in M10 step 5 (D213):

- The adapter uses `arboard` 3.7 (`default-features = false`) in `lune-ui-winit`, one `Clipboard` per app, behind a backend trait so tests use a fake. Direct OS calls were rejected (X11 selections need a thread and a protocol).
- The cap is `MAX_PASTE_BYTES` = 33,554,432 bytes (32 MiB, three times the 11 MB document the code view measured). A longer read is `ClipboardUnavailable`, a longer write is dropped, nothing is truncated; text at the cap passes whole.
- A failed write is silent, as Lunegit's is; the app may add a toast.
- Linux and Windows runtime behaviour is unverified here (D15); both are compile-checked.

## Notes for the planner

New in M3 (PD9). R-platform-5 now reads: clipboard in scope (this entry), file dialogs and outbound or cross-window drag out of scope (D43). OSC 52 clipboard access in the terminal is Luneterm's, not the toolkit's [L src/Lunegit.Desktop/ViewModels/TerminalTabViewModel.cs:33-41].
