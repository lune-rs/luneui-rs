//! The `side-panel` page: the host on the panel glass showing one of three panels (a row list
//! each), with a row of buttons that switch the shown panel and a switch for the window's active
//! state. The glass layer is the page's choice (the widget names no preset); inactive, the host
//! draws the fallback fill. A switch shows the new panel with the drawer motion.

use std::time::Duration;

use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::{CARD, PANEL, THEME};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, ButtonOptions};
use lune_ui::widgets::side_panel::SidePanelOptions;

use super::{caption, card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "side-panel",
    title: "Side panel",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
/// The host's top left and size.
const HOST: Rect = Rect::new(X, Y + 72.0, 360.0, 440.0);
const KEYS: [u64; 3] = [1, 2, 3];
/// Neutral stand-ins for an app's panels.
const TITLES: [&str; 3] = ["Items", "Branches", "Changes"];
const ROW: f32 = 28.0;

fn live() -> Box<dyn PageState> {
    Box::new(Live::new(1, true, None))
}

fn inactive() -> Box<dyn PageState> {
    Box::new(Live::new(1, false, None))
}

/// Panel 1 shown, then panel 2 at 200 ms.
fn show_later() -> Box<dyn PageState> {
    Box::new(Live::new(1, true, Some(Duration::from_millis(200))))
}

/// The host and three panels; the knobs choose the panel and how many rows it holds.
struct Live {
    panel: u64,
    active: bool,
    switch_at: Option<Duration>,
    rows: u32,
}

impl Live {
    fn new(panel: u64, active: bool, switch_at: Option<Duration>) -> Live {
        Live {
            panel,
            active,
            switch_at,
            rows: 10,
        }
    }
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Side panel");
        let (ox, oy) = (card.x + 24.0, card.y + 24.0);
        let host = on_card(card, HOST);
        heading(ui, "SIDE PANEL", ox, oy);
        if self.switch_at.is_some_and(|t| ui.time() >= t) {
            self.panel = 2;
        }
        // The buttons that switch the shown panel.
        let ghost = ButtonOptions {
            kind: ButtonKind::Secondary,
            small: true,
            icon: None,
        };
        for (i, (key, title)) in KEYS.iter().zip(TITLES).enumerate() {
            let r = Rect::new(ox + i as f32 * 116.0, oy + 24.0, 108.0, 24.0);
            if ui.button_with(("switch", *key), r, title, &ghost).clicked {
                self.panel = *key;
            }
        }
        let opts = SidePanelOptions {
            panel: self.panel,
            keys: &KEYS,
            active: self.active,
            glass: PANEL,
            under: CARD.premultiplied(),
            name: TITLES[(self.panel - 1) as usize],
        };
        let rows = self.rows;
        ui.side_panel("host", host, &opts, |ui, inner| {
            let title = Spec::plain(
                TITLES[(opts.panel - 1) as usize],
                Style::ui(THEME.font_base, THEME.fg_bright).medium(),
            );
            ui.painter().text(&title, inner.x, inner.y + 2.0);
            for i in 0..rows {
                let r = Rect::new(inner.x, inner.y + 32.0 + i as f32 * ROW, inner.w, ROW);
                let p = ui.painter();
                if i % 2 == 0 {
                    p.fill(r, 6.0, THEME.surface_hover);
                }
                let line = Spec::plain(
                    format!("{} {}", TITLES[(opts.panel - 1) as usize], i + 1),
                    Style::ui(THEME.font_base, THEME.fg_secondary),
                );
                p.text_in(&line, r, 8.0);
            }
        });
        ui.toggle(
            "active",
            Rect::new(ox, host.bottom() + 16.0, 40.0, 24.0),
            &mut self.active,
            "Window active",
        );
        caption(ui, "Window active", ox + 52.0, host.bottom() + 19.0);
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let mut at = (self.panel.clamp(1, 3) - 1) as usize;
        if panel.choice(ui, "Panel", &TITLES, &mut at) {
            self.panel = KEYS[at];
        }
        let mut rows = self.rows as f32;
        if panel.number(ui, "Rows", &mut rows, (0.0, 30.0, 1.0)) {
            self.rows = rows.round() as u32;
        }
    }
}

const SWITCH_2: (f32, f32) = (X + 116.0 + 54.0, Y + 24.0 + 12.0);

const SCENES: &[UiScene] = &[
    // The first panel shown, settled.
    UiScene {
        id: "side-panel-shown",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The window is inactive: the solid fallback replaces the glass.
    UiScene {
        id: "side-panel-inactive",
        scale: 2.0,
        tier: MotionTier::Full,
        state: inactive,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The show is `moderate` (350 ms): half way 175 ms after the first frame of the new panel.
    UiScene {
        id: "side-panel-show-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: show_later,
        script: &[Step {
            at_ms: 200,
            events: &[],
        }],
        capture_ms: 375,
        expect: Expect::Moving,
    },
];

/// What the bench replays: switch panels with the buttons, then Tab and Enter.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(SWITCH_2.0, SWITCH_2.1)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 600,
        events: &[Ev::PointerAt(X + 54.0, SWITCH_2.1)],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1200,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 1300,
        events: &[Ev::Key(NamedKey::Enter)],
    },
];
