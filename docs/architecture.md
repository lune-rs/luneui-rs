# Architecture

State after M2 (seams): the toolkit is five crates. A pure core (`lune-ui-core`, no dependencies)
holds the painter, the display list, widgets, the theme and the four seam types; three adapter
crates implement the seams on wgpu/glyphon, winit and the OS glass; the facade `lune-ui` re-exports
all of them under the module paths the toolkit has always had (D9, D24 to D38). The code moved from
Luneterm 296cfdc's `crates/lune-ui` in M1 (D10) and was cut along its seams in M2.

## Workspace layout

| Path | What |
|---|---|
| `crates/lune-ui-core` | Pure core: painter, `Scene`, widgets, theme, `TextSystem`, `RawInput`/`FrameOutput`, `Backdrop`, fakes. No dependencies |
| `crates/lune-ui-wgpu` | wgpu renderer and glyphon text: one shared `Gpu`, per-target `FrameRenderer`, `WindowSurface`, dumps, `TextCache` |
| `crates/lune-ui-winit` | winit adapter: input translation, output application, glass window attributes |
| `crates/lune-ui-platform` | OS glass (`Material`) per platform; the only crate with `unsafe` |
| `crates/lune-ui` | Facade: `lib.rs` only, re-exports the four crates at the old paths |
| `apps/gallery` | Reference scenes: a glass window, `--dump <dir>` (offscreen PNG frames on a headless device, no window), `--list`. App code, outside `crates/` (D12): it bundles fonts |
| `tools/dump-compare` | Compares two dirs of PNG frame dumps pixel by pixel (the pixel gate's compare tool) |
| `scripts/` | Gates and generators, see "Boundary rules" and "Pixel gates" |
| `docs/components/` | Catalogue, one file per component; `docs/components/INDEX.md` is generated |
| `licenses/`, `THIRD_PARTY_NOTICES.md` | Licence texts and the per-asset notice rows |

Scripts: `scripts/check-boundaries.sh` (rules 1 to 11, see below; fixed-string exceptions in
`scripts/boundaries.allow`), `scripts/gen-component-index.sh` (catalogue index; `--check` is in the
gate), `scripts/pixel-gate.sh`, `scripts/pixel-gate-luneterm.sh`, `scripts/check-targets.sh`.

## Crates and modules

The app owns the winit event loop; no crate starts one or reads the clock or the environment (the
app passes time, sizes, fonts, theme, scale and hit values in), so frames are deterministic.

### `lune-ui-core` (`crates/lune-ui-core/src/`)

| Module (file) | Purpose | Key items |
|---|---|---|
| `color` (`color.rs`) | Straight-alpha colour and blend helpers | `Rgba = [u8; 4]`, `premul`, `over`, `with_alpha` |
| `theme` (`theme.rs`) | Theme tokens widgets read; the dark preset with its glass layers | `Theme`, `DARK`, `lunegit_dark::{THEME, WINDOW, PANEL, CARD, ...}` |
| `acrylic` (`acrylic.rs`) | Avalonia acrylic colour math and the noise opacity | `AcrylicColors`, `NOISE_OPACITY` |
| `paint` (`paint.rs`) | Immediate-mode painter: four layers, clip, transform, hit regions generic over the app's hit type `H`; builds a `Scene`; holds `&mut dyn TextSystem` | `Painter<H>`, `Rect`, `StrokeStyle`, `Xform` |
| `scene` (`scene.rs`) | The display list: plain data plus the draw-order contract | `Scene`, `Layer`, `LayerGeometry`, `Shape`, `StrokeVertex`, `TextItem`, `CustomItem`, `BackdropRequest`, `LayerId`, `LAYER_COUNT`, `SCENE_LAYERS`, `FrameParams` |
| `text` (`text.rs`) | Text specs and the text seam | `TextSystem`, `TextKey`, `Spec`, `Style`, `Font` |
| `input` (`input.rs`) | The input seam: what the app hands in, what the toolkit asks back | `RawInput`, `Event`, `Key`, `NamedKey`, `Modifiers`, `ImeEvent`, `FrameOutput`, `WindowCommand`, `CursorIcon`, `printable` |
| `backdrop` (`backdrop.rs`) | The material seam | `Backdrop`, `Material`, `BackdropKind`, `GlassLayer` |
| `window` (`window.rs`) | Edge resize, double click, caption buttons (no windowing type) | `ResizeEdge`, `edge_at`, `DoubleClick`, `caption_buttons` |
| `icon` (`icon.rs`) | Codicon codepoints as Nerd Font glyphs; no font file bundled | `glyph`, `ADD`, `GEAR`, `CHECK`, ... |
| `widgets` (`widgets/mod.rs`) | Controls generic over `H` | `button`, `toggle`, `checkbox`, `combo`, `stepper`, `field`, `popup_panel` |
| `widgets::menu`, `dropdown`, `dialog` | The overlays | `menu`, `MenuEntry`, `menu_open_anim`; `dropdown`; `dialog`, `DialogSpec` |
| `chrome` (`chrome.rs`) | Tab chrome outline geometry | `outline`, `TabSpan` |
| `testing` (`testing/`) | Fakes and contract suites of the seams, public so apps can use them | `RecordingRenderer`, `FakeText`, `text_system_contract`, `ScriptedInput`, `NullMaterial`, `backdrop_contract` |

`lib.rs` re-exports `Backdrop`, `BackdropKind`, `GlassLayer` and `theme::lunegit_dark`. Tests: unit
tests in the modules and `tests/window.rs`; none needs a GPU or a window.

### `lune-ui-wgpu` (`crates/lune-ui-wgpu/src/`)

| Module (file) | Purpose | Key items |
|---|---|---|
| `gpu` (`gpu.rs`, `shader.wgsl`) | The GPU every target shares: device, queue, noise texture, per colour format the pipelines and the glyph atlas; headless-capable | `Gpu`, `instance`, `Uploaded`, `Globals` |
| `frame` (`frame.rs`) | One target's renderer: the scene, blur, target pass sequence over a core `Scene` | `FrameRenderer`, `FrameParams` |
| `blur` (`blur.rs`, `blur.wgsl`) | Backdrop blur | `Blur` |
| `surface` (`surface.rs`) | A window's swapchain; no winit dependency | `WindowSurface`, `Acquired` |
| `dump` (`dump.rs`) | Offscreen targets and GPU readback to PNG; no screen capture | `target`, `read_rgba`, `write_png` |
| `custom` (`custom.rs`) | App-owned GPU drawing inside a painted layer (the hook for app-specific drawing, D8) | `CustomDraw`, `CustomFrame`, `CustomItem` |
| `text_cache` (`text_cache.rs`) | glyphon shaping behind the core's `TextSystem` | `TextCache` |
| `platform` (private; `platform/{windows,other}.rs`) | The Windows DX12 backend choice | `configure` |

Asset: `assets/acrylic-noise.png` (Avalonia's texture). Tests: `tests/text.rs` (text contract),
`tests/targets.rs` (two targets on one `Gpu`).

### `lune-ui-winit` (`crates/lune-ui-winit/src/`)

| Module (file) | Purpose | Key items |
|---|---|---|
| `input` (`input.rs`) | winit events to `RawInput`; `FrameOutput` onto a window | `Translator`, `OutputState`, `cursor_icon` |
| `window` (`window.rs`) | Glass window attributes, `ResizeEdge` to winit's direction | `glass_attributes`, `resize_direction` |
| `platform` (private; `platform/{macos,windows,other}.rs`) | Per-platform window attributes | `glass` |

Tests: `tests/input.rs` (translator against `ScriptedInput`, key, button, cursor and edge mappings).

### `lune-ui-platform` (`crates/lune-ui-platform/src/`)

| Module (file) | Purpose | Key items |
|---|---|---|
| `material` (`material.rs`, `material/{macos,windows,other}.rs`) | OS backdrop per platform; the only `unsafe` is `material/macos.rs` | `apply`, `WindowBackdrop`, `Material` (re-export) |

### `lune-ui` (facade, `crates/lune-ui/src/lib.rs`)

One `pub use` per module keeps every `lune_ui::<module>` path. Three modules merge a core and an
adapter part: `text` (core text plus `TextCache`), `window` (core window helpers plus the winit
attributes), `input` (core model plus the winit translation). The facade also re-exports `testing`.
Boundary rule 11 keeps it free of items.

## Seams

Each seam is a core model (data or trait) that adapters implement, a fake in `lune_ui_core::testing`
and a contract test the real adapter passes too (R-nfr-8).

| Seam | Core model | Adapter | Fake | Contract test |
|---|---|---|---|---|
| Display list (renderer) | `scene::Scene` data and its draw-order contract; no renderer trait (D25) | `lune-ui-wgpu`: `Gpu::upload`, `FrameRenderer::draw` | `RecordingRenderer` | `records_the_documented_draw_order` (`testing/recording.rs`); two-target pixel equality in `lune-ui-wgpu/tests/targets.rs` |
| Text shaping | `text::TextSystem` (`prepare(spec, scale)`, `measure`) | `lune-ui-wgpu`: `TextCache` over glyphon | `FakeText` | `text_system_contract`, run in `lune-ui-wgpu/tests/text.rs` and on the fake |
| Input and output | `input::{RawInput, FrameOutput}` | `lune-ui-winit`: `Translator`, `OutputState` | `ScriptedInput` | `lune-ui-winit/tests/input.rs` (translator and scripted input agree) |
| OS glass | `backdrop::Backdrop` (`best_supported`, `apply`) | `lune-ui-platform`: `WindowBackdrop` over `apply` | `NullMaterial` | `backdrop_contract`, run on `NullMaterial`; OS glass is not run headless |
| Windowing | the core's window helpers and `FrameOutput::commands`; the app owns the event loop | `lune-ui-winit` | `ScriptedInput` | as input |

## Dependencies

Locked versions (`Cargo.lock`, unchanged since M1 and identical to Luneterm 296cfdc's; DECISIONS
D15 to D23 and D38 record each). Each crate's allowed set is enforced by boundary rule 9.

| Crate | Version | Used by | For |
|---|---|---|---|
| `lune-ui-core` | path | every other crate | none: it declares no dependency |
| `winit` | 0.30.13 | lune-ui-winit, lune-ui-platform, gallery | windowing |
| `wgpu` | 30.0.1 | lune-ui-wgpu, gallery | GPU renderer |
| `glyphon` | 0.12.0 (with `cosmic-text` 0.19.0) | lune-ui-wgpu | text shaping and rendering |
| `bytemuck` | 1.25.2 | lune-ui-wgpu | casting vertex data |
| `png` | 0.18.1 | lune-ui-wgpu, dump-compare | frame dumps, compare |
| `window-vibrancy` | 0.8.1 (macOS, Windows) | lune-ui-platform | OS glass |
| `objc2-app-kit` | 0.3.2 (macOS) | lune-ui-platform | NSView access for glass |
| `raw-window-handle` | 0.6.2 (macOS) | lune-ui-platform | native window handle |
| `pollster` | 0.4.0 | gallery; lune-ui-wgpu (dev) | block on async `Gpu::new` |

`window-vibrancy` also pulls `objc2` 0.5.3 and `objc2-app-kit` 0.2.2 beside `objc2` 0.6.5 and
`objc2-app-kit` 0.3.2. The facade `lune-ui` depends on the four crates only. Bundled assets and their
licences: `THIRD_PARTY_NOTICES.md`.

## Multi-window model

One `Gpu` and one `TextCache` serve the whole app; each window adds its own state:

- per app: `Gpu` (device, queue, noise texture, per-format pipelines and glyph atlas) and `TextCache`
  (text scale is a per-call argument, so windows at different scales keep their entries side by
  side instead of re-shaping everything on each scale change, D27);
- per window: a `WindowSurface` (swapchain), a `FrameRenderer` (everything a draw writes: globals,
  viewport, text renderers, blur textures, so two targets never share a write that lands at the next
  submit, D28), a `Painter` building a `Scene` and its hit list each frame, a `Translator`
  (`RawInput` in) and an `OutputState` (`FrameOutput` out);
- per headless target: a `FrameRenderer` and an offscreen texture from `dump::target`; no window.

Call `Gpu::end_frame` once per app frame, after every window drew; calling it per window only ages
text entries faster. `acquire` returns `Acquired::Redraw` instead of asking a window to redraw, and
the app calls `pre_present_notify` before `WindowSurface::present` (D29). Window-mode code (surfaces,
present, output commands, OS glass) is compile- and review-checked in M2; no gate opens a visible
window.

## Extension points

| Point | Mechanism |
|---|---|
| OS glass | A new `material` submodule (`crates/lune-ui-platform/src/material/`) behind `Backdrop` and `apply` |
| Another renderer | Consume `Scene` in the documented draw order; test the painter side with `RecordingRenderer` |
| Another text shaper | Implement `TextSystem`; pass `text_system_contract` |
| Another windowing library | Translate its events into `RawInput`, apply `FrameOutput`; script it with `ScriptedInput` |
| Overlay or widget | A new module (under `crates/lune-ui-core/src/widgets/` for controls) plus a catalogue file |
| Look | A `Theme` value handed to the `Painter`; `theme::DARK` and `lunegit_dark` are one |
| App-specific drawing | `CustomDraw`, one implementation per slot passed to `FrameRenderer::draw_with`; the gallery's `apps/gallery/src/custom.rs` is the worked example. The terminal-grid renderer built on it stays in Luneterm (D8) |
| Fonts | The app passes font bytes and family names to `TextCache::new`; the library bundles none (D5, D12) |

## Boundary rules

`scripts/check-boundaries.sh` runs these (output `rule N: <path>:<line>: <text>`; exit 1 on an
offender, 2 when a scanned path or a tool is missing):

| Rule | Checks | Backs |
|---|---|---|
| 1 | No `crates/*/Cargo.toml` names an app | hard rule 1 |
| 2, 3 | No file under `crates/` names luneterm, or app or terminal words (exceptions: `scripts/boundaries.allow`) | hard rule 1 |
| 4 | No `crates/*/src` file reads the clock or the environment | hard rule 2 |
| 5 | Every manifest inherits the workspace licence | hard rule 7 |
| 6 | `unsafe` only under `lune-ui-platform/src/platform/` or `src/material/`; every other crate's `lib.rs` has `#![forbid(unsafe_code)]` | hard rule 3 |
| 7 | `lune-ui-core/Cargo.toml` declares no dependency of any kind | crate rule |
| 8 | No third-party crate path or `extern crate` in a file under `lune-ui-core` | crate rule |
| 9 | Every crate dir is registered with its allowed dependency names; an unregistered dir or a `package =` rename fails; adapters name no `lune-ui-*` crate but the core | crate rule (adapters depend on the core only) |
| 10 | No OS predicate in `cfg` outside `platform/` or `material/` dirs, except `#[cfg]` on `mod x;` or `use` | hard rule 3 |
| 11 | `crates/lune-ui/src` holds only `lib.rs`, with no item definitions | crate rule (facade only) |

The script does not cover hard rules 4 (a new dependency needs a DECISIONS row; rule 9 forces a
script edit but does not check the row), 5, 6 (licence rows), 8, 9 or 10 (`gen-component-index.sh
--check` covers 10). Hard rule 3 reads: platform code lives in the adapter crate that implements the
seam, in `platform/` or `material/` files, so the Windows DX12 choice is in
`lune-ui-wgpu/src/platform/`, the winit attributes in `lune-ui-winit/src/platform/` and the glass in
`lune-ui-platform/src/material/` (D32).

## Known deviations

The two M1 deviations are resolved: the inline `cfg(target_os)` in `gpu.rs` and `window.rs` moved
into `platform/` files (step 8), and `Gpu::new` takes an optional compatible surface, so a headless
device needs no window (steps 3 and 4). New: none.

## API changes since M1

For M11's migration map (Luneterm's `lune-termview` uses `color`, `custom`, `paint`, `dump`; the
facade keeps those paths):

- `Painter::new` takes `&mut dyn TextSystem` (was `&mut TextCache` plus a global scale); the painter
  has a `scene` field (the display list) instead of GPU geometry;
- `FrameRenderer::new(gpu, format)` and `FrameRenderer::draw(gpu, text, scene, target, params)` /
  `draw_with(..., customs)` (a renderer per target, a scene instead of a painter);
- `Gpu::new(instance, compatible_surface)` (no window; `None` is headless), `Gpu::end_frame(text)`;
- `WindowSurface` and `Acquired` replace the surface held inside `Gpu`;
- `TextCache::new(fonts, ui_family, mono_family)` without a scale; `set_scale` is gone; scale is an
  argument of `TextSystem::prepare` and `measure`; new `TextKey`;
- `edge_at` returns the core's `ResizeEdge` (was winit's `ResizeDirection`); `resize_direction`
  maps it for winit;
