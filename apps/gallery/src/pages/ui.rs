//! The `ui` page: the controls composition under the overlay model. Scenes freeze a modal dialog
//! over the page (nothing under it reacts), a dialog the app stopped calling (still drawn, half
//! faded), and a popup over a page button (the popup takes the hover).

use std::time::Duration;

use lune_ui::icon;
use lune_ui::motion::MotionTier;
use lune_ui::ui::Ui;
use lune_ui::widgets::dialog::{Cancel, Dialog};
use lune_ui::widgets::menu::MenuRow;

use super::controls_ui::{Controls, glass};
use super::{card, card_shift};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "ui",
    title: "Ui",
    kind: PageKind::Ui(UiPage {
        state: plain,
        scenes: SCENES,
        bench: &[],
    }),
};

/// The app stops calling the dialog from here on (after its 250 ms open).
const STOP_AT: Duration = Duration::from_millis(300);

/// What sits over the controls.
#[derive(Debug, Clone, Copy)]
enum Over {
    Nothing,
    /// A confirm dialog, optionally not called from `stop_at` on.
    Dialog {
        stop_at: Option<Duration>,
    },
    /// A menu overlapping the primary button.
    Menu,
}

/// The controls with something over them.
struct Demo {
    controls: Controls,
    over: Over,
}

fn demo(over: Over) -> Box<dyn PageState> {
    Box::new(Demo {
        controls: Controls::new(),
        over,
    })
}

fn plain() -> Box<dyn PageState> {
    demo(Over::Nothing)
}

fn modal() -> Box<dyn PageState> {
    demo(Over::Dialog { stop_at: None })
}

fn stopped() -> Box<dyn PageState> {
    demo(Over::Dialog {
        stop_at: Some(STOP_AT),
    })
}

fn popup() -> Box<dyn PageState> {
    demo(Over::Menu)
}

/// Where the menu opens: over the primary button (34, 78, 90 x 32).
const MENU_AT: (f32, f32) = (60.0, 90.0);

impl PageState for Demo {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "luneui gallery");
        self.controls.draw(ui, area);
        match self.over {
            Over::Nothing => {}
            Over::Dialog { stop_at } => {
                if stop_at.is_none_or(|t| ui.time() < t) {
                    let d = Dialog {
                        title: "Discard changes",
                        message: "The document has unsaved changes. Closing it will discard them.",
                        extra_h: 0.0,
                        confirm: "Discard",
                        cancel: Cancel::Default,
                    };
                    ui.dialog("dialog", &d, &glass(), |_, _, _, _| {});
                }
            }
            Over::Menu => {
                let rows = [
                    MenuRow::Item {
                        glyph: Some(icon::ADD),
                        text: "New Window",
                        shortcut: "Cmd+N",
                    },
                    MenuRow::Item {
                        glyph: Some(icon::COPY),
                        text: "Copy",
                        shortcut: "Cmd+C",
                    },
                ];
                let (dx, dy) = card_shift(area);
                ui.menu("menu", &rows, (MENU_AT.0 + dx, MENU_AT.1 + dy));
            }
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let mut at = match self.over {
            Over::Nothing => 0,
            Over::Menu => 1,
            Over::Dialog { .. } => 2,
        };
        // A dialog is modal: it is closed in the dialog, not here.
        if panel.choice(
            ui,
            "Over the controls",
            &["Nothing", "Menu", "Dialog"],
            &mut at,
        ) {
            self.over = match at {
                1 => Over::Menu,
                2 => Over::Dialog { stop_at: None },
                _ => Over::Nothing,
            };
        }
    }
}

const PRIMARY: &[&str] = &["primary"];

const SCENES: &[UiScene] = &[
    // The controls at a scale that is not a whole number of device pixels per logical one (R-nfr-6):
    // 1560 by 990 physical px.
    UiScene {
        id: "ui-controls",
        scale: 1.5,
        tier: MotionTier::Full,
        state: plain,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The pointer is over a page button under the open dialog: the button stays at rest.
    UiScene {
        id: "ui-modal-capture",
        scale: 2.0,
        tier: MotionTier::Full,
        state: modal,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(PRIMARY)],
        }],
        capture_ms: 600,
        expect: Expect::Settled,
    },
    // Open by 260 ms; the app stops calling at 300 ms (t0); the close is `dismiss`, fast
    // (180 ms): half way at t0 + 90 ms.
    UiScene {
        id: "ui-exit-retention",
        scale: 2.0,
        tier: MotionTier::Full,
        state: stopped,
        script: &[
            Step {
                at_ms: 260,
                events: &[],
            },
            Step {
                at_ms: 300,
                events: &[],
            },
        ],
        capture_ms: 390,
        expect: Expect::Moving,
    },
    // The pointer is over the overlap of the primary button and the menu's first item.
    UiScene {
        id: "ui-popup-over-page",
        scale: 2.0,
        tier: MotionTier::Full,
        state: popup,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(80.0, 100.0)],
            },
            Step {
                at_ms: 300,
                events: &[],
            },
        ],
        capture_ms: 600,
        expect: Expect::Settled,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::testing::FakeText;

    /// R-nfr-6: the fractional scene exists, is named by its scale, is whole pixels (1560 by 990)
    /// and draws the controls.
    #[test]
    fn the_controls_scene_at_1_5_is_named_sized_and_runs() {
        let scene = SCENES
            .iter()
            .find(|s| s.scale == 1.5)
            .expect("a 1.5x scene");
        assert_eq!(scene.name(), "ui-controls@1.5x");
        assert_eq!(scene.size(), (1560, 990));
        // Every other name keeps its integer form.
        assert_eq!(SCENES[1].name(), "ui-modal-capture@2x");
        let (display, _) = run_scene("ui", scene, &mut FakeText).expect("runs");
        assert!(display.layers.iter().any(|l| !l.texts.is_empty()));
    }

    /// Under a modal dialog the pointer over a page button changes nothing: the scene equals the
    /// same scene without the pointer move.
    #[test]
    fn the_button_under_the_modal_shows_its_rest_look() {
        let with_pointer = SCENES[0];
        let mut without = with_pointer;
        without.script = &[];
        let a = run_scene("ui", &with_pointer, &mut FakeText)
            .expect("runs")
            .0;
        let b = run_scene("ui", &without, &mut FakeText).expect("runs").0;
        assert!(a == b, "the page button reacted under the modal");
    }
}
