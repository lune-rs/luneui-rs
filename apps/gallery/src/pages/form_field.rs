//! The `form-field` page: a stacked field, a stacked field in error and an inline one, each around
//! a real text field.

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::field::FieldState;
use lune_ui::widgets::form_field::FormField;

use super::{card, heading};
use crate::page::{Expect, Page, PageKind, PageState, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "form-field",
    title: "Form field",
    kind: PageKind::Ui(UiPage {
        state: states,
        scenes: SCENES,
        bench: &[],
    }),
};

const X: f32 = 34.0;
const WIDTH: f32 = 360.0;
const TOP: f32 = 82.0;
/// The controls' height.
const CONTROL: f32 = 32.0;

struct States {
    name: String,
    email: String,
    port: String,
}

fn states() -> Box<dyn PageState> {
    Box::new(States {
        name: "Lune".to_owned(),
        email: String::new(),
        port: "8080".to_owned(),
    })
}

impl PageState for States {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Form field");
        heading(ui, "NORMAL, ERROR, INLINE", card.x + 24.0, card.y + 24.0);
        let mut y = TOP;

        let normal = FormField {
            label: "Display name",
            hint: Some("Shown to other members."),
            error: None,
            inline: false,
            control_size: (0.0, CONTROL),
        };
        let name = &mut self.name;
        y += 24.0
            + ui.form_field("name", Rect::new(X, y, WIDTH, 0.0), &normal, |ui, r| {
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
            + ui.form_field("email", Rect::new(X, y, WIDTH, 0.0), &error, |ui, r| {
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
        ui.form_field("port", Rect::new(X, y, WIDTH, 0.0), &inline, |ui, r| {
            ui.text_field("input", r, port, "Port", FieldState::default())
                .id
        });
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
