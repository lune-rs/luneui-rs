//! The `spinner` page: the sync glyph turning once per 1200 ms while busy (D117), a static glyph
//! when stopped and under Reduced, and a switch that starts and stops it.

use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::spinner::SpinnerOptions;

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "spinner",
    title: "Spinner",
    kind: PageKind::Ui(UiPage {
        state: busy,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn busy() -> Box<dyn PageState> {
    Box::new(Live { busy: true })
}

fn idle() -> Box<dyn PageState> {
    Box::new(Live { busy: false })
}

const SWITCH: &[&str] = &["busy"];

/// A switch and three spinners (14, 20 and 28 px, the last in a caller colour).
struct Live {
    busy: bool,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Spinner");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "SPINNERS", x, y);
        let ty = y + 24.0;
        ui.toggle("busy", Rect::new(x, ty, 40.0, 24.0), &mut self.busy, "Busy");
        let label = Spec::plain("Busy", Style::ui(THEME.font_base, THEME.fg_secondary));
        ui.painter().text(&label, x + 52.0, ty + 3.0);

        let sy = ty + 64.0;
        let accent = THEME.accent;
        for (i, (size, color)) in [(14.0, None), (20.0, None), (28.0, Some(accent))]
            .into_iter()
            .enumerate()
        {
            let cx = x + 14.0 + i as f32 * 64.0;
            ui.spinner(
                ("spinner", i),
                (cx, sy),
                &SpinnerOptions {
                    busy: self.busy,
                    size,
                    color,
                    name: "Indexing",
                },
            );
        }
    }
}

const SCENES: &[UiScene] = &[
    // The spin starts on the first frame: a quarter turn at 300 ms.
    UiScene {
        id: "spinner-spinning",
        scale: 2.0,
        tier: MotionTier::Full,
        state: busy,
        script: &[],
        capture_ms: 300,
        expect: Expect::Moving,
    },
    UiScene {
        id: "spinner-spinning",
        scale: 1.0,
        tier: MotionTier::Full,
        state: busy,
        script: &[],
        capture_ms: 300,
        expect: Expect::Moving,
    },
    UiScene {
        id: "spinner-stopped",
        scale: 2.0,
        tier: MotionTier::Full,
        state: idle,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "spinner-reduced",
        scale: 2.0,
        tier: MotionTier::Reduced,
        state: busy,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The switch is clicked at 0 ms; the capture is 250 ms into the first run.
    UiScene {
        id: "spinner-start-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: idle,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(SWITCH), Ev::Click],
        }],
        capture_ms: 250,
        expect: Expect::Moving,
    },
];

/// What the bench replays: stop, start and stop again with the switch (Space while focused).
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Key(NamedKey::Space)],
    },
    Step {
        at_ms: 1200,
        events: &[Ev::Key(NamedKey::Space)],
    },
];
