---
id: widget.banner-empty-state
title: Banners, empty states and loading gates
area: widget
status: done
consumers: lunegit
source: crates/lune-ui-core/src/widgets/banner.rs
owner: —
summary: Indexing banner (warning tone, spinning sync icon) and the empty-repository view: centred icon, title, subtitle, action buttons
---

## What it is

Two Lunegit surfaces: the indexing banner (a warning-tone strip with a spinning sync icon and one line of text) and the empty-repository view (a centred icon, a title, a subtitle and action buttons) [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:8-26], [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:10-35].

## Needs

- Drawn through the painter (R-widget-2) and the image primitive for the empty-state icon (R-core-9).
- The spinner in the banner (`widget.spinner`); buttons in the empty state (`widget.button`).

## Existing code

`src/Lunegit.Desktop/Controls/IndexingBanner.axaml`, `src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml`. The startup loading view and the EULA and Git gate views are named in the survey; they were not read and are not specified here (Open questions).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `src/Lunegit.Desktop/Controls/IndexingBanner.axaml:6-26`; `src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:8-40`; `src/Lunegit.Desktop/App.axaml:162,209,213`; `src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:42-46`.
Wane has no banner or empty-state component; Wane's callout is the nearest look and is compared under `widget.callout` (conflict noted there). No Wane conflict for this entry.

### Anatomy

Banner:
- Fill `StatusWarningDark`, 1 px border `StatusWarningLight`, radius 6, padding 10 horizontal and 8 vertical [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:9-13], [L src/Lunegit.Desktop/App.axaml:209].
- Sync icon 14 x 14 in `StatusWarningLight`, spinning through `SpinBehavior` [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:15-20].
- Text 13 px, wrapped, `StatusWarningLight`, centred vertically; spacing 8 between icon and text [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:14,21-25].

Empty-repository view:
- Centred column, spacing 24, max width 520 [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:13-14].
- Icon: app icon, 100 x 100, uniform stretch [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:17-21], [L src/Lunegit.Desktop/App.axaml:213].
- Title: 18 px `FontSizeL` [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:24-27], [L src/Lunegit.Desktop/App.axaml:162], `ForegroundPrimary`.
- Subtitle: 13 px, centred, wrapped, `ForegroundSecondary` [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:30-35].
- Actions: row of `Button.empty-state-action`, spacing 12 [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:38-39]; each at least 140 wide, 51 high (padding 16 above and below, a 17 px line, a 1 px `border` ring that scales with the press), radius `item_radius`, the ghost button's look (the Secondary's clear rest, and the press scale that `empty-state-action` keeps and the dialog buttons lost, D269) [L src/Lunegit.Desktop/App.axaml:440-450], [D136].

### States

| state | look | cite |
|---|---|---|
| banner, shown | warning fill and border, spinning icon | [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:9-19] |
| banner, hidden | not drawn | [L src/Lunegit.Desktop/Controls/IndexingBanner.axaml:9] (owner's visibility; the banner has no state of its own) |
| empty view | centred content as Anatomy | [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:13-35] |
| action button hover, pressed | per `widget.button` classed-button rows | [L src/Lunegit.Desktop/App.axaml:455-458], [L src/Lunegit.Desktop/Themes/Animations.axaml:88-109] |
| disabled | opacity 0.4, inert | [widget.button], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| action hover | background | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:88-96], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:42-46] |
| action press | scale 0.98 | press | fast | quad-out | snap | [L src/Lunegit.Desktop/Themes/Animations.axaml:97-109], [L .claude/docs/animation-rules.md:103] |

The banner has no transition of its own; the spinner's rotation is in `widget.spinner`.

### Input and focus

- Banner: no input.
- Empty view: the action buttons take Tab and Enter [core.focus]; clicks run the owner's commands.
- Layer: `Base`; no overlay, no focus scope.

### Retained state

- Transient: action hover and press [core.ui].
- Persistent: none.
- App-owned: banner visibility, the banner text, the empty-view commands (passed every frame).
- Exit retention: no.

### Accessibility

- Banner: role `status` with a polite live region when shown; name = the text [infra.accessibility].
- Empty view: role `static-text` for the title and subtitle; the action buttons are `button`s named by the owner; the icon is role `image` with a name from the owner or hidden.

