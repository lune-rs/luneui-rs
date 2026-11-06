---
id: layout.side-panel
title: Side panel host and panels
area: layout
status: done
consumers: lunegit
source: crates/lune-ui-core/src/widgets/side_panel.rs
owner: —
summary: Side panel host that shows one of nine panels by key, with a fade and 6 px slide on show
---

## What it is

The side-panel host: an acrylic column of nine panels, one shown at a time by the active panel key [L src/Lunegit.Desktop/Views/SidePanelView.axaml:9-53]. Repository, branches, commit, file explorer, scratchpad, help, settings, notification history and AI sessions are the nine; their contents are app code (Git UI). The toolkit gets the host chrome and the show motion; the panel contents are out of scope.

## Needs

- Host that switches panels (R-layout-1).
- Row lists inside panels (R-layout-3).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/SidePanelView.axaml`
- `src/Lunegit.Desktop/Views/SidePanels/RepositoryPanelView.axaml`
- `src/Lunegit.Desktop/Views/SidePanels/BranchesPanelView.axaml`
- `src/Lunegit.Desktop/Views/SidePanels/CommitPanelView.axaml`
- `src/Lunegit.Desktop/Views/SidePanels/FileExplorerPanelView.axaml`

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/Views/SidePanelView.axaml:1-58` (host, nine panels by key);
`src/Lunegit.Desktop/Themes/Animations.axaml:314-335` (`.side-panel-anim`, D2); `src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:160-163`
(Reduced drops every transform); `.claude/docs/animation-rules.md:108` (side panel / drawer row).
Conflicts: none between sources. Wane has no side-panel entry; this repo has no side-panel code.

### Anatomy

- Host: acrylic card at radius 8 while the window is active [L src/Lunegit.Desktop/Views/SidePanelView.axaml:9]; solid `WindowFallbackBackground` while inactive [L src/Lunegit.Desktop/Views/SidePanelView.axaml:19].
- Inner: a `glass-panel` border with padding 10 [L src/Lunegit.Desktop/Views/SidePanelView.axaml:20]; the border is 1 px `SurfaceBorder` (`border`) at radius 8 [L src/Lunegit.Desktop/App.axaml:294-298], drawn inside the host's rect, so the content starts `RING` 1 + `PAD` 10 = 11 in (C21).
- Panels: nine views, one visible; visibility = `ActivePanelKey` equals the panel's key [L src/Lunegit.Desktop/Views/SidePanelView.axaml:22-53].
- The column width is set by the window, not the host; it snaps [L src/Lunegit.Desktop/Themes/Animations.axaml:314-317].

### States

| state | look | cite |
|---|---|---|
| active window | acrylic host, radius 8 | [L src/Lunegit.Desktop/Views/SidePanelView.axaml:9] |
| inactive window | solid fallback fill | [L src/Lunegit.Desktop/Views/SidePanelView.axaml:19] |
| panel shown | the keyed panel visible, with the show motion | [L src/Lunegit.Desktop/Views/SidePanelView.axaml:22-53], [L src/Lunegit.Desktop/Themes/Animations.axaml:318-335] |
| panel hidden | not drawn | [L src/Lunegit.Desktop/Views/SidePanelView.axaml:22-53] |
| focus-visible | the focused control in the panel | [core.focus] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| panel shown | opacity 0→1 | drawer | moderate | quad-out | fade | [L src/Lunegit.Desktop/Themes/Animations.axaml:318-335], [L .claude/docs/animation-rules.md:108] |
| panel shown | translateX −6→0 px | drawer | moderate | sine-in-out | fade (transform dropped under Reduced) | [L src/Lunegit.Desktop/Themes/Animations.axaml:316,318-335], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:160-163] |

n/a for the column width: it snaps, because it drives the splitter [L src/Lunegit.Desktop/Themes/Animations.axaml:314-317]. Never animated: the panel contents' rows [L .claude/docs/animation-rules.md:145].
Panel hide: no motion in the source (the keyed visibility switches at once).

### Input and focus

- Pointer: none on the host; the panels take their own input.
- Keyboard: none on the host; panel switching is by the icon rail (layout.icon-rail).
- Tab stop: none on the host; the panel's controls are the stops [core.focus].
- Layer: Base, in the content row beside the page [R crates/lune-ui-core/src/scene.rs:17-22] [derived].
- Focus scope: none on the host; a show moves focus to the panel's first control: open question.
- Pointer capture while pressed: none.

