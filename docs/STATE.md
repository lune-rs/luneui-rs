<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-04 (M3 review round 1 fixes: milestone order of new-code dependencies, clipboard types in M4, colour tokens in D58)

## Now

M3 (catalogue specs) is implemented on branch `orch/toolkit-v1`, awaiting review and merge, on top of
M2 (seams and crate split). All 97 catalogue entries carry a `## Spec` (format: DECISIONS
D49, README "Body sections") with Lunegit, Wane or repo `file:line` cites; none is a `stub`.
`docs/motion.md` fixes the motion vocabulary (D50); `docs/requirements.md` has no open item;
`core.painter` holds the aggregate table of new painter primitives (image, rotate, dash,
clip-rounded, sector, path-fill, arc), the list M5 builds. DECISIONS D40 to D58 record the M3 choices,
including the interaction-model additions M4 builds from: a `Toast` layer above `Modal` (D54), the
36 roles and 15 states (D55), new `Theme` tokens (D58, which supersedes D56), `{n:spec}` formatting through an `ArgFormat`
hook (D52). The code is the M2 five-crate toolkit (pure `lune-ui-core`, adapters `lune-ui-wgpu`,
`lune-ui-winit`, `lune-ui-platform`, facade `lune-ui`); M3 changed docs only, so the 15 gallery scenes
are unchanged. Window-mode code is compile- and review-checked only: no gate opens a visible window.
Review round 1 moved `data.stat`, `data.kv-list` and `data.progress` to M6 and `infra.shortcut-routing` to M5
(entries come after what they depend on); `overlay.flyout` ships its surface in M7 and tray anchoring in M10; the
`ClipboardRequest` type is M4, its fake M5, its adapter M10.

Wane is a third consumer (2026-11-04): its UI spec is catalogued, R-platform-6 narrowed (tray, flyout
and glass menu in toolkit; close-to-tray, single instance, autostart and notifications app-side).

## Next

1. Review and merge M3, then M4 (roadmap): the interaction model in core (`Ui`, ids, retained state,
   layers, focus, semantic tree, string table, motion system) and the retrofit of the moved widgets,
   built from the `core.ui`, `core.focus`, `infra.accessibility`, `infra.localization` and
   `motion.transitions` specs.
2. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
3. `scripts/export-snapshot.sh` for Luneterm's vendored snapshot (D13), before Luneterm M9. The
   migration map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None. Wane's OS behaviours (close-to-tray, single instance, autostart, notifications; R-platform-6)
  are assumed app-side and documented (answer 2026-11-03, reversible).
