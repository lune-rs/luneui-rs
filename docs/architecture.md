# Architecture

State after M1 (extraction): `crates/lune-ui` is Luneterm 296cfdc's `crates/lune-ui`, moved as is
(D10). The split into core and adapters is M2 (D9) and has not started. Everything below describes
what is in the tree today, then where it is going.

## Workspace layout

| Path | What |
|---|---|
| `crates/lune-ui` | The toolkit, one crate, Luneterm's module paths and pub API unchanged |
| `apps/gallery` | Reference scenes: a glass window, `--dump <dir>` (offscreen PNG frames), `--list`. App code, outside `crates/` (D12): it reads the clock and bundles fonts |
| `tools/dump-compare` | Compares two dirs of PNG frame dumps pixel by pixel (the pixel gate's compare tool) |
| `scripts/` | Gates and generators, see "Pixel gates" and the list below |
| `docs/components/` | Catalogue, one file per component; `INDEX.md` is generated |
| `licenses/`, `THIRD_PARTY_NOTICES.md` | Licence texts and the per-asset notice rows |

Scripts: `check-boundaries.sh` (hard rules 1, 2, 3, 4 and 6 as grep rules over `crates/`, fixed-string
exceptions in `boundaries.allow`), `gen-component-index.sh` (catalogue index; `--check` is in the
gate), `pixel-gate.sh`, `pixel-gate-luneterm.sh`, `check-targets.sh`.

## `lune-ui` module map

All under `crates/lune-ui/src/`. The app owns the winit event loop; the crate never starts one and
never reads the clock or the environment (the app passes time, sizes, fonts, theme, scale and hit
values in), so frames are deterministic.

| Module (file) | Purpose | Key items |
|---|---|---|
| `color` (`color.rs`) | Straight-alpha colour and blend helpers | `Rgba = [u8; 4]`, `premul`, `over`, `with_alpha` |
| `theme` (`theme.rs`) | Theme tokens widgets read; the dark preset with its glass layers | `Theme`, `lunegit_dark::{THEME, WINDOW, PANEL, CARD, ...}` |
| `acrylic` (`acrylic.rs`) | Avalonia acrylic colour math and the noise opacity | `AcrylicColors`, `NOISE_OPACITY` |
| `paint` (`paint.rs`) | Immediate-mode painter: four layers, clip, transform, hit regions generic over the app's hit type `H` | `Painter<H>`, `Rect`, `LayerId`, `StrokeStyle`, `Xform` |
| `custom` (`custom.rs`) | App-owned GPU drawing inside a painted layer (the hook for app-specific drawing, D8) | `CustomDraw`, `CustomFrame`, `CustomItem` |
| `text` (`text.rs`) | Shaped-text cache over glyphon; the app registers its fonts | `TextCache`, `Spec`, `Style`, `Font` |
| `icon` (`icon.rs`) | Codicon codepoints as Nerd Font glyphs; no font file bundled | `glyph`, `ADD`, `GEAR`, `CHECK`, ... |
| `widgets` (`widgets/mod.rs`) | Controls generic over `H` | `button`, `toggle`, `checkbox`, `combo`, `stepper`, `field`, `popup_panel` |
| `widgets::menu`, `dropdown`, `dialog` | The overlays | `menu`, `MenuEntry`, `menu_open_anim`; `dropdown`; `dialog`, `DialogSpec` |
| `chrome` (`chrome.rs`) | Tab chrome outline geometry | `outline`, `TabSpan` |
| `gpu` (`gpu.rs`, `shader.wgsl`) | wgpu setup, pipelines, geometry upload | `Gpu`, `instance`, `LayerGeometry`, `Uploaded` |
| `blur` (`blur.rs`, `blur.wgsl`) | Backdrop blur | `Blur`, `Backdrop` (blur-pass struct) |
| `frame` (`frame.rs`) | The scene, blur, target pass sequence | `FrameRenderer`, `FrameParams` |
| `dump` (`dump.rs`) | Offscreen targets and GPU readback to PNG; no screen capture | `target`, `read_rgba`, `write_png` |
| `material` (`material.rs`, `material/{macos,windows,other}.rs`) | OS backdrop per platform; the only `unsafe` is `material/macos.rs` | `Material`, `apply` |
| `window` (`window.rs`) | Glass window attributes, edge resize, double click, caption buttons | `glass_attributes`, `edge_at`, `DoubleClick`, `caption_buttons` |

`lib.rs` also defines `GlassLayer`, `BackdropKind` and the `Backdrop` trait (capability levels), and
re-exports `theme::lunegit_dark`. Assets: `assets/acrylic-noise.png` (Avalonia's texture).
Tests: unit tests in the modules and `tests/window.rs`; none needs a GPU or a window.

## Dependencies

Locked versions (`Cargo.lock`, identical to Luneterm 296cfdc's; DECISIONS D15-D23 record each):

| Crate | Version | Used by | For |
|---|---|---|---|
| `winit` | 0.30.13 | lune-ui, gallery | windowing |
| `wgpu` | 30.0.1 | lune-ui, gallery | GPU renderer |
| `glyphon` | 0.12.0 (with `cosmic-text` 0.19.0) | lune-ui | text shaping and rendering |
| `bytemuck` | 1.25.2 | lune-ui | casting vertex data |
| `png` | 0.18.1 | lune-ui, dump-compare | frame dumps, compare |
| `window-vibrancy` | 0.8.1 (macOS, Windows) | lune-ui | OS glass |
| `objc2-app-kit` | 0.3.2 (macOS) | lune-ui | NSView access for glass |
| `raw-window-handle` | 0.6.2 (macOS) | lune-ui | native window handle |
| `pollster` | 0.4.0 | gallery | block on async `Gpu::new` |

`window-vibrancy` also pulls `objc2` 0.5.3 and `objc2-app-kit` 0.2.2 beside `objc2` 0.6.5 and
`objc2-app-kit` 0.3.2. Bundled assets and their licences: `THIRD_PARTY_NOTICES.md`.

## Extension points

| Point | Mechanism |
|---|---|
| OS glass | A new `material` submodule (`crates/lune-ui/src/material/`) behind `Material` and `apply` |
| Overlay or widget | A new module (under `widgets/` for controls) plus a catalogue file |
| Look | A `Theme` value handed to the `Painter`; the `lunegit_dark` preset is one |
| App-specific drawing | `CustomDraw`, one implementation per slot passed to `FrameRenderer::draw_with`; the gallery's `apps/gallery/src/custom.rs` is the worked example. The terminal-grid renderer built on it stays in Luneterm (D8) |
| Fonts | The app passes font bytes and family names to `TextCache::new`; the library bundles none (D5, D12) |

## Target split (D9, M2)

One crate with swap points inside becomes five, so the compiler enforces the seams:

- `lune-ui-core`: pure; painter, widgets, theme, colour, geometry; no wgpu, winit or glyphon types in
  its signatures; traits for the renderer, the text shaper, the OS glass and the window;
- `lune-ui-wgpu`: GPU renderer and glyphon text, implementing the core's traits;
- `lune-ui-winit`: window helpers;
- `lune-ui-platform`: OS glass (`Material`) per platform;
- `lune-ui`: facade re-exporting the pieces (keeps `lune_ui::...` paths for consumers).

Adapters depend on the core only. As each seam lands it gets a contract test and a fake (headless
target, null material), each as its own gated step.

## Known deviations, to fix in M2

- Inline `cfg(target_os)` outside `material`: `crates/lune-ui/src/gpu.rs` (`instance`, Windows
  backend choice) and `crates/lune-ui/src/window.rs` (three `platform` modules). Hard rule 3 wants
  platform code in `#[cfg]` modules of the crate that owns the trait. Moved as is (D10).
- `Gpu::new` needs a live window (`Arc<Window>` plus a wgpu instance made from the event loop's
  display handle); there is no headless device, so the gallery's `--dump` opens a hidden window and
  renders offscreen. M2 introduces a headless target.
- The repo-root `README.md` still describes the pre-extraction scaffolding.

## Provenance of reworded comments

On the move, 12 comment lines that named the source app's look were reworded so library source
carries no app word (rule 1, D14). No identifier or behaviour changed; the original wording and
what it pointed at:

| File | Original pointed at |
|---|---|
| `lib.rs` (doc, line 2) | "styled after Lunegit"; the original pointed at Luneterm's glass-styling doc (not in this repo) |
| `lib.rs` (doc, lines 10 and 20) | the `lunegit_dark` preset; its values are copied from Lunegit's `DarkTheme.axaml` |
| `lib.rs` (`Backdrop` doc) | Lunegit's `TransparencyLevelHint` (Avalonia) |
| `acrylic.rs`, `gpu.rs`, `shader.wgsl`, `material/macos.rs` | "same pixels as Lunegit's", "match Lunegit", "Lunegit's conic stop table", "what Avalonia (and so Lunegit) uses": the Avalonia look Lunegit uses |
| `chrome.rs` | Port of Lunegit `Controls/Terminal/TerminalChromeGeometry.cs`, `BuildLoopFigure` |
| `icon.rs` | "Lunegit uses the Codicons icon pack" |
| `widgets/dialog.rs`, `widgets/mod.rs` | the Lunegit dialog and Lunegit's Fluent theme overrides (`DarkTheme.axaml`) |

Source of truth for the original text: `git -C <Luneterm> show 296cfdc:crates/lune-ui/<file>`.

## Pixel gates

A drawing change must not move pixels unless the look change is intended (hard rule 8, D11). Dumps
are PNGs written by offscreen readback (no screen capture); they need a logged-in macOS GUI session
because the hidden winit window is still created. Each gate writes only under `target/`.

| Gate | Proves | Compare | Exit codes |
|---|---|---|---|
| `scripts/pixel-gate-luneterm.sh` | The M1 move: this repo's `lune-ui` draws the same pixels as Luneterm's. Phases: lock parity (every registry package in this repo's `Cargo.lock` is in Luneterm's lock at the rev; `--parity-only` stops here), run A (Luneterm's own crate) and run B (the scratch clone's `lune-ui` repointed at this repo's crate), compare with Luneterm's `dump_compare`. Checks which crate each run built and that each dir holds exactly 18 PNGs | `18 files, 18 identical, 0 differ, max delta 0` | 0 pass; 1 parity mismatch or pixel difference; 2 usage or setup error |
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
