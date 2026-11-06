---
id: core.ui
title: Ui frame context
area: core
status: done
consumers: any
source: crates/lune-ui-core/src/ui.rs, crates/lune-ui-core/src/ui
owner: —
summary: Immediate-mode Ui frame: ids, transient and persistent retained state, Response, layers and modality, Repaint
---

## What it is

The per-window frame context every component is built on (D6 of the roadmap, requirement R-core-8). A
frame is built from `RawInput`, a `Theme` and a `TextSystem`; widgets call `ui.interact(id, rect,
sense)` and draw through the existing `Painter<H>`, which stays public; finishing the frame yields a
`Scene` and a `FrameOutput` that carries `Repaint`, the cursor, the IME area, clipboard requests,
window commands and the semantic tree. Ids come from app keys under a parent scope. State is
retained by id in two lifetimes. This entry restates the C4 contract of the roadmap as the spec M4
builds; `core.focus`, `infra.accessibility`, `infra.localization` and `motion.transitions` hold the
parts that have their own entry.

## Needs

- Stable widget ids from app keys, retained transient and persistent state, `Response` (R-core-8).
- Layered, call-ordered hit testing with modal capture and pointer capture, replacing the app-computed hover of today (R-core-2, R-core-8).
- Time, size, scale, theme and hit values come in; nothing reads a clock or the environment (R-nfr-1).
- One `Ui` frame, one retained store and one `FrameOutput` per window (R-nfr-6, R-nfr-8).
- A semantic tree and focus per frame (R-infra-4, R-widget-3).

## Existing code