- `BackdropRequest` (core) is what a glass menu records; the blur's own uniform is private;
- `theme::DARK`, a neutral alias of `lunegit_dark::THEME`;
- new: `RawInput`, `FrameOutput`, `Translator`, `OutputState`, `Backdrop::apply`, `WindowBackdrop`,
  the `testing` module.

## Provenance of reworded comments

On the move, 12 comment lines that named the source app's look were reworded so library source
carries no app word (rule 1, D14). No identifier or behaviour changed; the original wording and
what it pointed at (paths are the files' current homes):

| File | Original pointed at |
|---|---|
| `lune-ui-core/src/lib.rs` (doc) | "styled after Lunegit"; the original pointed at Luneterm's glass-styling doc (not in this repo) |
| `lune-ui-core/src/lib.rs` (doc) | the `lunegit_dark` preset; its values are copied from Lunegit's `DarkTheme.axaml` and `App.axaml` |
| `lune-ui-core/src/backdrop.rs` (`Backdrop` doc) | Lunegit's `TransparencyLevelHint` (Avalonia) |
| `lune-ui-core/src/acrylic.rs`, `lune-ui-wgpu/src/gpu.rs`, `lune-ui-wgpu/src/shader.wgsl`, `lune-ui-platform/src/material/macos.rs` | "same pixels as Lunegit's", "match Lunegit", "Lunegit's conic stop table", "what Avalonia (and so Lunegit) uses": the Avalonia look Lunegit uses |
| `lune-ui-core/src/chrome.rs` | Port of Lunegit `Controls/Terminal/TerminalChromeGeometry.cs`, `BuildLoopFigure` |
| `lune-ui-core/src/icon.rs` | "Lunegit uses the Codicons icon pack" |
| `lune-ui-core/src/widgets/dialog.rs`, `lune-ui-core/src/widgets/mod.rs` | the Lunegit dialog and Lunegit's Fluent theme overrides (`DarkTheme.axaml`) |

Source of truth for the original text: `git -C <Luneterm> show 296cfdc:crates/lune-ui/<file>`.

## Pixel gates

A drawing change must not move pixels unless the look change is intended (hard rule 8, D11). Dumps
are PNGs written by offscreen readback (no screen capture). The working tree dumps on a headless
device (no event loop, no window, D30); a base revision from before M2 step 4 still opens a hidden
winit window, so a gate against such a base needs a logged-in macOS GUI session. Each gate writes
only under `target/`, except that `scripts/pixel-gate.sh` extracts its base source under `$TMPDIR`
(outside the repo, so no cargo config in the repo tree reaches the base build; D16 of the M2 plan).

| Gate | Proves | Compare | Exit codes |
|---|---|---|---|
| `scripts/pixel-gate-luneterm.sh` | M1 only: this repo's `lune-ui` drew the same pixels as Luneterm's. Phases: lock parity (every registry package in this repo's `Cargo.lock` is in Luneterm's lock at the rev; `--parity-only` stops here and still applies), run A (Luneterm's own crate), run B (the scratch clone's `lune-ui` repointed at this repo's crate), compare with Luneterm's `dump_compare`. Since M2 step 2 run B cannot build against the new API, so a full run exits 2 by design (roadmap D4) | `18 files, 18 identical, 0 differ, max delta 0` | 0 pass; 1 parity mismatch or pixel difference; 2 usage or setup error |
| `scripts/pixel-gate.sh [--base <rev>] [--allow <id>...]` | A change since `<rev>` (default `HEAD`) leaves the gallery's reference scenes unchanged. Dumps `git archive <rev>` and the working tree, compares with `dump-compare --allow-new`. `--allow <id>` names a scene whose frames may differ (reported only); an id not in `gallery --list` is an error | `15 files, 15 identical` | 0 pass; 1 delta or scene missing from the candidate; 2 usage or setup error (a base without `apps/gallery` included: nothing to compare is not a pass) |
| `scripts/check-targets.sh [--skip-linux]` | The workspace lints and builds beyond the host: Windows `clippy` (needs the `x86_64-pc-windows-msvc` target) and Linux fmt, clippy and tests in a `rust:1.92` container. Not a pixel gate; it covers the code the host `cfg` hides | `windows: passed`, `linux: passed` or `skipped (<reason>)` | 0 every half that ran passed; 1 a check failed; 2 usage |

Dump file names: the gallery writes `<id>@<scale>x.png` (logical 1040x660; `gallery --list` prints
the 15 names); Luneterm's spike writes `NN-<name>@<scale>x.png` (18 scenes). `tools/dump-compare`
(`cargo run -p dump-compare -- <baseline> <candidate> [--max-delta N] [--allow-new]`) compares per
pixel, max channel delta including alpha; a file missing from the candidate, a file only in the
candidate (unless `--allow-new`), a size or colour-type mismatch or a delta above `N` (default 0)
exits 1; it prints `N files, M identical, K differ, max delta D`.

Re-baselining: a deliberate look change passes the gate with `--allow <id>` for the scenes it
moves, and records the reason in `docs/DECISIONS.md` in the same commit (hard rule 8). Dumps are not
committed (D11).
