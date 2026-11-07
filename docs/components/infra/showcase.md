---
id: infra.showcase
title: Gallery showcase
area: infra
status: done
consumers: any
source: apps/gallery/src/showcase/mod.rs, apps/gallery/src/showcase/nav.rs, apps/gallery/src/showcase/settings.rs, apps/gallery/src/showcase/knobs.rs, apps/gallery/src/showcase/labs/mod.rs
owner: —
summary: The gallery's window, one Ui page that hosts every other page behind a searchable sidebar, with global settings, per-component knobs and labs; the live place to see and try the whole toolkit
---

## What it is

`cargo run -p gallery` opens the showcase (`apps/gallery/src/showcase`), a 1600 x 900 glass window of three
panes. On the left a search field and a tree of groups, pages and scenes (Foundation, Widgets, Overlays,
Layout, Data, Text, Platform and Infra, Labs). In the middle the chosen page, live and interactive, on a
canvas that scrolls when the pane is narrower than the page. On the right the knobs of the page, then the
global settings. Choosing a scene replays its script in real time on the page's own starting state and then
leaves the page live. No pictures: everything on screen is the toolkit drawing.

The shell is an ordinary Ui page, so its scenes run headless like any other. A hosted page draws on a
`Stage` (a rect of the window) and lays itself out from the card it is given (D248, D249).

Keys: Cmd or Ctrl `]` and `[` walk the sidebar's pages, Cmd or Ctrl `Q` quits; every other key reaches the page.

## Needs

None from a consumer: this is the gallery's own tool, not a toolkit component. It changes nothing under
`crates/` (D246); what the toolkit would need to show more is in `docs/STATE.md`, Next.

## Existing code

`apps/gallery/src/showcase/mod.rs` (the shell: panes, hosting, scene replay), `nav.rs` (the groups, the tree
rows and the search), `settings.rs` (the global settings), `knobs.rs` (the knob rows), `labs/` (the seven labs).
`PageState` gained the hooks `knobs`, `tier`, `frame_params`, `material`, `time_scale`, `zoom`, `inject` and
`epoch` (`apps/gallery/src/page.rs`); `window.rs` applies them.

## Spec

### Global settings (right pane, under the knobs)

Theme (the preset and the five Lunegit files), Motion (tier, speed 0.1x to 2x, the five durations and the
tooltip delay), Zoom (1x, 1.5x, 2x), Glass (per layer: tint, tint opacity, material opacity; the acrylic
level; a noise strength slider (0 is off) and the four values of the luminosity curve (applied to the three layers); the floating surfaces' tint, opacity and border: `menu_glass` and `menu_popup_border`, D8), Blur (sigma to 64, saturation, the kernel cap and the working size 1/2, 1/4 or 1/8), OS material (the five kinds), Reset.
They outlive a change of page and go back only by Reset. Each reaches a frame through `FrameTweaks` or the composed theme (D268); a page with its own theme keeps these
overrides, as the glass layers do.

### Knobs (right pane, first)

A page declares its knobs in `PageState::knobs`, each row bound to a field of the page (`Knobs::toggle`,
`choice`, `number`, `text`, `button`). A knob's starting value is what the page drew without it, so the scenes
did not move. A page that was a gallery of fixed rows gains a sample the knobs shape. Knobs that open an
overlay come last: an open modal blocks the pane. Every page of the sidebar has knobs except four, named in
`knobs::tests::NO_KNOBS` with their reasons:

- `locale`: its dialog is modal and always open, so the pane could not be pressed.
- `shortcuts`: both menus are open from the start and a press outside dismisses them before it reaches the pane.
- `lab-tokens`: it lists the running theme; the settings pane's theme and glass controls are its knobs.
- `lab-clipboard`: its Copy and Paste buttons are the options.

`ime`, `file-drop` and `tray` say on the showcase's stage what is real OS input and what is scripted.

### Labs

Glass (the three layers over stripes and colour), Blur (a backdrop over moving content; it owns the frame's one
backdrop), Effects (custom draw, glass off, the tab outline), Motion (the 15 kinds and 7 easings,
each replayed on a press), Tokens (every theme colour, size and radius), Clipboard (copy and paste on a press
only), JSON (the bounded reader on typed text).

### Gallery scenes

- `showcase-home`: the shell on its first page, the Foundation group open.
- `showcase-search`: a search typed into the field narrows the tree.
- `showcase-settings`: the settings pane under another theme, a reduced tier, half speed, a wider blur and no noise.
- `showcase-lab-motion`: the motion lab, chosen by a cue: the seven easings and fifteen kinds at rest.
- `showcase-table`: a search for the table, then a press on its row, which takes the content pane.

### Build

- Milestone: gallery-showcase, steps 1 to 9. The decisions are D246 to D254.
- Done (D47, M11): tests `apps/gallery/src/page.rs::every_ui_page_draws_inside_its_stage, apps/gallery/src/showcase/mod.rs::a_click_on_a_tree_row_selects_it, apps/gallery/src/showcase/mod.rs::a_hosted_page_never_paints_over_the_settings_pane, apps/gallery/src/showcase/mod.rs::a_page_that_changes_layer_leaves_the_sidebar_and_settings_unclipped, apps/gallery/src/showcase/mod.rs::a_click_on_a_setting_changes_it_and_a_page_change_keeps_it, apps/gallery/src/window.rs::half_speed_halves_the_time_that_passes_and_a_change_never_jumps_it, apps/gallery/src/window.rs::zoom_keeps_the_pointer_on_the_pixel_it_is_over, apps/gallery/src/showcase/knobs.rs::a_knob_on_each_widget_page_changes_the_page, apps/gallery/src/showcase/knobs.rs::every_page_of_the_sidebar_has_a_knob_or_a_reason_not_to, apps/gallery/src/showcase/labs/clipboard.rs::the_clipboard_is_touched_on_a_press_only`; page `showcase` (scenes `showcase-home`, `showcase-search`, `showcase-settings`, `showcase-lab-motion`, `showcase-table`); unverified here: clicks in a real window (the tests drive the shell with scripted pointer events and `Translator` calls), a live swap of the OS material on a running window (`material::apply` on change, its error shown in the pane), the feel of speed and zoom, the IME candidate window under zoom, Copy and Paste against the OS clipboard, and the look of the longest knob panels.
