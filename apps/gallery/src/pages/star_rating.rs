//! The `star-rating` page: a live row (window mode, the bench and the hover scene) and a specimen
//! sheet of the values painted through the public look (D16).

use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::ui::Ui;
use lune_ui::widgets::star_rating::{MAX, StarLook, StarState, button_rect, paint};

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "star-rating",
    title: "Star rating",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live { value: 3 })
}

fn unrated() -> Box<dyn PageState> {
    Box::new(Live { value: 0 })
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

/// The card's content corner (the card sits at (10, 34), the content 24 px inside it).
const X: f32 = 34.0;
const Y: f32 = 58.0;
const AT: (f32, f32) = (X, Y + 24.0);

/// The centre of live star `n` (1-based).
const fn star(n: u32) -> (f32, f32) {
    (AT.0 + 26.0 * (n - 1) as f32 + 12.0, AT.1 + 12.0)
}

/// A rating of five stars.
struct Live {
    value: u32,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        card(ui, stage, "Star rating");
        heading(ui, "STAR RATING", X, Y);
        ui.star_rating("stars", AT, &mut self.value, MAX, "Rating");
        let text = if self.value == 0 {
            "Unrated".to_owned()
        } else {
            format!("{} of {MAX}", self.value)
        };
        caption(ui, &text, X, AT.1 + 36.0);
    }
}

/// Values 0 and 3 of 5, and 3 with star 4 hovered, each drawn from [`StarState`]s through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        card(ui, stage, "Star rating states");
        heading(ui, "STAR RATING STATES", X, Y);
        let theme = *ui.theme();
        let rows = [
            ("Value 0", 0, None),
            ("Value 3 of 5", 3, None),
            ("Hover on star 4", 3, Some(4)),
        ];
        for (i, (name, value, hover)) in rows.into_iter().enumerate() {
            let y = Y + 24.0 + i as f32 * 40.0;
            for n in 1..=MAX {
                let state = StarState {
                    filled: n <= value,
                    hovered: hover == Some(n),
                    ..StarState::default()
                };
                paint(
                    ui.painter(),
                    button_rect((X, y), n - 1),
                    &StarLook::of(&theme, state),
                );
            }
            caption(ui, name, X + 150.0, y + 5.0);
        }
    }
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "star-rating-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Star 4 rest to hover: `instant` (120 ms), half way at 60 ms.
    UiScene {
        id: "star-rating-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: unrated,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(star(4).0, star(4).1)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
];

/// What the bench replays: hover, click, Tab, the arrows and Enter.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(star(4).0, star(4).1)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::ArrowRight)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Key(NamedKey::Enter)],
    },
];
