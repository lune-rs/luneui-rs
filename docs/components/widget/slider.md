---
id: widget.slider
title: Slider
area: widget
status: done
consumers: any
source: crates/lune-ui-core/src/widgets/slider.rs
owner: —
summary: Slider: 4 px track with accent fill and a 12 px knob (14 hovered); look derived from the toggle knob and the progress track; arrow keys step
---

## What it is

`slider`: a horizontal value slider, 4 px track, accent fill up to the value, a 12 px knob (14 hovered). Arrow keys step the value. Neither Lunegit nor Wane has a slider, so every look value is derived from a sibling entry and cites [D51] (hard rule 8).

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-5).
- Arrow-key stepping and focus (R-widget-3); semantic node with the slider role (R-infra-4).
- Value change under the motion tiers (R-motion-2).

## Existing code

`crates/lune-ui-core/src/widgets/slider.rs` (`Ui::slider`, `SliderOptions`, `SliderLook`, `paint`). The knob geometry derives from `crates/lune-ui-core/src/widgets/toggle.rs`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:28-47`; `crates/lune-ui-core/src/theme.rs:62,64,66`; Wane `docs/ui/components.md:403-415,627-640`; `docs/ui/mocks/css/tokens.css:22,40`.
Instrument: `git grep -n "<Slider"` over `src/Lunegit.Desktop` at 33a1dd41 gives no hits; `git grep -i slider` over `docs/ui/components.md` at 69931d6 gives none. No source to conflict with.
Derivation sources: `widget.toggle` (knob), `data.progress` (track and fill), `widget.scrollbar` (thumb colour family) [D51].

### Anatomy

- Track: height 4, radius 2 (half the height: a CSS radius of 4 clamps to it in the mock, `sd_round_rect` does not, [D135]), full width of the rect; fill `surface_overlay` #ffffff15 [W docs/ui/components.md:632], [W docs/ui/mocks/css/tokens.css:22]; derived from `data.progress` [D51].
- Value fill: `accent` from the track start to the knob [W docs/ui/components.md:632], [W docs/ui/mocks/css/tokens.css:40]; `accent` is the toggle's on fill [R crates/lune-ui-core/src/theme.rs:64].
- Knob: circle 12 (14 hovered), centred vertically on the track, centre at the value position [derived: widget.toggle knob geometry, R crates/lune-ui-core/src/widgets/mod.rs:28-29]; fill `on_accent` #FFFFFF, opaque: the knob rides an accent fill, which is the toggle's on state; `fg_primary` (the toggle's off knob) is #FFFFFFCC and let the fill's end show through [derived: widget.toggle, R crates/lune-ui-core/src/widgets/mod.rs:45], [W docs/ui/mocks/css/components.css:169-170], [D134].
- Hit area: the track grown by 4 px across and 6 px up and down, as the toggle's hit area [derived: widget.toggle, R crates/lune-ui-core/src/widgets/mod.rs:47].

### States

| state | look | cite |
|---|---|---|
| rest | track `surface_overlay`, fill `accent`, knob 12 `on_accent` | [derived: widget.toggle, R crates/lune-ui-core/src/widgets/mod.rs:29,45], [derived: data.progress, W docs/ui/components.md:632] [D51] |
| hover | knob 14 | [derived: widget.toggle, R crates/lune-ui-core/src/widgets/mod.rs:29] [D51] |
| focus-visible | 1 px outline, offset 2, around the knob | [widget.button] [D51] |
| disabled | opacity 0.4, inert | [widget.button], [L src/Lunegit.Desktop/App.axaml:481-483] [D51] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| value by keys | knob x and fill width | value | fast | sine-in-out | snap | [derived: widget.toggle knob move, L .claude/docs/animation-rules.md:84], [D40], [D51] |
| rest ↔ hover | knob size 12 ↔ 14 | hover | instant | linear | size snap | [L .claude/docs/animation-rules.md:102], [D40], [D51] |
| focus-visible on/off | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40], [D51] |

Never animated: the knob while it follows the pointer during a drag, because that is high-frequency content [L .claude/docs/animation-rules.md:148]. Only a key or an accessibility step (increment, decrement) tweens the knob; a press, a drag, a set-value and an app-driven change snap, and a press also ends a tween in flight, so while pressed the knob equals the value on every frame [D119]. A key step during the tween retargets from the current position.

### Input and focus

- Pointer: a press anywhere in the hit area sets the value at once and starts following (no drag threshold); the following has no transition, keeps going off the track, and ends with the release, whose frame still takes the pointer's last move [D119]. The pointer maps through the transform stack onto the whole track, not the clipped hit rect. While the pointer holds the slider, keys and accessibility requests are dropped.
- Keyboard: Right and Up increment, Left and Down decrement, by one stop [contract: widget.slider role, arrow keys]; every arrow of a frame counts, in arrival order. From an off-step value the key goes to the stop beside it.
- Stops: `min + k * step`, and `max` itself when `step` does not divide the range; the widget clamps to `[min, max]` and snaps every value it writes to the nearest stop [D119]. `step <= 0` is continuous, with a key step of `(max - min) / 100`. A step below the `f32` resolution of the value cannot move it.
- A value in hand is not rewritten until the user acts: an off-step or out-of-range value survives idle frames and hover (out of range it draws at the end). `changed` is set only when the stored value changed.
- Degenerate input (`min >= max`, a value, `min`, `max` or `step` not finite, a rect with no width): no panic, no change, fraction 0, knob at the start.
- Tab stop: yes [core.focus]; focus-visible after keyboard navigation [core.focus].
- Layer: `Base`; no overlay, no focus scope.
- Pointer capture while dragging [core.input].

### Retained state

- Transient: hover, drag in progress, focus ring, the fraction the knob last aimed at (a different one on the next frame is the app's change and snaps) [core.ui].
- Persistent: none.
- App-owned: `value`, `min`, `max`, `step` (passed every frame).
- Exit retention: no.

### Accessibility

- Role `slider`; value = the number with min, max and step the owner passes; states focusable, focused, disabled; actions `increment`, `decrement` and `set-value` [infra.accessibility]. Increment and decrement arrive as Up and Down [D71]; set-value settles on the nearest stop, a NaN or a non-number is ignored and an infinity goes to that end. Numbers that are not finite are reported as 0.

### Strings

None: the slider has no text; the label is the owner's.

### Gallery scenes

- `slider-states`: rest, hover, focus-visible, disabled, value mid-track; settled.
- `slider-value-mid`: value one step up by keys, t = 90 ms (half of `fast`).
- `slider-hover-mid`: knob rest → hover, t = 60 ms (half of `instant`).
- `slider-focus-mid`: focus-visible on, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill
- New: none

### Acceptance

- Scripted: Right and Left change the value by one step; the value stops at `min` and `max` (Open questions); a disabled slider changes nothing.
- Pixel probes at scale 2: track 8 px high, radius 4 px (a semicircular end); knob `on_accent`; knob 24 px (28 hovered); knob centre at the value position.
- Conformance: knob x over 60 Hz samples is monotonic, ends exact, settles within 180 ms + 1 frame; Reduced snaps.
- Semantic node: role slider; value follows `value`; injected `AccessAction` increment and decrement change it by one step.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_slider.rs`: keys `right_and_left_change_the_value_by_one_step_and_stop_at_the_ends`, `every_arrow_of_a_frame_counts_in_arrival_order`, `disabled_changes_nothing`; pointer `a_press_at_three_quarters_sets_the_value_at_once_with_no_drag_threshold`, `a_drag_follows_on_every_frame_with_the_knob_on_the_value_and_nothing_in_flight`; clamp and steps `max_stays_reachable_when_the_step_does_not_divide_the_range`, `every_pointer_result_is_in_range_and_on_a_step_from_min`, `degenerate_input_changes_nothing_and_draws_the_knob_at_the_start`; pixel probes `pixel_probes_at_scale_2`, `hover_grows_the_knob_and_a_held_knob_keeps_the_size_off_the_track`, `no_fill_at_the_start_and_a_full_fill_at_the_end`; conformance `a_key_step_tweens_to_exactly_the_value_monotonically_within_fast_and_a_frame`, `reduced_and_off_snap_a_key_step`, `every_motion_row_conforms_under_every_tier`; node `the_node_is_a_slider_whose_value_follows_and_whose_steps_arrive_as_actions`, `increment_and_decrement_step_and_tween_like_the_keys_with_no_focus`.

