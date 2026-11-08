---
id: overlay.tooltip
title: Tooltips
area: overlay
status: done
consumers: lunegit, wane
source: crates/lune-ui-core/src/widgets/tooltip.rs
owner: —
summary: Hover tooltip: glass shell radius 8, padding 10 by 8, one 800 ms show delay, text or small rich content, also on disabled controls
---

## What it is

A small glass panel that names or explains the control under the pointer after a delay: the full
title of a truncated tab, the exact value of a chart bar, the name of an icon-only button, the state
behind a coloured dot. It is a `overlay.popover` with a fixed shell, a show delay and no input of its
own. Lunegit's tooltips are Avalonia's `ToolTip` with one shared glass style: 108 `ToolTip.Tip`
sites, one explicit delay.

## Needs

- Tooltips and flyouts (R-overlay-3).
- Placement, flip, shift and dismiss from the popover (R-overlay-4).
- Wane — Tooltips: truncated ids, the bar chart values, the tray icon state and icon-only buttons need `overlay.tooltip` (R-overlay-3).

## Existing code

Built in M5 (D102): `crates/lune-ui-core/src/widgets/tooltip.rs` (`Ui::tooltip`, `tooltip_with`, `hover_dwell`). Design source (not code to move): Lunegit `src/Lunegit.Desktop/App.axaml:700-717` (the style), `Views/TerminalPanelView.axaml:263-264` (the one explicit delay),
`Controls/LazyHoverHost.cs` (a hover dwell that is not a tooltip), `Views/AiProjectDetailView.axaml:88-116` (disabled-control tooltips).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/App.axaml:696-720, Views/TerminalPanelView.axaml:259-266,329-334,350, Controls/LazyHoverHost.cs:1-120,
Views/AiProjectDetailView.axaml:88-118, Views/RepositoryContentView.axaml:396-424, Themes/Animations.axaml and Themes/AnimationsReduced.axaml (no `ToolTip` selector in either: `grep -c ToolTip` gives 0 and 0);
greps over `src/Lunegit.Desktop/**/*.axaml`: `ToolTip.Tip=` on 108 lines, `ShowDelay` on one line (counted 2026-11-04, `[derived: grep count]`); Wane docs/ui/components.md:213,647,726,756-758.
Conflict, the two numbers: the 150 ms in `LazyHoverHost` is a dwell before a hover branch label is built on a commit row, a filter for rows that flick under a parked cursor while a list scrolls [L src/Lunegit.Desktop/Controls/LazyHoverHost.cs:16-22,61]; it is not a tooltip delay.
The 800 ms is the only explicit tooltip delay, on the terminal tab tooltip [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:263-264]. The roadmap's C5 line "dwell 150 ms, delay token" joins the two. Resolution ([D57]): the tooltip `show_delay` is 800 ms of continuous hover
(`Theme` motion token, app-overridable); the 150 ms dwell is the minimum continuous hover for lazily built hover content and is subsumed by the 800 ms for tooltips. Every other Lunegit tooltip uses Avalonia's default delay, which is not stated in the Lunegit source.

### Anatomy

- Shell: Lunegit's is radius 8, clipped, fill `BackdropDark`, 1 px `SurfaceBorder` border, content padding 10 horizontal and 8 vertical [L src/Lunegit.Desktop/App.axaml:700-717]. The toolkit draws the one glass surface instead (`glass_surface` at radius 9: the backdrop, `menu_glass` frost, the `menu_popup_border` ring, D8), with that padding.
- Content: plain text (the tab title, a button name) or a small stack: a bold line, a secondary line, an optional separated line; Lunegit's icon row is not reproduced, a tooltip being inert (anything clickable belongs in a popover), spaced 4 [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:401-422], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:263].
- Placement: below the anchor, flipped and shifted inside the window by `overlay.popover` [overlay.popover]; Lunegit leaves placement to Avalonia (not in the files read).
- Used for: a truncated tab title [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:263], the name of an icon-only header button [L src/Lunegit.Desktop/Views/AiProjectDetailView.axaml:92,109,114], the exact bar-chart value, truncated ids, the tray icon state in words [W docs/ui/components.md:647,726,756-758].
- Disabled anchors still show the tooltip: Lunegit sets `ShowOnDisabled` so a disabled icon-only button is not an unexplained glyph [L src/Lunegit.Desktop/Views/AiProjectDetailView.axaml:93,110,114-116].
- The tooltip never takes input: it is not hit-testable [derived: a tooltip that covered its anchor would flicker; no source states it].

