---
id: overlay.popover
title: Anchored popover
area: overlay
status: in-progress
consumers: any
source: crates/lune-ui-core/src/widgets/popover.rs
owner: orch/toolkit-v1
summary: Anchored overlay placement and lifecycle: flip and shift inside the window, dismiss on outside click, Escape and focus loss, Popup layer, focus scope
---

## What it is

The shared mechanism under every anchored overlay: it places a rect of known size next to an anchor
rect, flips to the opposite side when it does not fit, shifts to stay inside the window, opens and
closes with motion tokens, and dismisses on an outside click, on Escape and on focus loss. It lives
on the `Popup` layer and pushes a focus scope. Dropdown, menu, tooltip, flyout and the search
pickers are built on it or share its rules; today each repeats the flip by hand
(`crates/lune-ui-core/src/widgets/dropdown.rs:19-23`, `widgets/menu.rs:95-100`).

## Needs

- Anchored popover placement and dismiss (R-overlay-4).
- Tooltips and flyouts sit on it (R-overlay-3); the existing popup panel, dropdown and menu are its first consumers (R-overlay-1).
- Layers, modality and focus scopes (R-core-8).

## Existing code

Built in M5 (D101): `crates/lune-ui-core/src/widgets/popover.rs` (`place`, `Ui::popover`); the dropdown and the menu now call `place`. The placement rule had existed twice inline: `crates/lune-ui-core/src/widgets/dropdown.rs:19-23` (flip) and `crates/lune-ui-core/src/widgets/menu.rs:95-100` (flip and shift);
the plain shell is `popup_panel` at `crates/lune-ui-core/src/widgets/mod.rs:200-203`; the layer order is `LayerId` at `crates/lune-ui-core/src/scene.rs:17-22`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: this repo crates/lune-ui-core/src/widgets/dropdown.rs:1-60, widgets/menu.rs:1-135, widgets/mod.rs:196-203, scene.rs:10-30;
Lunegit src/Lunegit.Desktop/Views/DiffView.axaml:18-40,100-125, Controls/LinkSearchPicker.axaml:36-50,71, Controls/FieldSearchPicker.axaml:54-58, Views/SidePanels/CommitPanelView.axaml:360-372,
Themes/Animations.axaml:241-267, Themes/DarkTheme.axaml:300-310, .claude/docs/animation-rules.md:98-110; Wane docs/ui/components.md:543-556,702-717.
Conflicts: (1) the open animation. Lunegit's popups open with opacity only, Base, quad-out, "by necessity" because a scale keyframe crashes on inline runs [L src/Lunegit.Desktop/Themes/Animations.axaml:241-249]; its own rule table wants opacity plus scale 0.96 to 1 at Base [L .claude/docs/animation-rules.md:109];
the moved menu opens in 0.18 s with an ease-out curve from 4 px lower at 97 percent [R crates/lune-ui-core/src/widgets/menu.rs:19-22,46-49]. The moved code is pixel-gated and wins: the popover pops like the menu (`pop`, `fast`, `cubic-out`), see Transitions.
(2) The shell radius: Lunegit's options flyout uses an outer radius 12 and an inner 11 [L src/Lunegit.Desktop/Views/DiffView.axaml:28-33], the picker popup 8 [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:45], the moved menu 9 [R crates/lune-ui-core/src/widgets/menu.rs:74] and the moved panel 8 [R crates/lune-ui-core/src/widgets/mod.rs:201].
The popover therefore draws no shell: the consumer passes its own, and the default is `popup_panel`.

### Anatomy

