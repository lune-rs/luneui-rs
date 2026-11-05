---
id: layout.scroll-area
title: Scroll area
area: layout
status: in-progress
consumers: any
source: crates/lune-ui-core/src/widgets/scroll_area.rs
owner: orch/toolkit-v1
summary: Scrolling region: per-id offset, wheel and trackpad scrolling, scroll-into-view, follow-tail, offset kept across content replacement
---

## What it is

A clipped viewport over taller or wider content, with a per-id scroll offset kept by the container
(persistent state), wheel and trackpad scrolling from input deltas, scroll-into-view, an optional
follow-tail mode and anchor-based offset preservation when the content is replaced. Its overlay thumb
is `widget.scrollbar`. Lunegit gets all of this from Avalonia's `ScrollViewer` plus two small
behaviours (`ScrollOffsetPreserver`, `AutoScrollBehavior`); the toolkit has to own it.

## Needs

- Scroll regions that keep offset (R-layout-2).
- Offset as persistent state under keep-alive (R-core-8).
- The scrollbar widget draws the thumb (R-widget-2).

## Existing code

Built in M5 (D96 to D99): `crates/lune-ui-core/src/widgets/scroll_area.rs` (`Ui::scroll_area`, `scroll_area_untranslated`, `scroll_to`). Design source (not code to move): Lunegit `src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs`,
`Controls/AutoScrollBehavior.cs`, `Views/RepositoryContentView.axaml.cs:101-152` (scroll-into-view).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:1-147,
Controls/AutoScrollBehavior.cs:1-104, Views/RepositoryContentView.axaml.cs:101-152, Views/SplitDiffView.axaml.cs:102-113, Views/Settings/LogsSettingsView.axaml:212-226,
Views/UnifiedDiffView.axaml:45-56, App.axaml:588-659; Wane docs/ui/components.md:403-416; this repo crates/lune-ui-core/src/paint.rs:160-175.
Conflict: none. Lunegit has no smooth wheel scrolling (Avalonia's native step); Wane says wheel and touchpad scroll [W docs/ui/components.md:409] without a duration. The smooth step is a
toolkit decision under [D40].

### Anatomy

- Viewport: the given rect, clipping its content (`set_clip`), with content drawn translated by the negative offset (`set_xform`) [R crates/lune-ui-core/src/paint.rs:166-172].
- Offset (x, y) is clamped to `[0, max(0, extent - viewport)]` per axis; non-finite or non-positive input gives 0 [L src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:78-85].
- Extent is measured from this frame's content, so the clamp uses the current content, never a previous one [derived: Lunegit's restore must wait for a layout pass because reading `Extent` right after a rebind returns the previous content's extent, ScrollOffsetPreserver.cs:24-28; an immediate-mode frame draws the content first].
- Bar visibility per axis: Auto, Hidden (wheel and keys still scroll) or Disabled (no scrolling) [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:49-50], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:244-247].
- The thumb and its 9 px track overlay the content, taking no layout space [L src/Lunegit.Desktop/App.axaml:588-605], [widget.scrollbar].
- Scroll-into-view centres the target: `target = index * rowHeight - (viewport - rowHeight) / 2`, clamped to `[0, max(0, extent - viewport)]` [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml.cs:150-151]; a target outside the content is a no-op, not a wrong scroll [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml.cs:127-132].
- Follow-tail (log views): when the extent grows, scroll to the end; the app turns it on and off with a flag [L src/Lunegit.Desktop/Controls/AutoScrollBehavior.cs:62-66], [L src/Lunegit.Desktop/Views/Settings/LogsSettingsView.axaml:220].
- Offset preservation: restore the previous offset when the content is replaced by new content with the same identity key, and only then (below under Retained state).
- Paired panes (split diff): the second area's offset follows the first's, one direction per frame, guarded against feedback [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:107-113].

### States

| state | look | cite |
|---|---|---|
| rest, content fits | no thumb | [W docs/ui/components.md:407] |
| rest, content overflows | content translated by the offset; thumb hidden until hover | [L src/Lunegit.Desktop/App.axaml:629], [W docs/ui/components.md:407] |
| pointer over the area | thumb visible (see `widget.scrollbar`) | [L src/Lunegit.Desktop/App.axaml:640-644] |
| scrolling (wheel, drag, keys) | offset changes; no row or child animates (virtualized content) | [L .claude/docs/animation-rules.md:145] |
| follow-tail on | offset pinned to the end while the extent grows | [L src/Lunegit.Desktop/Controls/AutoScrollBehavior.cs:62-66] |
| disabled | bars Disabled: no scrolling and no thumb | [L src/Lunegit.Desktop/App.axaml:666-667] |

Hover, pressed and focus-visible are properties of the content and of the thumb, not of the area: the area is not a Tab stop (see Input and focus).

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| wheel step (non-precise delta) | offset | scroll | fast | quad-out | snap | [D40] |
| scroll-into-view within 2 viewports | offset | scroll | fast | sine-in-out | snap | [D40], [L .claude/docs/animation-rules.md:84] |
| thumb reveal | thumb opacity | hover | fast | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:622-630] |

Precise (trackpad) deltas, thumb drag and PageUp or PageDown set the offset immediately: the input already is the motion. A scroll-into-view of more than 2 viewports snaps
[derived: flying over a virtualized million-row list reads as noise; Lunegit sets `Offset` directly, RepositoryContentView.axaml.cs:138]. Never animated: virtualized rows, per-row state and
high-frequency content while the offset changes [L .claude/docs/animation-rules.md:145,148].