### States

| state | look | cite |
|---|---|---|
| hidden | nothing drawn | [core.ui] |
| pending | the pointer is over the anchor, the delay has not elapsed; nothing drawn | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:264] |
| shown | the glass shell with its content | [L src/Lunegit.Desktop/App.axaml:700-717] |
| shown on a disabled anchor | same | [L src/Lunegit.Desktop/Views/AiProjectDetailView.axaml:93,110] |
| flipped or shifted | the popover's placement when it does not fit | [overlay.popover] |

Hover, pressed, focus-visible and disabled do not apply to the tooltip itself (it takes no input); they belong to the anchor.

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| appear | opacity | appear | fast | quad-out | keep | [D40], [L .claude/docs/animation-rules.md:105] |
| dismiss | opacity | dismiss | instant | quad-in | keep | [D40], [L .claude/docs/animation-rules.md:106] |

Lunegit's tooltip has no transition in any tier: neither tier file has a `ToolTip` selector (Evidence), so the fade is a [D40] addition from the appear and dismiss rows of the animation rules. The delay is not an animation. Never animated: the position while the anchor moves, the content.

### Input and focus

- Pointer: hover over the anchor for `show_delay` starts it; leaving the anchor starts the dismiss at once (teardown is never delayed, only the build; the 120 ms dismiss fade then runs: the same shape as `LazyHoverHost`) [L src/Lunegit.Desktop/Controls/LazyHoverHost.cs:9-14]. A press on the anchor hides it [derived: the Avalonia default is not read].
- Keys: the tooltip owns none; Escape dismisses it as the topmost overlay [core.ui]. Keyboard focus on the anchor shows it after the same delay [derived: keyboard users have no hover; no source states it; Open questions].
- Tab stop: no. Layer: `Popup`. No focus scope: a tooltip never takes or moves focus [core.focus].
- A hover that begins while a list scrolls restarts the timer for each new anchor, so a row flicking under a parked cursor never shows one [L src/Lunegit.Desktop/Controls/LazyHoverHost.cs:16-22].

### Retained state

- Transient: the hover start time for the anchor id, shown or not, the fade phase; dropped when the anchor is no longer called, which also cancels a pending tooltip [core.ui].
- Persistent: none. App-owned: the content, the delay override.
- Exit retention: yes: Full 120 / Reduced 120 / Off 0 ms [core.ui], [overlay.popover].

### Accessibility

- Role `tooltip`; name = the tooltip text; no states beyond `hidden`; the anchor's description is the tooltip text, and an icon-only anchor also carries it as its name, as Lunegit pairs `ToolTip.Tip` with `AutomationProperties.Name` [L src/Lunegit.Desktop/Views/AiProjectDetailView.axaml:92-94]; actions: dismiss [infra.accessibility].
- Not a live region.

### Strings

