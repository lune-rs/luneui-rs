# Requirements

What luneui-rs must provide. Each requirement has an id `R-<area>-<n>` that component files in
`docs/components/` cite in their Needs section. Tags: **[existing]** the code exists today in
Luneterm's `crates/lune-ui` (from its module inventory, 2026-11-03); **[new]** no code anywhere;
**[open]** reserved for a requirement whose shape is not decided; no item has been open since M3
(2026-11-04), when each was resolved to `[new]` or `[existing]` with the catalogue entries and
DECISIONS rows it rests on.

Nothing here is a design: the designs are the component specs in `docs/components/` and the motion
standard in `docs/motion.md`. Numbers appear where a source states one.

**State at close (M11, 2026-11-07).** Every requirement below is met or is explained where it is not,
and none is `[open]`. The catalogue holds 97 entries: 91 `done` (the done standard of DECISIONS D47
holds for each by something that runs here, `docs/components/README.md` "Status"), 2 `in-progress` by
their own text (`platform.material` and `render.surface`: what they name needs a real OS run, D234)
and 4 `external`. Ids no entry cites are met by a script, a rule or a manifest, said on the id
(R-nfr-2, R-nfr-4, R-nfr-7, R-ext-3, R-ext-5). What only a real OS shows is listed in
`docs/STATE.md`, "Unverified here"; none of it is ticked.

## Consumers

