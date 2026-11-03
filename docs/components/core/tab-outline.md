---
id: core.tab-outline
title: Tab outline geometry
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/chrome.rs
owner: orch/toolkit-v1
summary: Tab-chrome outline loop geometry
---

## What it is

`outline` builds the tab chrome loop from `TabSpan`s, with `CARD_RADIUS`, `SEAM_RADIUS`, `FLARE_RADIUS` and `STROKE`.

## Needs

- Tab outline geometry (R-core-7).

## Existing code

`crates/lune-ui-core/src/chrome.rs:6-9` (constants), `:14` (`TabSpan`), `:20` (`outline`); 226 lines; tests at `:172`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b, Luneterm 296cfdc. Read: `crates/lune-ui-core/src/chrome.rs:1-226` (constants 6-9, `TabSpan` 14, `outline` 20, tests 186-226);
`crates/lune-ui-wgpu/src/shader.wgsl:106-107,122,135-141,145-157` (stroke modes, sweep alpha table, comet);
Luneterm `crates/lune-ui/src/shader.wgsl:106-107,135-155` (`git show 296cfdc`; the same shader in Luneterm);
Lunegit `.claude/docs/animation-rules.md:147` (the terminal card outline: a sibling overlay, three stroke passes at most, running only while a command executes).
Conflicts: none. The radii are the repo's (`CARD_RADIUS` 8, `SEAM_RADIUS` 8, `FLARE_RADIUS` 6, `STROKE` 1, [R crates/lune-ui-core/src/chrome.rs:6-9]).

### Anatomy

- Card corner radius 8, seam radius 8, flare radius 6, stroke 1 px [R crates/lune-ui-core/src/chrome.rs:6-9].
- `outline(width, height, seam, tab)` builds the loop: a notch at the tab, or a flush-left tab that uses the card corner [R crates/lune-ui-core/src/chrome.rs:20,186-226].
- Sweep: a conic sweep of alpha over the loop; the alpha table has a faint level 64/255 and breakpoints 0.1528, 0.3056, 0.6944, 0.8472 [R crates/lune-ui-wgpu/src/shader.wgsl:135-141], [LT crates/lune-ui/src/shader.wgsl:135-141].
- Comet: a head with a tail along the contour length [R crates/lune-ui-wgpu/src/shader.wgsl:156-157].

### States

| state | look | cite |
|---|---|---|
| idle | the 1 px outline | [R crates/lune-ui-core/src/chrome.rs:9] |
| running, sweep | conic alpha sweep around the loop | [R crates/lune-ui-wgpu/src/shader.wgsl:151-155] |
| running, comet | a comet head with a tail | [R crates/lune-ui-wgpu/src/shader.wgsl:156-157] |

### Transitions

n/a — the running sweep and comet are driven by the app's time (the shader's offset `style.z`, [R crates/lune-ui-wgpu/src/shader.wgsl:155]); the period has no source and is an Open question. Its kind would be `spin`; reduced snaps.

Runs only while a command executes (Lunegit's terminal card, [L .claude/docs/animation-rules.md:147]). Never animated: the terminal itself.

### Input and focus

n/a — the outline takes no input.

### Retained state

- Transient: the sweep offset, from the app's time.
- Persistent: none.
- App-owned: the tab span, the card size, the running flag.
- Exit retention: no.

### Accessibility

n/a — decorative outline; the tab's own node is the tab-strip's (widget.tab-strip).

### Strings

n/a — no text.

### Gallery scenes

- `chrome` (existing): the card with a tab outline, settled. [R apps/gallery/src/scenes.rs:173-178]
- New scenes for the running state are open (see Open questions).

### Painter primitives

- Existing: stroke, stroke-sweep, stroke-comet, clip-rect. [R crates/lune-ui-core/src/paint.rs:329]
- New: none.

### Acceptance

- Existing tests (by path, `crates/lune-ui-core/src/chrome.rs`): `notched_loop_reaches_card_top_only_at_the_tab` (:186 region), `flush_left_tab_uses_card_corner` (:206 region), `no_tab_loop_stays_below_seam` (:221 region).
- Done standard still needs: a sweep-table probe (the four breakpoints) and a comet probe at scale 2, and the running-state conformance against the period the app passes.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: core.painter, core.theme, widget.tab-strip.

## Open questions

- Running period of the sweep and comet: no source for the duration; open.

- The name `chrome` and its tab-bar assumption read as app-shaped; whether it is generic is for the planner.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.
