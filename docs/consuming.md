# Consuming the toolkit

How an app takes luneui-rs in before a remote exists: a vendored snapshot, written by
`scripts/export-snapshot.sh` from a git revision (DECISIONS D13, roadmap D16). Once a remote exists
the app switches to a git dependency on the same crates and the `vendor/` directory goes.

## What the snapshot holds

`scripts/export-snapshot.sh <dest> [--rev <rev>]` (default `HEAD`) runs `git archive` of an allow list
at `<rev>`; the working tree is never read, so a dirty tree changes nothing and `SNAPSHOT` (one line,
the full sha) is true:

| in | why |
| --- | --- |
| `Cargo.toml` with `members = ["crates/*"]`, `rust-toolchain.toml` | the workspace the crates inherit their version, edition and lints from; `members` is rewritten because the repo's `apps/*` and `tools/*` are not shipped |
| `crates/*/Cargo.toml`, `src`, `assets` | the five crates: `lune-ui-core`, `lune-ui-wgpu`, `lune-ui-winit`, `lune-ui-platform` and the facade `lune-ui` |
| `LICENSE-*`, `THIRD_PARTY_NOTICES.md`, `licenses/` | licence text travels with the code (hard rule 6); `licenses/` goes whole, `lunegit-fixtures.txt` included: it is our note and holds no Lunegit content |
| `SNAPSHOT` | the revision |

