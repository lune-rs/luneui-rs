---
id: widget.callout
title: Callout
area: widget
status: in-progress
consumers: wane
source: —
owner: toolkit-v1 M6
summary: Callout: static notice with 1 px tone border, 14 px icon, optional figure and action button; not dismissible
---

## What it is

`callout`: static notice with a 1 px border, a 14 px icon, text, an optional figure and an optional action button at the right; neutral, data, warn or err [W docs/ui/components.md:490-493]. The Lunegit indexing banner is the design source for the border, radius and icon [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:8-24].

## Needs

- Wane controls the toolkit lacks (R-widget-4).

## Existing code

`crates/lune-ui-core/src/widgets/callout.rs` (M6): `CalloutOptions`, `Ui::callout`. Gallery page `apps/gallery/src/pages/callout.rs`; tests `crates/lune-ui-core/tests/widget_callout.rs`. Design source, not code moved: `src/Lunegit.Desktop/Controls/IndexingBanner.axaml:8-24` [W docs/ui/components.md:496]. Related stub: `widget.banner-empty-state`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:487-499`; `docs/ui/mocks/css/tokens.css:13,20,25,49-57`; Lunegit `src/Lunegit.Desktop/Controls/IndexingBanner.axaml:8-26`, `src/Lunegit.Desktop/App.axaml:209`.
Conflicts: (1) Fill: the Lunegit banner fills with the solid key `StatusWarningDark` [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:9]; Wane's callout fill is the tone at 1A [W docs/ui/components.md:492]. The fill is an Open question. (2) Text: the Lunegit banner text is in the tone colour [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:23]; Wane's text is primary [W docs/ui/components.md:492]. Open question. (3) Radius 6 and padding 8 vertical, 10 horizontal agree across sources [L src/Lunegit.Desktop/App.axaml:209], [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:13], [W docs/ui/components.md:492].

### Anatomy

- Padding 8 vertical, 10 horizontal, inside the border [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:13]; the content starts 1 px border + 10 px in.
- Radius 6 [L src/Lunegit.Desktop/App.axaml:209]; 1 px tone border [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:10-11].
- Icon 14 px in a 20 px slot; gap 8 [W docs/ui/components.md:492]; Lunegit icon 14 px, spacing 8 [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:14,17-18].
- Text 13/20 [W docs/ui/components.md:492]; Lunegit `FontSizeBase` 13 [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:22].
- Figure: mono, bright, weight 500, never gold; the one number the callout is about [W docs/ui/components.md:490,682].
- Action: a small `widget.button` (24 high) at the right [W docs/ui/components.md:490]; 8 px between the text, the figure and the action; the box grows to the button's height when that is taller than the text.
- Neutral: fill `--surface-input` #ffffff0a [W docs/ui/mocks/css/tokens.css:20]; border `--border-subtle` [W docs/ui/mocks/css/tokens.css:25]. Tone fills and borders: badge recipe, tone at 1A and 44 [W docs/ui/components.md:492], [W docs/ui/mocks/css/tokens.css:50-51,53-54,56-57]. No full-width bar, no accent stripe [W docs/ui/components.md:492].

### States

| state | look | cite |
|---|---|---|
| neutral | neutral fill and border, primary text, secondary icon | [W docs/ui/components.md:492] |
| data | data tone border and icon, tone fill at 1A | [W docs/ui/components.md:492], [W docs/ui/mocks/css/tokens.css:58-61] |
| warn | warn tone border and icon, tone fill at 1A | [W docs/ui/mocks/css/tokens.css:52-54] |
| err | err tone border and icon, tone fill at 1A | [W docs/ui/mocks/css/tokens.css:55-57] |

Static: none; not dismissible in the mocks [W docs/ui/components.md:491].

### Transitions

n/a — static notice; not dismissible [W docs/ui/components.md:491].

### Input and focus

n/a — no focus of its own. The optional action button takes focus as a `widget.button` [widget.button].

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: tone, text, figure, action presence (passed every frame).
- Exit retention: no.

### Accessibility

- Role `note` (Wane's callouts are `role="note"` [W docs/ui/mocks/overview.html:28]; [D55]); live region polite when the notice appears after an action (`CalloutOptions.announce`), since the callouts report state changes (budget alert, proxy offline) [W docs/ui/components.md:491]; name = the text, value = the figure. The action button's node is the note's child.

### Strings

None: the notice text is app data.

### Gallery scenes

- `callout-tones`: neutral, data, warn, err, each with a figure and an action button; settled.

### Painter primitives

- Existing: fill, ring, icon, text
- New: none

### Acceptance

- Pixel probes at scale 2: radius 12 px; 2 px border; icon 28 px in a 40 px slot; text starts 78 px from the box edge (2 px border, 20 px padding, the 40 px slot, 16 px gap; the first draft said 36, which is neither the Wane nor the Lunegit anatomy).
- Conformance: n/a — no transition.
- Semantic node: role `note`; live region polite on appear.

### Build

- Milestone: M6
- Depends on: widget.button, core.painter, text.shaping

## Open questions

- None. (`design-gap`, the callout's tint and primary text against the Lunegit banner's solid `StatusWarningDark` and tone text [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:9], is answered by [D129]: a banner's solid fill and a callout's tint are two widgets with two looks, each per its source; the callout keeps Wane's.)

## Notes for the planner

Wane P0. Related stub `widget.banner-empty-state` is a separate entry (step 8).
