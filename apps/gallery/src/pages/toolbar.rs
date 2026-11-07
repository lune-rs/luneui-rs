//! The `toolbar` page: the bar over the window glass with four actions and a labelled button, a
//! switch for the window's active state and one for an action's enabled flag. The glass layer is
//! the page's choice (the widget names no preset); inactive, the bar draws the fallback fill.

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::{CARD, THEME, WINDOW};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::toolbar::{ACTION_SIZE, GROUP_GAP, ToolbarOptions, label_width};

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "toolbar",
    title: "Toolbar",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live::of(true, true))
}

fn inactive() -> Box<dyn PageState> {
    Box::new(Live::of(false, true))
}

/// The push action disabled.
fn push_disabled() -> Box<dyn PageState> {
    Box::new(Live::of(true, false))
}

/// The actions: key, glyph, and the name that is also the tooltip.
const ACTIONS: [(&str, &str, &str); 3] = [
    ("fetch", icon::SYNC, "Fetch"),
    ("pull", icon::CHEVRON_DOWN, "Pull"),
    ("push", icon::ARROW_LEFT, "Push"),
];

const BRANCH: &str = "feature/glass-toolbar";

/// The toolbar and two switches for the window being active and the push action being enabled.
/// The knobs set the branch label and whether the other two actions are enabled.
struct Live {
    active: bool,
    push_enabled: bool,
    branch: String,
    fetch_enabled: bool,
    pull_enabled: bool,
}

impl Live {
    fn of(active: bool, push_enabled: bool) -> Self {
        Self {
            active,
            push_enabled,
            branch: BRANCH.to_owned(),
            fetch_enabled: true,
            pull_enabled: true,
        }
    }
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Toolbar");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "MAIN TOOLBAR", x, y);
        let bar = Rect::new(card.x, y + 24.0, card.w, 52.0);
        let opts = ToolbarOptions {
            active: self.active,
            glass: WINDOW,
            under: CARD.premultiplied(),
            name: "Main toolbar",
        };
        let (push_enabled, fetch_enabled, pull_enabled) =
            (self.push_enabled, self.fetch_enabled, self.pull_enabled);
        let branch = &self.branch;
        ui.toolbar("bar", bar, &opts, |ui, cols| {
            // The actions touch (spacing 0); the branch label sits one group gap after the last.
            let mut ax = cols.left.x;
            for (key, glyph, name) in ACTIONS {
                let at = Rect::new(ax, cols.left.y, ACTION_SIZE, ACTION_SIZE);
                let enabled = match key {
                    "push" => push_enabled,
                    "fetch" => fetch_enabled,
                    _ => pull_enabled,
                };
                ui.toolbar_action(key, at, glyph, name, enabled);
                ax += ACTION_SIZE;
            }
            let w = label_width(ui.painter(), icon::SEARCH, branch);
            let at = Rect::new(ax + GROUP_GAP, cols.left.y, w, ACTION_SIZE);
            ui.toolbar_label("branch", at, icon::SEARCH, branch, "Branch");
        });

        let ty = bar.y + bar.h + 32.0;
        let label = |ui: &mut Ui<'_>, text: &str, x: f32, y: f32| {
            let spec = Spec::plain(text, Style::ui(THEME.font_base, THEME.fg_secondary));
            ui.painter().text(&spec, x, y + 3.0);
        };
        ui.toggle(
            "active",
            Rect::new(x, ty, 40.0, 24.0),
            &mut self.active,
            "Window active",
        );
        label(ui, "Window active", x + 52.0, ty);
        ui.toggle(
            "push-enabled",
            Rect::new(x + 240.0, ty, 40.0, 24.0),
            &mut self.push_enabled,
            "Push enabled",
        );
        label(ui, "Push enabled", x + 292.0, ty);
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.text(ui, "Branch label", &mut self.branch);
        panel.toggle(ui, "Fetch enabled", &mut self.fetch_enabled);
        panel.toggle(ui, "Pull enabled", &mut self.pull_enabled);
    }
}

const FETCH: &[&str] = &["bar", "fetch"];
const PULL: &[&str] = &["bar", "pull"];
const PUSH: &[&str] = &["bar", "push"];

const SCENES: &[UiScene] = &[
    // The pointer rests on the second action: its hover fill has settled and its tooltip has
    // appeared (800 ms delay, a frame at 800 ms starts the fade) and settled.
    UiScene {
        id: "toolbar-actions",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(PULL)],
            },
            Step {
                at_ms: 800,
                events: &[],
            },
        ],
        capture_ms: 1200,
        expect: Expect::Settled,
    },
    // The window is inactive: the fallback fill replaces the glass.
    UiScene {
        id: "toolbar-inactive",
        scale: 2.0,
        tier: MotionTier::Full,
        state: inactive,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The push action is disabled (0.4 opacity, transparent) and the pointer rests on it: no hover
    // fill; its tooltip still shows (a disabled anchor keeps it, `overlay.tooltip`); settled.
    UiScene {
        id: "toolbar-disabled",
        scale: 2.0,
        tier: MotionTier::Full,
        state: push_disabled,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(PUSH)],
            },
            Step {
                at_ms: 800,
                events: &[],
            },
        ],
        capture_ms: 1200,
        expect: Expect::Settled,
    },
    // Press is `fast` (180 ms): half way at 90 ms.
    UiScene {
        id: "toolbar-action-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(FETCH), Ev::Press],
        }],
        capture_ms: 90,
        expect: Expect::Moving,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "toolbar-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(PULL)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
];

/// What the bench replays: hover an action, click it, Tab to the next and press Enter.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(FETCH)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::PointerOver(PULL)],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::Enter)],
    },
];