- Inputs: the anchor rect, the popover size, the preferred side and alignment, the window rect (logical px). Lunegit's placement vocabulary is a side plus an alignment: `BottomEdgeAlignedLeft`, `BottomEdgeAlignedRight`, `TopEdgeAlignedRight` [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:40], [L src/Lunegit.Desktop/Views/DiffView.axaml:108], [L src/Lunegit.Desktop/Views/SidePanels/CommitPanelView.axaml:366].
- Gap from the anchor: 4 px [R crates/lune-ui-core/src/widgets/dropdown.rs:21,22].
- Flip: when the preferred side overflows the window minus an 8 px margin (`below + h > window.h - 8`), use the opposite side at the same gap, and never above the 8 px margin (`max(anchor.y - 4 - h, 8)`) [R crates/lune-ui-core/src/widgets/dropdown.rs:19-23], [R crates/lune-ui-core/src/widgets/menu.rs:96-100].
- Shift: the x position is clamped to `[8, window.w - w - 8]` [R crates/lune-ui-core/src/widgets/menu.rs:95]. Wane asks for the same rule: "flips or shifts a surface to stay on screen" [W docs/ui/components.md:707].
- Size limits come from the content: Lunegit's picker popup has a minimum width 220 and a maximum height 320 [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:47-48]; the flyouts set a minimum content width of 220 and 240 [L src/Lunegit.Desktop/Views/DiffView.axaml:110], [L src/Lunegit.Desktop/Views/SidePanels/CommitPanelView.axaml:368].
- Shell: `Surface::Panel` (default) is `popup_panel` (radius 8, fill `dropdown_bg`, 1 px `border` ring) [R crates/lune-ui-core/src/widgets/mod.rs:200-203]; `Surface::Glass` is the menu's glass (`glass_panel`: the backdrop, `menu_glass`, the `menu_popup_border` ring, radius 9) [D143]; `Surface::None` draws nothing. A closing popover replays what its content drew, the backdrop included, with the fade; Lunegit's glass shells are a layered pair of borders with 1 px and 4 px padding [L src/Lunegit.Desktop/Views/DiffView.axaml:26-40], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:300-303].
- Layer `Popup`, above `Page`; the layers beneath it go to the offscreen scene so a glass shell can blur them [R crates/lune-ui-core/src/scene.rs:17-26].
- Not placed here: tray-anchored popovers (a separate OS window, `platform.tray-icon`, `overlay.flyout`, [D48]).

### States

| state | look | cite |
|---|---|---|
| closed | nothing drawn | [core.ui] |
| opening | shell fades and pops in (see Transitions) | [R crates/lune-ui-core/src/widgets/menu.rs:45-49] |
| open, preferred side | shell at the anchor with a 4 px gap | [R crates/lune-ui-core/src/widgets/dropdown.rs:21-23] |
| open, flipped | shell on the opposite side of the anchor | [R crates/lune-ui-core/src/widgets/dropdown.rs:19-23] |
| open, shifted | shell clamped 8 px inside the window edge | [R crates/lune-ui-core/src/widgets/menu.rs:95] |
| closing | shell fades out; it takes no input while closing | [R crates/lune-ui-core/src/widgets/menu.rs:51-59,62,69,123] |

Hover, pressed and focus-visible belong to the content; the popover itself is not interactive. A disabled anchor opens nothing (the app does not call it).

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| open | opacity, 4 px upward slide, scale 0.97 to 1 about the top-left corner | pop | fast | cubic-out | fade | [R crates/lune-ui-core/src/widgets/menu.rs:19-22,46-49], [D40] |
| dismiss | opacity, 2 px downward slide, scale 1 to 0.98 | dismiss | instant | ease-in | fade | [R crates/lune-ui-core/src/widgets/menu.rs:51-59] |

The moved menu's open is 0.18 s and its close 0.12 s [R crates/lune-ui-core/src/widgets/menu.rs:21-22]; they map to `fast` and `instant` (motion.md, [motion]). The Reduced tier is opacity only [L .claude/docs/animation-rules.md:157], and Lunegit's popup is opacity only in every tier [L src/Lunegit.Desktop/Themes/Animations.axaml:250-257].
Never animated: the popover's position while it follows a moving anchor (it snaps to the new placement), its content layout, scrolling content.

### Input and focus

- Pointer: a press outside the popover dismisses it (light dismiss) [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:41], [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:57], [W docs/ui/components.md:549]. Whether that press is also delivered to what is under it: open question (below).
- Keys: Escape dismisses the topmost overlay only [W docs/ui/components.md:549], [core.ui]. Other keys go to the focused widget inside.
- Focus loss dismisses: the window losing focus, or focus moving outside the popover [W docs/ui/components.md:707].
- Layer: `Popup`. Hit order: later call is on top within the layer, only the topmost hit gets hover and press [core.ui]. A popover is not modal: lower layers keep hover and keys outside its rect.
- Focus scope: pushed on open; focus moves to the first focusable widget inside, or the anchor keeps it for a tooltip-like popover with no focusable content; Tab cycles inside the scope while open; closing pops the scope and restores focus to the previous widget or the anchor [core.focus].
- Pointer capture: a press that starts inside keeps the pointer until release even if it ends outside [core.ui].

### Retained state

