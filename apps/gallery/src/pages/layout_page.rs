//! The `page` page: the page card (`layout.page`) with a header (title, subtitle and two actions), a
//! scrolling body of a stat strip over a zebra table that runs edge to edge (so the card's rounded
//! clip shows at the corners) and, in the wizard variant, a pinned footer. The header's primary
//! action swaps the body for another page. The glass layer is the page's choice (the widget names
//! no preset); inactive, the card is the solid fallback.

use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::{CARD, PANEL, THEME};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::button::ButtonKind;
use lune_ui::widgets::page::{PageOptions, PageSlot};
use lune_ui::widgets::stat::StatOptions;

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "page",
    title: "Page card",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The table's row height and the number of rows of the short and the tall body.
const ROW: f32 = 32.0;
const SHORT: u32 = 6;
const TALL: u32 = 24;
/// The header actions: Export, then the primary that swaps the page.
const EXPORT_W: f32 = 80.0;
const NEXT_W: f32 = 112.0;
const ACTIONS_W: f32 = EXPORT_W + 8.0 + NEXT_W;
const BUTTON_H: f32 = 32.0;

fn live() -> Box<dyn PageState> {
    Box::new(Live::base())
}

fn short() -> Box<dyn PageState> {
    Box::new(Live {
        rows: SHORT,
        ..Live::base()
    })
}

fn tall_no_footer() -> Box<dyn PageState> {
    Box::new(Live {
        footer: false,
        rows: TALL,
        ..Live::base()
    })
}

fn inactive() -> Box<dyn PageState> {
    Box::new(Live {
        active: false,
        rows: SHORT,
        ..Live::base()
    })
}

struct Live {
    /// Which body is shown; the header's primary action adds one.
    key: u64,
    active: bool,
    footer: bool,
    rows: u32,
}

impl Live {
    fn base() -> Live {
        Live {
            key: 0,
            active: true,
            footer: true,
            rows: TALL,
        }
    }
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let area = card(ui, "Page card");
        heading(ui, "PAGE", area.x + 24.0, area.y + 18.0);
        let rect = Rect::new(area.x + 12.0, area.y + 40.0, area.w - 24.0, area.h - 52.0);
        let opts = PageOptions {
            title: if self.key % 2 == 0 {
                "Overview"
            } else {
                "Requests"
            },
            subtitle: Some("Last 30 days, all upstreams"),
            active: self.active,
            glass: PANEL,
            under: CARD.premultiplied(),
            actions_w: ACTIONS_W,
            footer: self.footer,
            swap_key: self.key,
            name: "",
        };
        let (key, rows) = (self.key, self.rows);
        let mut next = false;
        ui.page("card", rect, &opts, |ui, slot| match slot {
            PageSlot::Actions(at) => {
                let y = at.y + (at.h - BUTTON_H) / 2.0;
                ui.button(
                    "export",
                    Rect::new(at.x, y, EXPORT_W, BUTTON_H),
                    "Export",
                    ButtonKind::Secondary,
                );
                next |= ui
                    .button(
                        "next",
                        Rect::new(at.right() - NEXT_W, y, NEXT_W, BUTTON_H),
                        "Next page",
                        ButtonKind::Primary,
                    )
                    .clicked;
                0.0
            }
            PageSlot::Body(view, inner) => body(ui, view.viewport, inner, key, rows),
            PageSlot::Footer(row) => {
                ui.button(
                    "back",
                    Rect::new(row.x, row.y, 96.0, row.h),
                    "Back",
                    ButtonKind::Secondary,
                );
                ui.button(
                    "continue",
                    Rect::new(row.right() - 112.0, row.y, 112.0, row.h),
                    "Continue",
                    ButtonKind::Primary,
                );
                0.0
            }
        });
        if next {
            self.key += 1;
        }
    }
}

