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
| `apps/gallery/assets/lunegit/` (Lunegit's theme and locale files) | Lunegit has no open licence (D206). The script fails (exit 1) on any staged path under `assets/lunegit`, and on any exported file whose blob equals a fixture's, under any name |

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

**The version is 0.2.0 (D5).** Luneterm's own in-tree `crates/lune-ui` is 0.1.0; two packages named
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
  when `Gpu::is_lost()` says the device is gone (it reports, it does not recover; D8);
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

An app measures its own. The toolkit's figure for the gallery's release build is recorded in
`docs/STATE.md` when a milestone closes.

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
