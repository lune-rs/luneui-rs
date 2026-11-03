<!-- Append-only. Add a row when a choice has a real alternative. Never edit or delete an old row:
supersede it with a new row that says "supersedes D<n>". Do not re-open a decision unless the user asks. -->
# Decisions

D1-D5 carry settled decisions from Luneterm's `CLAUDE.md` ("Decisions", made 2026-10-29); the source
is cited per row.

| ID | Date | Decision | Why | Rejected |
|---|---|---|---|---|
| D1 | 2026-11-03 | Native Rust; no CSS or web engine; styles are Rust tokens (Luneterm decision 5) | small binary, one toolchain, deterministic frames | a web/CSS engine, hosted JS |
| D2 | 2026-11-03 | Third-party crates only for platform layers: windowing (winit), GPU (wgpu), text shaping (glyphon), OS glass (window-vibrancy, objc2). Adapted from Luneterm decision 2 (PTY dropped, text shaping added) | keep the toolkit small and its dependencies swappable | a UI framework (egui, iced, Slint) as the base |
| D3 | 2026-11-03 | Library-first and app-agnostic: no app words, no clock or environment reads, no owned event loop (Luneterm decision 5) | several apps share it: Luneterm, a possible Rust rebuild of Lunegit, others | per-app copies; keeping the code inside Luneterm |
| D4 | 2026-11-03 | The look follows Lunegit: default glass is Lunegit's layer stack; Liquid Glass and off are options; Linux gets an opaque fallback only (Luneterm decision 4) | one visual identity across the apps | a new look |
| D5 | 2026-11-03 | UI font RznSans (Luneterm decision 4); fonts are supplied by the app, the toolkit bundles none unless a later decision says so | house font; keeps font licences out of the library | system or Google fonts |
| D6 | 2026-11-03 | The toolkit is extracted from Luneterm `crates/lune-ui`, because a second consumer is planned (Luneterm `docs/architecture.md`, "Split points": "`lune-ui` moves when a second app starts") | share one toolkit | keeping it inside Luneterm |
| D7 | 2026-11-03 | Licence MIT OR Apache-2.0, matching Luneterm (user answer) | consistency with Luneterm | a single licence |
| D8 | 2026-11-03 | `lune-termview` (Luneterm's terminal-grid renderer) stays in Luneterm; the toolkit only exposes the `CustomDraw` hook it builds on (user decision). Overrides the Luneterm `docs/architecture.md` "Split points" line that says termview moves with `lune-ui` | terminal-grid rendering is app domain; keeps the toolkit app-agnostic | moving `lune-termview` here with the toolkit |
