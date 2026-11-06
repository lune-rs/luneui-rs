---
id: platform.material
title: OS glass materials
area: platform
status: in-progress
consumers: luneterm
source: crates/lune-ui-platform/src/material.rs
owner: Luneterm M9 to M11 runs
summary: Backdrop seam in core (Material, Backdrop, NullMaterial); OS glass behind it: macOS NSVisualEffect and Liquid Glass, Windows acrylic and mica, opaque elsewhere
---

## What it is

Core: `Material`, `BackdropKind`, `GlassLayer` and the `Backdrop` trait (`best_supported`, `apply`); fake `testing::NullMaterial` and `testing::backdrop_contract`. Platform crate: `apply(&winit::window::Window, Material)`, `apply_rounded` (the same with the glass view's corners rounded, M10) and `WindowBackdrop`, which implements `Backdrop` over it, dispatching to `#[cfg]` submodules: `material/macos.rs` (NSVisualEffectView, NSGlassEffectView), `material/windows.rs` (DWM acrylic, mica, tabbed via `window-vibrancy`), `material/other.rs` (glass returns `Err`).

## Needs

- macOS glass (R-platform-1); Windows glass (R-platform-2); opaque fallback elsewhere (R-platform-3).
- The only `unsafe` blocks (3, in `lune-ui-platform/src/material/macos.rs`) stay in this crate under `src/material/`, each with a `// SAFETY:` comment (R-nfr-3; boundary rule 6).

## Existing code

`crates/lune-ui-core/src/backdrop.rs:27` (`Material`), `:49` (`Backdrop`); `crates/lune-ui-platform/src/material.rs:40` (`apply`), `:47` (`apply_rounded`), `:57` (`WindowBackdrop`); `crates/lune-ui-platform/src/material/macos.rs:89`, `:91`, `:100` (`unsafe`); 67 + 103 + 36 + 23 lines. The contract test runs on the fake in `crates/lune-ui-core/src/testing/backdrop.rs`; OS glass itself is untested here.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b, Luneterm 296cfdc. Read: this repo crates/lune-ui-core/src/backdrop.rs:1-60, crates/lune-ui-core/src/testing/backdrop.rs:1-64, crates/lune-ui-platform/src/material.rs:1-50, material/macos.rs:1-95, material/windows.rs:1-30, material/other.rs:1-15,
crates/lune-ui-core/src/theme.rs:88-118, docs/DECISIONS.md D31 (row 40); Lunegit src/Lunegit.Desktop/Views/DiffView.axaml:72-83, Themes/DarkTheme.axaml:226-235, Theming/ThemeApplicator.cs:284-301; Wane docs/ui/components.md:204; Luneterm docs/glass-styling.md:1-54.
Conflict: none about the seam. Wane's measured flat colours (`--glass-window` rgba 54 54 54 .81) are not used: Wane itself says to reuse the WINDOW layer constants [W docs/ui/components.md:204].

### Anatomy

- Requests: `Material` is one of `Vibrancy` (light vibrancy on macOS, acrylic on Windows, kept active when the window is unfocused), `VibrancyFollowsWindow` (the same, following the window's active state on macOS), `LiquidClear` (clear liquid glass on macOS, Mica on Windows), `LiquidRegular` (regular liquid glass on macOS, tabbed Mica on Windows), `Opaque` (no OS material); `is_glass()` is true for all but `Opaque` [R crates/lune-ui-core/src/backdrop.rs:26-47].
- Capability: `Backdrop::best_supported()` gives `Blur`, `Transparent` or `Opaque`; callers fall back down `Blur`, `Transparent`, `Opaque`; `apply` first clears what was applied earlier, `Opaque` always succeeds and a glass material the platform cannot show is an `Err` [R crates/lune-ui-core/src/backdrop.rs:14-22,49-56].
- macOS: the effect view goes into the window's frame view below the content view, so it can never cover what the GPU draws; `Vibrancy` is `NSVisualEffectMaterial::Light` in the active state (deprecated by Apple, and exactly what Avalonia uses), `VibrancyFollowsWindow` the follows-window state, the two liquid kinds `NSGlassEffectView` with the clear and regular styles [R crates/lune-ui-platform/src/material/macos.rs:1-2,22-50].
- Windows: `Vibrancy` and `VibrancyFollowsWindow` are DWM acrylic, `LiquidClear` Mica, `LiquidRegular` tabbed Mica, all through `window-vibrancy`; clearing a never-applied backdrop is harmless [R crates/lune-ui-platform/src/material/windows.rs:1-27].
- Other platforms: opaque only; any glass material is an `Err("no OS glass on this platform...")` [R crates/lune-ui-platform/src/material/other.rs:1-15].
- Rounding: `apply_rounded(window, material, radius)` is `apply` with the OS glass view's corners at `radius` logical px, for a window whose card is rounded: the macOS effect view takes the radius (`window-vibrancy`'s `radius` argument and `LiquidGlassOptions::radius`); Windows rounds the window itself and other platforms have no glass, so both ignore it [R crates/lune-ui-platform/src/material.rs:42-53], [R crates/lune-ui-platform/src/material/macos.rs:29-35,69-75]. The tray popover window is its one caller [R apps/gallery/src/popover.rs:160], [platform.tray-icon].
- Unsafe: three blocks, all in `material/macos.rs`, each with a `// SAFETY:` comment resting on winit's guarantee that the view is live and on the main thread; `apply` therefore takes `&winit::window::Window` [R crates/lune-ui-platform/src/material/macos.rs:85-101], [D31].
- App-drawn layers on top of the OS backdrop, as Avalonia acrylic terms: `GlassLayer { tint, tint_opacity, material_opacity }`; window layer (tint opacity 1, material 0.7), panel layer (0.8, 0.7), card and header-tint layer (1, 0.8) [R crates/lune-ui-core/src/backdrop.rs:5-12], [R crates/lune-ui-core/src/theme.rs:88-109], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:226-235]. Lunegit draws each as an acrylic border over the desktop (`BackgroundSource="Digger"`) with a tint colour and the two opacities [L src/Lunegit.Desktop/Views/DiffView.axaml:72-82].
- The layer stack, back to front: the OS backdrop, the window acrylic, the panel acrylic, the card, the header tint, the tab chrome outline, the content [LT docs/glass-styling.md:7-20].
- The inactive window: all acrylic layers are hidden and a solid fill shows instead; Lunegit does this to work around an Avalonia macOS bug that draws the inactive material flat grey; Luneterm first tries keeping the effect view `active` always, which is `Material::Vibrancy` [L src/Lunegit.Desktop/Theming/ThemeApplicator.cs:284-301], [LT docs/glass-styling.md:24-28], [R crates/lune-ui-core/src/backdrop.rs:28-29]. `platform.window` holds the inactive look.
- Offscreen frames do not show the OS backdrop: the gallery dumps show the app's acrylic and blur layers over a flat target, not what the OS composites (headless dumps, D30).
- Open items carried from Luneterm: blur radius and noise texture (Lunegit uses Avalonia's acrylic defaults; match them; not compared here, see Open questions); Linux has no glass by decision (opaque dark theme with the inactive solid colours) [LT docs/glass-styling.md:51-54].

### States

| state | look | cite |
|---|---|---|
| `Vibrancy` | OS vibrancy or acrylic, active whatever the window state | [R crates/lune-ui-core/src/backdrop.rs:28-29], [R crates/lune-ui-platform/src/material/macos.rs:31-36] |
| `VibrancyFollowsWindow` | as above, following the window's active state (macOS) | [R crates/lune-ui-platform/src/material/macos.rs:37-42] |
| `LiquidClear` | clear liquid glass (macOS) or Mica (Windows) | [R crates/lune-ui-platform/src/material/macos.rs:43-46], [R crates/lune-ui-platform/src/material/windows.rs:22] |
| `LiquidRegular` | regular liquid glass (macOS) or tabbed Mica (Windows) | [R crates/lune-ui-platform/src/material/macos.rs:47-50], [R crates/lune-ui-platform/src/material/windows.rs:23] |
| `Opaque` | no OS material | [R crates/lune-ui-core/src/backdrop.rs:37-38] |
| glass unsupported | `Err`; the caller falls back to `Opaque` | [R crates/lune-ui-platform/src/material/other.rs:11-15] |
| hover, pressed, focus-visible, disabled | not applicable: a material has no pointer or focus state | [derived: it is window state] |

### Transitions

n/a — the OS composites the material; applying a new one clears and re-applies it with no tween [R crates/lune-ui-platform/src/material.rs:31-36]. Whether the OS cross-fades the change is the OS's. The app's own layers swap on window activity as a hard cut [L src/Lunegit.Desktop/Views/DiffView.axaml:73-83].

### Input and focus

n/a — a material takes no input.

### Retained state

- Transient: none in the toolkit. Persistent: the OS effect view (platform state) until cleared. App-owned: which `Material` it chose, the fallback it landed on, the layer constants.
- Exit retention: no.

### Accessibility

n/a — no node; a backdrop is not content. High-contrast and reduced-transparency OS settings may make the app choose `Opaque`; the toolkit reads no OS setting (hard rule 2), so the app passes the choice [derived: hard rule 2].

### Strings

None: no text.

### Gallery scenes

- `glass-off` (existing): the in-app layers without glass.
- `acrylic` (existing): the app-drawn acrylic layers.
- `blur-contrast` (existing): blur over high-contrast content.
- `unfocused` (existing): the inactive window with the solid fill.
- The OS material itself has no scene: dumps cannot show it (Anatomy).

### Painter primitives

- Existing: acrylic, backdrop (the in-app blur request over the layers beneath a popup).
- New: none.

### Acceptance

- Contract (existing): `backdrop_contract` passes on `NullMaterial`: P1 `apply(Opaque)` is Ok; P2 when `best_supported` is `Opaque`, every glass material is an Err; P3 `apply(Opaque)` twice in a row is Ok; and it flags a broken backdrop (`contract_catches_broken_backdrops`) [R crates/lune-ui-core/src/testing/backdrop.rs:28-63].
- `NullMaterial` records the materials it accepted, in order [R crates/lune-ui-core/src/testing/backdrop.rs:5-24].
- Compile checks: the Windows target builds with clippy `-D warnings` on the macOS host; the Linux target in Docker [D15].
- Boundaries: the three `unsafe` blocks stay under `src/material/` with a SAFETY comment each (the boundary script) [R crates/lune-ui-platform/src/material/macos.rs:85-101].
- Unverified here, no hardware or VM: what the OS composites for each material, Windows acrylic and Mica on real hardware, liquid glass on macOS versions without it. Luneterm's go/no-go gate and its M9 to M11 runs cover them [D15]. The done standard (D18) for this entry needs those runs.
- Tests that run here: `crates/lune-ui-core/src/testing/backdrop.rs`, module `tests`: `null_material_meets_the_contract` (P1 to P3 on `NullMaterial`, which accepted only `Opaque`) and `contract_catches_broken_backdrops` (a `Liar` fails P2, a `Refuser` fails P1). The Windows and Linux compile checks are `scripts/check-targets.sh`. The three `unsafe` blocks and their SAFETY comments are `scripts/check-boundaries.sh`. `WindowBackdrop` over a real window is not run through `backdrop_contract` anywhere; the tray popover applies `Vibrancy` through `apply_rounded` on a real window in the platform smoke (`gallery --smoke-platform`), which prints an error and does not fail on one.
- Pixel probes, conformance, semantic node: n/a.

### Build

- Milestone: M11 (done standard; code exists). Class B (D234): stays `in-progress` after the M11 audit; see Open questions.
- Depends on: core.theme, platform.window, render.blur

## Open questions

- Windows glass is unverified on hardware by this inventory; Luneterm's go/no-go gate covers it (Luneterm decision 3).
- `apply` takes a winit `Window` (D31); a `HasWindowHandle` form needs a new SAFETY argument and a main-thread marker, which nothing before M11 needs.
- Blur radius and the noise texture match Avalonia's acrylic defaults only by Luneterm's open item [LT docs/glass-styling.md:51-52]. Not run here: the comparison needs side-by-side shots of the OS material and Lunegit on real machines; Luneterm's M9 to M11 runs make it.
- Class B (D234): this entry's own Acceptance says "The done standard (D18) for this entry needs those runs" (what the OS composites for each material, Windows acrylic and Mica on hardware, liquid glass on macOS versions without it). None can happen in this environment, so the entry stays `in-progress` with `owner:` naming who runs them. Everything in the entry that runs here is listed under Acceptance, "Tests that run here".

## Notes for the planner

The seam landed in M2 step 7. `apply` still takes a winit `Window` (DECISIONS D31: a `HasWindowHandle` form needs a new SAFETY argument).