### Build

- Milestone: M6
- Depends on: widget.toggle, data.progress, widget.scrollbar, core.focus, core.ui, infra.accessibility, motion.transitions
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_slider.rs::right_and_left_change_the_value_by_one_step_and_stop_at_the_ends, crates/lune-ui-core/tests/widget_slider.rs::disabled_changes_nothing, crates/lune-ui-core/tests/widget_slider.rs::a_press_at_three_quarters_sets_the_value_at_once_with_no_drag_threshold, crates/lune-ui-core/tests/widget_slider.rs::pixel_probes_at_scale_2, crates/lune-ui-core/tests/widget_slider.rs::a_key_step_tweens_to_exactly_the_value_monotonically_within_fast_and_a_frame, crates/lune-ui-core/tests/widget_slider.rs::every_motion_row_conforms_under_every_tier, crates/lune-ui-core/tests/widget_slider.rs::the_node_is_a_slider_whose_value_follows_and_whose_steps_arrive_as_actions`; page `slider`; unverified here: none.

## Open questions

- Resolved [D119] `pointer-gap`: a press anywhere in the hit area sets the value.
- Resolved [D119] `clamp-gap`: the widget clamps and snaps.
- Still open, a note: `no-consumer`: neither Lunegit nor Wane uses a slider today; the entry is kept for the M6 control set (R-widget-5). It changes no behaviour.

## Notes for the planner

New control with no source. Every look value above is derived and cites [D51]; a value that a reviewer cannot trace to a sibling entry is a defect.
