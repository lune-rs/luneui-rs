//! The `panel` page: the outlined group box (`layout.panel`) in its five kinds. The window shows a
//! panel of each kind in a grid; every scene draws one kind large: a flush list under a hairline
//! header, a strip of stat cells, a card nested in a panel with its button pinned to the bottom,
//! and the unavailable line.

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::button::ButtonKind;
use lune_ui::widgets::panel::{PanelKind, PanelOptions, PanelSlot, TEXT_INSET};
use lune_ui::widgets::stat::StatOptions;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "panel",
    title: "Panel",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: &[],
    }),
};

/// Which panels a state draws.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Show {
    All,
    Flush,
    Strip,
    Card,
    Unavailable,
}

struct Live(Show);

fn live() -> Box<dyn PageState> {
    Box::new(Live(Show::All))
}

fn flush() -> Box<dyn PageState> {
    Box::new(Live(Show::Flush))
}

fn strip() -> Box<dyn PageState> {
    Box::new(Live(Show::Strip))
}

fn nested() -> Box<dyn PageState> {
    Box::new(Live(Show::Card))
}

fn unavailable() -> Box<dyn PageState> {
    Box::new(Live(Show::Unavailable))
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "Panel");
        let (x, y) = (area.x + 24.0, area.y + 18.0);
        if self.0 != Show::All {
            let at = Rect::new(x, y + 24.0, 640.0, 360.0);
            match self.0 {
                Show::Flush => {
                    heading(ui, "FLUSH", x, y);
                    flush_panel(ui, at);
                }
                Show::Strip => {
                    heading(ui, "STRIP", x, y);
                    strip_panel(ui, at);
                }
                Show::Card => {
                    heading(ui, "CARD", x, y);
                    card_panel(ui, at);
                }
                Show::Unavailable => {
                    heading(ui, "UNAVAILABLE", x, y);
                    unavailable_panel(ui, Rect::new(at.x, at.y, at.w, 200.0));
                }
                Show::All => {}
            }
            return;
        }
        // The grid: two columns, two rows, with the strip hugging its height.
        let w = (area.w - 48.0 - 16.0) / 2.0;
        let h = (area.h - 18.0 - 24.0 - 24.0 - 16.0 - 24.0) / 2.0;
        let (x2, y2) = (x + w + 16.0, y + 24.0 + h + 16.0 + 24.0);
        heading(ui, "FLUSH", x, y);
        flush_panel(ui, Rect::new(x, y + 24.0, w, h));
        heading(ui, "CARD", x2, y);
        card_panel(ui, Rect::new(x2, y + 24.0, w, h));
        heading(ui, "UNAVAILABLE", x, y2 - 24.0);
        unavailable_panel(ui, Rect::new(x, y2, w, h));
        heading(ui, "STRIP", x2, y2 - 24.0);
        strip_panel(ui, Rect::new(x2, y2, w, h));
    }
}

/// A row of a list: a zebra fill across the panel, the label 8 px in and the status at the right.
fn list(ui: &mut Ui<'_>, r: Rect, rows: &[(&str, &str)]) -> f32 {
    for (i, (label, status)) in rows.iter().enumerate() {
        let row = Rect::new(r.x, r.y + i as f32 * 32.0, r.w, 32.0);
        let p = ui.painter();
        if i % 2 == 0 {
            p.fill(row, 0.0, THEME.surface_hover);
        }
        let text = Spec::plain(*label, Style::ui(THEME.font_base, THEME.fg_primary));
        p.text(&text, row.x + TEXT_INSET + 8.0, row.y + 6.0);
        let status = Spec::plain(
            *status,
            Style::mono(THEME.font_base, 20.0, THEME.fg_secondary),
        );
        let w = p.measure(&status).0;
        p.text(&status, row.right() - TEXT_INSET - 8.0 - w, row.y + 6.0);
    }
    rows.len() as f32 * 32.0
}

