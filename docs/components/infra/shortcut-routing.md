---
id: infra.shortcut-routing
title: Keyboard shortcut routing
area: infra
status: done
consumers: any
source: crates/lune-ui-core/src/shortcuts.rs
owner: —
summary: A scoped chord registry resolved in three phases (preempt, focused widget, bubble); Lunegit has no central one
---

## What it is

A registry that maps key chords to command ids and decides who sees a key first. Lunegit has no
central registry and no user-editable shortcuts: each view binds its own keys (a window-level tunnel
handler for Escape and Ctrl or Cmd+K, a `KeyBinding` for Cmd+T on the terminal panel and Cmd+C in the
diff views). The toolkit makes the same behaviour one data structure: scoped bindings, three phases,
an enable flag the app sets per frame. The registry is data the app can change at run time; a remap
screen is app-side.

## Needs

- Shortcut routing across scopes (R-infra-2).
- Keys reach the focused widget, then unclaimed keys reach the registry (R-widget-3, R-core-8).
- No OS or environment read: the app says which modifier is the platform's primary one (R-nfr-1).

## Existing code

Built in M5 (D92, D93): `crates/lune-ui-core/src/shortcuts.rs` (`Shortcuts`, `Binding`, `Chord`, `Scope`, `CommandId`; `FrameConfig.shortcuts`, `FrameOutput.fired`). Before it, the moved menu drew a shortcut hint as app text (`crates/lune-ui-core/src/widgets/menu.rs:10-16`; the
gallery menu shows "Cmd+N", "Cmd+C", "Cmd+," at `apps/gallery/src/scenes.rs:379-383`). Lunegit
(`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, 33a1dd41), relative to its root:
- `src/Lunegit.Desktop/MainWindow.axaml.cs:30,91-113` (window tunnel handler: Escape, Ctrl or Cmd+K)
- `src/Lunegit.Desktop/Views/TerminalPanelView.axaml:17-19` (Meta+T), `.axaml.cs:334-341` (Esc cancels a drag)
- `src/Lunegit.Desktop/Views/UnifiedDiffView.axaml.cs:52-56`, `SplitDiffView.axaml.cs:52-56` (Cmd+C)
- `src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml.cs:14-42`, `Views/SidePanels/CommitPanelView.axaml.cs:34-41`
- `src/Lunegit.Desktop/Controls/Terminal/TerminalKeyRouter.cs` (which keys the terminal claims)

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/MainWindow.axaml.cs:25-35,85-113,
src/Lunegit.Desktop/Views/TerminalPanelView.axaml:15-20, src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:334-341,
src/Lunegit.Desktop/Views/UnifiedDiffView.axaml.cs:45-70, src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:48-60,
src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml.cs:10-43, src/Lunegit.Desktop/Views/SidePanels/CommitPanelView.axaml.cs:28-42,
src/Lunegit.Desktop/Views/AiProjectDetailView.axaml.cs:62-72, src/Lunegit.Desktop/Controls/Terminal/TerminalKeyRouter.cs:1-80;
`crates/lune-ui-core/src/input.rs:50-65,97-138`; `apps/gallery/src/scenes.rs:366-393`. A search for `HotKey`, `KeyBindings`, `KeyGesture`,
`KeyDown +=` and `OnKeyDown` in src/Lunegit.Desktop found no central registry; a search for "shortcut" in src/Lunegit.Core,
src/Lunegit.Desktop/ViewModels and src/Lunegit.Desktop/Views found no user-editable shortcut setting (two hits, neither about key
bindings).
Conflict: none between sources; Lunegit's rules differ by view (Cmd+Enter in the commit box accepts Control or Meta
[L src/Lunegit.Desktop/Views/SidePanels/CommitPanelView.axaml.cs:36-38], Cmd+K uses Meta on macOS and Control elsewhere
[L src/Lunegit.Desktop/MainWindow.axaml.cs:100-105]), so the chord type offers both forms. Wane lists keyboard
navigation as a requirement but gives no shortcut table [W docs/ui/components.md:728].

### Anatomy

- `Chord { modifiers, key }`: `key` is a `Key` (named or character) and `modifiers` a set of Shift, Alt, `Primary`, and Ctrl and Logo spelled out; `Primary` is Cmd on macOS and Ctrl elsewhere, resolved from a flag the app passes with the frame, never read from the OS [R crates/lune-ui-core/src/input.rs:87-95,130-138], [L src/Lunegit.Desktop/MainWindow.axaml.cs:102-104], [D3].
- `CtrlOrLogo`: a chord that accepts either one, for bindings Lunegit makes both-platform (Cmd or Ctrl+Enter in the commit box) [L src/Lunegit.Desktop/Views/SidePanels/CommitPanelView.axaml.cs:36-38].
- `Shortcuts`: a table of scoped bindings `(scope, chord, command id, phase)`; the command id is the app's own key (a string or enum). The toolkit does not run commands; it reports `Fired { command }` in `FrameOutput` or the `Response` of the call that registered it [derived: the app owns behaviour, D3].
- Scopes, outermost first: global (the window), page, overlay (one per open overlay, pushed and popped with its focus scope), widget (the focused widget) [core.focus], [derived: Lunegit binds at the window, the dialog view, the panel and the diff view].
- Phases for one key press, as in Lunegit's tunnel then bubble: (1) preempt: bindings marked `preempt` in the top overlay scope and the global scope see the key before the focused widget (Escape closes a dialog and Primary+K opens search before any control; the dialog view's own tunnel handler takes Escape, Up, Down, Enter first) [L src/Lunegit.Desktop/MainWindow.axaml.cs:30,93-111], [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml.cs:14,23-40]; (2) the focused widget, which consumes the keys it handles and every printable key when it is a text input [core.focus], [R crates/lune-ui-core/src/input.rs:57-60]; (3) bubble: remaining bindings from the innermost scope outward (Cmd+C on the diff view, Cmd+T on the terminal panel) [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml.cs:52-56], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:17-19]. The first binding that fires consumes the press; it is not offered again.
- Enable flag: each binding carries `enabled`, set by the app every frame; Lunegit's Cmd+K fires only when no dialog is open and a repository is active, and the terminal's Esc binding swallows the key only while a drag is in flight [L src/Lunegit.Desktop/MainWindow.axaml.cs:105-107], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:336-341].
- Widgets that claim keys: a text input or an editor claims printable keys, a terminal-like widget claims Control chords but never Logo chords, so Cmd shortcuts reach the registry [L src/Lunegit.Desktop/Controls/Terminal/TerminalKeyRouter.cs:39-65,71-79], [D8].
- Key repeat: a binding fires on press, and on repeat only when it asks for it (`repeat: bool` on the event) [R crates/lune-ui-core/src/input.rs:50-56], [derived: no source states it].
- Hints: a helper formats a chord for a menu row ("Cmd+K" or "Ctrl+K" from the primary flag); the moved menu takes the hint as a string [R crates/lune-ui-core/src/widgets/menu.rs:10-16].
- Not in v1: chord sequences and per-user remapping UI (Lunegit has neither; its bindings are all single chords) [L src/Lunegit.Desktop/MainWindow.axaml.cs:100-111].

### States

n/a — a registry, not a component.

### Transitions

n/a — no animation.

### Input and focus

- Keys only: `Event::Key` with `modifiers` and `repeat`, after the focused widget has declined it [R crates/lune-ui-core/src/input.rs:50-56].
- Tab stop: no. Layer: an overlay scope belongs to its overlay's layer; scopes stack with the overlay stack, and a modal overlay's scope hides the page and global bindings except those marked `preempt` [core.ui], [core.focus].
- Dismiss rules: Escape and the outside click act on the top overlay; Escape registered as a binding in the overlay scope is the same rule [core.ui].

### Retained state

- Transient: none. Persistent: none.
- App-owned: the whole table, the per-frame enable flags and the primary-modifier flag; the app may rebind at run time.
- Exit retention: no.

### Accessibility

n/a — no widget. A binding's chord is shown as text on the menu row it belongs to; the semantic node has no shortcut field in v1 [infra.accessibility].

### Strings

None: chords are formatted with fixed ASCII names (Cmd, Ctrl, Alt, Shift, then the key), as the moved menu's hints are [R apps/gallery/src/scenes.rs:379-383]; command names are the app's text.

### Gallery scenes

- `menu` (existing): the shortcut hints as the app supplies them.
- `shortcut-hints`: a menu whose hints are formatted by the helper, once with the primary flag set for macOS ("Cmd+N") and once for Windows ("Ctrl+N"), settled.

### Painter primitives

- Existing: text, fill.
- New: none.

### Acceptance

- Scripted keys with injected time: Primary+K with the flag set to Logo fires `open-search` and Ctrl+K does not; with the flag unset the reverse; with `enabled` false nothing fires [L src/Lunegit.Desktop/MainWindow.axaml.cs:100-107].
- Preempt: with a dialog open and a text field focused, Escape fires the overlay binding and the field never sees it; with no dialog the field sees Escape first [L src/Lunegit.Desktop/MainWindow.axaml.cs:93-97].
- Bubble: Cmd+C fires the diff view's binding when the focused widget declines it and does not fire when a text input consumed it [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml.cs:52-56].
- A consumed press is not offered to another scope; an unbound chord falls through to the focused widget; a printable key typed in a text input never reaches an unmodified binding.
- `CtrlOrLogo+Enter` fires for Ctrl+Enter and for Logo+Enter [L src/Lunegit.Desktop/Views/SidePanels/CommitPanelView.axaml.cs:36-38].
- Repeat: a binding without `repeat` fires once for a held key.
- Hint helper: known answers "Cmd+N", "Ctrl+N", "Cmd+," for the primary flag on and off.
- Tests, bullet by bullet (`crates/lune-ui-core/tests/shortcuts.rs`). `the_primary_flag_picks_command_or_control_both_ways`, `a_disabled_binding_never_fires_and_consumes_nothing`; preempt: `escape_with_a_dialog_fires_the_overlay_binding_and_the_field_never_sees_it`, `escape_with_no_dialog_reaches_the_field_first`; bubble: `bubble_fires_when_the_focused_widget_declines_and_not_when_it_consumed`, `within_the_focused_widget_itself_and_not_an_unrelated_scope`; scopes: `the_innermost_scope_wins_and_a_press_fires_one_binding`, `a_modal_hides_page_and_global_bindings_that_do_not_preempt`, `an_overlay_binding_fires_for_a_press_the_dialog_declined`; consumed, unbound and printable keys: `a_press_the_field_took_is_not_offered_to_another_scope` (also an unbound chord reaching the field and a plain letter never reaching an unmodified binding); `ctrl_or_logo_enter_fires_for_both`; repeat: `a_binding_without_repeat_fires_once_for_a_held_key`; hints: `hints_use_the_fixed_order_and_the_keymap`; also `nothing_fires_while_the_focused_widget_composes`, `a_fired_command_makes_the_frame_repaint_now`.

### Build

- Milestone: M5 (the registry is core, no platform code; `overlay.command-palette` of M7 binds through it; `core.ui`'s key phase and `core.focus` of M4 have an empty registry until then). M5 built the registry (steps 3 and 6; the page is `shortcuts`, the `shortcut-hints` scene). `overlay.command-palette` of M7 binds through it. Platform mapping done in M10 step 4: `lune_ui::input::keymap()` picks `Keymap::MAC` or `Keymap::PC` per platform (D212), so an app no longer branches on the OS itself. M11 audited it against D47 (the `Done` line). Unverified here: the Windows and Linux run time (the lint for both is `scripts/check-targets.sh`).
- Depends on: core.input, core.focus, core.ui.
- Done (D47, M11): tests `crates/lune-ui-core/tests/shortcuts.rs::the_primary_flag_picks_command_or_control_both_ways, crates/lune-ui-core/tests/shortcuts.rs::escape_with_a_dialog_fires_the_overlay_binding_and_the_field_never_sees_it, crates/lune-ui-core/tests/shortcuts.rs::bubble_fires_when_the_focused_widget_declines_and_not_when_it_consumed, crates/lune-ui-core/tests/shortcuts.rs::a_press_the_field_took_is_not_offered_to_another_scope, crates/lune-ui-core/tests/shortcuts.rs::ctrl_or_logo_enter_fires_for_both, crates/lune-ui-core/tests/shortcuts.rs::a_binding_without_repeat_fires_once_for_a_held_key, crates/lune-ui-core/tests/shortcuts.rs::hints_use_the_fixed_order_and_the_keymap`; page `shortcuts` (scene `shortcut-hints`); unverified here: Windows and Linux run time.

## Open questions

- Answered (D93, `FrameOutput.fired`): whether the registry reports `Fired` through `FrameOutput` or through the `Response` of a `ui.shortcut(command)` call: both are cheap; the choice affects how an app reads it and is M5's to make with the gallery's shortcut page.
- Still open (no consumer): whether a user-visible remap needs a persisted form (a JSON chord string): Lunegit has none, so no source gives a format; it would use `infra.json` if an app asks.

## Notes for the planner

The stub asked whether a central registry exists and whether shortcuts are user-editable: neither,
verified by the searches in Evidence. The terminal's own key encoding (`TerminalKeyRouter`) stays in
the terminal app (D8); only its rule that Logo chords are never encoded matters here.