### Strings

None. The banner text, the empty view's title and subtitle and the action labels are the app's: `Ui::banner` and `Ui::empty_state` take them as arguments, so the entry's `luneui.banner-empty-state.*` keys are not added ([D128]).

### Gallery scenes

- `banner-shown`: banner with the spinner, 300 ms in (the spinner runs: moving).
- `banner-empty-state`: empty-repository view with two actions, settled.
- `banner-empty-state-disabled`: the empty view disabled (0.4 opacity) with the pointer on the first action: no hover fill, settled.
- `banner-empty-state-action-mid`: action pressed, t = 90 ms (half of `fast`).
- `banner-empty-state-hover-mid`: action rest → hover, t = 60 ms (half of `instant`).

### Painter primitives

- Existing: fill, ring, icon, text
- New: rotate, image

`rotate` is the banner's spinning icon (through `widget.spinner`); `image` is the empty-state icon (`src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:17`).

### Acceptance

- Pixel probes at scale 2: banner radius 12 px, 2 px border; empty icon 200 x 200 px; subtitle centred; action buttons at least 280 wide and 102 high with a 2 px ring.
- Conformance: action press scale over 60 Hz samples settles within 180 ms + 1 frame; Reduced snaps the scale.
- Semantic node: banner role status, polite live region; empty-view title role static-text.
- Tests, bullet by bullet. Banner (`crates/lune-ui-core/tests/widget_banner.rs`): `the_banner_is_the_warning_fill_with_a_one_px_light_border`, `a_one_line_banner_is_the_icon_and_padding_high_and_a_long_one_wraps`, `the_banner_spins_while_busy_and_holds_still_when_not`, `the_banner_node_is_a_polite_status_named_by_the_text`, `pixel_probes_at_scale_two_banner`. Empty view: `the_column_is_centred_with_the_entrys_sizes_and_spacing`, `the_column_is_never_wider_than_520_or_the_rect`, `the_subtitle_rows_are_each_centred`, `an_action_click_reports_its_index_once`, `title_and_subtitle_are_static_text_and_actions_are_buttons`, `the_picture_node_is_hidden_and_none_means_no_picture`, `pixel_probes_at_scale_two_empty_view`, `each_action_has_a_one_px_border_ring_on_the_button_rect`. Conformance: `the_action_press_scale_settles_within_180_ms_and_reduced_snaps_it`, `the_action_hover_fill_tweens_over_instant_and_a_settled_view_is_idle`.

### Build

- Milestone: M6
- Depends on: widget.spinner, widget.button, core.image, text.shaping
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_banner.rs::the_banner_is_the_warning_fill_with_a_one_px_light_border, crates/lune-ui-core/tests/widget_banner.rs::the_banner_node_is_a_polite_status_named_by_the_text, crates/lune-ui-core/tests/widget_banner.rs::pixel_probes_at_scale_two_banner, crates/lune-ui-core/tests/widget_banner.rs::pixel_probes_at_scale_two_empty_view, crates/lune-ui-core/tests/widget_banner.rs::an_action_click_reports_its_index_once, crates/lune-ui-core/tests/widget_banner.rs::the_action_press_scale_settles_within_180_ms_and_reduced_snaps_it`; page `banner` (4 scenes); unverified here: none (limit with a row: the actions have no glyph, ruling C13, D136).

## Open questions

None. Closed by [D128] and [D129]: `scope` (the startup loading view and the EULA and Git gate views stay app code), `design-gap` (the banner's solid fill and the callout's tint are two widgets with two looks, each per its source). The empty view has no field for the picture's name, so its `Image` node is always hidden.

Marked gap (ruling C13, [D136]): Lunegit stacks a 24 px glyph over each action's label, spacing 8 [L src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml:38-62]; `EmptyState.actions` has no glyph, which is an API change outside M6 step 13. The actions' Secondary fill (#10) also differs from Lunegit's `SurfaceOverlay` (#15).

## Notes for the planner

Lunegit-only (consumers lunegit). `EmptyStateLogoSize` 100 and `FontSizeL` 18 are the drawn values.