| out | why |
| --- | --- |
| `apps/` (the gallery), `tools/`, `docs/` | not the toolkit |
| `crates/*/tests` | the tests of `lune-ui-wgpu` read the gallery's fonts, so they cannot compile in a snapshot; the repo's own gate runs them |
| `Cargo.lock` | the consumer's lock rules; ours names packages the snapshot does not hold |
| `apps/gallery/assets/lunegit/` (Lunegit's theme and locale files) | gallery test data, not toolkit code (D206; licensed MIT OR Apache-2.0 since D284). The script fails (exit 1) on any staged path under `assets/lunegit`, and on any exported file whose blob equals a fixture's, under any name |

Exit codes: 0 done, 1 a check failed (the destination is as it was), 2 usage or setup (also a
non-empty destination that is not an earlier snapshot, left untouched). A destination holding a
`SNAPSHOT` is replaced whole, after the new tree has passed every check.

## Vendoring it

```toml
# the app's workspace Cargo.toml
[workspace]
exclude = ["vendor/luneui"]          # the snapshot is its own workspace root

[workspace.dependencies]
lune-ui = { path = "vendor/luneui/crates/lune-ui" }   # the facade; features: accesskit
```

Depend on the facade `lune-ui`: it re-exports the four adapters under the paths a consumer uses
(`lune_ui::gpu`, `frame`, `surface`, `custom`, `dump`, `window`, `input`, `text`, `access`, `tray`,
`popover`, `material`, `testing`, `lunegit_dark`, ...). The snapshot builds as a path dependency with
`cargo check --offline` from a scratch consumer, and `cargo metadata --offline` passes inside it.

**The version is 0.2.0 (D238).** Luneterm's own in-tree `crates/lune-ui` is 0.1.0; two packages named
`lune-ui` at one version cannot stand in one lock. At 0.2.0 they can, so a port goes crate by crate
instead of in one commit. Keep the in-tree crate until the last importer has moved, then delete it.

The snapshot carries no tests: run the toolkit's gate in the toolkit's repo.

## What the app does

The toolkit has no composition root. It never starts an event loop and never reads the clock or the
environment. The app:

- builds the event loop and the windows, and wires the adapters: `gpu::instance(Some(Box::new(
  event_loop.owned_display_handle())))`, `Gpu::new(instance, Some(&surface))`, a `WindowSurface` per
  window, a `FrameRenderer` per target, one `TextCache` for the fonts it supplies;
- supplies fonts, time (`RawInput.time`), sizes, scale, the `Theme` (`lunegit_dark::THEME` or one
  loaded with `Theme::from_json`), the string table and the shortcut table, every frame;
- turns `FrameOutput` into window and OS actions (`WindowCommand`, cursor icon, clipboard, repaint
  request, accessibility tree); the toolkit calls none of them;
- keeps `Gpu::end_frame(&mut text)` once per frame after every target drew;
- rebuilds `Gpu`, `TextCache`, every `FrameRenderer`, every `WindowSurface` and every registered image
  when `Gpu::is_lost()` says the device is gone (it reports, it does not recover; D241);
- enables the facade's `accesskit` feature to bridge accessibility;
- for a tray item: calls `tray::set_event_handler` first and once, creates the `Tray` on the main
  thread after the event loop has started, and on macOS builds the event loop with
  `with_activate_ignoring_other_apps(false)` when a popover must not take the front
  (`docs/components/platform/tray-icon.md`).

`apps/gallery` is the worked example: `apps/gallery/src/window.rs` (the window, the surface, the frame
loop), `popover.rs` (a tray host), `smoke.rs` and `main.rs`.

## Unverified here

The tests run against scripted input and fake adapters. What only a real OS shows is not ticked
anywhere: the OS materials on real machines, transparent windows and round corners, snap layouts, the
tray item's placement and the popover's focus behaviour, input-method composition, and a presented
frame. `docs/STATE.md` keeps the list; the platform smoke (`gallery --smoke-platform`) checks the tray
and the popover on a logged-in session.

## Release size

An app measures its own. The toolkit's figure for the gallery's release build (`cargo build --release -p gallery`,
the repo's `[profile.release]`: fat LTO, one codegen unit, abort on panic, stripped; the size of `target/release/gallery`
on macOS, read with `wc -c`) is **9,869,520 bytes** at a9d1292, the M11 close. It was 9,448,688 at the M9 merge (8f499d0)
and 9,869,408 at the M10 tip; the M10 growth is the AccessKit bridge, the tray stack and `arboard` (DECISIONS D216, D213).
The figure covers the gallery with every feature on, including `accesskit`; an app that leaves the feature off is smaller.
`docs/STATE.md` keeps the latest reading.

## Migration map: Luneterm 296cfdc `lune-ui` to the snapshot

Built from `docs/architecture.md` "API changes since M1" (the log; this map links it) and from the
compile errors of Luneterm's `glass-ui` spike. Run in a scratch clone under this repo's `target/`
(Luneterm itself is not touched; it adopts in its own M9): Luneterm at 296cfdc, the snapshot
(`scripts/export-snapshot.sh`, revision `2f066e9`) in `vendor/luneui`, the workspace manifest edited as
above.

- `cargo metadata --offline` passes.
- `cargo check --offline -p lune-termview --all-targets` passes **with no source change**:
  `lune-termview` imports only `lune_ui::custom::{CustomDraw, CustomFrame}`, `paint::Rect`,
  `color::Rgba` and `dump::{target, read_rgba, write_png}`, and the facade keeps those paths.
- `cargo check --offline -p glass-ui --all-targets` fails with 24 errors in each of its two targets,
  the same 24. A fixed error can expose more (the spike was not ported); every one of the 24 has a row.

Lines are the spike's, at 296cfdc.

| old (Luneterm 296cfdc `lune-ui`) | new | note |
| --- | --- | --- |
| `gpu::instance(event_loop.owned_display_handle())` (`main.rs:99`) | `gpu::instance(Some(Box::new(event_loop.owned_display_handle())))` | the argument is `Option<Box<dyn WgpuHasDisplayHandle>>`; `None` is a headless instance |
| `Gpu::new(window.clone(), &instance)` (`main.rs:100`) | `let surface = instance.create_surface(window.clone())?; Gpu::new(instance, Some(&surface)).await` | `Gpu` no longer holds a window or a surface; it takes the instance by value |
| `gpu.config.format`, `gpu.alpha_mode` (`main.rs:103,104`, `dump.rs:164`) | `WindowSurface::new(&gpu, surface, (w, h))?`, then `surface.format()`, `surface.alpha_mode()` | the surface is its own type; `dump.rs` draws offscreen and may use any `wgpu::TextureFormat` |
| `TextCache::new(scale, fonts, ui_family, mono_family)` (`main.rs:107`) | `TextCache::new(&fonts, ui_family, mono_family)` | no scale at creation |
| `text.set_scale(s)` (`main.rs:171,384`) | gone | the scale is an argument of `TextSystem::prepare` and `measure`, which the `Painter` passes (`Painter::new(s, win, &mut text, hover, theme)`) |
| `FrameRenderer::new(&gpu)` (`main.rs:128`) | `FrameRenderer::new(&mut gpu, surface.format())` | a renderer per target and colour format |
| `gpu.config.format` for `TermState::new` (`main.rs:130`) | `surface.format()` | |
| `r.gpu.window` (`main.rs:162,343`) | keep the `Arc<Window>` in the app's own state | `Gpu` holds no window |
| `r.gpu.resize(w, h)` (`main.rs:167`) | `surface.resize(&gpu, w, h)` | on the `WindowSurface` |
| `edge_at(..)` returning winit's `ResizeDirection` (`main.rs:186`) | `edge_at(..)` returns `ResizeEdge`; `window::resize_direction(edge)` maps it for `window.drag_resize_window` | the core has no winit type |
| `r.gpu.acquire()?` returning `Option<frame>` (`main.rs:363`) | `surface.acquire(&gpu)?` returning `Acquired::{Frame(texture), Skip, Redraw}` | `Skip`: nothing to draw (hidden or minimised); `Redraw`: the surface was reconfigured, ask for another frame |
| `r.gpu.config.width`, `.height` (`main.rs:367`, two errors) | `surface.size()` | physical px |
| `r.gpu.present(frame)` (`main.rs:369`) | `surface.present(&gpu, frame)` | |
| `r.frame.end_frame(&mut text)` (`main.rs:370`, `dump.rs:169`) | `gpu.end_frame(&mut text)` | once per app frame, after every target drew |
| `let Painter { layers, hits, backdrop, .. } = p` (`main.rs:391`) | `let Painter { scene, hits, .. } = p` | the painter records a `Scene` (`scene.layers`, `scene.backdrop`), not GPU geometry |
| `frame.draw_with(&gpu, &mut text, layers, backdrop, target, params, &mut customs)` (`main.rs:409`) | `frame.draw_with(&mut gpu, &mut text, &scene, target, params, &mut customs)` | 6 arguments: the scene replaces layers and backdrop |
| `frame.draw(&gpu, &mut text, layers, backdrop, target, params)` (`main.rs:421`) | `frame.draw(&mut gpu, &mut text, &scene, target, params)` | 5 arguments |
| `term.metrics(p.text, s, p.s)` with `p.text: &mut TextCache` (`terminal_view.rs:201`) | pass the app's `TextCache` itself, not `p.text` | `Painter::text` is `&mut dyn TextSystem`; `lune-termview` still takes a `TextCache` and is unchanged |

Not errors, but new since 296cfdc and worth a look when porting: `Theme` gained fields (a
`Theme { .. }` literal outside the toolkit needs `glass`), `Event` gained the file-drop variants (the
enums are `#[non_exhaustive]`: match with a wildcard), and `lune_ui::testing` holds the fakes.

## API changes of the library pass

The library pass after the gallery showcase (DECISIONS D255 to D282) changed the public API with no version bump (the plan's D24): Luneterm pins the toolkit by git rev (`lune-ui` at `5216931`, `Cargo.toml:16`), not by version, so
the version stays and this list is the migration map for it. The log with the reasons is `docs/architecture.md`, "API changes since M1", "Since the library pass".
A consumer that has not yet adopted the toolkit needs none of it. One that vendored a snapshot from before `1340d15` reads the rows below.

| old | new | note |
| --- | --- | --- |
| `scene.layers[LayerId::Popup as usize]` and the other fixed layers hold every overlay's drawing | `scene.layers[id]` holds what was drawn outside any overlay; `Scene::overlays` holds each overlay's own `Layer`; `Scene::in_order()` is the draw list, `Scene::merged(id)` folds a fixed layer and its overlays | a renderer or recorder walks `in_order()` (D256) |
| `LayerId::Toast` is 5, `LAYER_COUNT` 6 | `LayerId::Outside` is 5, `Toast` 6, `LAYER_COUNT` 7 | code that indexes by number moves; `Ui::outside_modals` runs a closure on `Outside` (D259) |
| `Scene::backdrop`, `Scene::split`, `BackdropRequest::layer`; one blur a frame, the last request wins | `Layer::backdrops`, `Scene::backdrops()`, `Scene::floor()`; a blur per request at its layer, every request stands | D257 |
| `Painter::acrylic` anywhere, replacing what is below | on the fixed `Base` layer only; anywhere else it draws as `Painter::frost` (`frost`, `frost_radii`) | a backdrop request now replaces its region (`mix(sharp, blurred, ..)`), so nothing opens a hole to the desktop (D258) |
| `Painter::on` drops the clip stack | `Painter::on` changes only the layer; overlays (`enter`) still clear it | a `pop_clip` across a layer change restores what its push replaced (D265) |
| `FrameParams { size, scale, blur_sigma, saturation }` | add `noise_opacity` and `blur: BlurLimits`, or start from `FrameParams::default()` | the noise strength and the blur's kernel cap and working size were constants (D264) |
| `GlassLayer { .. }` literal | add `curve: LuminosityCurve::AVALONIA` | `GlassLayer::premultiplied()` keeps its signature (D264) |
| `OutputState::apply(window, out)` | `OutputState::apply(window, out, ui_scale)` | the input-method area leaves in physical pixels (D266) |
| `caption_rect` placing from x = 0, y = 0 | places from the window's own origin | the gallery's `caption_in` workaround (a translate) is gone (D266) |
| `Ui::flyout_window(..)`, `Ui::menu_window(..)` in the painter's window | the same, plus `Ui::flyout_window_in(key, area, ..)` and `Ui::menu_window_in(key, area, ..)` for an area of the window | D266 |
| a registration-order Tab sequence | `Ui::tab_scope(order, f)` | D267 |
| `UiStore::prev_hits` (crate-private) | `testing::hit_regions(&UiStore) -> Vec<HitRegion>` | D266 |
| `DialogGlass { fill, panel, noise }` | `DialogGlass { fill, noise }`, `DialogGlass::of(&Theme)` | D261 |
| `DialogOptions::scrim_blur` false by default | true by default | D261 |
| `Surface::Panel`, `popup_panel`, `DropdownOptions::glass`, `Theme::dropdown_bg`, `toast_bg`, `toast_border` | gone: `menu_glass` and `menu_popup_border` drive every floating surface (`popover::glass_surface`) | the keys `BackdropDark`, `ToastBackground` and `ToastBorder` load as unknown (in `skipped`) (D260, D261) |
| `Theme::window_fallback`, `lunegit_dark::INACTIVE_FALLBACK`, `active` on `PageOptions`, `ToolbarOptions`, `SidePanelOptions`, `SettingsPage`, `BoardOptions` | gone: an inactive window stays glass | the key `WindowFallbackBackground` loads as unknown (D262) |
| `RingKind::PrimaryButton`, `Edge::on_accent`, `Theme::on_accent_text`, `accent_hover` | gone | the buttons follow Lunegit's dialog buttons: primary on `surface_input`, secondary clear, press scale on the ghost only (D269) |
| `lunegit_dark::TAB_HEADER_HEIGHT`, `BODY_PADDING` | gone | read by nothing in the repo (D276) |
| `Tab::progress` draws a spinner; `SPINNER_SIZE`, `SPINNER_MARGIN` | `Tab::progress` shines the title in that tone; the constants are gone | the tab is no wider (D270) |
| `TreeOptions { guides, .. }` | add `guide_style: GuideStyle`, `chevron: ChevronStyle`, `stub: GuideStub` (`..Default::default()`) | `Theme::tree_guide` colours the guides (D271) |
| a text selection drawn by each widget | `widgets::selection::paint(p, rows, open, color)` over `Painter::fill_joined`; `Theme::selection_radius`, `selection_pad` | D272 |
| `KvOptions { .. }` | add `copy: bool` (off by default) | D275 |
| `SidePanelOptions { .. }` | add `card_moves: bool` (off by default) | D281 |
| `ScrollOptions { .. }` | add `bar_room: (f32, f32)` (`..Default::default()`) | the scroll bars keep clear of a control that shares their corner (D279) |
| `RowHeights` of an edited document keep stale heights | `RowHeights::splice(at, removed, added)`; `EditLayout::extend_hit` is a provided method | D279, D280 |
| a `Theme { .. }` literal | add `tree_guide`, `selection_radius`, `selection_pad`; drop the gone fields above | the theme file has 47 colour keys, `MenuGlassBackground` among them |
| a `match` on `Material` with no wildcard | add an arm for `Material::Custom(OsMaterial)`, or a wildcard; `Material::os_material(self)` gives every variant's descriptor | additive: the five names keep their meaning (D285) |
| a `match` on `Skip` with no wildcard | add arms for `UnknownGlass(String)`, `BadGlass(String)`, `BadMaterial(String)` | the theme file's `glass` section (D286) |
| a `ThemeFile { .. }` literal | add `material: None` | `ThemeFile.material: Option<Material>` is the file's `glass.material`; it is not in `Theme` (D286) |

Behaviour that changed with no signature change, worth a look when porting: the hold on `Raised` (an overlay drops to the layer it asked for when no older modal is
left, D255) and one order for "above" (D263); a key that arrives before a release in one frame belongs to the press (D278); a hit inside a cluster of several glyphs
goes by the cluster's midpoint (D280); an overlay opened in a disabled scope draws dimmed (D277); a ring's width follows the transform's scale (D277). Every scene
the gate moved is listed with its reason in D282.

## Your own glass look

A look's glass is data (D285, D286): a theme file with a `glass` section is the whole look, and no toolkit change is needed for a new one. `Theme::from_json(bytes, &base)` returns a `ThemeFile`: `theme` is the drawing values, and `material` is the OS material of `glass.material`, which the app gives its window with `lune_ui::material::apply(window, material)` (or `WindowBackdrop`). The material is not in `Theme`.

Schema. Keys are camelCase and case-sensitive; every `glass` member is optional; numbers are in [0, 1]; a colour is `#RRGGBB` or `#AARRGGBB`. The file around `glass` is the usual theme file, which requires `colors` and `acrylic` (empty objects are enough; `Theme::from_json` returns `Err(Missing("colors"))` without them).

```json
{
  "name": "My look",
  "colors": {},
  "acrylic": {},
  "glass": {
    "window": { "tint": "#000000", "tintOpacity": 1, "materialOpacity": 0.1, "blackMaxOpacity": 0.6 },
    "panel":  { "tint": "#000000", "tintOpacity": 1, "materialOpacity": 0.1, "blackMaxOpacity": 0.55 },
    "card":   { "tint": "#000000", "tintOpacity": 1, "materialOpacity": 0.1, "blackMaxOpacity": 0.65 },
    "material": {
      "macos":   { "kind": "vibrancy", "effect": "UltraDark", "state": "Active" },
      "windows": { "kind": "acrylic", "tint": "#A0000000", "dark": true }
    }
  }
}
```

- A layer (`window`, `panel`, `card`) takes `tint`, `tintOpacity`, `materialOpacity`, and the luminosity curve: `midPoint`, `whiteMaxOpacity`, `midPointMaxOpacity`, `blackMaxOpacity`.
- `material` is one of the strings `"Vibrancy"`, `"VibrancyFollowsWindow"`, `"LiquidClear"`, `"LiquidRegular"`, `"Opaque"`, or an object `{ macos?, windows? }`:
  - `macos`: `{ "kind": "vibrancy", "effect", "state": "Active" | "Inactive" | "FollowsWindow", "appearance"?: "Dark" | "Light" }`, where `effect` is one of the names in `VIBRANCY_EFFECTS` (`crates/lune-ui-core/src/theme/file.rs`, e.g. `UltraDark`); or `{ "kind": "liquid", "style": "Clear" | "Regular", "tint"?, "opaque"? }`.
  - `windows`: `{ "kind": "acrylic", "tint"?, "dark"? }`, `{ "kind": "mica", "dark"? }` or `{ "kind": "tabbed", "dark"? }`.
- Faults are not fatal. A bad layer member is skipped and listed in `ThemeFile.skipped` by its path (`Skip::BadGlass("window.blackMaxOpacity")`, `Skip::UnknownGlass`); the other values apply. The material is all-or-nothing: any fault inside `glass.material` gives `material: None` and one `Skip::BadMaterial`, never a partial material. A file with no `glass` key loads as it did before.
- Tuning the built-in look: `lune_ui::near_black` (`crates/lune-ui-core/src/theme/near_black.rs`) is the one file. Every constant has a comment that says which way raising it moves the look. Its tests pin the approved values; a deliberate retune edits them too.

**Where no code changes stops.**

- Covered as data: every macOS `NSVisualEffectMaterial` name, state and forced appearance; liquid style, tint and opacity; Windows acrylic tint and dark flag, Mica and Tabbed; each layer's tint, opacities and curve. The header tint is an app value (the gallery's `near_black::HEADER_TINT`), not a file key.
- Still needs toolkit code: a macOS or Windows material `window-vibrancy` does not expose; a new blending mode; Windows `apply_blur` (not exposed; the Windows-10-era path); a new platform.
- Not settable by anyone: macOS's blur strength (the OS fixes it per material); the tint on Windows builds 22523 and later (the OS ignores it). The app-drawn blur sigma is already a per-frame value the app passes.
- A value a platform cannot honour is an `Err` from `apply` (the macOS or Windows arm is `None`, or the platform has no glass), never silently another material.

**The fallback.** On `Err`, apply `Material::Opaque` and use an opaque theme. This is the path Luneterm takes today (`crates/luneterm/src/app.rs:312-325` in Luneterm), so a look that cannot be shown on a platform degrades to the opaque one with no extra code.

**Unverified.** The macOS `UltraDark` look under a Light system appearance, and the Windows acrylic tint and dark flag on builds 22523 and later, have not been seen on a real machine (D285).