/// A stat strip over `rows` zebra rows that run to the card's edges. Returns the height.
fn body(ui: &mut Ui<'_>, viewport: Rect, inner: Rect, key: u64, rows: u32) -> f32 {
    let (a, b, c) = if key % 2 == 0 {
        ("1,284,300", "8,412", "38%")
    } else {
        ("902,114", "5,031", "41%")
    };
    let cells = [
        StatOptions {
            label: "Tokens saved",
            value: a,
            ..StatOptions::default()
        },
        StatOptions {
            label: "Requests",
            value: b,
            ..StatOptions::default()
        },
        StatOptions {
            label: "Compression",
            value: c,
            ..StatOptions::default()
        },
    ];
    let strip = ui.stat_strip("strip", Rect::new(inner.x, inner.y, inner.w, 0.0), &cells);
    let top = inner.y + strip + 16.0;
    for i in 0..rows {
        let r = Rect::new(viewport.x, top + i as f32 * ROW, viewport.w, ROW);
        let p = ui.painter();
        if i % 2 == 0 {
            p.fill(r, 0.0, THEME.surface_hover);
        }
        let label = Spec::plain(
            format!("Upstream {}", i + 1),
            Style::ui(THEME.font_base, THEME.fg_primary),
        );
        p.text(&label, inner.x, r.y + 6.0);
        let value = Spec::plain(
            format!("{} ms", 120 + 7 * i),
            Style::mono(THEME.font_base, 20.0, THEME.fg_secondary),
        );
        let w = p.measure(&value).0;
        p.text(&value, inner.right() - w, r.y + 6.0);
    }
    strip + 16.0 + rows as f32 * ROW
}

const NEXT: &[&str] = &["card", "next"];
const BODY: &[&str] = &["card", "body"];

/// The swap script: hover the primary, click it, then a frame at 200 ms is the first with the new
/// key (the state changes after the click's frame), so the swap starts there.
const SWAP: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(NEXT)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 200,
        events: &[],
    },
];

const SCENES: &[UiScene] = &[
    // The wizard variant: header with subtitle and two actions, a short body, the pinned footer.
    UiScene {
        id: "page-header-footer",
        scale: 2,
        tier: MotionTier::Full,
        state: short,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "page-header-footer",
        scale: 1,
        tier: MotionTier::Full,
        state: short,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // A body taller than the card, scrolled by 300 px, with no footer: the rows reach the card's
    // bottom corners, where the rounded clip cuts them.
    UiScene {
        id: "page-body-scroll",
        scale: 2,
        tier: MotionTier::Full,
        state: tall_no_footer,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(BODY)],
            },
            Step {
                at_ms: 100,
                events: &[Ev::Scroll {
                    x: 0.0,
                    y: -300.0,
                    precise: true,
                }],
            },
        ],
        capture_ms: 700,
        expect: Expect::Settled,
    },
    // The window is inactive: the solid fallback replaces the acrylic.
    UiScene {
        id: "page-inactive",
        scale: 2,
        tier: MotionTier::Full,
        state: inactive,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The swap is `moderate` (350 ms): half way at 175 ms after the first new-key frame.
    UiScene {
        id: "page-swap-opacity",
        scale: 2,
        tier: MotionTier::Full,
        state: short,
        script: SWAP,
        capture_ms: 375,
        expect: Expect::Moving,
    },
    // Reduced keeps the fade at the mapped duration (`fast`, 180 ms) and drops the slide.
    UiScene {
        id: "page-swap-reduced",
        scale: 2,
        tier: MotionTier::Reduced,
        state: short,
        script: SWAP,
        capture_ms: 290,
        expect: Expect::Moving,
    },
];

/// What the bench replays: the wheel over the body, then the header action.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(BODY)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Scroll {
            x: 0.0,
            y: -3.0,
            precise: false,
        }],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Scroll {
            x: 0.0,
            y: 3.0,
            precise: false,
        }],
    },
    Step {
        at_ms: 800,
        events: &[Ev::PointerOver(NEXT)],
    },
    Step {
        at_ms: 900,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1400,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 1500,
        events: &[Ev::Key(NamedKey::Enter)],
    },
];
