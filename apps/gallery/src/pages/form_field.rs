//! The `form-field` page: a stacked field, a stacked field in error and an inline one, each around
//! a real text field.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::field::FieldState;
use lune_ui::widgets::form_field::FormField;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "form-field",
    title: "Form field",
    kind: PageKind::Ui(UiPage {
        state: states,
        scenes: SCENES,
        bench: &[],
    }),
};

const WIDTH: f32 = 360.0;
/// The controls' height.
const CONTROL: f32 = 32.0;

/// Three fields; the knobs set the first one's label, hint, error and layout.
struct States {
    name: String,
    email: String,
    port: String,
    label: String,
    hint: bool,
    error: String,
    inline: bool,
}

fn states() -> Box<dyn PageState> {
    Box::new(States {
        name: "Lune".to_owned(),
        email: String::new(),
        port: "8080".to_owned(),
        label: "Display name".to_owned(),
        hint: true,
        error: String::new(),
        inline: false,
    })
}

impl PageState for States {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Form field");
        heading(ui, "NORMAL, ERROR, INLINE", card.x + 24.0, card.y + 24.0);
        let (x, mut y) = (card.x + 24.0, card.y + 48.0);

        let normal = FormField {
            label: &self.label,
            hint: self.hint.then_some("Shown to other members."),
            error: (!self.error.is_empty()).then_some(self.error.as_str()),
            inline: self.inline,
            control_size: (if self.inline { 120.0 } else { 0.0 }, CONTROL),
        };
        let name = &mut self.name;
        y += 24.0
            + ui.form_field("name", Rect::new(x, y, WIDTH, 0.0), &normal, |ui, r| {
                ui.text_field("input", r, name, "Name", FieldState::default())
                    .id
            });

        let error = FormField {
            label: "Email",
            hint: Some("We only write about your account."),
            error: Some("Enter an address like name@example.com."),
            inline: false,
            control_size: (0.0, CONTROL),
        };
        let email = &mut self.email;
        y += 24.0
            + ui.form_field("email", Rect::new(x, y, WIDTH, 0.0), &error, |ui, r| {
                ui.text_field("input", r, email, "Email", FieldState::default())
                    .id
            });

        let inline = FormField {
            label: "Port",
            hint: Some("The proxy listens here."),
            error: None,
            inline: true,
            control_size: (120.0, CONTROL),
        };
        let port = &mut self.port;
        ui.form_field("port", Rect::new(x, y, WIDTH, 0.0), &inline, |ui, r| {
            ui.text_field("input", r, port, "Port", FieldState::default())
                .id
        });
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.text(ui, "Label", &mut self.label);
        panel.toggle(ui, "Hint", &mut self.hint);
        panel.text(ui, "Error", &mut self.error);
        panel.toggle(ui, "Inline", &mut self.inline);
    }
}

const SCENES: &[UiScene] = &[UiScene {
    id: "form-field-states",
    scale: 2.0,
    tier: MotionTier::Full,
    state: states,
    script: &[],
    capture_ms: 0,
    expect: Expect::Settled,
}];
