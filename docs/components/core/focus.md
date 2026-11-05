---
id: core.focus
title: Keyboard focus
area: core
status: in-progress
consumers: any
source: crates/lune-ui-core/src/ui/focus.rs
owner: orch/toolkit-v1
summary: Tab order by call order within focus scopes, focus-visible, scope push, trap and restore, roving focus in composites
---

## What it is

Which widget receives keys, how Tab moves between widgets, when the focus ring shows, and how an
overlay takes and gives back focus. Focus is part of the `Ui` frame (`core.ui`): the focused id is
retained state of the window, Tab order follows call order within the current focus scope, a
focus-visible ring appears after keyboard navigation, and keys route to the focused widget. Opening
an overlay pushes a focus scope (a modal traps Tab inside it); closing it pops the scope and restores
focus to the widget focused before it opened, or to its anchor.

## Needs

- Keyboard focus order, focus-visible and key handling inside widgets (R-widget-3).
- The focused id, retained per window, and routing of key events (R-core-8).
- Focus indication as part of accessibility (R-infra-4).

## Existing code

None for focus itself. Today a widget takes `focused: bool` from the app: `field` and `combo` draw the
accent border and a caret when told (`crates/lune-ui-core/src/widgets/mod.rs:50-66,119-142`);
`RawInput.focused` and `Event::Focus` carry window focus (`crates/lune-ui-core/src/input.rs:23-24,61-62`); `NamedKey::Tab` exists
(`crates/lune-ui-core/src/input.rs:102`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:50-144`,
`crates/lune-ui-core/src/input.rs:14-65,97-128`; Lunegit src/Lunegit.Desktop/MainWindow.axaml:240-250,
src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml.cs:10-43, src/Lunegit.Desktop/Views/Dialogs/AddWorktreeDialogView.axaml.cs:15-30,
src/Lunegit.Desktop/Controls/FilterBar.axaml:25-45, src/Lunegit.Desktop/Controls/FilterBar.axaml.cs:60-66,
src/Lunegit.Desktop/Controls/SearchPickerBase.cs:296-320, .claude/docs/animation-rules.md:98-110; Wane
docs/ui/mocks/css/components.css:120-132,170-200,230-236,286-292,308-314, docs/ui/components.md:295-297,309-311,335-339,351-353,419-423,449-451,561-563,575-577,728.
Conflict: Lunegit removes Fluent's focus border on its embedded text boxes (transparent in every state)
[L src/Lunegit.Desktop/Controls/FilterBar.axaml:28-41] and styles no keyboard focus ring for buttons or
rows, so it gives no ring to follow; Wane specifies the ring, so the component only Wane draws wins
and its values are used [W docs/ui/mocks/css/components.css:125]. Lunegit's only trap is the dialog card's
`TabNavigation="Cycle"` [L src/Lunegit.Desktop/MainWindow.axaml:249]; it has no focus restore on close
(searched `FocusManager`, `RestoreFocus`, `PreviousFocus` in src/Lunegit.Desktop: none), so restore is a
C4-only rule [core.ui].

### Anatomy

- Focus identity: the focused `WidgetId` of the window, held in the retained store; `ui.interact(id, rect, Sense::focusable)` registers a tab stop in call order [core.ui].
- Tab order: Tab moves to the next tab stop in call order within the current focus scope, Shift+Tab to the previous; the order wraps inside a scope [derived: C4 "Tab order follows call order within the current focus scope"; the wrap follows Lunegit's `Cycle` on the dialog card, `src/Lunegit.Desktop/MainWindow.axaml:249`].
- Focus scopes: a stack. The base scope is the window; an overlay pushes a scope on open. A modal scope traps Tab: it cycles inside the overlay and never reaches a lower layer [derived: C4], [L src/Lunegit.Desktop/MainWindow.axaml:249], [W docs/ui/components.md:563].
- Initial focus on open: the overlay names the widget to focus (a text field for a search or form dialog), else its first tab stop; Lunegit focuses the first text box when a dialog attaches [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml.cs:15-16], [L src/Lunegit.Desktop/Views/Dialogs/AddWorktreeDialogView.axaml.cs:19].
- Restore on close: popping the scope returns focus to the widget focused before the overlay opened, or to the overlay's anchor widget when that is gone [core.ui]. Scopes are a stack ([D83]): closing an overlay whose scope is not the active one (focus is in a later overlay still open) moves no focus, a later overlay whose saved focus was inside the closing one inherits the closing one's saved focus, and a restore never lands under an open modal. A passive overlay with no stop leaves focus where it is on open ([D82]).
- Focus-visible: the ring shows after keyboard navigation (Tab, Shift+Tab, arrow keys, a key press on the widget) and hides after a pointer press; Wane styles the ring on `:focus-visible` [W docs/ui/mocks/css/components.css:125,174,197]. A text input shows its focus as an accent border at all times (focus-within) [W docs/ui/mocks/css/components.css:234,290], matching the moved field and combo [R crates/lune-ui-core/src/widgets/input_box.rs:29-34].
- Ring look: button and ghost button: 1 px `fg_secondary` outline, offset -1 (inset); primary button: outline `on_accent_text`; toggle, checkbox and radio: 1 px `fg_secondary` outline at offset 2; fields, steppers and the open combo: border `accent` [W docs/ui/mocks/css/components.css:125,130,174,197-198,234,290,312], theme fields [R crates/lune-ui-core/src/theme.rs:13,20-25].
- Pointer: a press on a focusable widget focuses it; a press on a non-focusable widget leaves focus alone; clicking a form label focuses its control [W docs/ui/components.md:451].
- Keys: routed to the focused widget; keys it does not consume go to the shortcut registry; Tab and Shift+Tab are consumed here unless the focused widget keeps Tab as text (a code editor) [infra.shortcut-routing], [derived: no source says whether a text field keeps Tab; Wane lists Tab among its behaviours, docs/ui/components.md:353 is not a ruling].
- Activation keys: Enter and Space activate a button; Space toggles a toggle and a checkbox; Enter, Space and Down open a combo, Esc closes it; Up, Down and Enter pick in a dropdown [W docs/ui/components.md:297,311,325,381,535].
- Roving focus in composites: a radio group, tab list, menu, nav list or list is one tab stop; Tab enters it once, arrow keys move the active child inside it [W docs/ui/components.md:339,423], [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml.cs:29-36].
- Escape: dismisses the top overlay first, unless a live pointer gesture holds its capture, which cancels first (`core.ui`, D175); in a search field it closes its dropdown [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:304-308].
- Programmatic focus: `ui.request_focus(id)` from the app; `AccessAction::Focus` calls it [infra.accessibility]; Lunegit posts a focus request to a text box after a view appears [L src/Lunegit.Desktop/Controls/FilterBar.axaml.cs:65].
- Window focus: `RawInput.focused` false hides the ring and stops caret blink; the same widget is focused again when the window regains focus [R crates/lune-ui-core/src/input.rs:23-24,61-62], [derived: no source states it].
- Disabled widgets are not tab stops [W docs/ui/components.md:295].

### States

| state | look | cite |
|---|---|---|
| not focused | no ring | [core.focus] |
| focused by pointer | no ring on buttons, toggles, checkboxes and radios; accent border on text inputs | [W docs/ui/mocks/css/components.css:125,234] |
| focus-visible | 1 px `fg_secondary` outline (inset on buttons, offset 2 on toggles, checkboxes, radios); `on_accent_text` on a primary button | [W docs/ui/mocks/css/components.css:125,130,174,197-198] |
| focus-within (text input) | border `accent` | [W docs/ui/mocks/css/components.css:234,290,312] |
| scope trapped | Tab cycles inside the modal; lower layers show no focus | [L src/Lunegit.Desktop/MainWindow.axaml:249] |
| disabled | not a tab stop, no ring | [W docs/ui/components.md:295,297] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| ring appears and disappears | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104] |
| text input gains and loses focus | border colour | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |
| focus moves between widgets | ring on the old widget fades out while the new one fades in; no moving ring | focus | fast | linear | keep | [D40] |

Wane's focus rules are static (no transition on the ring [W docs/ui/mocks/css/components.css:125]); the fade is
Lunegit's kind `focus`. Never animated: nothing beyond the motion.md list.

### Input and focus

- Pointer: press focuses a focusable widget; the ring stays hidden for pointer focus.
- Keyboard: Tab, Shift+Tab move; arrows rove inside composites; Enter and Space activate; Escape dismisses the top overlay [core.ui].
- Tab stop: yes for every focusable widget, once per composite.
- Layer: a scope belongs to its overlay's layer; scopes stack `Base` < `Page` < `Popup` < `Modal` < `Toast` with the overlay stack ([D54]) [core.ui].
- Scope rules: push on open; a modal traps Tab; pop restores focus to the previous widget or the anchor; Escape and an outside click act on the top overlay only; focus loss closes a flyout and a light-dismiss popup [W docs/ui/components.md:577], [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:57].
- Pointer capture: a pressed widget keeps the pointer until release; it does not move focus on release [core.ui].

### Retained state

- Transient: the ring's fade, the pointer-versus-keyboard origin of the current focus (decides focus-visible) [core.ui].
- Persistent: the focused `WidgetId` and the scope stack, held in the window's store; under `ui.keep_alive` a hidden tab keeps its focused widget [derived: C4 gives no rule for a focused widget that vanishes].
- App-owned: none; the app may ask for focus (`request_focus`) and reads `Response.focused`.
- Exit retention: no (a closing overlay's scope is popped on the frame the app stops calling it; its visuals are retained, its focus is not).

### Accessibility

- The focused id and its `focused` state are in the tree; `focusable` is set on every tab stop; `AccessAction::Focus` moves focus [infra.accessibility].
- Roles: none of its own; composites expose their role (`radio-group`, `tab-list`, `menu`, `list`, `tree`) and the active child is the focused node [infra.accessibility].
- Live region: none. Focus indication is the ring and the accent border, drawn with the contrast the tokens give; no value is asserted here [derived: R-infra-4 names contrast and no source gives a ratio].

### Strings

None: no user-visible text.

### Gallery scenes

- `focus-ring-kinds`: a button, a primary button, a toggle, a checkbox, a radio, a text field, a stepper and a combo, each focus-visible, settled.
- `focus-ring-appearing`: the button's ring appearing at t = 90 ms (half of `fast`).
- `focus-modal-initial`: a confirm dialog open with its first control focus-visible, settled (extends `dialog-confirm`).
- `focus-pointer`: the same controls focused by pointer (no rings, accent border on the field), settled.

### Painter primitives

- Existing: ring, fill, xform (alpha for the ring fade).
- New: none.

### Acceptance

- Tab order: three tab stops drawn in order A, B, C: Tab goes A, B, C, A (wraps); Shift+Tab reverses; a disabled widget is skipped [derived: C4], [W docs/ui/components.md:295].
- Tab inside a modal never leaves it, in 20 consecutive Tab presses; a page widget under the modal never reports `focused` [derived: C4 contract test].
- Closing a modal restores focus to the widget that was focused before it opened (or its anchor when that widget is gone) [derived: C4 contract test].
- Focus-visible: after a scripted Tab the ring is on; after a pointer press on another widget it is off; a text input shows its accent border either way.
- A radio group is one tab stop: Tab enters once, Down moves the selection, a second Tab leaves [W docs/ui/components.md:339].
- Pixel probes at scale 2: ring 1 px outline = 2 device px, inset -1 on a button, offset 2 on a toggle (a 40 x 20 track gives an outline box 44 x 24 logical), colour `fg_secondary` [W docs/ui/mocks/css/components.css:125,174].
- Conformance: the ring fade passes the 60 Hz harness under Full, Reduced and Off [D41].
- Semantic node: the focused node id equals `FrameOutput`'s focused id; `AccessAction::Focus` focuses the target and shows no ring (it is not keyboard navigation) [infra.accessibility].

### Build

- Milestone: M4. The radio row of `focus-ring-kinds` was deferred [D74] and landed with `widget.radio-group` in M6 [D116], [D120]; the scene's delta is the one allowed change of that step.
- Depends on: core.ui, core.input, motion.transitions, infra.accessibility.

## Open questions

- Resolved [D72]: a focused widget that is not registered for one frame loses focus unless a live keep-alive scope covers it.
- Tab inside a code editor or terminal view: whether the widget keeps Tab as text or releases it is that widget's spec (`text.code-editor-view`); the registry here only reads the widget's answer.

## Notes for the planner

Lunegit offers no ring values and no restore; Wane's `:focus-visible` rules are the only look source.
The tab-stop registration rides on `ui.interact`, so a widget that forgets `Sense::focusable` is not
reachable: the M4 widget retrofit (toggle, button, checkbox...) must set it.
