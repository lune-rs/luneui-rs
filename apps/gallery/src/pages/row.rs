//! The `row` page: setting rows with a toggle at the right, item rows on the 40 / 1fr / 88 / 176
//! grid, the four states, and the hover and off fades caught mid-way.

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::row::{ItemRow, SettingRow};
use lune_ui::widgets::tone::Tone;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "row",
    title: "Row",
    kind: PageKind::Ui(UiPage {
        state: setting,
        scenes: SCENES,
        bench: &[],
    }),
};

/// The rows' left edge and width, and the first row's top, in window coordinates (the card's
/// content area under a heading).
const X: f32 = 34.0;
const WIDTH: f32 = 560.0;
const TOP: f32 = 82.0;
const GAP: f32 = 2.0;
/// The toggle's width and its slot.
const CONTROL: f32 = 40.0;

#[derive(Clone, Copy)]
enum Mode {
    /// Two setting rows with descriptions.
    Setting,
    /// Four item rows.
    Items,
    /// Rest, hover (the scene's pointer), off and disabled.
    States,
}

struct Rows {
    mode: Mode,
    /// The first setting row's switch; the row is off while it is.
    on: bool,
    second: bool,
    checked: [bool; 4],
    /// The setting rows' knobs: the first one's title, whether both show a description and how
    /// wide the control's slot is.
    title: String,
    describe: bool,
    control_w: f32,
}

fn state(mode: Mode) -> Box<dyn PageState> {
    Box::new(Rows {
        mode,
        on: true,
        second: false,
        checked: [true, false, false, true],
        title: "Launch at login".to_owned(),
        describe: true,
        control_w: CONTROL,
    })
}

fn setting() -> Box<dyn PageState> {
    state(Mode::Setting)
}

fn items() -> Box<dyn PageState> {
    state(Mode::Items)
}

fn states() -> Box<dyn PageState> {
    state(Mode::States)
}

impl PageState for Rows {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Row");
        let (x, top) = (card.x + 24.0, card.y + 48.0);
        match self.mode {
            Mode::Setting => {
                heading(ui, "SETTING ROW", card.x + 24.0, card.y + 24.0);
                let mut y = top;
                let first = SettingRow {
                    title: &self.title,
                    description: self
                        .describe
                        .then_some("Open the app when you sign in to this computer."),
                    off: !self.on,
                    control_w: self.control_w,
                };
                let (on, title) = (&mut self.on, &self.title);
                y += GAP
                    + ui.setting_row("first", Rect::new(x, y, WIDTH, 0.0), &first, |ui, slot| {
                        ui.toggle("switch", slot, on, title);
                    });
                let second = SettingRow {
                    title: "Compact lists",
                    description: self.describe.then_some("Show more rows in every list."),
                    off: !self.second,
                    control_w: self.control_w,
                };
                let on = &mut self.second;
                ui.setting_row(
                    "second",
                    Rect::new(x, y, WIDTH, 0.0),
                    &second,
                    |ui, slot| {
                        ui.toggle("switch", slot, on, "Compact lists");
                    },
                );
            }
            Mode::Items => {
                heading(ui, "ITEM ROWS", card.x + 24.0, card.y + 24.0);
                let rows = [
                    ("Index the workspace", "done", Tone::Ok, "1.2 s"),
                    ("Compress the history", "running", Tone::Data, "42 s"),
                    ("Upload the report", "waiting", Tone::Neutral, "-"),
                    ("Verify the signature", "failed", Tone::Err, "0.4 s"),
                ];
                let mut y = top;
                for (i, (title, badge, tone, value)) in rows.into_iter().enumerate() {
                    let checked = &mut self.checked[i];
                    y += GAP
                        + ui.item_row(
                            ("item", i),
                            Rect::new(x, y, WIDTH, 0.0),
                            &ItemRow { name: title },
                            |ui, c| {
                                ui.checkbox(
                                    ("check", i),
                                    Rect::new(c[0].x, c[0].y + 8.0, 16.0, 16.0),
                                    checked,
                                    "",
                                );
                                let style = Style::ui(THEME.font_base, THEME.fg_primary);
                                ui.painter()
                                    .text(&Spec::plain(title, style), c[1].x, c[1].y + 6.0);
                                ui.badge(("badge", i), (c[2].x, c[2].y + 7.0), badge, tone, false);
                                // Right-aligned to the column's end, primary.
                                let mono = Style::mono(THEME.font_base, 20.0, THEME.fg_primary);
                                let value = Spec::plain(value, mono);
                                let w = ui.painter().measure(&value).0;
                                ui.painter().text(&value, c[3].right() - w, c[3].y + 6.0);
                            },
                        );
                }
            }
            Mode::States => {
                heading(
                    ui,
                    "REST, HOVER, OFF, DISABLED",
                    card.x + 24.0,
                    card.y + 24.0,
                );
                let mut y = top;
                for (i, (title, off, disabled)) in [
                    ("Rest", false, false),
                    ("Hover", false, false),
                    ("Off", true, false),
                    ("Disabled", false, true),
                ]
                .into_iter()
                .enumerate()
                {
                    let row = SettingRow {
                        title,
                        description: None,
                        off,
                        control_w: CONTROL,
                    };
                    let mut on = !off;
                    y += GAP
                        + ui.with_enabled(!disabled, |ui| {
                            ui.setting_row(
                                ("row", i),
                                Rect::new(x, y, WIDTH, 0.0),
                                &row,
                                |ui, slot| {
                                    ui.toggle("switch", slot, &mut on, title);
                                },
                            )
                        });
                }
            }
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "First on", &mut self.on);
        panel.toggle(ui, "Second on", &mut self.second);
        panel.text(ui, "First title", &mut self.title);
        panel.toggle(ui, "Descriptions", &mut self.describe);
        panel.number(ui, "Control width", &mut self.control_w, (24.0, 120.0, 1.0));
    }
}

/// The middle of the first setting row's text, clear of its switch.
const FIRST_ROW: (f32, f32) = (X + 200.0, TOP + 31.0);
/// The first setting row's switch (62 high: the track is centred).
const FIRST_SWITCH: (f32, f32) = (X + WIDTH - 12.0 - CONTROL / 2.0, TOP + 31.0);
/// The second state row (56 high, 2 apart).
const HOVER_ROW: (f32, f32) = (X + 200.0, TOP + 58.0 + 28.0);

const SCENES: &[UiScene] = &[
    UiScene {
        id: "row-setting",
        scale: 2.0,
        tier: MotionTier::Full,
        state: setting,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "row-item-cols",
        scale: 2.0,
        tier: MotionTier::Full,
        state: items,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "row-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: states,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(HOVER_ROW.0, HOVER_ROW.1)],
        }],
        capture_ms: 600,
        expect: Expect::Settled,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "row-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: setting,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(FIRST_ROW.0, FIRST_ROW.1)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // The click at 0 flips the switch; the row reads it at 10 ms and fades the title for `fast`
    // (180 ms): half way at 100 ms.
    UiScene {
        id: "row-off-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: setting,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(FIRST_SWITCH.0, FIRST_SWITCH.1), Ev::Click],
            },
            Step {
                at_ms: 10,
                events: &[],
            },
        ],
        capture_ms: 100,
        expect: Expect::Moving,
    },
];
