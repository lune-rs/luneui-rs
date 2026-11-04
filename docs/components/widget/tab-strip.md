---
id: widget.tab-strip
title: Tab strip
area: widget
status: in-progress
consumers: luneterm, lunegit
source: —
owner: toolkit-v1 M6
summary: Tab strip: tabs with close, new-tab button, overflow scrolling, drag reorder, and the connected tab outline with a running cue
---

## What it is

A 32 px tab strip over a card: tabs of 108 to 270 px, a close button shown on the active and hovered
tab, a new-tab button, horizontal scrolling when tabs overflow, drag reorder, and one continuous 1 px
outline (`core.tab-outline`) that wraps the active tab and flows round the card body, so the tab and
its content read as one surface. A busy cue animates that outline while a command runs. Lunegit builds
it for terminal sessions (`TerminalPanelView`); Luneterm's M10 lists the same strip (new, close,
rename, switch, close-confirm). The strip is generic: the app owns what a tab means, its title, order,
rename and close policy.

## Needs

- Tab outline geometry as the visual base (R-core-7).
- Close-confirm uses the modal dialog (R-overlay-1); rename is a dialog with a text field in Lunegit (R-widget-1).
- Reorder through `infra.drag-and-drop` (R-infra-5); scrolling through `layout.scroll-area` (R-layout-2).
- A tab control as one of the controls Lunegit has that the toolkit lacks (R-widget-2).

## Existing code

The widget: `crates/lune-ui-core/src/widgets/tab_strip.rs` (`Ui::tab_strip`, `TabKey`, `Tab`, `TabStripOptions`, `TabEvent`, `BusyCue`, `CONTENT_SWAP`), tests `crates/lune-ui-core/tests/widget_tab_strip.rs`, page and scenes `apps/gallery/src/pages/tab_strip.rs`.
Geometry: `crates/lune-ui-core/src/chrome.rs:5-20` (constants and `outline`), tests `:172-226`; see `core.tab-outline`. The gallery `chrome` scene draws it (`apps/gallery/src/scenes.rs:173`).
Lunegit's strip: `src/Lunegit.Desktop/Views/TerminalPanelView.axaml:22-125,203-374` and `Views/TerminalPanelView.axaml.cs:255-480`, `Controls/Terminal/TerminalTabChrome.cs` (outline control),
`Controls/Terminal/TerminalChromeGeometry.cs`, `Controls/Terminal/TerminalChromeAnimation.cs`, `Controls/Terminal/TerminalTabDragController.cs`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/TerminalPanelView.axaml:17-125,196-374, Views/TerminalPanelView.axaml.cs:95-106,255-475,
Controls/Terminal/TerminalChromeGeometry.cs:30-60, Controls/Terminal/TerminalTabChrome.cs:195-356,475-512, Controls/Terminal/TerminalChromeAnimation.cs:12-34,
Controls/Terminal/TerminalTabDragController.cs:25-44, ViewModels/TerminalPanelViewModel.cs:100-120,574-612,656-659,705-734,829-834, src/Lunegit.Core/Settings/TerminalOutlineAnimationResolver.cs:1-35,
Themes/Animations.axaml:182-224, Themes/AnimationsReduced.axaml:119-150, .claude/docs/animation-rules.md:143-150,
src/Lunegit.Infrastructure/Localization/en.json:777-832; this repo crates/lune-ui-core/src/chrome.rs:1-20,172-226. Wane has no tab strip section.
Conflict: a comment in Lunegit's tab template says the tab "is capped at MaxWidth=180" [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:327] while the style sets `MaxWidth` 270 [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:60]; the setter wins (270), the comment is stale.

### Anatomy

