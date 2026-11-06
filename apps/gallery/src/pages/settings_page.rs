//! The `settings-page` page: the settings page frame (`layout.settings-page`) with a section of
//! three setting rows (a toggle, a combo with its list and a stepper) and the restart banner.
//! Changing the toggle sets the restart flag, as an app would for a setting that needs one; the
//! banner's action clears it. The glass layer is the page's choice (the widget names no preset);
//! inactive, the page is the solid fallback.

use std::time::Duration;

use lune_ui::lunegit_dark::{CARD, PANEL};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::row::SettingRow;
use lune_ui::widgets::settings_page::SettingsPage;
use lune_ui::widgets::stepper::StepRange;

use super::controls_ui::OPTIONS;
use super::{card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "settings-page",
    title: "Settings page",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The page's rect inside the card (the card is at (10, 34), the heading 24 px in).
const PAGE_RECT: Rect = Rect::new(34.0, 82.0, 972.0, 544.0);
/// The content host's inset, and the rows' margin inside it (`GeneralSettingsView.axaml:148`,
/// `Margin="12,0"`: the rows stretch).
const HOST_INSET: f32 = 20.0;
const ROW_MARGIN: f32 = 12.0;
/// The controls' widths.
const TOGGLE_W: f32 = 40.0;
const COMBO_W: f32 = 210.0;
const STEPPER_W: f32 = 140.0;
const STEPS: StepRange = StepRange {
    min: 1.0,
    max: 50.0,
    step: 1.0,
};

fn live() -> Box<dyn PageState> {
    Box::new(Live::new(false, true, None))
}

/// The first row inside `with_enabled(false, ..)`.
fn row_disabled() -> Box<dyn PageState> {
    let mut live = Live::new(false, true, None);
    live.row_disabled = true;
    Box::new(live)
}

fn restart() -> Box<dyn PageState> {
    Box::new(Live::new(true, true, None))
}

fn inactive() -> Box<dyn PageState> {
    Box::new(Live::new(false, false, None))
}

/// The flag rises at 200 ms.
fn restart_later() -> Box<dyn PageState> {
    Box::new(Live::new(false, true, Some(Duration::from_millis(200))))
}

struct Live {
    restart: bool,
    active: bool,
    restart_at: Option<Duration>,
    updates: bool,
    font: usize,
    combo_open: bool,
    recent: u32,
    /// Draw the first row inside `with_enabled(false, ..)`.
    row_disabled: bool,
}

impl Live {
    fn new(restart: bool, active: bool, restart_at: Option<Duration>) -> Live {
        Live {
            restart,
            active,
            restart_at,
            updates: true,
            font: 0,
            combo_open: false,
            recent: 10,
            row_disabled: false,
        }
    }
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Settings page");
        heading(ui, "SETTINGS PAGE", card.x + 24.0, card.y + 24.0);
        if self.restart_at.is_some_and(|t| ui.time() >= t) {
            self.restart = true;
            self.restart_at = None;
        }
        let page = SettingsPage {
            active: self.active,
            glass: PANEL,
            under: CARD.premultiplied(),
            restart: self.restart,
            name: "General",
        };
        let (updates, font, combo_open, recent) = (
            &mut self.updates,
            &mut self.font,
            &mut self.combo_open,
            &mut self.recent,
        );
        let mut changed = false;
        let first_enabled = !self.row_disabled;
        let resp = ui.settings_page("page", on_card(card, PAGE_RECT), &page, |ui, vp| {
            let mut y = vp.y;
            let before = *updates;
            let row = SettingRow {
                title: "Check for updates",
                description: Some("Look for a new version when the app starts."),
                off: !*updates,
                control_w: TOGGLE_W,
            };
            y += 2.0
                + ui.with_enabled(first_enabled, |ui| {
                    ui.setting_row(
                        "updates",
                        Rect::new(vp.x + ROW_MARGIN, y, vp.w - 2.0 * ROW_MARGIN, 0.0),
                        &row,
                        |ui, slot| {
                            ui.toggle("switch", slot, updates, "Check for updates");
                        },
                    )
                });
            changed |= before != *updates;

            let row = SettingRow {
                title: "Interface font",
                description: Some("Applies to every window."),
                off: false,
                control_w: COMBO_W,
            };
            y += 2.0
                + ui.setting_row(
                    "font",
                    Rect::new(vp.x + ROW_MARGIN, y, vp.w - 2.0 * ROW_MARGIN, 0.0),
                    &row,
                    |ui, slot| {
                        let at = Rect::new(slot.x, slot.y + (slot.h - 32.0) / 2.0, COMBO_W, 32.0);
                        let r =
                            ui.combo("combo", at, OPTIONS[*font].0, combo_open, "Interface font");
                        if r.clicked && !r.changed {
                            *combo_open = false;
                        }
                        if *combo_open {
                            let d = ui.dropdown("list", r.rect, r.id, &OPTIONS, *font);
                            if let Some(i) = d.picked {
                                *font = i;
                            }
                            if d.picked.is_some() || d.dismissed {
                                *combo_open = false;
                            }
                        }
                    },
                );

            let row = SettingRow {
                title: "Recent items",
                description: None,
                off: false,
                control_w: STEPPER_W,
            };
            y += ui.setting_row(
                "recent",
                Rect::new(vp.x + ROW_MARGIN, y, vp.w - 2.0 * ROW_MARGIN, 0.0),
                &row,
                |ui, slot| {
                    let at = Rect::new(slot.x, slot.y + (slot.h - 32.0) / 2.0, STEPPER_W, 32.0);
                    let s = ui.stepper("stepper", at, *recent, STEPS, "Recent items");
                    if let Some(set) = s.set {
                        *recent = set as u32;
                    } else if let Some(step) = s.step {
                        *recent = recent.saturating_add_signed(step).clamp(1, 50);
                    }
                },
            );
            y - vp.y
        });
        if changed {
            self.restart = true;
        }
        if resp.restart {
            self.restart = false;
        }
    }
}

