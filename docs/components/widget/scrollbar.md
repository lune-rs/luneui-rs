---
id: widget.scrollbar
title: Scrollbars
area: widget
status: planned
consumers: lunegit, wane
source: Lunegit: src/Lunegit.Desktop/App.axaml
owner: —
summary: Overlay scrollbar: 9 px transparent track, 6 px accent thumb, auto-hide while the pointer is away, thumb drag
---

## What it is

The overlay scrollbar of `layout.scroll-area`: a 9 px transparent track with a 6 px rounded thumb in
the accent colour, shown only while the pointer is over the scroll area or the thumb is held. It
takes no layout space. Lunegit styles Avalonia's `ScrollBar` once for the whole app (`App.axaml`);
Wane mocks it and leaves dragging, the fade and keyboard paging to the native build.

## Needs

- Scrollbar widget (R-widget-2).
- Scroll regions that keep offset (R-layout-2): the bar reads and writes the area's offset.
- Wane — States: hidden; visible while the pointer is over the scroll area (fade `--dur-fast`); thumb hover (`--scroll-thumb-hover`, accent at .9); content fits (no thumb). (R-widget-2)
- Wane — Sizes: track 9 wide, transparent; thumb 6 wide at right 1, radius 3 (`--radius-thumb`), minimum length 28, fill `--scroll-thumb` (accent at .55). (R-widget-2)
- Wane — Behaviour: wheel and touchpad scroll; drag the thumb; PageUp and PageDown in the scroll area. (R-widget-2)

## Existing code

None in `lune-ui` today. Design source (not code to move): Lunegit `src/Lunegit.Desktop/App.axaml:588-659` (style), `Themes/DarkTheme.axaml:164-165` (thumb fills),
`Themes/Animations.axaml:616-630` and `Themes/AnimationsReduced.axaml:438-449` (the fade); Wane `docs/ui/mocks/css/components.css:419-442`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/App.axaml:588-668, Themes/DarkTheme.axaml:164-165, Themes/Animations.axaml:616-632,
Themes/AnimationsReduced.axaml:436-451, Views/UnifiedDiffView.axaml:45-56, Views/TerminalPanelView.axaml:240-247, Views/SplitDiffView.axaml.cs:180-189; Wane docs/ui/components.md:403-416,
docs/ui/mocks/css/components.css:419-442, docs/ui/mocks/css/tokens.css:44-45,113; this repo crates/lune-ui-core/src/theme.rs:56-67.
Conflict: Wane puts the thumb "at right 1" inside the 9 px track [W docs/ui/mocks/css/components.css:430]; Lunegit's style sets a 6 px thumb in a 9 px bar and states no offset [L src/Lunegit.Desktop/App.axaml:598-601,622-624]
(the 3 px slack is placed by Avalonia's template, not read). Wane's 1 px from the end is taken because Lunegit states none. Wane's fill `#767d858c` equals the
`accent` token at alpha 0x8C = 55 percent [W docs/ui/mocks/css/tokens.css:44], [R crates/lune-ui-core/src/theme.rs:64].

### Anatomy

- Vertical bar: 9 px wide (`Width` and `MinWidth` 9); horizontal: 9 px high [L src/Lunegit.Desktop/App.axaml:598-605]. Track and background transparent [L src/Lunegit.Desktop/App.axaml:595-597].
- Thumb: 6 px across (vertical: `Width` 6; horizontal: `Height` 6), minimum length 28 (`MinHeight` vertical, `MinWidth` horizontal), corner radius 3 [L src/Lunegit.Desktop/App.axaml:622-639].
- Thumb length = `viewport / extent * track`, floored at 28 [derived: the proportional thumb of every scrollbar; the 28 floor is cited]; thumb travel `track - thumb` maps to offset `0 .. extent - viewport` [derived: same].
- Arrow buttons, track rectangles and the corner block where the two bars meet are hidden [L src/Lunegit.Desktop/App.axaml:606-614], [L src/Lunegit.Desktop/App.axaml:657-659].
- Thumb fill: `accent` at alpha 0.55; hovered and pressed: `accent` at 0.9 [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:164-165]; the moved dark preset's accent is #767D85 [R crates/lune-ui-core/src/theme.rs:64]. The look derives from the `accent` token with these two alphas ([D51] states the same rule for new looks).
- Wane's thumb sits 1 px from the track's right edge [W docs/ui/mocks/css/components.css:430].
- Bar visibility per scroll area: Auto, Hidden or Disabled [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:49-50], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:244-247].
- No thumb when the content fits [W docs/ui/mocks/css/components.css:442].
- A press that starts on the bar is not passed to the content's own press handling: the split diff's line-selection handler skips presses on a scroll bar or a button [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:180-189].

### States