### Input and focus

- Pointer: wheel and trackpad scroll the area under the pointer; a delta routes per axis (D178): each component goes to the topmost scroll area under the pointer that is live on that axis (a `wheel_x` strip counts as live on both, since it reads the whole gesture as its own), and an area live on both takes both. An area at its end keeps a delta on a live axis and does not pass it up (derived by the single-topmost-hit rule, [core.ui]); a delta on an axis it is not live on goes to the area around it. An area with `Bars::Disabled` on both axes registers no scroll sense, so it is outside that rule: the composites that nest an area and know their extent first (the list, the table, the tree, the tab strip, the text area, the toast history, the board and its columns) disable an axis with nothing to scroll, and a delta over them reaches the area around them (D176). Thumb drag is `widget.scrollbar`.
- Keys when the area or a child holds focus and the child does not use the key: PageUp and PageDown scroll by one viewport [W docs/ui/components.md:409]; Home and End: no source read states them (Open questions).
- Tab stop: no; the focusable content is. Scroll-into-view runs when focus moves to a child outside the viewport [core.focus].
- Layer: the layer of its content. No focus scope.

### Retained state

- Transient: wheel animation target, thumb opacity [core.ui].
- Persistent: the offset (x, y), the follow-tail latch and the preservation identity key; owned by the area and kept under `ui.keep_alive` for a hidden tab or collapsed panel [core.ui]; a replaced document keeps its offset only while the identity key matches [L src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:44-50].
- App-owned: the content, its identity key, the follow-tail flag, the bar visibility modes.
- Exit retention: no.
- Preservation rule, as Lunegit does it: restore only if the area has rendered before, the new key is non-null and equal (ordinal) to the last key; a null key never restores [L src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:62-65]; the key must be the one the app uses to decide "same document", a bare file path is not enough [L src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:44-50]; a saved offset of 0 or less is not restored [L src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:115-116]; the restore is dropped if the key changed before it applies [L src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:125-128]; a change under 0.5 px is not written [L src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:131-132]; a new key starts at the top [L src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:109].

### Accessibility

- Role `scroll-area`; name = the label the app passes, or none; value = the offset as a number with min 0, max `extent - viewport`; states focusable only when it has a focusable child; actions scroll-into-view (routed to the child), increment and decrement (one wheel step) [infra.accessibility].
- A scroll area emits one node and its children nest under it [infra.accessibility].

### Strings

None: the area has no text of its own.

### Gallery scenes

- `scroll-area-rest`: content 3 times the viewport at offset 0, no pointer, settled.
- `scroll-area-hover`: pointer over the area, thumb visible at 40 percent offset, settled.
- `scroll-area-fits`: content shorter than the viewport, pointer over, no thumb, settled.
- `scroll-area-disabled`: bars Disabled with overflowing content, settled.
- `scroll-area-follow-tail`: log content growing by one line, follow-tail on, settled.
- `scroll-area-wheel`: one wheel step from offset 0, t = 90 ms (half of `fast`).
- `scroll-area-into-view`: scroll-into-view of a row 1 viewport away, t = 90 ms.
- `scroll-area-thumb-reveal`: pointer enters the area, t = 90 ms.

### Painter primitives

- Existing: clip-rect, xform (translate by the negative offset).
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): a wheel delta moves the offset by the step and clamps at both ends; a trackpad delta moves it 1:1 with no animation; PageDown moves it by the viewport height.
- Preservation, known answers from Lunegit's pure functions: (hasRendered false) is not restored; (null key) is not restored; (same key) is restored; (different key) is not; offset 700 with extent 1000 and viewport 400 clamps to 600; offset NaN, infinite or -5 gives 0; extent below viewport gives 0 [L src/Lunegit.Desktop/Controls/ScrollOffsetPreserver.cs:62-85].
- Keep-alive: a hidden area under `ui.keep_alive` keeps its offset for 600 undrawn frames; outside any scope it is gone after one frame [core.ui].
- Scroll-into-view: row index 100, row height 45, viewport 400 gives 4322.5 before the clamp (`100 * 45 - (400 - 45) / 2`, the formula at RepositoryContentView.axaml.cs:150); an out-of-range index changes nothing.
- Pixel probes at scale 2: the clip edge is exact at the viewport rect; content at offset 40 is shifted by 80 device px.
- Conformance: the wheel step and scroll-into-view run through `motion::conformance`: monotonic, exact end, settled within 180 ms plus one frame; Reduced and Off snap [motion].
- Semantic node: role `scroll-area`, value follows the offset; an injected `AccessAction` scroll-into-view moves the target into the viewport [infra.accessibility].

### Build

- Milestone: M5. Built in M5 (steps 7 and 10; the page is `scroll-area`, 8 scenes). Open for `done` (D47): M11 re-checks the whole bar on every entry; the Windows and Linux run time is unverified (the Linux and Windows lint is `scripts/check-targets.sh`).
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions, layout.stack, widget.scrollbar

## Open questions

- Answered (D99): Home and End, and arrow keys when the area itself has focus: no source read states them (Wane lists PageUp and PageDown only [W docs/ui/components.md:409]); M5 decides.
- Answered (D99): horizontal wheel (Shift plus wheel) mapping is not specified by any source read.

## Notes for the planner

New in M3 (PD9). Offset preservation is a property of the persistent store, not a separate behaviour: the identity key is a call argument and the clamp runs once the content's extent is known in the same frame.