- Transient: the open animation phase, hover and press of its content [core.ui].
- Persistent: none; the popover's content owns its own (a list inside keeps its selection) under keep-alive.
- App-owned: whether it is open, the anchor rect, the content, the preferred side and alignment, the dismiss callback result.
- Exit retention: yes: Full 120 / Reduced 120 / Off 0 ms (the glass blur fades with it); a consumer with no motion of its own (the moved dropdown, [overlay.dropdown]) snaps and retains nothing. The id stays alive after the app stops calling it, draws from retained state and reports `Repaint::Now` until it settles [core.ui], [R crates/lune-ui-core/src/widgets/menu.rs:22]; Reduced uses the shortest token as Lunegit's dialog exit does [L src/Lunegit.Desktop/Services/DialogService.cs:87-92].

### Accessibility

- No role of its own: the role is the content's (`menu`, `list`, `tooltip`, `dialog` for a modal one) [infra.accessibility]. The popover supplies `hidden` for the closed state and the focus scope; the node of its content sets `modal` false.
- Action: dismiss on the content's node closes it [infra.accessibility].

### Strings

None: the popover has no text of its own.

### Gallery scenes

- `dropdown` (existing): the preferred side, below the anchor.
- `dropdown-flipped` (existing): the flip above the anchor.
- `menu-opening` (existing): the open motion at t = 90 ms.
- `popover-below`: a 220 x 120 popover under an anchor, gap 4, settled.
- `popover-shifted`: the anchor 4 px from the right window edge, popover clamped 8 px inside, settled.
- `popover-flipped-clamped`: a tall popover near the bottom that fits on neither side, clamped to the 8 px margin, settled.
- `popover-scale-1`: the same placement at scale 1, settled.
- `popover-closing`: the dismiss at t = 60 ms (half of `instant`).
- `popover-opening`: the open at t = 90 ms (half of `fast`).
- `popover-glass`: the glass surface, settled.

### Painter primitives

- Existing: fill-radii, ring, backdrop, acrylic, xform (the slide and scale), clip-rect.
- New: none.

### Acceptance

- Placement (pure function, known answers from the moved code): window 800 x 600, anchor (100, 100, 200, 32), popover height 120: below, `y = 136`. Anchor `y = 500`: `below + h = 656 > 592`, so flipped, `y = 500 - 4 - 120 = 376`. Anchor `y = 20` with height 700: clamps to 8. Anchor `x = 790`, width 220: shifted to `800 - 220 - 8 = 572`.
- Behaviour (scripted input): a press outside dismisses once; Escape dismisses only the topmost of two stacked popovers; focus loss dismisses; closing restores focus to the anchor; Tab inside never leaves while open [core.ui], [core.focus].
- Exit retention: a popover the app stops calling completes its dismiss, then reports Idle [core.ui].
- Pixel probes at scale 2: gap 8 device px; margin 16 device px; shell radius 16.
- Conformance lines: open and dismiss pass `motion::conformance` at 60 Hz (monotonic, exact end, settled within 180 ms and 120 ms plus one frame); Reduced fades, Off snaps [motion].
- Semantic-node test: the content's node appears while open and is `hidden` once closed; an injected `AccessAction` dismiss closes it.

### Build

- Milestone: M5, M7 step 4 (`Surface::Glass`). Built in M5 (steps 9 and 10; the page is `popover`). The carried limit is settled [D137]: a popover opened above a dialog draws and takes input on the `Raised` layer, chosen once at open; a third level that overlaps its parent shares `Raised` and interleaves (fills under text). Open for `done` (D47): M11 re-checks the whole bar on every entry; the Windows and Linux run time is unverified (the Linux and Windows lint is `scripts/check-targets.sh`).
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions, core.input

## Open questions

- Answered (D101, consumed): does the dismissing press also reach the widget under it? Lunegit's pickers set `IsLightDismissEnabled` and no pass-through option in the files read [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:41]; the modal rule of C4 sends it to the dismiss rule only. M5 decides (default: consumed).
- Answered (D101, clamp and scroll its content): a popover larger than the window on both sides: clamp and scroll its content, or truncate; no source states it.
- Answered (D101): side placement (left and right of the anchor) for sub-menus: none of the Lunegit placements read is a side placement; M5 adds it with menus.

## Notes for the planner

New in M3 (PD9). `overlay.dropdown`, `overlay.menu`, `overlay.tooltip` and `overlay.flyout` should adopt this placement in M5 and keep their pixel-gated looks; the gallery dumps `dropdown`, `dropdown-flipped`, `menu` and `menu-opening` stay at zero delta.
