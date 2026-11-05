---
id: layout.tree-view
title: Tree view and tree guides
area: layout
status: in-progress
consumers: lunegit, wane
source: Lunegit: src/Lunegit.Desktop/Controls/TreeGuideControl.cs
owner: toolkit-v1 M7
summary: Tree rows with guide lines (trunk and rounded elbow) per depth; expansion is a set of node keys, animated on the container
---

## What it is

Tree rows with guide lines. `TreeGuideControl` draws the trunk and rounded elbow per row, in a 16 px indent per depth [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:15-21]. Expansion is a per-row flag in Lunegit's view model [L src/Lunegit.Desktop/ViewModels/SidePanelRow.cs:29-41]; the toolkit keeps expansion as a set of node keys (plan contract, Retained state).

`Ui::tree_view(key, rect, &TreeOptions { rows, label, guides, toggle_on_row }, cells) -> TreeResponse { response, activated, toggled, context }` [D151]: the app passes pre-order `TreeRow { key, depth, text, glyph, has_children }`; the toolkit derives what is visible, last-child and the ancestor lines in one walk per frame and builds only the rows in view. `cells(ui, &TreeRowCtx)` draws the right side of a row (it is not called for a row in a moving block, and the text is cut at the row's edge, so a cell that needs room gets it from short names). `Ui::tree_state(key)` is the retained `TreeState { expanded, selected, cursor }`, with `retain`.

## Needs

- Tree rows with guide lines (R-layout-3).
- Wane — Variants: child (`__row--child`, depth 1; indent 16 per depth through `--depth`); has-next continues the guide to the next sibling. (R-layout-3)
- Wane — States: rest; hover (`--surface-hover`). Not expandable or collapsible in the mocks. (R-layout-3)
- Wane — Sizes: Rows 28, radius 6, padding-left 8 + depth x 16; guide at x 16, 1 px `--border`; elbow 8 x 14 with radius 3.5 (`--radius-elbow`); parts: name 13, model RznMono 11 tertiary, tokens 11, cost RznMono 13 right-aligned (min 72). (R-layout-3)
- Wane — Behaviour: Static in the mocks. Expand and collapse, selection and keyboard navigation come with the catalogue stub's real requirements. (R-layout-3)

## Existing code

`crates/lune-ui-core/src/widgets/tree_view.rs` (`Ui::tree_view`, `Ui::tree_state`, `TreeRow`, `TreeOptions`, `TreeRowCtx`, `TreeResponse`, `TreeState`, `guide_paths`), on `Ui::scroll_area_untranslated` (rows are placed in `f64`, as in the virtual list). Tests: `crates/lune-ui-core/tests/widget_tree_view.rs`. Page: `apps/gallery/src/pages/tree_view.rs`.

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
- Guide: 1 px `border`, round caps and joins [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:20,201]; trunk at column centre 8 [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:19]. From the row's left edge: `elbowX = floor((depth-1)*16 + 8) + 0.5`, `stubRight = depth*16`, `cy = h/2`, `r = min(3.5, (stubRight - elbowX)/2, cy)`; an ancestor column k at `floor(k*16 + 8) + 0.5`, full height [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:136-190]. The painter's stroke has square ends: at a row boundary two ends meet with no overlap, and the stub's free end gets its round cap as a half disc past it. Lunegit strokes all of a row's lines as one geometry so the translucent colour does not double at the junction; the painter has no union, so the branch is drawn from the trunk's edge instead [R crates/lune-ui-core/src/widgets/tree_view.rs `clear_of_trunk`]. `guide_paths` gives the lines of the formulas.
- Elbow: 8 wide × 14 high with radius 3.5 [W docs/ui/components.md:263-276], [L src/Lunegit.Desktop/Controls/TreeGuideControl.cs:21], Lunegit's quadratic curve.
- Chevron slot 14, on every row when any row has children, after the left padding, then 4; `icon::CHEVRON_RIGHT` or `CHEVRON_DOWN` in `fg_tertiary`; a section header's chevron is 14 [L src/Lunegit.Desktop/Views/SidePanels/BranchesPanelView.axaml:149-151]. Glyph 14 in `fg_secondary`, then 6. Text `font_base`, `fg_primary` (`fg_bright` selected). The gaps have no source; the visual batch may change `CHEVRON_GAP` and `GLYPH_GAP`.
- Has-next: the trunk continues to the next sibling [W docs/ui/components.md:263-276].
- Parts: name 13; model RznMono 11 tertiary; tokens 11; cost RznMono 13 right-aligned, min 72 [W docs/ui/components.md:263-276].

### States

| state | look | cite |
|---|---|---|
| rest | no row fill | [W docs/ui/components.md:263-276] |
| hover | fill `surface_hover` #FFFFFF10 | [W docs/ui/components.md:263-276], [R crates/lune-ui-core/src/theme.rs:60] |
| expanded (container) | children shown; a per-node flag in Lunegit | [L src/Lunegit.Desktop/ViewModels/SidePanelRow.cs:29-41] |
| collapsed (container) | children hidden | [L src/Lunegit.Desktop/ViewModels/SidePanelRow.cs:29-41] |
| selected | fill `surface_selected` #FFFFFF18; hovered `surface_active` #FFFFFF20 | [D151], Lunegit's selected and selected hover |
| focus-visible | the 1 px `fg_secondary` ring at radius 6 on the cursor's row; the tree holds the focus, the cursor marks the row | [D151], [core.focus] |
| disabled | the scope's look; no toggle, no select, the keys taken; every node `Disabled` | [D140] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| expand or collapse | container height (children enter or leave) | expand | base | sine-in-out | snap | [D40], [L .claude/docs/animation-rules.md:84] |
| rest ↔ hover | row fill | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:125-130] |
| selection change | row fill | selection | fast | linear | keep | as the list |

