---
id: platform.tray-icon
title: Tray icon with status dot
area: platform
status: planned
consumers: wane
source: —
owner: —
summary: 16 px monochrome glyph with a status dot (green serving, amber degraded, grey paused), rendered to OS bitmaps at 1x and 2x; opens toolkit glass menus
---

## What it is

The notification-area icon (Windows) or menu-bar status item (macOS): a 16 px monochrome glyph with a
6 px status dot at its bottom-right corner (green serving, amber degraded, grey paused). The OS draws
the tray slot; the toolkit supplies the icon as OS bitmaps at 1x and 2x, rendered from the same vector
with the dot composited in, plus the tooltip text, and it opens its own glass menu or flyout from a
click, anchored to the icon's rectangle (D20, D48). Left click opens the flyout (Windows) or the menu
(macOS); right click opens the menu on Windows. Close-to-tray policy, single instance, autostart and
OS notifications are app-side (D43).

## Needs

- Tray icon with status dot, its glass flyout and glass menu in tray-anchored popover windows (R-platform-6).
- Placement that keeps a surface on screen and dismisses on outside click and focus loss (R-overlay-4): `overlay.popover`.
- Wane — Variants: `--2x` (the same icon at 32 px for high-DPI trays) (R-platform-6)
- Wane — States: serving (`__dot--ok`), degraded (`__dot--warn`), paused (`__dot--paused`). The three differ only by the dot colour; the glyph stays monochrome. (R-platform-6)
- Wane — Sizes: 16 px glyph; 6 px dot at right -2 / bottom -2 with a 1.5 px cut-out ring in the surface colour; `--2x`: 32 px glyph, 12 px dot, 3 px ring. The glyph is the provisional crescent, not a Codicon. (R-platform-6)
- Wane — Behaviour: Left click opens the flyout (Windows) or the menu (macOS); right click opens the menu on Windows. The tooltip names the state in words, the second channel for colour-vision. Not mocked: animation, badge counts. (R-platform-6)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design sources (not code to move): Wane `docs/ui/mocks/css/components.css:691-701` (the icon), `docs/ui/mocks/css/tokens.css:12,49,52,62,97` (colours), `docs/ui/mocks/tray.html?surface=icon-states` (the states, scene `tray-icon-states`, `docs/ui/mocks/scenes.txt:33`),
`docs/mockups/Tray.dc.html:44` (the provisional crescent).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Wane docs/ui/components.md:207-219,571-583,702-717,750-758, docs/ui/mocks/css/components.css:689-701, docs/ui/mocks/css/tokens.css:12,49,52,62,97,107, docs/ui/mocks/scenes.txt:30-33;
this repo docs/DECISIONS.md D20 and D43 (as written in the roadmap; the repo rows D43 and D48 land with M3 step 1), crates/lune-ui-core/src/chrome.rs:1-20 (arcs flattened to a polyline). Lunegit has no tray icon or flyout (grep for `flyout|tray` over `src/Lunegit.Desktop` finds only context and split-button flyouts), so Wane is the only design source.
Conflict: none between sources. D20 against the mock: the mock draws the surfaces in a web page; the toolkit draws the menu and flyout as toolkit glass in their own OS windows, placed from the tray rect (D20, D48).

### Anatomy

