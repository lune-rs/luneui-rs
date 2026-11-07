//! The Ui "controls" composition: every M4 widget laid out like the legacy `controls` scene
//! (`scenes.rs`, `controls_content`), drawn through the `Ui`. The core pages (ui, focus, a11y,
//! locale) share it, so what they show is the real widgets and not a stand-in.

use lune_ui::icon;
use lune_ui::lunegit_dark;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::dialog::DialogGlass;
use lune_ui::widgets::field::FieldState;
use lune_ui::widgets::label::LabelRole;
use lune_ui::widgets::stepper::StepRange;

use super::heading;

/// The dropdown's options: text and whether it can be picked.
pub(crate) const OPTIONS: [(&str, bool); 4] = [
    ("RznSans", true),
    ("Sans Serif", true),
    ("Monospace", true),
    ("Coming soon", false),
];

/// The dialog glass of the house look.
pub(crate) fn glass() -> DialogGlass {
    DialogGlass::of(&lunegit_dark::THEME)
}

const STEPS: StepRange = StepRange {
    min: 0.0,
    max: 99.0,
    step: 1.0,
};

/// The app-side state of the composition.
#[derive(Debug)]
pub(crate) struct Controls {
    toggle_on: bool,
    toggle_off: bool,
    check_on: bool,
    check_off: bool,
    value: u32,
    field_empty: String,
    field_focused: String,
    combo_open: bool,
    font: usize,
}

impl Controls {
    pub(crate) fn new() -> Self {
        Self {
            toggle_on: true,
            toggle_off: false,
            check_on: true,
            check_off: false,
            value: 12,
            field_empty: String::new(),
            field_focused: "lune".to_owned(),
            combo_open: false,
            font: 0,
        }
    }

    /// The composition inside `card`: buttons, toggles, checkboxes, a stepper, fields, a combo
    /// with its dropdown while open, and the label and icon column.
    pub(crate) fn draw(&mut self, ui: &mut Ui<'_>, card: Rect) {
        let x = card.x + 24.0;
        let y = card.y + 24.0;

        heading(ui, "BUTTONS", x, y);
        let by = y + 20.0;
        let pw = button_width(ui.painter(), "Primary");
        ui.button(
            "primary",
            Rect::new(x, by, pw, 32.0),
            "Primary",
            ButtonKind::Primary,
        );
        let sw = button_width(ui.painter(), "Secondary");
        ui.button(
            "secondary",
            Rect::new(x + pw + 8.0, by, sw, 32.0),
            "Secondary",
            ButtonKind::Secondary,
        );

        let y = y + 68.0;
        heading(ui, "TOGGLES AND CHECKBOXES", x, y);
        let ty = y + 20.0;
        ui.toggle(
            "toggle-on",
            Rect::new(x, ty, 40.0, 24.0),
            &mut self.toggle_on,
            "On",
        );
        ui.toggle(
            "toggle-off",
            Rect::new(x + 64.0, ty, 40.0, 24.0),
            &mut self.toggle_off,
            "Off",
        );
        ui.checkbox(
            "check-on",
            Rect::new(x + 150.0, ty, 140.0, 24.0),
            &mut self.check_on,
            "Checked",
        );
        ui.checkbox(
            "check-off",
            Rect::new(x + 300.0, ty, 140.0, 24.0),
            &mut self.check_off,
            "Unchecked",
        );

        let y = y + 60.0;
        heading(ui, "STEPPER", x, y);
        let s = ui.stepper(
            "stepper",
            Rect::new(x, y + 20.0, 140.0, 32.0),
            self.value,
            STEPS,
            "Amount",
        );
        if let Some(set) = s.set {
            self.value = set as u32;
        } else if let Some(step) = s.step {
            self.value = self.value.saturating_add_signed(step).min(99);
        }

        let y = y + 60.0;
        heading(ui, "FIELDS", x, y);
        ui.text_field(
            "field-empty",
            Rect::new(x, y + 20.0, 210.0, 32.0),
            &mut self.field_empty,
            "Search",
            FieldState::default(),
        );
        ui.text_field(
            "field-focused",
            Rect::new(x + 230.0, y + 20.0, 210.0, 32.0),
            &mut self.field_focused,
            "Name",
            FieldState::default(),
        );

        let y = y + 60.0;
        heading(ui, "COMBO", x, y);
        let combo_rect = Rect::new(x, y + 20.0, 210.0, 32.0);
        ui.combo(
            "combo",
            combo_rect,
            OPTIONS[self.font].0,
            &mut self.combo_open,
            "Font",
        );
        if self.combo_open {
            let anchor = ui.id("combo");
            let r = ui.dropdown("dropdown", combo_rect, anchor, &OPTIONS, self.font);
            if let Some(i) = r.picked {
                self.font = i;
            }
            if r.picked.is_some() || r.dismissed {
                self.combo_open = false;
            }
        }

        // Right column: labels and icons.
        let rx = x + 480.0;
        let ry = card.y + 24.0;
        heading(ui, "LABELS", rx, ry);
        let lines = [
            ("label-primary", "Primary text", LabelRole::Strong),
            ("label-secondary", "Secondary text", LabelRole::Base),
            ("label-tertiary", "Tertiary text", LabelRole::Small),
            ("label-title", "Title text", LabelRole::Section),
            ("label-caption", "Small caption", LabelRole::Small),
        ];
        for (i, (key, text, role)) in lines.into_iter().enumerate() {
            ui.label(key, (rx, ry + 20.0 + i as f32 * 22.0), text, role);
        }

        let iy = ry + 168.0;
        heading(ui, "ICONS", rx, iy);
        let glyphs = [
            icon::ADD,
            icon::GEAR,
            icon::ELLIPSIS,
            icon::CHROME_CLOSE,
            icon::EDIT,
            icon::COPY,
            icon::INFO,
            icon::CHEVRON_DOWN,
            icon::ARROW_LEFT,
            icon::CHECK,
        ];
        let fg = ui.theme().fg_primary;
        for (i, g) in glyphs.into_iter().enumerate() {
            let r = Rect::new(rx + i as f32 * 36.0, iy + 20.0, 28.0, 28.0);
            ui.painter().text_centered(&icon::glyph(g, 18.0, fg), r);
        }
    }
}
