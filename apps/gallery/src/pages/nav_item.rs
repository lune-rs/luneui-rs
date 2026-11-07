//! The `nav-item` page: a live navigation list (window mode, the bench and the mid-transition
//! scenes) and a specimen sheet of every state painted through the public look (D16).

use std::time::Duration;

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::ui::Ui;
use lune_ui::widgets::nav_item::{NavEntry, NavLook, NavState, paint};

use super::{card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "nav-item",
    title: "Navigation item",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live::new(None))
}

/// The list with the current screen changing to the third at 200 ms.
fn select_later() -> Box<dyn PageState> {
    Box::new(Live::new(Some(Duration::from_millis(200))))
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
/// The live list's top left and width; rows are 32 high, 34 apart.
const LIST: Rect = Rect::new(X, Y + 24.0, 220.0, 0.0);

/// The centre of live row `i`.
const fn row(i: usize) -> (f32, f32) {
    (LIST.x + 110.0, LIST.y + 34.0 * i as f32 + 16.0)
}

/// Five screens; the fourth is disabled. The knobs set the current one, the first label, whether
/// the rows carry glyphs, whether the fourth is enabled and the list's width.
struct Live {
    selected: u64,
    select_at: Option<Duration>,
    first_label: String,
    icons: bool,
    fourth_enabled: bool,
    width: f32,
}

impl Live {
    fn new(select_at: Option<Duration>) -> Self {
        Self {
            selected: 1,
            select_at,
            first_label: "Overview".to_owned(),
            icons: true,
            fourth_enabled: false,
            width: LIST.w,
        }
    }
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Navigation item");
        let (ox, oy) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "NAVIGATION", ox, oy);
        if self.select_at.is_some_and(|t| ui.time() >= t) {
            self.selected = 3;
        }
        let icons = self.icons;
        let entry = |key, label, glyph, enabled| NavEntry {
            key,
            label,
            icon: icons.then_some(glyph),
            enabled,
        };
        let entries = [
            entry(1, self.first_label.as_str(), icon::INFO, true),
            entry(2, "Sessions", icon::SYNC, true),
            entry(3, "Requests", icon::SEARCH, true),
            entry(4, "Local models", icon::COPY, self.fourth_enabled),
            entry(5, "Settings", icon::GEAR, true),
        ];
        let list = Rect {
            w: self.width,
            ..on_card(card, LIST)
        };
        ui.nav_list("nav", list, &entries, &mut self.selected, "Screens");
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let mut selected = self.selected as f32;
        if panel.number(ui, "Current", &mut selected, (1.0, 5.0, 1.0)) {
            self.selected = selected.round() as u64;
        }
        panel.text(ui, "First label", &mut self.first_label);
        panel.toggle(ui, "Glyphs", &mut self.icons);
        panel.toggle(ui, "Fourth enabled", &mut self.fourth_enabled);
        panel.number(ui, "Width", &mut self.width, (120.0, 400.0, 1.0));
    }
}

/// Every state, each drawn from a [`NavState`] through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Navigation item states");
        let (ox, oy) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "NAV ITEM STATES", ox, oy);
        let theme = *ui.theme();
        let rest = NavState::default();
        let active = NavState {
            active: true,
            ..rest
        };
        let cases = [
            ("Rest", rest, false),
            (
                "Hover",
                NavState {
                    hovered: true,
                    ..rest
                },
                false,
            ),
            ("Active", active, false),
            (
                "Active, hover",
                NavState {
                    hovered: true,
                    ..active
                },
                false,
            ),
            (
                "Focus-visible",
                NavState {
                    focus_visible: true,
                    ..rest
                },
                false,
            ),
            (
                "Pressed",
                NavState {
                    pressed: true,
                    ..rest
                },
                false,
            ),
            ("Disabled", rest, true),
        ];
        for (i, (name, state, disabled)) in cases.into_iter().enumerate() {
            let r = Rect::new(ox, oy + 24.0 + i as f32 * 34.0, 220.0, 32.0);
            let look = NavLook::of(&theme, state);
            let p = ui.painter();
            if disabled {
                p.push_xform(Xform {
                    origin: (0.0, 0.0),
                    scale: 1.0,
                    dx: 0.0,
                    dy: 0.0,
                    alpha: 0.4,
                });
            }
            paint(p, r, name, Some(icon::INFO), &look);
            if disabled {
                p.pop_xform();
            }
        }
    }
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "nav-item-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "nav-item-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(row(2).0, row(2).1)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // The current screen changes at 200 ms: `fast` (180 ms), half way at 90 ms.
    UiScene {
        id: "nav-item-select-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: select_later,
        script: &[],
        capture_ms: 290,
        expect: Expect::Moving,
    },
    // Tab focuses the list's stop, the current row; the ring is `fast`: half way at 90 ms.
    UiScene {
        id: "nav-item-focus-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::Tab],
        }],
        capture_ms: 90,
        expect: Expect::Moving,
    },
];

/// What the bench replays: hover, click, Tab, the arrows and Enter.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(row(2).0, row(2).1)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::ArrowUp)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Key(NamedKey::Enter)],
    },
];
