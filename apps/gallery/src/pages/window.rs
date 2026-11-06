//! The `window` page (`platform.window`, M11): the caption buttons through the `Ui` (rest,
//! maximized, hovered, mid-fade), the six-pixel resize border with its eight regions as `edge_at`
//! answers them, and the inactive window under Wane's theme values. The window mode runs the live
//! buttons: Maximize flips the restore glyph; Minimize and Close are the app's, and here do nothing.

use lune_ui::chrome;
use lune_ui::color::premul;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{LayerId, Rect};
use lune_ui::text::{Spec, Style};
use lune_ui::theme::Theme;
use lune_ui::ui::Ui;
use lune_ui::widgets::caption::{CaptionButton, CaptionOptions};
use lune_ui::window::{EDGE_BORDER, ResizeEdge, edge_at};

use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::scenes::{H, TITLE_BAND, W};

use super::{caption, card};

pub(crate) static PAGE: Page = Page {
    id: "window",
    title: "Window chrome",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// Wane's inactive window: the solid `--glass-inactive` for the window, near-black, instead of the
/// preset's `#333333` grey (the user's decision of 2026-11-03).
const WANE_INACTIVE: [u8; 4] = [0x0B, 0x0B, 0x0B, 0xFF];
/// Wane's `--glass-inactive-card`, the page card of an inactive window.
const WANE_INACTIVE_CARD: [u8; 4] = [0x14, 0x14, 0x14, 0xFF];

/// The caption buttons' height in the title band (Wane's `--titlebar-h`).
const BUTTON_HEIGHT: f32 = 32.0;

fn live() -> Box<dyn PageState> {
    Box::<Caption>::default()
}

fn maximized() -> Box<dyn PageState> {
    Box::new(Caption { maximized: true })
}

fn edges() -> Box<dyn PageState> {
    Box::new(Edges)
}

fn wane() -> Box<dyn PageState> {
    Box::new(Inactive)
}

/// The window with live caption buttons.
#[derive(Default)]
struct Caption {
    maximized: bool,
}

impl PageState for Caption {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "luneui gallery");
        caption(ui, "Caption buttons", card.x + 24.0, card.y + 24.0);
        let win = ui.painter().window;
        let opts = CaptionOptions {
            height: BUTTON_HEIGHT,
            maximized: self.maximized,
            active: true,
        };
        if ui.caption_buttons("caption", win, opts) == Some(CaptionButton::Maximize) {
            self.maximized = !self.maximized;
        }
    }
}

/// The six pixel border, each of its eight regions in its own tone, found by asking `edge_at` at
/// the middle of the region and named from the answer.
struct Edges;

impl PageState for Edges {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "luneui gallery");
        let t = *ui.theme();
        let (w, h, b) = (W as f32, H as f32, EDGE_BORDER);
        let regions = [
            (Rect::new(0.0, 0.0, b, b), t.ok),
            (Rect::new(b, 0.0, w - 2.0 * b, b), t.warn),
            (Rect::new(w - b, 0.0, b, b), t.err),
            (Rect::new(w - b, b, b, h - 2.0 * b), t.data),
            (Rect::new(w - b, h - b, b, b), t.data_hi),
            (Rect::new(b, h - b, w - 2.0 * b, b), t.gold),
            (Rect::new(0.0, h - b, b, b), t.paused),
            (Rect::new(0.0, b, b, h - 2.0 * b), t.accent),
        ];
        caption(ui, "Resize border, 6 px", card.x + 24.0, card.y + 24.0);
        for (i, (r, tone)) in regions.into_iter().enumerate() {
            let centre = (r.x + r.w / 2.0, r.y + r.h / 2.0);
            let name = edge_at(centre, w, h).map_or("none", edge_name);
            ui.painter().fill(r, 0.0, tone);
            // The legend: a swatch and the name `edge_at` gave, two columns of four.
            let (col, row) = ((i / 4) as f32, (i % 4) as f32);
            let (x, y) = (card.x + 24.0 + col * 160.0, card.y + 56.0 + row * 22.0);
            ui.painter()
                .fill(Rect::new(x, y + 2.0, 12.0, 12.0), 2.0, tone);
            caption(ui, name, x + 20.0, y);
        }
    }
}

fn edge_name(e: ResizeEdge) -> &'static str {
    match e {
        ResizeEdge::North => "North",
        ResizeEdge::South => "South",
        ResizeEdge::East => "East",
        ResizeEdge::West => "West",
        ResizeEdge::NorthEast => "North-east",
        ResizeEdge::NorthWest => "North-west",
        ResizeEdge::SouthEast => "South-east",
        ResizeEdge::SouthWest => "South-west",
    }
}

/// The inactive window under Wane's values: a solid near-black window, a `#141414` card, no
/// glass, caption glyphs in `fg_quaternary`. The window fill is the theme's `window_fallback`; the
/// card has no token, so the page names it.
struct Inactive;

impl PageState for Inactive {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let theme = *ui.theme();
        let win = ui.painter().window;
        let band = TITLE_BAND;
        let margin = 10.0;
        let card = Rect::new(margin, band, win.w - 2.0 * margin, win.h - band - margin);
        let p = ui.painter();
        p.on(LayerId::Base);
        let solid = premul(theme.window_fallback);
        p.acrylic(win, 0.0, solid, [0.0; 4], false);
        p.acrylic(
            card,
            chrome::CARD_RADIUS,
            premul(WANE_INACTIVE_CARD),
            solid,
            false,
        );
        let spec = Spec::plain(
            "luneui gallery",
            Style::ui(12.0, theme.fg_secondary).medium(),
        );
        p.text_centered(&spec, Rect::new(0.0, 0.0, win.w, band - 2.0));
        caption(ui, "Inactive, Wane", card.x + 24.0, card.y + 24.0);
        let opts = CaptionOptions {
            height: BUTTON_HEIGHT,
            maximized: false,
            active: false,
        };
        ui.caption_buttons("caption", win, opts);
    }

    fn theme(&self) -> Option<Theme> {
        Some(Theme {
            window_fallback: WANE_INACTIVE,
            ..THEME
        })
    }
}

const MIN: &[&str] = &["caption", "minimize"];
const MAX: &[&str] = &["caption", "maximize"];
const CLOSE: &[&str] = &["caption", "close"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "window-caption",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "window-caption-maximized",
        scale: 2.0,
        tier: MotionTier::Full,
        state: maximized,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "window-caption-hover-min",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(MIN)],
        }],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    UiScene {
        id: "window-caption-hover-close",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(CLOSE)],
        }],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "window-caption-hover-fade",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(CLOSE)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    UiScene {
        id: "window-edges",
        scale: 2.0,
        tier: MotionTier::Full,
        state: edges,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "window-inactive-wane",
        scale: 2.0,
        tier: MotionTier::Full,
        state: wane,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];

/// What the bench replays: the pointer along the three buttons, a click on the middle one (the
/// restore glyph flips), then away.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(MIN)],
    },
    Step {
        at_ms: 250,
        events: &[Ev::PointerOver(MAX)],
    },
    Step {
        at_ms: 300,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 600,
        events: &[Ev::PointerOver(CLOSE)],
    },
    Step {
        at_ms: 900,
        events: &[Ev::PointerAt(500.0, 300.0)],
    },
];