- Header strip 32 px high; tabs stretch to the full 32 px (no vertical centring) [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:218,240-243]. Tabs sit in a horizontal stack with gap 5, between tabs only, so the first tab is flush in the card's top-left corner [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:252-256].
- Tab: min width 108, max width 270, corner radius 8 8 0 0 (equal to the card radius), transparent fill and no border of its own [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:54-60]. Content is two columns: the activate area (`*`) and the close column (`Auto`) [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:303].
- Activate area: padding 10, 0, 4, 0, filling the whole tab so the padded edge activates too [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:61-62,306-311]; progress spinner icon 10 x 10 with margin right 4, shown only while progress is reported [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:317-325]; title 11 px (`FontSizeS`), ellipsis trimming, margin right 4 [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:329-334], [L src/Lunegit.Desktop/App.axaml:158].
- Close button: 20 x 20, icon 10 x 10 in `fg_tertiary`, margin right 8, vertically centred; hidden (no reserved gap) unless the tab is active or hovered [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:338-356,78-93].
- New-tab button: 20 x 20, plus icon 14 x 14 in ForegroundMuted, hover fill `surface_hover`, right of the scrolling strip [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:111-122,365-373].
- Overflow: the strip scrolls horizontally with the bar hidden (wheel and trackpad scroll) to keep the 32 px header clean [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:240-247].
- Title colour: ForegroundMuted inactive, `fg_primary` active [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:71-77]. ForegroundMuted is the solid #B0B0B0 [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:13]; `Theme` has no such token today [R crates/lune-ui-core/src/theme.rs:11-18], so the token `fg_muted` is added in M4 ([D58]).
- Connected outline: one closed polyline that wraps the active tab and the card body; card radius 8, seam radius 8, flare radius 6 (concave foot where the tab meets the seam), stroke 1; below a visible tab width of 12 (twice the flare radius) the notch is dropped and the loop falls back to the body silhouette [R crates/lune-ui-core/src/chrome.rs:5-10], [L src/Lunegit.Desktop/Controls/Terminal/TerminalChromeGeometry.cs:43-58]. The active tab has no fill and no border of its own: its identity is the loop plus a header-tint punch-out [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:22-27].
- Outline brush: `border_subtle` normally, `border` while the panel has focus within it; focus is expressed solely by the loop's brush [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:26-33].
- Busy cue (a command is running): the outline gets one of three treatments: conic sweep (a bright arc travels the loop, period 2400 ms, with a 3 px bloom under the 1 px line at opacity 0.25), comet (a dash of 0.14 of the contour travelling, period 2400 ms), breath (opacity 0.38 to 1.0, period 1800 ms) [L src/Lunegit.Desktop/Controls/Terminal/TerminalChromeAnimation.cs:16-33], [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabChrome.cs:246-356]. Brush: `accent`, retinted to `status_error_light` #EF9A9A or `status_warning_light` #FFD666 by the progress state ([D132]), the same signal as the spinner [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:35-46].
- Context menu per tab (through `overlay.menu`): Close, Close Others, Close All, Rename, Duplicate, Copy Working Directory; the last two are app items [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:265-298].
- Tab tooltip: the full title, with a show delay of 800 ms [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:263-264].

### States

| state | look | cite |
|---|---|---|
| rest (inactive) | transparent, title ForegroundMuted (#B0B0B0), close hidden | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:55,71-73,82-85] |
| hover | fill `surface_hover` (top corners radius 8), close visible | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:67-69,90-93] |
| pressed | the tab activates on press, so pressed shows as active; the activate button paints no fill of its own | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:136-140], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:102-109] |
| active | title `fg_primary`, close visible, outline notch wraps the tab | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:75-77,86-89,22-27] |
| panel focused | outline brush `border` | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:31-33] |
| panel unfocused | outline brush `border_subtle` | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:28-30] |
| focus-visible (tab) | outline ring per the toolkit's focus ring on the activate area | [core.focus] |
| progress | spinner icon in the tab, tinted `accent`, `status_error_light` #EF9A9A or `status_warning_light` #FFD666 ([D132]) | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:123-125,317-325] |
| busy (command running) | outline sweep, comet or breath (Reduced: breath; Off: static) | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabChrome.cs:246-272], [L src/Lunegit.Core/Settings/TerminalOutlineAnimationResolver.cs:17-28] |
| dragging | dragged tab tracks the pointer, neighbours shifted; a neighbour's title shows through the dragged tab's translucent fill, as in Lunegit (no ZIndex or opacity on the drag) | [infra.drag-and-drop], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:54-69] |
| overflow | strip scrolled; a partially visible tab has its notch dropped under 12 px | [L src/Lunegit.Desktop/Controls/Terminal/TerminalChromeGeometry.cs:54-58] |
| empty (no tabs) | strip holds only the new-tab button; the app shows its empty state | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:365-373] |
| disabled | not specified by either source | [derived: no `:disabled` style in TerminalPanelView.axaml:54-125]; see Open questions |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest to hover, rest to active | tab fill and outline brush | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:182-198] |
| close button appears and hides | close opacity | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:199-207] |
| neighbour tabs slide while dragging | tab x offset | move | fast | sine-in-out | snap | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:39-44], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:419-431] |
| dragged tab settles into its slot | tab x offset | settle | fast | sine-in-out | snap | [D40], [infra.drag-and-drop] |
| activate: content swap (the card body's crossfade) | outgoing and incoming content opacity | page-swap | base | sine-in-out | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:209-224], [L src/Lunegit.Desktop/ViewModels/TerminalPanelViewModel.cs:829-834] |
| outline brush focus change | outline colour | focus | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:182-186] |