None: the text is supplied by the app (Lunegit's are app strings such as `terminal.tab.close.tooltip`, [L src/Lunegit.Infrastructure/Localization/en.json:781]).

### Gallery scenes

- `tooltip-text`: a one-line tooltip under an icon button, settled.
- `tooltip-rich`: a bold line, a secondary line, a separator and a line of text, settled. No icons or buttons: a tooltip is inert (D102), so anything clickable belongs in a [popover](popover.md) (R11, [D274]).
- `tooltip-flipped`: the anchor near the bottom edge, tooltip above, settled.
- `tooltip-shifted`: the anchor near the right edge, tooltip shifted inside, settled.
- `tooltip-disabled-anchor`: a disabled icon button with its tooltip, settled.
- `tooltip-pending`: the pointer 700 ms over the anchor, nothing drawn, t = 700 ms.
- `tooltip-appearing`: t = 900 ms (800 ms delay plus half of `fast`).
- `tooltip-dismissing`: pointer left, t = 60 ms (half of `instant`).

### Painter primitives

- Existing: fill-radii, ring, backdrop, text, icon, clip-rect, fill-over.
- New: none.

### Acceptance

- Behaviour (scripted pointer, injected time): hover for 799 ms draws nothing; 800 ms draws the tooltip; leaving starts the dismiss at once (fully hidden after 120 ms); moving to another anchor restarts the timer; a hover on an anchor that stops being called never shows one; a disabled anchor shows it; scrolling a list under a parked pointer shows none [core.ui].
- Placement: the tooltip stays inside the window with an 8 px margin (the popover acceptance) [overlay.popover].
- Pixel probes at scale 2: radius 16 device px, padding 20 px horizontal and 16 px vertical around the text, 2 px border.
- Conformance: appear and dismiss pass `motion::conformance`; Reduced keeps the fade, Off snaps [motion].
- Semantic node: role `tooltip` with the text while shown and no node while hidden; the anchor's description equals the text.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_tooltip.rs`: behaviour `the_delay_is_800_ms_of_hover`, `leaving_starts_the_dismiss_at_once_and_it_is_hidden_after_120_ms`, `moving_to_another_anchor_restarts_the_timer`, `an_anchor_the_app_stops_calling_never_shows_and_calling_again_restarts`, `a_disabled_anchor_shows_it`, `a_list_scrolling_under_a_parked_pointer_shows_none`, `a_focus_visible_anchor_shows_it_after_the_same_delay`; placement `it_stays_inside_the_window_flipped_and_shifted`; pixel probes `pixel_probes_at_scale_2`, `long_text_wraps_at_320`; conformance `the_fade_is_opacity_only_reduced_keeps_it_and_off_snaps`, `leaving_off_tier_hides_at_once`; node `the_semantic_nodes_follow_the_tooltip`.

### Build

- Milestone: M5. Built in M5 (steps 9 and 10; the page is `tooltip`, 8 scenes). A tooltip over a dialog draws on the `Raised` layer, chosen once at open [D137]; a third level that overlaps its parent shares `Raised` and interleaves (fills under text). M11 audited it against D47 (the `Done` line). Unverified here: the Windows and Linux run time (the lint for both is `scripts/check-targets.sh`).
- Depends on: overlay.popover, core.ui, core.focus, infra.accessibility, motion.transitions
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_tooltip.rs::the_delay_is_800_ms_of_hover, crates/lune-ui-core/tests/widget_tooltip.rs::leaving_starts_the_dismiss_at_once_and_it_is_hidden_after_120_ms, crates/lune-ui-core/tests/widget_tooltip.rs::a_disabled_anchor_shows_it, crates/lune-ui-core/tests/widget_tooltip.rs::a_list_scrolling_under_a_parked_pointer_shows_none, crates/lune-ui-core/tests/widget_tooltip.rs::it_stays_inside_the_window_flipped_and_shifted, crates/lune-ui-core/tests/widget_tooltip.rs::pixel_probes_at_scale_2, crates/lune-ui-core/tests/widget_tooltip.rs::the_fade_is_opacity_only_reduced_keeps_it_and_off_snaps, crates/lune-ui-core/tests/widget_tooltip.rs::the_semantic_nodes_follow_the_tooltip`; page `tooltip`; unverified here: Windows and Linux run time (limit with a row: one level above a modal, D137).

## Open questions

- Answered (D102, yes): show on keyboard focus: no source states it. Wane's accessibility need (the tray tooltip is the second channel for colour vision [W docs/ui/components.md:756]) argues for it; M5 decides.
- Answered (D102, `font_s`, 320): text size and maximum width: Lunegit's style sets neither (the content presenter inherits the text defaults [L src/Lunegit.Desktop/App.axaml:710-713]); M5 picks from the theme's `font_s` and `font_base`.
- Answered (D102, no skip-delay): whether a second tooltip shown soon after the first skips the delay (Avalonia has a between-show delay): not in the files read.

## Notes for the planner

Replaces the stub's question "does Lunegit have tooltips at all": it has 108 sites and one shared glass style. `LazyHoverHost` is not a tooltip.