const REQUESTS: &[(&str, &str)] = &[
    ("POST /v1/messages", "200"),
    ("POST /v1/messages", "200"),
    ("GET /v1/models", "200"),
    ("POST /v1/messages", "429"),
    ("POST /v1/messages", "200"),
    ("GET /v1/models", "200"),
    ("POST /v1/messages", "200"),
    ("POST /v1/messages", "500"),
];

fn flush_panel(ui: &mut Ui<'_>, at: Rect) {
    let opts = PanelOptions {
        title: Some("Recent requests"),
        meta: Some("last hour"),
        kind: PanelKind::Flush,
        ..PanelOptions::default()
    };
    ui.panel("flush", at, &opts, |ui, slot| match slot {
        PanelSlot::Body(r) => list(ui, r, REQUESTS),
        PanelSlot::Footer(_) => 0.0,
    });
}

fn strip_panel(ui: &mut Ui<'_>, at: Rect) {
    let cells = [
        StatOptions {
            label: "Tokens saved",
            value: "1,284,300",
            sub: Some("+12% vs last week"),
            sub_tone: lune_ui::widgets::tone::Tone::Ok,
            ..StatOptions::default()
        },
        StatOptions {
            label: "Requests",
            value: "8,412",
            sub: Some("3 failed"),
            sub_tone: lune_ui::widgets::tone::Tone::Warn,
            ..StatOptions::default()
        },
        StatOptions {
            label: "Compression",
            value: "38%",
            sub: Some("of input tokens"),
            ..StatOptions::default()
        },
    ];
    let opts = PanelOptions {
        kind: PanelKind::Strip,
        fit: true,
        ..PanelOptions::default()
    };
    ui.panel("strip", at, &opts, |ui, slot| match slot {
        PanelSlot::Body(r) => ui.stat_strip("cells", r, &cells),
        PanelSlot::Footer(_) => 0.0,
    });
}

/// A plain panel holding a card with its own footer button, pinned to the card's bottom.
fn card_panel(ui: &mut Ui<'_>, at: Rect) {
    let opts = PanelOptions {
        title: Some("Account"),
        meta: Some("2 keys"),
        kind: PanelKind::Plain,
        ..PanelOptions::default()
    };
    ui.panel("outer", at, &opts, |ui, slot| {
        let PanelSlot::Body(r) = slot else {
            return 0.0;
        };
        let inner = PanelOptions {
            title: Some("Credentials"),
            note: Some("Stored in the keychain"),
            kind: PanelKind::Card,
            footer_w: 96.0,
            ..PanelOptions::default()
        };
        let card_rect = Rect::new(r.x, r.y, r.w, r.h.min(190.0));
        ui.panel("card", card_rect, &inner, |ui, slot| match slot {
            PanelSlot::Body(b) => {
                let text = Spec::plain(
                    "API key  sk-ant-\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}",
                    Style::ui(THEME.font_base, THEME.fg_secondary),
                );
                ui.painter().text(&text, b.x, b.y + 4.0);
                28.0
            }
            PanelSlot::Footer(f) => {
                ui.button("save", f, "Save", ButtonKind::Primary);
                0.0
            }
        });
        card_rect.h
    });
}

fn unavailable_panel(ui: &mut Ui<'_>, at: Rect) {
    let opts = PanelOptions {
        title: Some("Proxy"),
        meta: Some("stopped"),
        kind: PanelKind::Plain,
        unavailable: Some("Unavailable while the proxy is stopped."),
        footer_w: 120.0,
        ..PanelOptions::default()
    };
    ui.panel("unavailable", at, &opts, |ui, slot| {
        if let PanelSlot::Footer(f) = slot {
            ui.button("start", f, "Start proxy", ButtonKind::Primary);
        }
        0.0
    });
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "panel-flush",
        scale: 2.0,
        tier: MotionTier::Full,
        state: flush,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "panel-strip",
        scale: 2.0,
        tier: MotionTier::Full,
        state: strip,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "panel-card",
        scale: 2.0,
        tier: MotionTier::Full,
        state: nested,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "panel-card",
        scale: 1.0,
        tier: MotionTier::Full,
        state: nested,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "panel-unavailable",
        scale: 2.0,
        tier: MotionTier::Full,
        state: unavailable,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];
