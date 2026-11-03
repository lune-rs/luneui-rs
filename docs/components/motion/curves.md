---
id: motion.curves
title: Easing curves and menu motion
area: motion
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/widgets/menu.rs
owner: orch/toolkit-v1
summary: cubic_bezier easing and the menu open and close values; becomes the cubic-out and ease-in tokens
---

## What it is

`cubic_bezier` and the menu's `MENU_IN` and `MENU_OUT` with `menu_open_anim` and `menu_close_anim`,
moved from Luneterm; the app passes the time in. They are the only two pixel-gated curves in the
toolkit, and `docs/motion.md` names them `cubic-out` and `ease-in` so every other component speaks in
tokens. The general easings, tiers and kinds are `motion.transitions` and `motion.animation-tiers`.

## Needs

- Easing and time-in animation values (R-motion-1, R-nfr-1).
- The moved curves stay bit-identical while the rest of the motion system is built around them (R-motion-3).

## Existing code

`crates/lune-ui-core/src/widgets/menu.rs:28` (`cubic_bezier`), `:25` (`MenuAnim`), `:21-22`
(`MENU_IN`, `MENU_OUT`), `:46` (`menu_open_anim`), `:52` (`menu_close_anim`); three unit tests at
`:167`, `:177`, `:185`. Drawn in the gallery's `menu` and `menu-opening` scenes
(`apps/gallery/src/scenes.rs:96,103`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/menu.rs:1-60,160-195`;
`apps/gallery/src/scenes.rs:90-110,363-393`; Wane docs/ui/mocks/css/tokens.css:115-119,
docs/ui/mocks/css/components.css:347-363; Lunegit .claude/docs/animation-rules.md:76-110,
src/Lunegit.Desktop/Themes/Animations.axaml:250-265.
Conflict: Lunegit's menu flyout fades opacity only over Base and `quad-out`
[L src/Lunegit.Desktop/Themes/Animations.axaml:250-256]; the moved menu moves 4 px and scales 0.97
over 0.18 s [R crates/lune-ui-core/src/widgets/menu.rs:46-49]. The moved code is pixel-gated and wins
[motion]. Wane's mock repeats the moved values [W docs/ui/mocks/css/components.css:347-349,360-363].

### Anatomy

- `cubic_bezier(x1, y1, x2, y2, t)`: CSS cubic-bezier solved for the curve's x by 24 bisection steps, `t` clamped to 0..1 [R crates/lune-ui-core/src/widgets/menu.rs:28-42].
- `cubic-out` is `cubic_bezier(0.215, 0.61, 0.355, 1)`: the open curve [R crates/lune-ui-core/src/widgets/menu.rs:46-48], also Wane's `--ease-out` [W docs/ui/mocks/css/tokens.css:119].
- `ease-in` is `cubic_bezier(0.42, 0, 1, 1)`: the close curve [R crates/lune-ui-core/src/widgets/menu.rs:52-56], Wane `ease-in` [W docs/ui/mocks/css/components.css:361].
- Open at progress `t` = elapsed / `MENU_IN` (0.18 s = token `fast`): opacity `e`, vertical offset `4 (1 - e)` px, scale `0.97 + 0.03 e`, about the top-left corner [R crates/lune-ui-core/src/widgets/menu.rs:21,45-49], [W docs/ui/mocks/css/components.css:362].
- Close at progress `t` = elapsed / `MENU_OUT` (0.12 s = token `instant`): opacity `1 - e`, offset `2 e` px, scale `1 - 0.02 e`; `None` once `t >= 1` [R crates/lune-ui-core/src/widgets/menu.rs:22,51-57], [W docs/ui/mocks/css/components.css:361,363].
- `MenuAnim` is the plain triple `(alpha, dy, scale)` [R crates/lune-ui-core/src/widgets/menu.rs:25].
- Target shape after M4: `motion::Easing::CubicBezier(x1, y1, x2, y2)` evaluates the same function; `Easing::CUBIC_OUT` and `Easing::EASE_IN` are the two constants; `widgets::menu::cubic_bezier`, `menu_open_anim` and `menu_close_anim` keep their paths and signatures and delegate [motion.transitions], [D10].

### States

n/a — a function, not a component. The two animation phases (opening, closing) are Transitions rows below.

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| menu open | opacity, offset 4 px, scale 0.97 to 1 | pop | fast | cubic-out | fade | [R crates/lune-ui-core/src/widgets/menu.rs:21,46-49], [motion] |
| menu close | opacity, offset 2 px, scale 1 to 0.98 | dismiss | instant | ease-in | fade | [R crates/lune-ui-core/src/widgets/menu.rs:22,52-57], [motion] |

Reduced drops the offset and the scale and keeps the opacity fade at the Reduced-mapped duration
[motion]. Never animated: a menu row's hover does not scale; nothing beyond the motion.md list.

### Input and focus

n/a — pure functions of progress; the menu's own input and focus are `overlay.menu`.

### Retained state

- Transient: none held here; the caller keeps the open time and passes `t` [R crates/lune-ui-core/src/widgets/menu.rs:45-46].
- Persistent: none. App-owned: until M4, the open and close start times.
- Exit retention: the close animation is the exit: `MENU_OUT` = 120 ms = `instant`; in M4 the menu's `WidgetId` stays alive until it ends [core.ui].

### Accessibility

n/a — no widget; the menu node is `overlay.menu`'s.

### Strings

None: no user-visible text.

### Gallery scenes

- `menu` (existing): settled, hover item 1, scale 2 [R apps/gallery/src/scenes.rs:96-101].
- `menu-opening` (existing): open at t = 90 ms (half of `fast`), scale 2 [R apps/gallery/src/scenes.rs:103-108].
- `curves-menu-closing`: the same menu closing at t = 60 ms (half of `instant`); new, drawn once `menu_close_anim` has a scene.
- `motion-curve-cubic-out`, `motion-curve-ease-in`: the curve plotted over 0..1 beside its token name, settled; shared with `motion.transitions`' `motion-easings` page.

### Painter primitives

- Existing: fill, stroke, text.
- New: none.

### Acceptance

- Existing unit tests keep passing unchanged: `cubic_bezier_endpoints`, `open_curve_ends_at_rest`, `close_curve_ends_at_none` [R crates/lune-ui-core/src/widgets/menu.rs:167,177,185].
- Zero pixel delta on `menu` and `menu-opening` before and after the move into `motion` (the pixel gate, CLAUDE.md) [R apps/gallery/src/scenes.rs:96,103].
- Known-answer table: `Easing::CUBIC_OUT` at t = 0.25, 0.5, 0.75 equals `cubic_bezier(0.215, 0.61, 0.355, 1, t)` to 1e-6; `menu_open_anim(0.5)` unchanged to the bit against the moved function [derived: the old and new paths must agree exactly].
- Conformance: both curves pass `motion::conformance` (monotonic, exact end, no NaN, within duration plus one frame) [D41].

### Build

- Milestone: M4 (becomes `Easing::CubicBezier` and the two constants); done by M11.
- Depends on: motion.transitions.

## Open questions

- None. (Where `cubic_bezier` lives is answered above: it moves into `motion` and the old path delegates.)

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1). Do not retune the two curves: the pixel
gate covers them only where a gallery scene draws them. The rest of the motion vocabulary is in
`docs/motion.md`.