/// The toggle of the first row: the row is 62 high and spans the host less its margins; the
/// toggle sits 12 in from the row's right edge.
const TOGGLE: (f32, f32) = (
    PAGE_RECT.x + PAGE_RECT.w - HOST_INSET - ROW_MARGIN - 12.0 - TOGGLE_W / 2.0,
    PAGE_RECT.y + HOST_INSET + 31.0,
);
/// The banner's action, near the page's bottom right.
const ACTION: (f32, f32) = (
    PAGE_RECT.x + PAGE_RECT.w - 20.0 - 56.0,
    PAGE_RECT.y + PAGE_RECT.h - 22.0,
);

const SCENES: &[UiScene] = &[
    // Three rows, no banner, settled.
    UiScene {
        id: "settings-page-rows",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The restart flag set from the first frame: the banner, settled.
    UiScene {
        id: "settings-page-restart",
        scale: 2.0,
        tier: MotionTier::Full,
        state: restart,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The reveal is `base` (250 ms): the flag rises at 200 ms, half way 125 ms after.
    UiScene {
        id: "settings-page-restart-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: restart_later,
        script: &[Step {
            at_ms: 200,
            events: &[],
        }],
        capture_ms: 325,
        expect: Expect::Moving,
    },
    // The first row is disabled (0.4 opacity): a click on its toggle changes nothing and raises no
    // restart banner, and the row draws no hover fill, settled.
    UiScene {
        id: "settings-page-disabled",
        scale: 2.0,
        tier: MotionTier::Full,
        state: row_disabled,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(TOGGLE.0, TOGGLE.1)],
            },
            Step {
                at_ms: 100,
                events: &[Ev::Click],
            },
        ],
        capture_ms: 600,
        expect: Expect::Settled,
    },
    // The window is inactive: the solid fallback replaces the glass.
    UiScene {
        id: "settings-page-inactive",
        scale: 2.0,
        tier: MotionTier::Full,
        state: inactive,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];

/// What the bench replays: flip the toggle (the banner reveals), then press the action.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(TOGGLE.0, TOGGLE.1)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 700,
        events: &[Ev::PointerAt(ACTION.0, ACTION.1)],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1300,
        events: &[Ev::Tab],
    },
];