`Ui` is `crates/lune-ui-core/src/ui.rs:114`, its parts in `src/ui/` (`id`, `store`, `pointer`, `keys`, `focus`, `animate`, `overlay`, `access`, `response`). The pieces it builds on (line numbers as of the M4 plan, before the move): `RawInput` and `FrameOutput` (`crates/lune-ui-core/src/input.rs:16-27,155-164`);
`LayerId` draw order (`crates/lune-ui-core/src/scene.rs:16-22`); the painter's flat `hits: Vec<(Rect, H)>`, the
app-supplied `hover: Option<H>`, `hit` and `hovered` (`crates/lune-ui-core/src/paint.rs:116-118,315-326`);
the modal idiom of a window-sized blocking hit (`crates/lune-ui-core/src/widgets/dialog.rs:46-48,69`) and the
gallery's popup dismiss hit (`apps/gallery/src/scenes.rs:369-371`); `DoubleClick` and its 400 ms
(`crates/lune-ui-core/src/window.rs:44-60`); `testing::ScriptedInput` (`crates/lune-ui-core/src/testing/input.rs:16`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/input.rs` (all),
`crates/lune-ui-core/src/scene.rs:1-30`, `crates/lune-ui-core/src/paint.rs:100-175,295-340`,
`crates/lune-ui-core/src/widgets/dialog.rs:1-110`, `crates/lune-ui-core/src/window.rs:36-60`,
`apps/gallery/src/scenes.rs:363-395,425-440`; Lunegit src/Lunegit.Desktop/Services/DialogService.cs:81-110,
.claude/docs/dialog-exit-timing.md:14-44, src/Lunegit.Desktop/MainWindow.axaml.cs:85-122, src/Lunegit.Desktop/MainWindow.axaml:240-250,
src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:27-44, src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:280-292,
src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:55-58; Wane docs/ui/components.md:535,549,563,577.
Conflict: Wane draws overlays on a `position: absolute` layer and says the mock "must become a popup layer
with window-edge flipping" [W docs/ui/components.md:540]; the moved code orders layers by draw order only and
records no layer on a hit region, so the layered hit test is new work, not a move [R crates/lune-ui-core/src/paint.rs:315-322].
Lunegit holds the dialog's result until its exit fade ends [L .claude/docs/dialog-exit-timing.md:18-44]; C4 keeps that
retention but moves it out of the caller: the app stops calling, the toolkit keeps drawing. C4 wins (D40 rationale: a frame is
reproducible and the caller never waits).

### Anatomy

- Inputs: `RawInput` (logical size, scale, app time as `Duration`, window focus, ordered events), a `Theme`, a `TextSystem`, and the `MotionTier` [R crates/lune-ui-core/src/input.rs:16-27], [motion.animation-tiers]. Time is whatever the app supplies [R crates/lune-ui-core/src/input.rs:5-6].
- Outputs: `Scene` and `FrameOutput`; the moved `FrameOutput` holds `cursor`, `ime_area` and `commands` [R crates/lune-ui-core/src/input.rs:155-164]; M4 adds `repaint: Repaint`, `clipboard: Vec<ClipboardRequest>`, the semantic `tree` and the focused node id [infra.accessibility], [platform.clipboard]. The `ClipboardRequest` type and the field land in M4 (the shape of the frame output is part of C4); only the adapter that executes the requests is M10, and the fake is M5 [platform.clipboard].
- `Repaint` is `Idle | Now | At(Time)`: Now while any transition is in flight or an exit is retained, and once (one catch-up frame) after a frame whose input changed state after the widgets drew (a click, a consumed key, an access action, a dismissal, a scroll or drag, a focus change at the end of the frame, a pending `request_focus`) or whose hover at the still pointer differs against its own new regions ([D81]); At(t) for the earliest requested deadline later than now; Idle when settled with no such input; the moved `WindowCommand::Redraw` ("draw another frame") stays for explicit requests [R crates/lune-ui-core/src/input.rs:178-179], [motion].
- `WidgetId`: a 64-bit hash of the parent scope's id and the app key; `ui.scope(key, |ui| ...)` nests; widgets take a key, not a number [derived: C4 "stable WidgetIds from app keys under a parent scope"; today a hit's identity is the app's own `H` value, `crates/lune-ui-core/src/paint.rs:315-322`].
- `ui.interact(id, rect, sense) -> Response`: registers the rect as a hit region on the current layer and returns the frame's reading. `Sense` flags: hover, click, double-click, drag, focusable.
- `Response`: `hovered`, `pressed`, `clicked`, `double_clicked`, `dragged` with a delta, `focused`, `changed` [derived: C4 Response fields]. `changed` is set when a widget altered the app-owned value it was handed (a toggle flipped, text edited); the app writes it back.
- Click: press and release on the same id with the release inside its rect; double click: two clicks closer than 400 ms of frame time [R crates/lune-ui-core/src/window.rs:43-44], [derived: the press-release rule has no source]; drag: begins after 4 px of travel from the press point, which is Lunegit's threshold for both the tab strip and the kanban board [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:29-31], [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:28].
- Layers: `Base` < `Page` < `Popup` < `Modal` < `Raised` < `Toast`, the moved `LayerId` order plus a `Toast` layer that M4 adds above `Modal` so toasts draw over dialogs as in Lunegit [L src/Lunegit.Desktop/MainWindow.axaml:232,272], [D54], and a `Raised` layer that M7 adds between them: `Ui::overlay` puts a popup or modal overlay that opens while a modal is active on `Raised`, once at open (or reopen), and it stays there to its close, so a popover, tooltip, combo list, menu or nested dialog over a dialog draws and takes input on top [D137]; overlays stacked further share `Raised`, so a third level that overlaps its parent interleaves with it. Then call order within a layer, later on top; only the topmost hit under the pointer gets hover and press [R crates/lune-ui-core/src/scene.rs:16-22], [core.ui]. The hit region records its layer (today it does not) [R crates/lune-ui-core/src/paint.rs:315-322].
- Overlay stack: an overlay (`dialog`, `menu`, `dropdown`, `popover`, `tooltip`, `toast`) registers with its layer, `modal: bool` and a dismiss rule; the stack's top is the one that receives Escape and the outside click; a passive overlay (non-modal, no focusable region, such as a toast) takes no focus on open and is the top for Escape or an outside press only when it asks for that dismissal ([D82]) [W docs/ui/components.md:535,549,563,577], [L src/Lunegit.Desktop/MainWindow.axaml.cs:91-98].
- Modal capture: while a modal overlay is open, layers below it get no hover, press, scroll or keys; a press outside the modal goes to its dismiss rule. Lunegit does the same by disabling hit testing under the open dialog and blurring it [L src/Lunegit.Desktop/MainWindow.axaml.cs:115-122]; the moved dialog blocks with a window-sized hit [R crates/lune-ui-core/src/widgets/dialog.rs:46-48,69].
- Pointer capture: a pressed widget keeps the pointer until release, even outside its rect; the tab strip steals the capture at the drag threshold [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:283-289].
- Dismiss: Escape and an outside click act on the top overlay only; a click on the dialog scrim and Escape close it [L src/Lunegit.Desktop/MainWindow.axaml.cs:85-98], [W docs/ui/components.md:563]; a light-dismiss popup closes on an outside click [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:57]; focus loss closes a flyout [W docs/ui/components.md:577].
- Retention is described under Retained state; focus and the semantic tree are `core.focus` and `infra.accessibility`; strings are `infra.localization`.
- Per-window: each window owns its `Ui` frame, `RawInput`, `FrameOutput`, focus, layers, hit regions and a retained store held by the app, so ids never collide across windows [derived: roadmap C2].
- The low-level `Painter<H>` and the moved draw functions stay public, and `ui.interact` coexists with `Painter::hit` in one frame [R crates/lune-ui-core/src/paint.rs:109-160], [D10].

### States

| state | look | cite |
|---|---|---|
| rest | the widget's own rest look | [core.ui] |
| hover | true for the topmost hit under the pointer only; none for a layer below an open modal | [core.ui], [L src/Lunegit.Desktop/MainWindow.axaml.cs:115-122] |
| pressed | set from press until release; survives the pointer leaving the rect | [core.ui], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:283-289] |
| dragging | after 4 px of travel while pressed | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:29-31] |
| focus-visible | owned by `core.focus` | [core.focus] |
| disabled | opacity 0.4 and inert: no hover, press, click or focus | [L src/Lunegit.Desktop/App.axaml:482], [W docs/ui/components.md:295] |
| exiting | an overlay the app stopped calling, still drawing until its exit transition ends | [L src/Lunegit.Desktop/Services/DialogService.cs:87-92] |

These are states of the interaction model; the look of each belongs to the component.

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| hover enter and leave | background and foreground colour | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102] |
| press and release | scale 0.98 | press | fast | quad-out | snap | [L .claude/docs/animation-rules.md:95,103] |
| focus ring on and off | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104] |
| overlay exit (the retained frames) | opacity | dismiss | fast | quad-out | keep | [L src/Lunegit.Desktop/Services/DialogService.cs:87-92], [motion] |

Retention length follows the tier: Full 180 ms (`fast`), Reduced 120 ms (`instant`), Off 0 ms
[L src/Lunegit.Desktop/Services/DialogService.cs:87-92]. Never animated: nothing beyond the motion.md list.

### Input and focus

- Pointer: hover, click, double-click, drag per `Sense`; the wheel scrolls the topmost scrollable under the pointer, each axis of the delta to the topmost one live on that axis (D178); buttons other than primary are reported by `PointerButton` [R crates/lune-ui-core/src/input.rs:67-76,80-85].
- Keys: routed in this order: the top overlay's dismiss rule (Escape; while a pointer gesture is live and holds its capture, the gesture takes it first, D175, D176), the focused widget, then the shortcut registry; Tab and Shift+Tab are `core.focus`; text (`Event::Text`) and IME go to the focused text widget only [core.focus], [infra.shortcut-routing], [R crates/lune-ui-core/src/input.rs:57-60].
- `AccessAction` events are routed as the matching pointer or key input [infra.accessibility].
- Layer: the layer a hit is registered on; overlays use `Popup`, `Modal` or `Raised` (chosen by the `Ui`, [D137]); toasts use `Toast` ([D54]). Focus scope: an overlay pushes one on open and pops it on close; a modal traps Tab [core.focus].
- Dismiss rules and pointer capture are in Anatomy; Escape and outside click never reach a layer below the top overlay, but for an Escape a live pointer gesture on such a layer takes before the overlay dismisses (a drag under a non-modal overlay keeps its capture, D175).

### Retained state

- Transient (hover, press, focus-ring and caret animations, transitions): dropped after one frame without use. Exception: an id whose exit transition is still running stays alive, keeps drawing from retained state after the app stops calling it, and reports `Repaint::Now` until it settles (a closing dialog: Full 180, Reduced 120, Off 0 ms; a closing menu, popover, flyout or tooltip: Full 120, Reduced 120, Off 0 ms, the moved `MENU_OUT`; a toast is removed at once) [L src/Lunegit.Desktop/Services/DialogService.cs:87-92], [L .claude/docs/dialog-exit-timing.md:18-44].
- Persistent (scroll offset, text edit with undo history, selection, expand and collapse, split ratio, sort): owned by the container widget that owns it. A list's selection is a set of row keys and a tree's expansion a set of node keys, so rows scrolled out of view lose nothing. It survives undrawn frames while an app-owned `ui.keep_alive(key, ...)` scope covers it (a hidden tab, a collapsed panel); outside any live scope it is dropped after one unused frame; the app ends a scope explicitly [derived: C4 retention rules].
- App-owned: every value the app passes each frame (`on`, `text`, `selected`...), the string table, the theme, the tier, the retained store itself (one per window).
- Exit retention: yes: a dialog Full 180 / Reduced 120 / Off 0 ms; a menu, popover, flyout or tooltip Full 120 / Reduced 120 / Off 0 ms; a toast none.

### Accessibility

n/a — the frame does not draw; every interactive widget emits one node per frame through `ui.node(...)` and the shape of that node is in [infra.accessibility].

### Strings

None: no user-visible text of its own. The string table API (`ui.tr`, `ui.tr_fmt`) is specified in [infra.localization].

### Gallery scenes

- `dialog-confirm`, `menu`, `dropdown` (existing): the overlays the model carries, settled.
- `ui-modal-capture`: the `controls` card under an open dialog, the pointer over a page button; the button shows its rest look (no hover), settled.
- `ui-exit-retention`: a dialog the app stopped calling at t = 90 ms (half of `fast`), Full tier: card and backdrop half faded and still drawn.
- `ui-popup-over-page`: a `Page` button and a `Popup` overlap with the pointer over the overlap; only the popup item shows hover, settled.

### Painter primitives

n/a — no drawing of its own; the frame builds on the existing painter [core.painter].

### Acceptance

- Retention tests: (1) a hidden tab under keep-alive keeps its scroll offset and undo history for 600 undrawn frames; (2) a dialog the app stops calling completes its exit transition, then reports `Idle`, with the length per tier (180, 120, 0 ms); (3) a list selection survives scrolling the selected row out of view; (4) an unused transient id is gone after one frame [derived: C4 contract tests].
- Input tests: (1) a click on a page button under an open modal does nothing; (2) Escape closes only the top overlay; (3) Tab inside a modal never leaves it; (4) closing restores the previous focus [derived: C4 contract tests], [core.focus].
- `Response` tests with scripted input and injected time: click needs press and release on the same id; a release outside the rect is not a click; double click inside 400 ms and not at 400 ms; drag reports no delta below 4 px and the exact delta after; a pressed widget keeps the pointer outside its rect [R crates/lune-ui-core/src/window.rs:44], [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:31].
- Layer tests: a later hit in the same layer wins; a `Popup` hit wins over a `Page` hit at the same point; hover and press go to the topmost only.
- `Repaint`: `Idle` on a settled frame with no state-changing input; `Now` while a transition is in flight and for one catch-up frame after state-changing input or a stale hover ([D81]: a dropdown's pick frame, a modal opening under a still pointer); `At(t)` equals the earliest deadline after now, and a request already due never hides a later one [motion].
- Determinism and isolation: the same `RawInput` script gives byte-identical `Scene` and `FrameOutput`; two windows' stores never share an id; no clock or environment read (boundary script rule 4 stays green) [R crates/lune-ui-core/src/input.rs:5-6].
- The moved widgets keep drawing at zero pixel delta (`controls`, `menu`, `dropdown`, `dialog-*`) when called through the old `Painter<H>` API [R apps/gallery/src/scenes.rs:96,103].
- Tests, bullet by bullet (`crates/lune-ui-core/tests/`). Retention: `ui_core.rs::a_hidden_tab_under_keep_alive_keeps_its_state_until_the_scope_ends` (600 undrawn frames; a stand-in scroll offset and undo history, the store being the thing under test), `ui_overlays.rs::an_overlay_the_app_stops_calling_keeps_drawing_until_its_close_ends` (180, 120, 0 ms, then `Idle`), `ui_core.rs::a_selection_survives_frames_where_the_selected_row_is_not_drawn`, `ui_core.rs::an_unused_transient_id_is_gone_after_one_frame`. Input: `ui_overlays.rs::a_click_on_a_page_button_under_an_open_modal_does_nothing`, `ui_overlays.rs::escape_closes_only_the_top_overlay`, `ui_focus.rs::tab_inside_a_modal_never_leaves_it`, `ui_focus.rs::closing_restores_the_previous_focus`. `Response`: `ui_core.rs::a_click_needs_press_and_release_on_the_same_id`, `a_release_outside_the_rect_is_no_click`, `double_click_is_strictly_under_400_ms`, `a_drag_starts_at_4_px_with_the_full_travel`, `a_capture_outside_the_rect_keeps_pressed_and_blocks_other_hover`. Layers: `ui_core.rs::a_later_hit_in_the_same_layer_wins`, `popup_over_page_wins_at_the_same_point`, `all_five_layers_are_ordered_pairwise`. `Repaint`: `ui_core.rs::repaint_follows_the_animation`, `repaint_at_the_earliest_future_request` (a due request never hides a later one), `ui_overlays.rs::a_modal_opening_under_a_still_pointer_asks_for_the_frame_with_the_right_hover` (D81). Determinism and isolation: `ui_core.rs::a_script_run_in_two_fresh_stores_gives_identical_frames`, `the_same_keys_in_two_stores_keep_independent_state`; the clock and environment rule is `scripts/check-boundaries.sh`. Transitions: the model's rows are realised in the widgets and proved there (`widget_button.rs::every_motion_row_conforms_under_every_tier` for hover and press; `widget_dialog.rs::close_hides_at_180_120_and_0_ms` for the exit); the frame's own animation is `ui_core.rs::a_colour_animates_and_settles_exactly` and `off_tier_snaps_and_a_tier_change_snaps_in_flight_values`.

### Build

- Milestone: M4.
- Depends on: core.input, core.painter, core.scene, core.theme, text.shaping, motion.transitions.
- Done (D47, M11): tests `crates/lune-ui-core/tests/ui_core.rs::a_hidden_tab_under_keep_alive_keeps_its_state_until_the_scope_ends, crates/lune-ui-core/tests/ui_core.rs::a_click_needs_press_and_release_on_the_same_id, crates/lune-ui-core/tests/ui_core.rs::a_drag_starts_at_4_px_with_the_full_travel, crates/lune-ui-core/tests/ui_core.rs::popup_over_page_wins_at_the_same_point, crates/lune-ui-core/tests/ui_core.rs::repaint_at_the_earliest_future_request, crates/lune-ui-core/tests/ui_core.rs::a_script_run_in_two_fresh_stores_gives_identical_frames, crates/lune-ui-core/tests/ui_overlays.rs::an_overlay_the_app_stops_calling_keeps_drawing_until_its_close_ends, crates/lune-ui-core/tests/ui_overlays.rs::a_click_on_a_page_button_under_an_open_modal_does_nothing, crates/lune-ui-core/tests/ui_overlays.rs::escape_closes_only_the_top_overlay, crates/lune-ui-core/tests/ui_focus.rs::tab_inside_a_modal_never_leaves_it, crates/lune-ui-core/tests/ui_focus.rs::closing_restores_the_previous_focus`; page `ui`; unverified here: none.

## Open questions

- None. Resolved: the `Ui` owns a `Painter<WidgetId>`, `painter.hover` is the hovered id and raw `Painter::hit` regions join the layered hit list as click and hover hits, so both APIs coexist in one frame [D61].

## Notes for the planner

C4 (roadmap) is the source of the model rules; Lunegit supplies the numbers (4 px drag threshold,
400 ms double click, exit retention) and the overlay dismiss behaviour. Lunegit has no focus
restore or layered hit testing of its own: those rules are C4-only and are tested, not copied.
