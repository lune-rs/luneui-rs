//! The `combo` page: a live combo (window mode, the bench and the mid-transition scenes) and a
//! specimen sheet of every state painted through the public look (D16).

use lune_ui::access::Action;
use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::ui::Ui;
use lune_ui::widgets::combo::{ComboLook, ComboState, paint};

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "combo",
    title: "Combo",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live { open: false })
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

/// Real combos through the `Ui`. The page plays the app: it keeps `open`, and a second click on
/// an open combo closes it (the combo never closes itself).
struct Live {
    open: bool,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Combo");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "COMBO", x, y);
        let r = ui.combo(
            "combo",
            Rect::new(x, y + 20.0, 210.0, 32.0),
            "RznSans",
            &mut self.open,
            "Font",
        );
        if r.clicked && !r.changed {
            self.open = false;
        }
        let mut closed = false;
        ui.with_enabled(false, |ui| {
            ui.combo(
                "disabled",
                Rect::new(x, y + 68.0, 210.0, 32.0),
                "RznSans",
                &mut closed,
                "Disabled font",
            );
        });
    }
}

/// Every state, each drawn from a [`ComboState`] through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Combo states");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "COMBO STATES", x, y);
        let theme = *ui.theme();
        let rest = ComboState::default();
        let cases = [
            ("Rest", rest, false),
            (
                "Hovered",
                ComboState {
                    hovered: true,
                    ..rest
                },
                false,
            ),
            ("Open", ComboState { open: true, ..rest }, false),
            (
                "Focus-visible",
                ComboState {
                    focus_visible: true,
                    ..rest
                },
                false,
            ),
            ("Disabled", rest, true),
        ];
        for (i, (name, state, disabled)) in cases.into_iter().enumerate() {
            let r = Rect::new(x, y + 20.0 + i as f32 * 48.0, 210.0, 32.0);
            let look = ComboLook::of(&theme, state);
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
            paint(p, r, "RznSans", &look);
            if disabled {
                p.pop_xform();
            }
            caption(ui, name, r.right() + 16.0, r.y + 8.0);
        }
    }
}

const COMBO: &[&str] = &["combo"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "combo-states",
        scale: 2,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "combo-hover-mid",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(COMBO)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // The expand action opens the combo; the border is `fast` (180 ms): half way at 90 ms.
    UiScene {
        id: "combo-open-mid",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::Access(COMBO, Action::Expand)],
        }],
        capture_ms: 90,
        expect: Expect::Moving,
    },
];

/// What the bench replays: hover, click (opens), Escape, Tab, Down, Enter.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(COMBO)],
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
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
];
