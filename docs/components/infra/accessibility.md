---
id: infra.accessibility
title: Semantic tree and accessibility
area: infra
status: planned
consumers: any
source: —
owner: —
summary: One semantic node per interactive widget per frame (role, name, value, states, actions), AccessAction routing, AccessKit bridge in M10
---

## What it is

The accessibility model of the toolkit. From M4 on, every interactive widget emits one semantic node
per frame, so components are born with it (DECISIONS D46). `FrameOutput` carries the whole tree and
the focused node id each frame. Assistive technology acts on a node through `AccessAction { target,
action, value }`, which arrives in `RawInput` and which the `Ui` routes to the widget as the matching
pointer or key input. `lune-ui-winit` bridges the tree to AccessKit behind a cargo feature in M10;
that bridge is a platform layer and needs its own DECISIONS row with its size measured.

## Needs

- A semantic tree every frame: roles, names, states, actions (R-infra-4).
- Focus indication and a focused node id (R-widget-3).
- Actions come back as input, so a screen reader can click, focus, step and dismiss (R-infra-4).
- A platform bridge to the OS accessibility API behind a cargo feature (R-infra-4, D46).

## Existing code

None. The model rides on `RawInput`/`FrameOutput` (`crates/lune-ui-core/src/input.rs:16-27,155-164`), whose
`Event` enum is `#[non_exhaustive]` (`crates/lune-ui-core/src/input.rs:31`), so adding an access event is a non-breaking
extension (DECISIONS D35). Lunegit names a few icon-only buttons through `AutomationProperties.Name`
(`src/Lunegit.Desktop/Views/AiProjectDetailView.axaml:94,111`) and has one test that reads the name back
(`src/Lunegit.Tests/Desktop/AiProjectDetailViewInteractionTests.cs:317-323`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/input.rs:14-65,154-180`;
Lunegit src/Lunegit.Desktop/Views/AiProjectDetailView.axaml:90-115,385-390, src/Lunegit.Tests/Desktop/AiProjectDetailViewInteractionTests.cs:310-325
(and a search of src/Lunegit.Desktop for `AutomationProperties` and `AutomationPeer`: 3 uses, all in that view);
Wane docs/ui/mocks/js/shell.js:115-118, docs/ui/mocks/overview.html:28-30, docs/ui/mocks/first-run.html:17-19,
docs/ui/components.md:224,423, docs/ui/README.md:104, docs/ui/mocks/checks.mjs:582-586.
Conflict: Lunegit carries almost no accessibility metadata (three `AutomationProperties.Name` uses in one
view); Wane's mocks use `role="navigation"`, `role="note"`, `role="menu"`, `aria-label` and `aria-current`
on a few elements. Neither gives a full model, so the node fields and enums are C4's (roadmap), and
Wane's roles that the enum lacks are recorded as `role-gap` below. No source gives contrast ratios.

### Anatomy

- Node: `id` (the widget's `WidgetId`), `role`, `name`, `description`, `value`, for text inputs the text and its selection as char offsets, `bounds` (logical, after `Xform`), child order, a live-region politeness, `states`, supported `actions` [derived: C4 semantic tree], [R crates/lune-ui-core/src/paint.rs:86-93].
- `role` (36 values, one enum): button, checkbox, switch, radio, radio-group, slider, spin-button, text-input, multiline-text, combo-box, list, list-item, tree, tree-item, tab-list, tab, menu, menu-item, dialog, alert, tooltip, table, row, cell, column-header, progress, scroll-area, toolbar, group, image, link, static-text, status, navigation, note, separator (the last three added by [D55]) [derived: C4 role enum].
- `name`: the accessible name: the widget's visible text, else the string the app or the toolkit supplies; `description`: secondary text such as a row's description; Lunegit names its icon-only buttons through the string table (`AutomationProperties.Name="{loc:Tr ...}"`) [L src/Lunegit.Desktop/Views/AiProjectDetailView.axaml:94,111], [infra.localization].
- `value`: text, or a number with `min`, `max` and `step` (slider, spin button, progress) [derived: C4].
- `states` (15 flags): focusable, focused, disabled, checked (including mixed), selected, expanded, pressed, modal, read-only, required, invalid, busy, hidden, current (the active page, step or location: Wane's `aria-current` [W docs/ui/components.md:423]), dragging (the item under a drag: kanban cards, reordered rows and tabs); the last two added by [D55] [derived: C4 states].
- `sort`: an optional node field on a `column-header`: ascending, descending or none, added by [D55] [derived: C4 node fields].
- `actions` (10): default (click), focus, increment, decrement, set-value, expand, collapse, scroll-into-view, show-menu, dismiss [derived: C4 actions].
- Live region: `polite` or `assertive`, set on toasts and status nodes [derived: C4].
- `AccessAction { target: WidgetId, action, value }` arrives as a new `Event::Access` in `RawInput.events` (an `#[non_exhaustive]` enum, so no break) [R crates/lune-ui-core/src/input.rs:30-32], [D35]; `value` carries the text or number for `set-value`.
- Routing: the `Ui` turns an action into the input the widget already handles. `default` is a click on the target (so `Response.clicked` and `changed` behave identically), `focus` is `request_focus` [core.focus], `increment` and `decrement` are Up and Down on a focused slider, spin button or stepper, `set-value` writes the value through the widget's own edit path, `expand` and `collapse` are Right and Left (or the toggle click) on a tree item or disclosure, `scroll-into-view` scrolls the nearest scroll area, `show-menu` is the secondary click or the menu key, `dismiss` is Escape on the top overlay [derived: C4 "routed as the matching pointer or key input"], [core.ui].
- `FrameOutput` carries `tree: SemanticTree` (nodes in call order, parent before children) and `focused: Option<WidgetId>` next to the moved cursor, IME area and commands [R crates/lune-ui-core/src/input.rs:155-164], [core.ui].
- Emission: widgets call `ui.node(id, role, ...)` inside `ui.interact`; containers (`list`, `tree`, `table`, `toolbar`, `group`, `dialog`) open a node and the nodes drawn inside are its children; a hidden or clipped-out widget emits nothing unless it is kept alive, and `hidden` marks content the app keeps but does not show [derived: C4].
- AccessKit bridge (M10): `lune-ui-winit` maps the tree to AccessKit nodes and AccessKit action requests back to `AccessAction`, behind a cargo feature; a new dependency needs a DECISIONS row as a platform layer with its release-size change noted (hard rule 5) [D46], [D2].
- Wane's markup that the mocks use: a navigation landmark labelled "Setup steps" [W docs/ui/mocks/js/shell.js:117], notes on callouts [W docs/ui/mocks/overview.html:28], a current-page mark on the active nav row [W docs/ui/components.md:423], a state-carrying `aria-label` on rule switches [W docs/ui/README.md:104], panels labelled with their title [W docs/ui/mocks/first-run.html:17].

### States

n/a — the tree has no look; each node's `states` mirror the widget's own states (focused, disabled, checked, selected, expanded, pressed, busy, invalid).

### Transitions

n/a — the tree is rebuilt every frame; nothing animates. A widget's node reports its target state, not the mid-transition value.

### Input and focus

- Pointer: none. Keyboard: none of its own; a screen reader's focus request is `AccessAction::Focus` [core.focus].
- Layer: the node of an overlay is a child of the dialog or menu node; a modal overlay's node has `modal` set, and the nodes below it are `hidden` from the tree for assistive technology while the modal is open [derived: C4 modal capture], [core.ui].
- Action routing and dismiss rules are in Anatomy; `AccessAction` reaches only the top overlay when a modal is open.

### Retained state

- Transient: none; the tree is built per frame from the widgets that were called.
- Persistent: none held here; the focused id is `core.focus`'s.
- App-owned: the names, descriptions and values the app supplies; the strings come through the string table [infra.localization].
- Exit retention: a retained exiting overlay (a closing dialog) keeps its node with `hidden` set until it settles [core.ui].

### Accessibility

- This entry defines the contract; a component's own `### Accessibility` subsection picks a role from the 36 values, a name source, and its states and actions.
- Resolved by [D55]: `navigation` (Wane's steps list is a navigation landmark with a label [W docs/ui/mocks/js/shell.js:117]), `note` (Wane's callouts are `role="note"` [W docs/ui/mocks/overview.html:28]; a warning callout that must be announced still uses `alert`), `separator` (a splitter or divider; a resizable one carries a value), the `current` state and the `dragging` state, and the `sort` field. The mapping to the nearest older role is no longer needed.
- Still open, listed in Open questions: heading level, multi-select lists, tab-to-panel association, composition range, ranged text, a keyboard move action.

### Strings

None: node names come from each widget's visible text or from keys in that widget's own entry (Lunegit builds accessible names from the string table [L src/Lunegit.Desktop/Views/AiProjectDetailView.axaml:94,111]). No key is defined here.

### Gallery scenes

- `a11y-nodes`: the `controls` scene with each node's bounds outlined and its role and name written beside it, settled (a gallery aid; it shows nothing the OS bridge adds).
- `controls`, `dialog-confirm`, `menu`, `dropdown` (existing): the scenes whose frames the contract test walks; every interactive item in them has a node once the M4 widgets emit nodes.

### Painter primitives

- Existing: stroke, text (for the `a11y-nodes` aid only).
- New: none.

### Acceptance

- Contract test: a scripted frame of every M4 widget yields the expected roles, names and states, and an injected `AccessAction` default on a checkbox node toggles it and sets `changed` [derived: C4 contract test].
- Every role, state and action value of the three enums is constructible and round-trips through the tree; the enums have 36, 15 and 10 members.
- Node bounds equal the painter rect after `Xform` for a widget drawn under a scale 0.97 transform [R crates/lune-ui-core/src/paint.rs:86-93].
- Child order equals call order; a parent precedes its children; the focused id in `FrameOutput` equals the node whose `focused` state is set [core.focus].
- A modal overlay's node is `modal`; nodes under it are not reachable by `AccessAction` while it is open [core.ui].
- A toast node has live politeness `polite` and an error toast `assertive` [derived: C4 live regions].
- `set-value` on a slider clamps to `min` and `max` and snaps to `step`; an out-of-range value never panics.
- M10: the bridge feature builds with and without the AccessKit dependency; its size change is in the commit message (hard rule 5).

### Build

- Milestone: M4 (model, tree, routing, contract test) and M10 (AccessKit bridge).
- Depends on: core.ui, core.focus, core.input, infra.localization.

## Open questions

- `role-gap` items other than the ones [D55] resolved stay open in their own entries: heading level (`text.markdown-preview`), multi-select reporting (`layout.virtualized-list`), tab-to-panel association (`widget.tab-strip`), composition range (`text.ime`), ranged text for large documents (`text.code-editor-view`) and a keyboard move or reorder action (`infra.drag-and-drop`). M4 settles them or keeps the nearest-role mapping.
- The mapping from this enum to AccessKit's roles and actions: no source in this repo; M10 reads AccessKit when it builds the bridge.
- A live-region node that updates every frame (a progress label): C4 gives politeness but no rate limit; M4 decides whether the bridge debounces.

## Notes for the planner

Every later entry's `### Accessibility` cites this one for the enums, so do not rename a role or an
action here without a roadmap revision. Lunegit gives almost nothing to port: the tree is new work
built from C4, and the contract test is the done standard (D47).
