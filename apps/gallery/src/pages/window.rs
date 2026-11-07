//! The `window` page (`platform.window`, M11): the caption buttons through the `Ui` (rest,
//! maximized, hovered, mid-fade), the six-pixel resize border with its eight regions as `edge_at`
//! answers them, and the caption of a window that is not the active one. The window mode runs the live
//! buttons: Maximize flips the restore glyph; Minimize and Close are the app's, and here do nothing.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::caption::{CaptionButton, CaptionOptions};
use lune_ui::window::{EDGE_BORDER, ResizeEdge, edge_at};

use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

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

/// The caption buttons' height in the title band (Wane's `--titlebar-h`).
const BUTTON_HEIGHT: f32 = 32.0;

fn live() -> Box<dyn PageState> {
    Box::new(Caption::default())
}

fn maximized() -> Box<dyn PageState> {
    Box::new(Caption {
        maximized: true,
        ..Caption::default()
    })
}

fn inactive() -> Box<dyn PageState> {
    Box::new(Caption {
        active: false,
        ..Caption::default()
    })
}

fn edges() -> Box<dyn PageState> {
    Box::new(Edges)
}

/// The caption buttons along the top right of `area`. The widget places them from the origin of
/// the window it is given, so an area that is not at the window's origin moves them there with a
/// translation (which carries their hit regions and nodes with it, and leaves the host's clip
/// where it is on screen).
fn caption_in(ui: &mut Ui<'_>, area: Rect, opts: CaptionOptions) -> Option<CaptionButton> {
    if area.x == 0.0 && area.y == 0.0 {
        return ui.caption_buttons("caption", area, opts);
    }
    ui.painter().push_translate(area.x, area.y);
    let clicked = ui.caption_buttons("caption", Rect::new(0.0, 0.0, area.w, area.h), opts);
    ui.painter().pop_translate();
    clicked
}

/// The window with live caption buttons.
struct Caption {
    maximized: bool,
    /// The window is the active one: the caption's own state, glyphs dim when it is not.
    active: bool,
}

impl Default for Caption {
    fn default() -> Self {
        Self {
            maximized: false,
            active: true,
        }
    }
}

impl PageState for Caption {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "luneui gallery");
        caption(ui, "Caption buttons", card.x + 24.0, card.y + 24.0);
        let win = stage.area(ui);
        let opts = CaptionOptions {
            height: BUTTON_HEIGHT,
            maximized: self.maximized,
            active: self.active,
        };
        if caption_in(ui, win, opts) == Some(CaptionButton::Maximize) {
            self.maximized = !self.maximized;
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "Maximized", &mut self.maximized);
        panel.toggle(ui, "Window active", &mut self.active);
    }
}

/// The six pixel border, each of its eight regions in its own tone, found by asking `edge_at` at
/// the middle of the region and named from the answer.
struct Edges;

impl PageState for Edges {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "luneui gallery");
        let t = *ui.theme();
        let win = stage.area(ui);
        let (w, h, b) = (win.w, win.h, EDGE_BORDER);
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
            ui.painter()
                .fill(Rect::new(r.x + win.x, r.y + win.y, r.w, r.h), 0.0, tone);
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
    // The caption of a window that is not the active one: glyphs in `fg_quaternary`. The caption
    // owns that state (D10); no widget's surface depends on it.
    UiScene {
        id: "window-caption-inactive",
        scale: 2.0,
        tier: MotionTier::Full,
        state: inactive,
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
