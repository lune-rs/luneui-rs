//! The `checkbox` page: live checkboxes (window mode, the bench and the mid-transition scenes) and a
//! specimen sheet of every state painted through the public look (D16).

use lune_ui::input::NamedKey;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::ui::Ui;
use lune_ui::widgets::checkbox::{CheckboxLook, CheckboxState, paint};

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "checkbox",
    title: "Checkbox",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Live {
        first: false,
        second: true,
        label: "Unchecked".to_owned(),
        enabled: true,
    })
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

/// Two real checkboxes through the `Ui`: the first starts unchecked, the second checked. The
/// knobs set both values, the first one's label and whether it takes input.
struct Live {
    first: bool,
    second: bool,
    label: String,
    enabled: bool,
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Checkbox");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "CHECKBOXES", x, y);
        let ty = y + 20.0;
        let (first, label) = (&mut self.first, &self.label);
        ui.with_enabled(self.enabled, |ui| {
            ui.checkbox("box", Rect::new(x, ty, 140.0, 24.0), first, label);
        });
        ui.checkbox(
            "box-on",
            Rect::new(x, ty + 32.0, 140.0, 24.0),
            &mut self.second,
            "Checked",
        );
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "First checked", &mut self.first);
        panel.toggle(ui, "Second checked", &mut self.second);
        panel.text(ui, "First label", &mut self.label);
        panel.toggle(ui, "First enabled", &mut self.enabled);
    }
}

/// Every state, each drawn from a [`CheckboxState`] through `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Checkbox states");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "CHECKBOX STATES", x, y);
        let theme = *ui.theme();
        let off = CheckboxState::default();
        let cases = [
            ("Unchecked", off, false),
            (
                "Checked",
                CheckboxState {
                    checked: true,
                    ..off
                },
                false,
            ),
            (
                "Hover, unchecked",
                CheckboxState {
                    hovered: true,
                    ..off
                },
                false,
            ),
            (
                "Hover, checked",
                CheckboxState {
                    checked: true,
                    hovered: true,
                    ..off
                },
                false,
            ),
            (
                "Focus-visible",
                CheckboxState {
                    focus_visible: true,
                    ..off
                },
                false,
            ),
            ("Disabled", off, true),
        ];
        for (i, (name, state, disabled)) in cases.into_iter().enumerate() {
            let r = Rect::new(x, y + 24.0 + i as f32 * 32.0, 160.0, 24.0);
            let look = CheckboxLook::of(&theme, state);
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
            paint(p, r, name, &look);
            if disabled {
                p.pop_xform();
            }
        }
    }
}

/// The live box, by its path.
const BOX: &[&str] = &["box"];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "checkbox-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The pointer enters, then clicks at 200 ms: the fill is `fast` (180 ms), half way at 90 ms.
    UiScene {
        id: "checkbox-fill-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(BOX)],
            },
            Step {
                at_ms: 200,
                events: &[Ev::Click],
            },
        ],
        capture_ms: 290,
        expect: Expect::Moving,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "checkbox-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(BOX)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // Tab focuses the box; the ring is `fast`: half way at 90 ms.
    UiScene {
        id: "checkbox-focus-mid",
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
        events: &[Ev::PointerOver(BOX)],
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