- **Luneterm** (this author's terminal, `/Users/ijat/Projects/Luneterm`): its milestones M9 to M11
  (the app end to end, tabs and settings, finish) consume the toolkit. M8 (the C ABI) does not.
  Today only the `glass-ui` spike exercises the painter, frame and window APIs.
- **A possible Rust rebuild of Lunegit** (the Avalonia git client): about 75 views, custom controls
  and AvaloniaEdit-based editors. Not a commitment; its needs are rough stubs in the catalogue.
- **Wane** (this author's LLM gateway desktop app, `/Users/ijat/Projects/wane`): its window and tray UI. Its
  HTML mocks and component spec (`docs/ui/components.md`, 35 components, P0-P2 build order) name what it
  needs; 20 of them are new here (2026-11-04).
- **Others**: any app that wants the same glass look. The toolkit stays app-agnostic.

## Functional

### Core (painter, colour, theme)

- R-core-1 [existing] Immediate-mode painter with ordered layers; fill, rounded fill, ring, stroke
  and acrylic primitives; clip and transform (`paint.rs`).
- R-core-2 [existing] Hit regions carry an app-defined hit type; the painter answers hit and hover
  queries (`paint.rs`).
- R-core-3 [existing] Straight-alpha RGBA values with premultiply, blend-over and alpha helpers
  (`color.rs`).
- R-core-4 [existing] Look is a `Theme` value plus glass layer constants; the Lunegit dark preset is
  one such value (`theme.rs`). Other looks are further values, not code forks.
- R-core-5 [existing] Avalonia acrylic colour math with tests (`acrylic.rs`).
- R-core-6 [existing] Codicon glyph constants; the app supplies the icon font (`icon.rs`).
- R-core-7 [existing] Tab-outline loop geometry (`chrome.rs`).
- R-core-8 [new] The `Ui` frame context: widget ids from app keys under a parent scope, transient
  and persistent retained state with keep-alive and exit retention, layered hit testing with modal
  capture, `Response`, `Repaint`, the string table and the semantic tree (`core.ui`, `core.focus`,
  D46).
- R-core-9 [new] Image drawing: a painter primitive that draws an app-supplied bitmap (`core.image`).
- R-core-10 [new] Painter primitives beyond today's fill, ring, stroke, acrylic, text, clip and
  transform: the set each catalogue spec lists, aggregated in `core.painter`.

### Text

- R-text-1 [existing] Shape and measure text with fonts the app registers; the toolkit bundles no
  font (`text.rs`, `TextCache::new`).
- R-text-2 [existing] Shaped-text cache keyed by content, with a scale setting (`text.rs`).
- R-text-3 [new] Multi-line text view and editor with syntax highlighting, for code, diffs and
  Markdown (Lunegit uses AvaloniaEdit + TextMate and Markdig).
- R-text-4 [new] IME composition input: preedit text, its cursor range and the candidate-window
  area, routed into text fields and editors. Not in today's `lune-ui`; Luneterm M9 lists IME among
  its scope items. Resolved: `text.ime` (M5) and the input events in `core.input` (D35).
- R-text-5 [new] Text selection, caret movement, undo and redo, and clipboard in text fields. The
  existing `field` draws a placeholder and a caret when focused and has no selection. Resolved:
  `text.text-edit` (M5), with the clipboard through `platform.clipboard` (M10).

### Render

- R-render-1 [existing] The scene, blur and target pass sequence over wgpu (`gpu.rs`, `frame.rs`).
- R-render-2 [existing] Backdrop blur pass (`blur.rs`).
- R-render-3 [existing] App-owned GPU drawing inside a layer through the `CustomDraw` hook; app
  specific surfaces (the terminal grid) draw through it, not in the toolkit (`custom.rs`, D8).
- R-render-4 [existing] Offscreen target, GPU readback and PNG writer, with no window or screen
  capture, so frames can be dumped for goldens (`dump.rs`).
- R-render-5 [existing] The Avalonia acrylic noise texture is bundled and licensed
  (`assets/acrylic-noise.png`, MIT).

### Platform (OS glass and window helpers)

- R-platform-1 [existing] macOS glass: NSVisualEffectView and NSGlassEffectView on the frame view
  (`material/macos.rs`).
- R-platform-2 [existing] Windows glass: DWM acrylic, mica and tabbed through `window-vibrancy`
  (`material/windows.rs`).
- R-platform-3 [existing] Linux and others: no OS glass; a glass `Material` returns an error and the
  app falls back to an opaque theme (`material/other.rs`).
- R-platform-4 [existing] Glass window attributes, edge resize hit test, double-click detection and
  caption button geometry for a custom caption (`window.rs`).
- R-platform-5 [new] Narrowed by D43. In the toolkit: the OS clipboard (`platform.clipboard`, M10),
  cursor icons (`core.input`, `platform.input`; the cursor output exists) and inbound OS file-drop
  events (`core.input`, `platform.input`, M10). App-side or out of v1: native file dialogs, outbound
  OS drag and drag between windows (Lunegit has no use). In-window drag and drop is R-infra-5.
- R-platform-6 [new] Narrowed by D43 and D48. In the toolkit: the system tray or menu-bar status item
  with a status dot (`platform.tray-icon`), and its glass flyout (`overlay.flyout`) and glass menu
  (`overlay.menu`) in tray-anchored popover windows. App-side, documented: the close-to-tray policy,
  single instance, autostart at sign-in and OS notifications (Wane lists them,
  `docs/ui/components.md:702-717`).

### Widgets

- R-widget-1 [existing] Label, toggle, combo, stepper, text field, button and checkbox, drawn through
  the painter and generic over the app's hit type (`widgets/mod.rs`).
- R-widget-2 [new] The controls Lunegit uses that the toolkit lacks: scrollbar (`widget.scrollbar`),
  tooltip (`overlay.tooltip`), star rating (`widget.star-rating`), tab control (`widget.tab-strip`),
  and the other Lunegit controls in the catalogue (`widget.filter-bar`, `widget.search-picker`,
  `widget.list-row-badge`, `widget.banner-empty-state`, `widget.toolbar`). Resolved by the M3
  specs; D42 maps the app-shaped ones. Built in M6: star rating, tab control, filter bar, search picker,
  list-row badge, banner and empty state, toolbar (the scrollbar and tooltip are M5).
- R-widget-3 [new] Keyboard focus order, focus-visible and key handling inside widgets: Tab order
  follows call order within a focus scope, overlays push and pop scopes. Not in today's `lune-ui`.
  Resolved: `core.focus` (M4) and the focus rules in `core.ui`.
- R-widget-4 [new] The controls Wane uses that the toolkit lacks: radio group, status dot, navigation
  item, badge, form field (label, control, hint), setting and item rows, callout, wizard steps,
  verdict row. Built in M6 (each with a page and scenes in the gallery).
- R-widget-5 [new] Controls with no Lunegit look: slider (`widget.slider`) and spinner
  (`widget.spinner`), their looks derived from sibling tokens (D51). Built in M6.

### Overlays

- R-overlay-1 [existing] Popup panel, modal dialog, dropdown (flips above its anchor near the window
  bottom) and glass context menu (`widgets/`). M7 upgraded them: menu keys, variants and anchored menus, the
  dropdown's glass and scrolling list, dialog variants, and an overlay above a dialog (the `Raised` layer, D137).
- R-overlay-2 [new] Toasts, with a history list (Lunegit has both). Built in M7.
- R-overlay-3 [new] Tooltips and flyouts. The tooltip is M5; the flyout's surface and the command palette are M7
  (the tray window and its placement are M10).
- R-overlay-4 [new] Anchored popover placement and dismiss: an anchor rect placed with flip and shift
  inside the window, dismissed on outside click, Escape and focus loss, opening and closing with
  motion tokens (`overlay.popover`).

### Layout and data views

- R-layout-1 [new] Layout containers. No layout module exists in `lune-ui` today; widgets take a
  `Rect` the app computed. Built in M7: page, panel, sidebar, side panel, icon rail and settings page (each takes
  its rect).
- R-layout-2 [new] Resizable split panes and scrolling regions that preserve offset. Split panes are M7.
- R-layout-3 [new] Virtualized lists and tree views with tree guide lines, for long lists (Lunegit
  uses virtualizing panels in several views). The tree view is M7.
- R-data-1 [new] Domain views Lunegit draws itself: commit graph rows, diff views (unified and
  split) with hunk actions and conflict strips, pie chart, kanban board with drag-and-drop.
  Resolved by D42: toolkit components `data.lane-graph`, `text.diff-view`, `data.pie-chart` and
  `data.kanban-board`; the conflict strip is external to the toolkit (its line bands and synced
  scroll fold into `text.code-editor-view`).
- R-data-2 [new] Dashboard data views Wane uses: data table, stat cell strip, key-value list, progress
  bar and stacked bar chart. Resolved: all five are toolkit components (`data.table`, `data.stat`,
  `data.kv-list`, `data.progress`, `data.bar-chart`).

### Motion

- R-motion-1 [existing] Cubic-bezier easing and menu open and close motion; the app passes the time
  in (`widgets/menu.rs`).
- R-motion-2 [new] General transitions and animation tiers, including a reduced-motion tier (Lunegit
  has `Animations.axaml` and `AnimationsReduced.axaml`): `motion.transitions` and
  `motion.animation-tiers`, with the vocabulary in `docs/motion.md`.
- R-motion-3 [new] The motion system and the 60 fps contract: closed-form values of the app's time, a
  60 Hz conformance harness for every transition, a bench budget at milestone acceptance and Idle
  when settled (D41).

### Infrastructure

- R-infra-1 [new] Localization: Lunegit loads `en.json` plus nine sample languages (de, es, fr, ja,
  ko, ms, pt-BR, ru, zh-CN). Today's `lune-ui` has none.
- R-infra-2 [new] Keyboard shortcut routing. Lunegit binds keys per view; a central registry was
  not located.
- R-infra-3 [new] Theme values loadable from JSON and re-appliable at run time (Lunegit's
  `ThemeApplicator` and sample themes).
- R-infra-4 [new] Accessibility (screen readers, focus indication, contrast). Not covered by
  today's `lune-ui`. Resolved by D46: a semantic tree every frame from M4 (`infra.accessibility`,
  with focus indication in `core.focus`) and an AccessKit bridge behind a cargo feature in M10.
- R-infra-5 [new] In-window drag and drop (pointer capture and drag state) for tab reorder, kanban
  cards and the split diff in Lunegit. OS-level drag and drop is R-platform-5.
- R-infra-6 [new] A small bounded JSON reader in core (input size, depth and string-length limits,
  fail-closed errors, no panics) behind theme and locale loading (`infra.json`, D44).

## Non-functional

- R-nfr-1 [existing] Deterministic frames: library `src` never reads the clock or the environment
  and never starts an event loop; the app passes time, sizes, fonts, theme and hit values in.
  Enforced by `scripts/check-boundaries.sh` rule 4.
- R-nfr-2 [new] App-agnostic: no app, terminal or domain words in library source (boundary
  rules 1 to 3). Not met by today's Luneterm `crates/lune-ui`: it names Lunegit in docs and in the
  `lunegit_dark` preset (21 lines in 11 files under rule 3 as written here). Met since M1: the
  extraction renamed or allow-listed those (`scripts/boundaries.allow`), and
  `scripts/check-boundaries.sh` (rules 1 to 3, in the gate) holds it.
- R-nfr-3 [existing] `unsafe` only in platform modules, each block with a `// SAFETY:` comment
  (boundary rule 6, `CLAUDE.md` hard rule 3). Luneterm's three blocks are in `material/macos.rs`.
- R-nfr-4 [existing] Size: the release profile is LTO, one codegen unit, abort on panic, stripped;
  adding a dependency means measuring the release binary and noting the change. No numeric budget
  in v1: each dependency commit carries a size note (hard rule 5), and M11 reports the release size.
  Met by the manifest: `[profile.release]` in the root `Cargo.toml` (`lto = "fat"`,
  `codegen-units = 1`, `panic = "abort"`, `strip = true`). The M11 figure for `cargo build --release
  -p gallery` is in `docs/STATE.md` and `docs/consuming.md` (9,869,520 bytes at a9d1292).
- R-nfr-5 [new] Performance targets, decided by D41: every transition passes the 60 Hz conformance
  harness, a settled page reports Idle, and `gallery --bench` reads p99 frame time at most 16.6 ms
  and no frame over 33 ms after warm-up on every gallery page (the budget half is a recorded
  reading, not a gate: D235, the user's waiver of 2026-11-07; D85 says how it is judged). No
  memory target in v1. The running-outline CPU budget of at most 3 percentage points (Luneterm
  `docs/lunegit-bug-ledger.md` row 16, roadmap M11) stays Luneterm's.
- R-nfr-6 [new] HiDPI: scale is a per-window, per-frame input and any positive scale works (the text
  cache already takes the scale per call, D27). Proven at scale 1 and 2 by M2's two-target test;
  fractional scales get a 1.5x gallery scene in M11 coverage: `ui-controls@1.5x`, 1560 by 990
  physical px, in the page `ui` and the catalogue entry `render.frame` (D243).
- R-nfr-7 [existing] Platforms: macOS and Windows get glass, Linux gets an opaque fallback
  (Luneterm decision 3). Met by `Material` and `lune_ui_platform::material` (`platform.material`)
  and held at compile time by `scripts/check-targets.sh` (Windows clippy, Linux in Docker); the run
  time on Windows and Linux is unverified here, and CI stays out (D15, kept: the repo has no remote).
- R-nfr-8 [new] Testability: each seam (text shaping, GPU, glass, windowing) gets a contract test
  and a fake (headless target, null material) once the extraction plan introduces it.

## Extraction

- R-ext-1 [new] The first move is behaviour-preserving: code moves from Luneterm's `lune-ui` as is;
  seams follow as separate gated steps.
- R-ext-2 [new] Pixel gate: the 18 scripted scenes of Luneterm's `glass-ui` spike, dumped before and
  after, compare at zero delta with its `dump_compare` tool, until a change is deliberate and
  re-baselined with its reason. The baseline form is decided: dumps are regenerated from
  `git archive <base>`, not committed (D11).
  Since M2 the gate is `scripts/pixel-gate.sh` over the gallery's own scenes (every name `gallery --list`
  prints, 421 at M11's close), the 18 spike scenes having served M1 only.
- R-ext-3 [new] Luneterm switches from its in-tree `lune-ui` to this repo's. How it depends on it
  before a remote exists is a vendored snapshot (D13); a git dependency follows once a remote exists.
  Met by `scripts/export-snapshot.sh` (D239) and `docs/consuming.md`, which also holds the migration
  map; `lune-termview --all-targets` checks against the snapshot with no source change.
- R-ext-4 [new] Every bundled asset or code has its licence text in `licenses/` and a row in
  `THIRD_PARTY_NOTICES.md`: the Avalonia noise texture and colour formulas, any font the toolkit
  ships, any shader taken from elsewhere. Both exist: `licenses/` and `THIRD_PARTY_NOTICES.md`.
- R-ext-5 [new] Crate names and split are set by the extraction plan; `lune-termview` stays in
  Luneterm (D8). Met by the manifest: `members` in the root `Cargo.toml` and `crates/` (five crates,
  `docs/architecture.md` "Workspace layout"); the snapshot's `members` is `["crates/*"]` (D239).

## Out of scope

- The terminal grid and its renderer (`lune-termview`), the VT engine, PTY handling: Luneterm.
- Git logic, repository state, AI-session features: Lunegit.
- A web or CSS engine: styles are Rust tokens.
- App-side, documented for apps (D43): native file dialogs, OS notifications, single instance,
  autostart, the close-to-tray policy, opening URLs (a link widget emits an event), and RTL
  mirroring.
- Outbound OS drag and drag between windows: Lunegit has no use for them (D43).