The content swap is the card body's job (the app composes the strip with its content); the strip only reports the activation. In Reduced the crossfade runs at Fast (180) and in Off it is zero [L src/Lunegit.Desktop/ViewModels/TerminalPanelViewModel.cs:829-834], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:144-147].
The busy cue is the documented long-loop exception, not a duration token: the sweep and comet run 2400 ms, the breath 1800 ms, only while a command runs, and stop when it ends; it is one of the two accepted motions beside the terminal, drawn as a sibling overlay with at most three stroke passes [L .claude/docs/animation-rules.md:147,172], [L src/Lunegit.Desktop/Controls/Terminal/TerminalChromeAnimation.cs:16-20]. The global tier wins: Off gives a static outline, Reduced forces breath (opacity only), Full follows the user's choice [L src/Lunegit.Core/Settings/TerminalOutlineAnimationResolver.cs:7-31].
Never animated: the strip's layout, tab widths changing as the close button shows (the button has no reserved gap and a width shift is not tweened), the content itself.

### Input and focus

- Pointer: press on a tab activates it and arms a reorder; click on the close button closes (a separate click target); click on the new-tab button adds a tab; right press on a tab opens its context menu; wheel and trackpad scroll the strip [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:303-311,338-348,365-368], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:244-247]. Double-click is not bound in the file read.
- Reorder: through `infra.drag-and-drop` (threshold 4 horizontal, nearest-slot target, edge auto-scroll, Escape cancels) [infra.drag-and-drop].
- Keys: Meta+T opens a new tab, which is the app's shortcut [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:17-19], [D122]; Escape cancels a drag [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:336-341]. No close or rename key [D122].
- Gesture: from the press to the end of a gesture the strip draws and feeds the drag machine the press frame's widths [D123]. A tab added, closed or swapped ends the gesture with no reorder. A change of the active tab reveals it only after the gesture, since a reveal scrolls the slots under a held pointer. No tooltip runs during a gesture.
- Menu: the rows of a tab with `closable` false leave out Close. The app's rows follow in `menu_extra`.
- Tab stops: the strip is one `tab-list` stop, roving: Left and Right wrap, Home and End go to the ends, Enter and Space activate, focus does not activate [D122]. The close buttons are no stops (a pointer target and a node); the new-tab button is the next stop.
- Layer: `Page` (inside the card). Context menu and the tooltip are overlays above [core.ui].
- Close policy is the app's: Lunegit confirms with a dialog (checkbox "Don't ask again") only when the session is alive and the prompt is not suppressed [L src/Lunegit.Desktop/ViewModels/TerminalPanelViewModel.cs:574-612]; rename opens a modal dialog with a text field [L src/Lunegit.Desktop/ViewModels/TerminalPanelViewModel.cs:656-659]. The strip emits `close_requested(tab)` and `rename_requested(tab)`; it never decides.
- No focus scope of its own.

### Retained state

