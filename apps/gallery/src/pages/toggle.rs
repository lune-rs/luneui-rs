//! The `toggle` page: live switches (window mode, the bench and the mid-transition scenes) and a
//! specimen sheet of every state painted through the public look (D16).

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::toggle::{ToggleLook, ToggleState, paint};

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;
use lune_ui::input::NamedKey;

pub(crate) static PAGE: Page = Page {
    id: "toggle",
    title: "Toggle",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        off: false,
        on: true,
        enabled: false,
        third: false,
        label: "On".to_owned(),
    })
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

fn label(ui: &mut Ui<'_>, text: &str, x: f32, y: f32) {
    let spec = Spec::plain(text, Style::ui(THEME.font_base, THEME.fg_secondary));
    ui.painter().text(&spec, x, y + 3.0);
}

/// Two real switches through the `Ui`: the first starts off, the second on; the third is
/// disabled. The knobs set the values, the third's enabled flag and the second's label.
struct Live {
    off: bool,
    on: bool,
    /// The third switch takes input, and its value.
    enabled: bool,
    third: bool,
    /// The second switch's label and accessible name.
    label: String,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Toggle");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "TOGGLES", x, y);
        let ty = y + 20.0;
        ui.toggle("switch", Rect::new(x, ty, 40.0, 24.0), &mut self.off, "Off");
        label(ui, "Off", x + 52.0, ty);
        ui.toggle(
            "switch-on",
            Rect::new(x + 160.0, ty, 40.0, 24.0),
            &mut self.on,
            &self.label,
        );
        label(ui, &self.label, x + 212.0, ty);
        let third = &mut self.third;
        ui.with_enabled(self.enabled, |ui| {
            ui.toggle(
                "disabled",
                Rect::new(x + 320.0, ty, 40.0, 24.0),
                third,
                "Disabled",
            );
        });
        label(ui, "Disabled", x + 372.0, ty);
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "First on", &mut self.off);
        panel.toggle(ui, "Second on", &mut self.on);
        panel.toggle(ui, "Third enabled", &mut self.enabled);
        panel.text(ui, "Second label", &mut self.label);
    }
}

/// Every state, each drawn from a [`ToggleState`] through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Toggle states");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "TOGGLE STATES", x, y);
        let theme = *ui.theme();
        let off = ToggleState::default();
        let cases = [
            ("Off", off, false),
            ("On", ToggleState { on: true, ..off }, false),
            (
                "Hover, off",
                ToggleState {
                    hovered: true,
                    ..off
                },
                false,
            ),
            (
                "Hover, on",
                ToggleState {
                    on: true,
                    hovered: true,
                    ..off
                },
                false,
            ),
            (
                "Focus-visible",
                ToggleState {
                    focus_visible: true,
                    ..off
                },
                false,
            ),
            ("Disabled", off, true),
        ];
        for (i, (name, state, disabled)) in cases.into_iter().enumerate() {
            let r = Rect::new(x + i as f32 * 130.0, y + 24.0, 40.0, 24.0);
            let look = ToggleLook::of(&theme, state);
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
            paint(p, r, &look);
            if disabled {
                p.pop_xform();
            }
            caption(ui, name, r.x, r.y + 36.0);
        }
        // With a label.
        let r = Rect::new(x + 6.0 * 130.0, y + 24.0, 40.0, 24.0);
        let on = ToggleState { on: true, ..off };
        paint(ui.painter(), r, &ToggleLook::of(&theme, on));
        label(ui, "Label", r.x + 52.0, r.y);
        caption(ui, "With label", r.x, r.y + 36.0);
    }
}

const SWITCH: &[&str] = &["switch"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "toggle-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Tab focuses the first switch and its ring settles; Space at 400 ms flips it; the knob
    // slide is `fast` (180 ms): half way at 90 ms.
    UiScene {
        id: "toggle-switching",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::Tab],
            },
            Step {
                at_ms: 400,
                events: &[Ev::Key(NamedKey::Space)],
            },
        ],
        capture_ms: 490,
        expect: Expect::Moving,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "toggle-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(SWITCH)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    UiScene {
        id: "toggle-focus-mid",
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

/// What the bench replays: hover, click, Tab, Space.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(SWITCH)],
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
        events: &[Ev::Key(NamedKey::Space)],
    },
];