| state | look | cite |
|---|---|---|
| rest, pointer away | thumb opacity 0 (bar auto-hidden) | [L src/Lunegit.Desktop/App.axaml:629] |
| visible | pointer anywhere over the scroll area: thumb opacity 1, fill `accent` at 0.55 | [L src/Lunegit.Desktop/App.axaml:640-644], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:164] |
| hover | pointer over the thumb: fill `accent` at 0.9 | [L src/Lunegit.Desktop/App.axaml:648-650], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:165] |
| pressed (dragging) | fill `accent` at 0.9 and the thumb stays visible even when the pointer leaves the area | [L src/Lunegit.Desktop/App.axaml:615-618,645-647,651-653] |
| content fits | no thumb | [W docs/ui/components.md:407] |
| focus-visible | none: the bar is not a Tab stop | [derived: no focus style in App.axaml:588-659; the scroll area scrolls by keys] |
| disabled | bar Disabled: no thumb, no scrolling | [L src/Lunegit.Desktop/App.axaml:666-667] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| thumb reveal and hide | thumb opacity | hover | fast | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:622-630] |
| thumb hover colour | fill alpha 0.55 to 0.9 | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [D40] |

Reduced keeps the opacity fade at the Instant duration [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:441-449]; Off is a hard cut and the thumb still auto-hides [L src/Lunegit.Desktop/Themes/Animations.axaml:618-619].
Lunegit sets the hover and pressed fills on the thumb's inner border with no transition of their own [L src/Lunegit.Desktop/App.axaml:648-653], so the colour tween is a [D40] addition under the hover kind. Never animated: the thumb position and length (they follow the offset
directly), nothing beyond the motion.md list.

### Input and focus

- Pointer: hover over the scroll area reveals the thumb; press on the thumb and drag with pointer capture until release, even outside the area [core.ui], [L src/Lunegit.Desktop/App.axaml:615-618]; wheel and trackpad belong to the area ([layout.scroll-area]).
- Keys: PageUp and PageDown scroll the area by a viewport [W docs/ui/components.md:409]; the bar has no key handling of its own and is not a Tab stop (see the States row).
- Layer: the layer of the scroll area; the thumb is hit-tested above the content it overlays [core.ui].
- No focus scope.

### Retained state

- Transient: thumb opacity, hover and press, the drag grab offset inside the thumb [core.ui].
- Persistent: none of its own; the offset lives in the owning scroll area [layout.scroll-area].
- App-owned: the visibility mode.
- Exit retention: no.

### Accessibility

n/a — no node of its own: the owning scroll area exposes the offset, its range and the scroll actions [infra.accessibility], [layout.scroll-area]. The role list has no scrollbar role; not a gap, because the area's node covers every action a scrollbar would.

### Strings

None: the bar has no text and no tooltip in the style read [L src/Lunegit.Desktop/App.axaml:588-659].

### Gallery scenes

- `scrollbar-hidden`: scroll area, pointer away, settled.
- `scrollbar-visible`: pointer over the area, thumb at 40 percent offset, settled.
- `scrollbar-thumb-hover`: pointer over the thumb, settled.
- `scrollbar-dragging`: thumb held, pointer outside the area, settled.
- `scrollbar-fits`: content shorter than the viewport, pointer over, no thumb, settled.
- `scrollbar-disabled`: bar Disabled, settled.
- `scrollbar-reveal`: pointer enters the area, t = 90 ms (half of `fast`).
- `scrollbar-hover-colour`: pointer enters the thumb, t = 60 ms (half of `instant`).

### Painter primitives

- Existing: fill-radii (thumb with radius 3), fill-over (the 0.55 and 0.9 alphas), clip-rect.
- New: none.

### Acceptance

- Behaviour: dragging the thumb by half the track moves the offset by half of `extent - viewport`; the thumb stays visible and keeps the grab after the pointer leaves the area; release ends the drag; a press on the thumb does not select content under it.
- Pixel probes at scale 2: a vertical bar occupies 18 device px; the thumb is 12 px wide with radius 6 device px; the minimum thumb length is 56 device px; the thumb fill equals `accent` at alpha 0x8C, and at 0xE6 hovered (Wane's `--scroll-thumb-hover` is `#767d85e6`, [W docs/ui/mocks/css/tokens.css:45]).
- Conformance: reveal and hover-colour transitions pass `motion::conformance`; Reduced runs the fade at 120 ms, Off snaps [motion].
- Semantic node: n/a, no node; the scroll area's node follows the offset [infra.accessibility].

### Build

- Milestone: M5
- Depends on: layout.scroll-area, core.ui, motion.transitions

## Open questions

- Click in the empty track: Lunegit hides every `RepeatButton` in the bar template [L src/Lunegit.Desktop/App.axaml:606-608]; whether Fluent's track page buttons are among them is not verified, so page-by-click is unspecified. M5 decides (default: no).
- Horizontal thumb offset inside the 9 px bar: Wane's 1 px from the end applies to the vertical bar only; the horizontal bar's placement is not stated by either source.

## Notes for the planner

Wane priority P0 (every screen with a scroll area, [W docs/ui/components.md:415]). The code block and the tables reuse it for horizontal overflow. The thumb fills are the first looks derived from the `accent` token with an alpha; M5 adds them as `Theme` values.