- Transient: hover, press, close opacity, the drag state, the busy phase clock (derived from the frame time the app passes) [core.ui].
- Persistent: the strip's scroll offset (in its scroll area) [layout.scroll-area].
- App-owned: the tab list and order, the active tab, titles, progress and busy flags, which tabs are closable [L src/Lunegit.Desktop/ViewModels/TerminalPanelViewModel.cs:109-113,718-726].
- Exit retention: no (a closed tab is removed by the app on the next frame; the outgoing content's fade belongs to the content swap).

### Accessibility

- Role `tab-list` for the strip with one `tab` per tab; name = the tab title; states selected (the active tab), focused, and `busy` while a command runs; actions: default (activate), focus, show-menu, scroll-into-view [infra.accessibility].
- Close button: role `button`, name = `luneui.tab-strip.close`; new-tab button: role `button`, name = `luneui.tab-strip.new`; each is its own node [infra.accessibility].
- The tab panel relationship (which content a tab controls) has no field in the node list: `role-gap: tab to panel association` (stays open, [D122]).

### Strings

- `luneui.tab-strip.new`: "New tab" (Lunegit: "New terminal tab", the word terminal dropped) [L src/Lunegit.Infrastructure/Localization/en.json:780].
- `luneui.tab-strip.close`: "Close tab" [L src/Lunegit.Infrastructure/Localization/en.json:781].
- `luneui.tab-strip.menu.close`: "Close" [L src/Lunegit.Infrastructure/Localization/en.json:821].
- `luneui.tab-strip.menu.close-others`: "Close Others" [L src/Lunegit.Infrastructure/Localization/en.json:822].
- `luneui.tab-strip.menu.close-all`: "Close All" [L src/Lunegit.Infrastructure/Localization/en.json:823].
- `luneui.tab-strip.menu.rename`: "Rename…" [L src/Lunegit.Infrastructure/Localization/en.json:824].

The confirm and rename dialog texts, "Duplicate Tab" and "Copy Working Directory" are app strings [L src/Lunegit.Infrastructure/Localization/en.json:782-785,825-832].

### Gallery scenes

- `chrome` (existing): the tab-outline chrome.
- `tab-strip-rest`: three tabs, the second active, settled.
- `tab-strip-hover`: tab 3 hovered, close visible, the hover settled and the tooltip's delay still running (`Waiting`).
- `tab-strip-unfocused`: the same strip with the panel unfocused (outline `border_subtle`), settled.
- `tab-strip-progress`: tab 1 with the spinner, tinted accent, then error and warning in two more tabs, moving.
- `tab-strip-busy-sweep`: active tab with the conic sweep at phase 0.25 (t = 600 ms), moving.
- `tab-strip-busy-comet`: comet at phase 0.25 (t = 600 ms).
- `tab-strip-busy-breath`: breath at its brightest (t = 900 ms).
- `tab-strip-overflow`: 12 tabs scrolled 105 px, the active first tab cut at the left edge to 3 px, so no notch, settled.
- `tab-strip-long-title`: a title longer than the 270 px maximum, ellipsis, settled.
- `tab-strip-empty`: no tabs, only the new-tab button, settled.
- `tab-strip-dragging`: tab 1 dragged between tabs 2 and 3 (see `dnd-dragging`), settled.
- `tab-strip-hover-fade`: tab 3 hover starts, t = 60 ms (half of `instant`).
- `tab-strip-slide`: neighbour sliding, t = 90 ms (half of `fast`).
- `tab-strip-settle`: released tab settling, t = 90 ms.
- `tab-strip-content-swap`: activation of tab 3, outgoing and incoming content at t = 125 ms (half of `base`).

### Painter primitives

- Existing: fill-radii (the hover fill, radii 8 8 0 0), stroke, stroke-sweep, stroke-comet, text, icon, acrylic (header tint punch-out), clip-rect, xform (drag offsets).
- New: none (the breath is an alpha on `stroke`; the flare arcs are flattened into the polyline [R crates/lune-ui-core/src/chrome.rs:1-4]).

### Acceptance

- Behaviour (scripted input, injected time): press on a tab activates it once; press on the close button reports `close_requested` and does not activate; press, move 4 px and release reorders (`infra.drag-and-drop` acceptance); Escape during a drag cancels with no reorder.
- Outline: the existing `chrome.rs` tests keep passing (`crates/lune-ui-core/src/chrome.rs:187-226`: notch reaches the card top only at the tab, flush-left tab uses the card corner, no-tab loop stays below the seam); a tab span narrower than 12 gives the no-notch loop.
- Pixel probes at scale 2: strip height 64 device px; tab min width 216 and max 540; close button 40 x 40 with a 20 px icon; gap 10; hover fill uses radii 16 16 0 0; the outline stroke is 2 device px.
- Busy cue: sweep angle at phase p is `p * 360` degrees; the comet dash is `0.14` of the contour; the breath opacity runs between 0.38 and 1.0 over 1800 ms; none of them runs when no command is running and `Repaint` is Idle when settled [L src/Lunegit.Desktop/Controls/Terminal/TerminalChromeAnimation.cs:60-66,105].
- Tier: Off gives the static outline; Reduced gives breath [L src/Lunegit.Core/Settings/TerminalOutlineAnimationResolver.cs:17-28].
- Conformance: hover, slide, settle and content-swap transitions pass `motion::conformance`.
- Semantic node: one `tab-list`, one `tab` per tab with `selected` on the active one; an injected `AccessAction` default activates the tab.

### Build

- Milestone: M6
- Depends on: core.tab-outline, infra.drag-and-drop, layout.scroll-area, overlay.menu, widget.button, core.ui, core.focus, infra.accessibility, motion.transitions

## Open questions

- `role-gap: tab to panel association` (see Accessibility; stays open, [D122]).
- Header tint: `Theme` has no token for the header's layer, so `TabStripOptions.header_tint` takes the colour from the app and `None` draws no tint; the gallery uses #00000024 ([D131]). It is a plain fill, drawn after the under shapes, so a hover fill sits under it, where Lunegit's sits over it.

Closed by [D122]: the keyboard model, the disabled tab (none), all three busy treatments ship with Sweep as the default. `fg_muted` landed in M4 ([D58]).

## Notes for the planner

Renamed from the Luneterm-area tab-strip entry (D10, D42): the strip is generic, the terminal parts (progress from OSC 9;4, the working directory, the live-session check) are app data passed in. `core.tab-outline` stays the geometry.