### Retained state

- Transient: the show motion progress [motion].
- Persistent: none on the host; each panel keeps its own state (owned by that panel).
- App-owned: `ActivePanelKey`, the panel contents [L src/Lunegit.Desktop/Views/SidePanelView.axaml:22-53].
- Exit retention: no.

### Accessibility

- Host: role `group`, name = the active panel's title (app text) [infra.accessibility].
- Panel switch: announced through the rail's button states (layout.icon-rail); no live region here.
- Live region: none.

### Strings

None: the host draws no text; panel titles are app text.

### Gallery scenes

- `side-panel-shown` (new): one panel (a row list) shown, settled.
- `side-panel-inactive` (new): the host with the window inactive, settled.
- `side-panel-show-mid` (new): show at t = 175 ms after the first frame of the new panel (half of moderate, 350 ms) [L src/Lunegit.Desktop/App.axaml:154].

### Painter primitives

- Existing: acrylic, fill, text, xform (translate for the −6 px slide). [R crates/lune-ui-core/src/paint.rs:243,215,274,171]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): changing the active key shows exactly one panel; the hidden panels draw nothing; the inactive window draws the fallback fill.
- Pixel probes at scale 2: host radius 16 px; ring 2 px in `border`; content 22 px in; slide 12 px at scale 2 (−6 px) at the start of the show.
- Conformance: show reaches opacity 1 and offset 0 at 350 ms; Reduced fades opacity only; Off snaps.
- Semantic node: `group` named by the panel title; hidden panels are not in the tree.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_side_panel.rs`: behaviour `exactly_one_panels_nodes_exist_and_hidden_panels_emit_none`, `a_hidden_panels_scroll_offset_and_text_edit_survive_600_undrawn_frames`, `a_hide_snaps_the_old_panel_is_gone_on_the_first_frame_of_the_change`, `a_show_moves_no_focus`, `inactive_draws_the_fallback_fill_and_no_acrylic`; pixel probes `pixel_probes_at_scale_two`, `the_card_is_the_acrylic_at_radius_8_with_the_given_under_and_content_inside_ring_and_10`; conformance `show_conforms_in_three_tiers`, `reduced_fades_the_opacity_only_and_off_snaps`, `a_show_fades_the_panel_in_and_slides_it_from_the_left_and_leaves_the_card`, `the_first_frame_of_a_host_does_not_animate`; node `the_tree_of_a_frame_has_the_host_group_first`; `a_disabled_host_is_disabled_all_the_way_down_and_consumes_no_key`.

### Build

- Milestone: M7.
- Depends on: core.acrylic, core.painter, layout.icon-rail, layout.panel, motion.transitions.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_side_panel.rs::exactly_one_panels_nodes_exist_and_hidden_panels_emit_none, crates/lune-ui-core/tests/widget_side_panel.rs::a_hidden_panels_scroll_offset_and_text_edit_survive_600_undrawn_frames, crates/lune-ui-core/tests/widget_side_panel.rs::pixel_probes_at_scale_two, crates/lune-ui-core/tests/widget_side_panel.rs::inactive_draws_the_fallback_fill_and_no_acrylic, crates/lune-ui-core/tests/widget_side_panel.rs::show_conforms_in_three_tiers, crates/lune-ui-core/tests/widget_side_panel.rs::the_tree_of_a_frame_has_the_host_group_first`; page `side-panel`; unverified here: none.

## Open questions

None. Closed by [D149]: the host is a toolkit component (`Ui::side_panel`, `SidePanelOptions { panel, keys, active, glass, under, name }`); the panels' contents are the app's. The host has no width of its own (the app passes the rect). A show moves no focus: the app knows the first control. `keys` lists every panel so hidden ones stay alive (`keep_alive` for the shown one, `mark_alive` for the rest): a hidden panel's scroll offset and text edit survive any number of undrawn frames. The show is `SHOW_OPACITY` (kept under Reduced) and `SHOW_SLIDE` (-6 to 0 px, dropped under Reduced) from the frame `panel` changes; the first frame does not animate; a show in flight restarts from the look it has; a hide snaps. Keyboard switching is the icon rail's (`layout.icon-rail`). The content is inset 10, above the radius of 8 (a rounded clip cannot cut text, [D148]).

## Notes for the planner

Only the host and the show motion are toolkit scope. Mostly Git-specific content: the panels themselves stay app code.