- Glyph: 16 px square, monochrome, colour `--text-primary` (`#ffffffcc`, the dark preset's `fg_primary`) [W docs/ui/mocks/css/components.css:694], [W docs/ui/mocks/css/tokens.css:12], [R crates/lune-ui-core/src/theme.rs:56]. OS tray icons are monochrome [W docs/ui/mocks/css/components.css:691]. The glyph is the provisional crescent, one stroke 1.5 px at 16 px; the final mark is the user's call and nothing here depends on it [W docs/ui/components.md:752]. A path of arcs is flattened to a polyline and stroked, as `chrome.rs` does for the tab outline [R crates/lune-ui-core/src/chrome.rs:1-4].
- Dot: 6 px diameter, a 1.5 px cut-out ring around it in the surface colour, so the outer circle is 9 px; positioned `right: -2px; bottom: -2px` from the 16 px box; the fill is clipped to the inner circle so no tone bleeds into the ring [W docs/ui/mocks/css/components.css:691-697].
- Dot colours: serving `--ok` `#6fd39a`, degraded `--warn` `#f2a65a`, paused `--paused` `#8a9099`; the ring defaults to `--tray-ring`, falling back to `--glass-inactive` `#0b0b0b` [W docs/ui/mocks/css/tokens.css:49,52,62,97], [W docs/ui/mocks/css/components.css:696-699].
- High-DPI variant `--2x`: 32 px glyph, 12 px dot, 3 px ring, dot at `right: -4px; bottom: -4px` [W docs/ui/mocks/css/components.css:700-701]. Both sizes are produced from the same vector at 1x and 2x.
- Overhang: the dot's outer circle (9 px at 1x, 18 px at 2x) reaches 2 px (4 px at 2x) past the 16 px glyph box on the right and bottom, since the dot sits at -2 px [derived: 9 = 6 + 2 * 1.5; `right: -2px` puts the outer edge 2 px outside]. A real bitmap holds the dot inside its bounds; the bitmap size or the glyph scale is an Open question.
- Rendering: the icon is drawn by the painter into an offscreen RGBA target at 16 and 32 px and handed to the OS as a bitmap; the OS draws the slot [W docs/ui/components.md:218]. On macOS D20 leaves the form to M10: a template glyph with the dot drawn separately, or a non-template image matched to the menu-bar appearance, because a coloured dot cannot survive a template image [D20].
- The ring colour: the mock's ring is the taskbar or menu-bar colour, which a real icon cannot know, so the dot needs a ring that works on light and dark trays [W docs/ui/components.md:218].
- Tooltip: the app passes a string that names the state in words ("Wane · serving on :4646", "Wane · anthropic degraded", "Wane · paused", from the fixtures); it is the second channel for colour vision, since ok and warn are close in lightness; the native tooltip itself is OS chrome [W docs/ui/components.md:213,756,758].
- Click behaviour: left click opens the flyout (Windows) or the menu (macOS); right click opens the menu on Windows [W docs/ui/components.md:213].
- Placement of the toolkit's glass surfaces, anchored to the tray icon's rectangle and kept on screen: macOS menu 4 px under the menu bar, flush right with the screen edge (a menu aligned to the status item's left edge would leave the screen); Windows flyout 12 px above the taskbar and 12 px from the right edge; Windows right-click menu 8 px above the taskbar [W docs/ui/components.md:707,757], [D20], [D48]. Each is a borderless, always-on-top, non-activating glass window; `overlay.popover`'s flip and shift rules keep it on screen and its dismiss rules apply (outside click, focus loss) [W docs/ui/components.md:582], [overlay.popover].
- Fallback: if M10's probe cannot make a non-activating popover window on macOS, a native menu on macOS only, with a DECISIONS row [D20].

### States

| state | look | cite |
|---|---|---|
| serving | glyph with a green dot `#6fd39a` | [W docs/ui/mocks/css/tokens.css:49], [W docs/ui/mocks/css/components.css:697] |
| degraded | glyph with an amber dot `#f2a65a` | [W docs/ui/mocks/css/tokens.css:52], [W docs/ui/mocks/css/components.css:698] |
| paused | glyph with a grey dot `#8a9099` | [W docs/ui/mocks/css/tokens.css:62], [W docs/ui/mocks/css/components.css:699] |
| 1x and 2x | the same icon at 16 px, or 32 px with the 12 px dot and 3 px ring | [W docs/ui/mocks/css/components.css:694,700-701] |
| hover, pressed, focus-visible | the OS highlights the tray slot; the toolkit draws none | [W docs/ui/components.md:218] |
| disabled | not applicable (no such tray state in the source) | [derived: the states list has three] |

### Transitions

n/a — the source marks animation as not mocked [W docs/ui/components.md:213]; a state change swaps the bitmap at once (the OS has no cross-fade for tray icons) [derived: the icon is an OS bitmap]. The flyout and menu open with the menu motion (`overlay.flyout`, `overlay.menu`).

### Input and focus

- Pointer: a left click on the icon opens the flyout or the menu; a right click opens the menu on Windows; a second left click on the open flyout closes it [W docs/ui/components.md:213,577]. The click arrives from the OS tray API (the app's tray bridge), with the icon's rect.
- Keys: none of the toolkit's; the OS reaches the tray by keyboard (Windows notification-area focus) [derived: OS behaviour; not specified by either source].
- Layer: none (OS); the surfaces it opens are popover windows [overlay.popover]. No focus scope on the icon.

### Retained state

- Transient: none in the toolkit. Persistent: none. App-owned: the state (serving, degraded, paused), the tooltip text, the tray rect, whether a surface is open.
- Exit retention: no.

### Accessibility

n/a — the OS exposes the item; the tooltip string is its accessible name and carries the state in words [W docs/ui/components.md:756]. The surfaces it opens have their own nodes (`overlay.menu`, `overlay.flyout`).

### Strings

None: the tooltip texts are app strings [W docs/ui/components.md:758].

### Gallery scenes

- `tray-icon-serving`, `tray-icon-degraded`, `tray-icon-paused`: the icon at scale 1 and 2 on the surface colour, settled (the dump writes `<id>@1x.png` and `<id>@2x.png`).
- `tray-icon-light-tray`: the paused icon on a light surface, to check the ring (Open questions), settled.
- `tray-icon-placement`: a screen rectangle with the menu bar, the macOS menu 4 px under it flush right and a status item near the right edge, settled; and `tray-icon-placement-windows`: the taskbar with the flyout 12 px above it and the menu 8 px above, settled.

### Painter primitives

- Existing: fill-radii (the dot and ring as circles), ring, stroke (the glyph), fill-over, xform (the 2x scale).
- New: none (the glyph is a flattened polyline; the circles are rounded fills).

### Acceptance

- Pixel probes (offscreen, scale 1 and 2): at 1x the dot's inner circle is 6 px across and filled `#6fd39a`, `#f2a65a` or `#8a9099` for the three states, the ring is 1.5 px in the ring colour, the glyph box is 16 px; at 2x 12 px, 3 px and 32 px [W docs/ui/mocks/css/components.css:694-701], [W docs/ui/mocks/css/tokens.css:49,52,62]. The three icons differ only in the dot pixels.
- Placement (pure function, known answers): menu bar bottom at y 24 (an example height), the macOS menu's top is 28 and its right edge is the screen's right edge; taskbar top at y 1040 on a 1080 screen, the flyout's bottom is 1028 and its right edge 12 px inside the screen's right edge, the menu's bottom is 1032 [W docs/ui/components.md:707,757].
- Behaviour (fake tray, scripted clicks): a left click opens the flyout and a second closes it; an outside click and focus loss close it; a right click on Windows opens the menu [W docs/ui/components.md:577].
- Real tray behaviour on macOS and Windows (icon in the OS slot, template images, non-activating windows) is unverified here: no GUI run, no VM (D15); Wane's hands-on run covers it later.
- Conformance: n/a, no transitions. Semantic node: n/a, the OS owns the item.

### Build

- Milestone: M10
- Depends on: core.painter, render.frame, overlay.flyout, overlay.menu, overlay.popover, platform.material, platform.window

## Open questions

- Bitmap size and dot placement: the mock's dot overhangs the 16 px glyph box by 2 px; M10 decides the bitmap size (for example 18 px) or the glyph scale.
- The ring colour for a light and a dark tray: the mock uses the window grey; a real icon cannot know the tray colour [W docs/ui/components.md:218].
- Template versus non-template image on macOS (D20): decided at M10 by a probe.
- The final glyph (the user's call) [W docs/ui/components.md:752].
- Which upstream states turn the tray amber is a product rule, not the toolkit's [W docs/ui/components.md:755].

## Notes for the planner

Wane priority P2. Placement values and the three states come only from Wane; Lunegit has no tray. Close-to-tray, single instance, autostart and notifications are app-side (D43, Q1).