Expand animates the container, never the rows: rows are not animated per row (motion.md list, [L .claude/docs/animation-rules.md:145]).
Lunegit has no expand animation (no expander selector); `base` and `sine-in-out` are the toolkit's choice for a
two-sided resize [L .claude/docs/animation-rules.md:84], the kind's default [motion.md, tree expand row].

The block is the rows under the node, each 28 high: they draw under a clip of `f` times the block's height, with no hit regions, no nodes and no `cells`; the rows below sit `(1 - f)` of its height above their place, and the extent follows. One key changed in `expanded` between two frames starts a block, whoever changed it (a key, a press, an accessibility action, the app); a change of the moving key retargets from where it is; any other change snaps it, and two keys at once snap. A key's block starts in its frame; a press or an action lands after that frame's walk, so its block starts on the next frame (D81's catch-up). Row fills snap while the content moves or a block is in flight (the list's still-frame rule).
Never animated: the per-row expand, virtualized rows during scroll [L .claude/docs/animation-rules.md:145].

### Input and focus

- Pointer: a press on a row selects it and moves the cursor; a press on the chevron toggles and does neither; `toggle_on_row` toggles from anywhere on a row with children, as Lunegit's folder rows do [D151]. A double click on the row activates; its second press does not toggle (with `toggle_on_row` its first has), and two quick presses on a chevron are two toggles. A secondary click reports `context`. Hover fills the row [W docs/ui/components.md:263-276].
- Keyboard [D151]: Up, Down, Home, End, PageUp and PageDown (by the rows the viewport holds) move the cursor, no wrap; Right expands, or moves to the first child; Left collapses, or moves to the parent; Enter activates; Space selects. Arrows select nothing. On a top row that is closed or a leaf, Left does nothing and is consumed. No type-ahead (no source). The cursor stays in view (nearest), also when it stands in a block still opening.
- Cursor [D151]: `None` until the tree first holds the focus (it then takes the selected row, else the first in view) or a row is pressed. It is always on a row it can reach: a row hidden or removed gives it to the nearest visible ancestor, by the path it remembers if the row left `rows`, else to its last index. Rows of a closing block are out of reach from the toggle on.
- Tab stop: the tree is one stop; rows hold no focus, as in the virtual list [core.focus].
- Layer: Base or Page, whatever the owner draws it on [derived: a container].
- Focus scope: none.
- Pointer capture while pressed: none in the sources read.

### Retained state

- Transient: row hover; the expand animation progress [motion].
- Persistent: the expansion as a set of node keys; the selection as a set of at most one row key; the cursor's key; owned by the container; survives under `ui.keep_alive` [core.ui]. A key that left the rows stays until `TreeState::retain`.
- App-owned: the node tree (keys, depth, children, the has-next flag) passed each frame [L src/Lunegit.Desktop/ViewModels/SidePanelRow.cs:16,48-53].
- Exit retention: no.

### Accessibility

- Role `tree` named `label`, with the focus action; rows `tree-item` with `Node.level` = depth + 1, states `expanded` on an open row and `selected` [infra.accessibility]. Rows are not focusable: the tree is the stop.
- Name = the row's name text; value = none; what `cells` adds hangs under the row's node [infra.accessibility].
- Actions: expand on a closed row with children, collapse on an open one, default (select, and toggle under `toggle_on_row`), scroll-into-view (centred) [infra.accessibility].
- Live region: none.

### Strings

None: row names and costs are app text.

### Gallery scenes

- `tree-view-depth` (new, at 2x and 1x): three depths with guides and has-next, settled.
- `tree-view-hover` (new): one row hovered, settled.
- `tree-view-expanded` (new): one folder expanded, settled.
- `tree-view-expand-mid` (new): expand at t = 125 ms (half of base, 250 ms) [L src/Lunegit.Desktop/App.axaml:153].
- `tree-view-selected` (new): one row selected, settled.
- `tree-view-focus` (new): the ring on the cursor's row, settled.

### Painter primitives

- Existing: fill, fill_radii, stroke, text. [R crates/lune-ui-core/src/paint.rs]
- New: none.
- The elbow is a polyline through `stroke`; the round cap is a `fill_radii` half disc.

### Acceptance

- Behaviour (scripted input, injected time): Up, Down, Right and Left move and toggle; expanding stores the key in the set and survives a rebuild; a click on a row selects it once. All in `crates/lune-ui-core/tests/widget_tree_view.rs`.
- Pixel probes at scale 2: row 56 px high; depth step 32 px; guide 2 px wide; elbow radius 7 px.
- Conformance: expand reaches full height at 250 ms (base) on the container; rows do not animate.
- Semantic node: role `tree`; each row `tree-item` with `expanded` and its level.

### Build

- Milestone: M7.
- Depends on: core.painter, core.ui, core.focus, infra.accessibility, motion.transitions.

## Open questions

None open. Answered by [D151] and `docs/motion.md`: the expand's duration and easing, the keyboard model and the Tab stop, the selection look, and Wane's "guide at x 16" against Lunegit's column centre 8 (Lunegit wins; the guide's origin is the row's left edge). The side-panel hover class on the branch rows stays unverified and changes nothing here: the row fill is the kind's `hover`, instant. `Reduced` and `Off` report `Idle` on the frame an app's write shows; a key's or a click's frame also asks D81's one catch-up, with nothing in flight.

## Notes for the planner

The stub is answered: `TreeGuideControl` is the guide, and there is no `TreeView` selector in Lunegit (grep empty). Wane priority P1. Native note: guide lines are painter strokes; no CSS-only effect. Full spec: Wane `docs/ui/components.md`, section `layout.tree-view`; mock: Wane `docs/ui/mocks/components.html`.
