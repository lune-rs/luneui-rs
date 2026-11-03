---
id: layout.tree-view
title: Tree view and tree guides
area: layout
status: planned
consumers: lunegit, wane
source: Lunegit: src/Lunegit.Desktop/Controls/TreeGuideControl.cs
owner: —
summary: Tree rows with guide lines (trunk and rounded elbow) per depth; expansion is a set of node keys, animated on the container
---

## What it is

Tree rows with guide lines. `TreeGuideControl` draws the trunk and rounded elbow per row, in a 16 px indent per depth [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:15-21]. Expansion is a per-row flag in Lunegit's view model [L src/Lunegit.Desktop/ViewModels/SidePanelRow.cs:29-41]; the toolkit keeps expansion as a set of node keys (plan contract, Retained state).

## Needs

- Tree rows with guide lines (R-layout-3).
- Wane — Variants: child (`__row--child`, depth 1; indent 16 per depth through `--depth`); has-next continues the guide to the next sibling. (R-layout-3)
- Wane — States: rest; hover (`--surface-hover`). Not expandable or collapsible in the mocks. (R-layout-3)
- Wane — Sizes: Rows 28, radius 6, padding-left 8 + depth x 16; guide at x 16, 1 px `--border`; elbow 8 x 14 with radius 3.5 (`--radius-elbow`); parts: name 13, model RznMono 11 tertiary, tokens 11, cost RznMono 13 right-aligned (min 72). (R-layout-3)
- Wane — Behaviour: Static in the mocks. Expand and collapse, selection and keyboard navigation come with the catalogue stub's real requirements. (R-layout-3)

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Controls/TreeGuideControl.cs`
- `src/Lunegit.Desktop/ViewModels/SidePanelRow.cs`
- `src/Lunegit.Desktop/Views/SidePanels/BranchesPanelView.axaml`

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit `src/Lunegit.Desktop/Controls/TreeGuideControl.cs:1-30,195-205`;
`src/Lunegit.Desktop/ViewModels/SidePanelRow.cs:12-53`; `src/Lunegit.Desktop/Themes/DarkTheme.axaml:101`;
`src/Lunegit.Desktop/STRUCTURE.md:290` (branches panel uses the guide gutter); `src/Lunegit.Desktop/Themes/Animations.axaml:125-140`
(side-panel row hover; no expand selector: `grep -n -i expand` empty). Wane `docs/ui/components.md:263-276`.
Conflicts (winner):
- Guide x: Wane "guide at x 16" [W docs/ui/components.md:263-276] vs the Lunegit guide drawn at the column centre, 8 px ([L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:19] `ColumnCenter = 8`) inside each 16 px indent. Lunegit wins: it is the source the guide drawing comes from (PD6).
- Guide colour: Wane `--border` (no value in the mock text); Lunegit `TreeGuideLine` #30FFFFFF [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:101]; the repo `border` token is #FFFFFF30 [R crates/lune-ui-core/src/theme.rs:62]. All three use alpha 0x30; the repo token is used.
- No moved code: the Lunegit draw is the only source of the guide shape.

### Anatomy

- Row: 28 high; radius 6; left padding 8 + depth × 16 [W docs/ui/components.md:263-276].
- Indent: 16 per depth [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:18] (`IndentWidth`).
- Guide: 1 px, round caps and joins [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:20,201]; trunk at column centre 8 [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:19].
- Elbow: 8 wide × 14 high with radius 3.5 [W docs/ui/components.md:263-276], [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:21].
- Has-next: the trunk continues to the next sibling [W docs/ui/components.md:263-276].
- Parts: name 13; model RznMono 11 tertiary; tokens 11; cost RznMono 13 right-aligned, min 72 [W docs/ui/components.md:263-276].

### States

| state | look | cite |
|---|---|---|
| rest | no row fill | [W docs/ui/components.md:263-276] |
| hover | fill `surface_hover` #FFFFFF10 | [W docs/ui/components.md:263-276], [R crates/lune-ui-core/src/theme.rs:60] |
| expanded (container) | children shown; a per-node flag in Lunegit | [L src/Lunegit.Desktop/ViewModels/SidePanelRow.cs:29-41] |
| collapsed (container) | children hidden | [L src/Lunegit.Desktop/ViewModels/SidePanelRow.cs:29-41] |
| selected | not in the sources read; open question | open question |
| focus-visible | not in the sources read; open question | open question |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| expand or collapse | container height (children enter or leave) | expand | base | sine-in-out | snap | [D40], [L .claude/docs/animation-rules.md:84] |
| rest ↔ hover | row fill | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:125-130] |

Expand animates the container, never the rows: rows are not animated per row (motion.md list, [L .claude/docs/animation-rules.md:145]).
Lunegit has no expand animation (no expander selector); `base` and `sine-in-out` are the toolkit's choice for a
two-sided resize [L .claude/docs/animation-rules.md:84] and the duration has no source (Open questions).
Never animated: the per-row expand, virtualized rows during scroll [L .claude/docs/animation-rules.md:145].

### Input and focus

- Pointer: click a row selects it; a click on the expand toggle expands or collapses [derived: Lunegit branch folders are collapsible, STRUCTURE.md:290]. Hover fills the row [W docs/ui/components.md:263-276].
- Keyboard: Up and Down move, Right expands, Left collapses: not in any source; open question.
- Tab stop: the tree is one stop; rows are reached by arrow keys [core.focus] (derived; see Open questions).
- Layer: Base or Page, whatever the owner draws it on [derived: a container].
- Focus scope: none.
- Pointer capture while pressed: none in the sources read.

### Retained state

- Transient: row hover; the expand animation progress [motion].
- Persistent: the expansion as a set of node keys; the selection as a set of row keys; owned by the container; survives under `ui.keep_alive` [core.ui].
- App-owned: the node tree (keys, depth, children, the has-next flag) passed each frame [L src/Lunegit.Desktop/ViewModels/SidePanelRow.cs:16,48-53].
- Exit retention: no.

### Accessibility

- Role `tree`; rows `tree-item` with states `expanded` or `collapsed`, `selected`, `focusable`; the depth comes from the tree level [infra.accessibility].
- Name = the row's name text; value = none; the cost cell is `cell` [infra.accessibility].
- Actions: expand, collapse, default (select), scroll-into-view [infra.accessibility].
- Live region: none.

### Strings

None: row names and costs are app text.

### Gallery scenes

- `tree-view-depth` (new): three depths with guides and has-next, settled.
- `tree-view-hover` (new): one row hovered, settled.
- `tree-view-expanded` (new): one folder expanded, settled.
- `tree-view-expand-mid` (new): expand at t = 125 ms (half of base, 250 ms) [L src/Lunegit.Desktop/App.axaml:153].

### Painter primitives

- Existing: fill, stroke, text. [R crates/lune-ui-core/src/paint.rs:215,329,274]
- New: none.
- The elbow uses the existing `stroke` with a round join.

### Acceptance

- Behaviour (scripted input, injected time): Up, Down, Right and Left move and toggle; expanding stores the key in the set and survives a rebuild; a click on a row selects it once. None exists.
- Pixel probes at scale 2: row 56 px high; depth step 32 px; guide 2 px wide; elbow radius 7 px.
- Conformance: expand reaches full height at 250 ms (base) on the container; rows do not animate.
- Semantic node: role `tree`; each row `tree-item` with `expanded` and its level.

### Build

- Milestone: M7.
- Depends on: core.painter, core.ui, core.focus, infra.accessibility, motion.transitions.

## Open questions

- Expand duration and easing: no source; the table's `base` and `sine-in-out` are a [D40] choice; open for the planner.
- Keyboard model (arrows, type-ahead) and the Tab stop: not in any source; open.
- Selection state look: not in any source; open.
- Wane's "guide at x 16" against Lunegit's column centre 8: the Lunegit value wins here; the planner confirms the cell of the Wane mock.
- Whether the rows of the branch panel use `side-panel-item` for hover (Animations.axaml:125-130): inferred, not verified; open.

## Notes for the planner

The stub is answered: `TreeGuideControl` is the guide, and there is no `TreeView` selector in Lunegit (grep empty). Wane priority P1. Native note: guide lines are painter strokes; no CSS-only effect. Full spec: Wane `docs/ui/components.md`, section `layout.tree-view`; mock: Wane `docs/ui/mocks/components.html`.
