//! The `locale` page: the confirm dialog and a menu drawn with a test locale that wraps every
//! string in brackets and lengthens it by 30 percent. It shows that every toolkit string goes
//! through the table and that the layout survives longer text.

use lune_ui::icon;
use lune_ui::motion::MotionTier;
use lune_ui::strings::{DEFAULTS, StringTable};
use lune_ui::ui::Ui;
use lune_ui::widgets::dialog::{Cancel, Dialog};
use lune_ui::widgets::menu::MenuRow;

use super::controls_ui::{Controls, glass};
use super::{card, card_shift};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "locale",
    title: "Locale",
    kind: PageKind::Ui(UiPage {
        state: pseudo,
        scenes: SCENES,
        bench: &[],
    }),
};

/// The app's own strings of this page: key and English text.
const APP_STRINGS: &[(&str, &str)] = &[
    ("app.dialog.title", "Discard changes"),
    (
        "app.dialog.message",
        "The document has unsaved changes. Closing it will discard them.",
    ),
    ("app.dialog.confirm", "Discard"),
    ("app.menu.new", "New Window"),
    ("app.menu.copy", "Copy"),
    ("app.menu.settings", "Settings..."),
];

/// `[` + `text` + 30 percent padding (rounded up) + `]`.
fn pseudo_text(text: &str) -> String {
    let pad = (text.chars().count() * 3).div_ceil(10);
    format!("[{text}{}]", "~".repeat(pad))
}

/// A table mapping every toolkit default and every app string of the page to its pseudo text.
fn pseudo_table() -> StringTable {
    let mut table = StringTable::default();
    for (key, text) in DEFAULTS.iter().chain(APP_STRINGS) {
        table
            .insert(*key, pseudo_text(text))
            .expect("toolkit keys and app keys are valid");
    }
    table
}

/// The page under `table`: the table's text for `key`, else `english`.
fn t(ui: &Ui<'_>, key: &str, english: &str) -> String {
    let s = ui.tr(key);
    if s == key { english } else { s }.to_owned()
}

struct Locale {
    controls: Controls,
    pseudo: bool,
    /// The dialog's own Cancel or Discard closed it.
    closed: bool,
    /// A press outside or a pick closed the menu.
    menu_closed: bool,
}

fn pseudo() -> Box<dyn PageState> {
    Box::new(Locale {
        controls: Controls::new(),
        pseudo: true,
        closed: false,
        menu_closed: false,
    })
}

impl PageState for Locale {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "luneui gallery");
        self.controls.draw(ui, area);
        let rows = [
            ("app.menu.new", Some(icon::ADD)),
            ("app.menu.copy", Some(icon::COPY)),
            ("app.menu.settings", Some(icon::GEAR)),
        ];
        let texts: Vec<String> = rows
            .iter()
            .map(|(key, _)| {
                let english = APP_STRINGS
                    .iter()
                    .find(|(k, _)| k == key)
                    .map_or(*key, |(_, v)| *v);
                t(ui, key, english)
            })
            .collect();
        let menu_rows: Vec<MenuRow<'_>> = rows
            .iter()
            .zip(&texts)
            .map(|((_, glyph), text)| MenuRow::Item {
                glyph: *glyph,
                text,
                shortcut: "",
            })
            .collect();
        let (dx, dy) = card_shift(area);
        if !self.menu_closed {
            let m = ui.menu("menu", &menu_rows, (24.0 + dx, 350.0 + dy));
            self.menu_closed = m.picked.is_some() || m.dismissed;
        }

        let title = t(ui, "app.dialog.title", "Discard changes");
        let message = t(
            ui,
            "app.dialog.message",
            "The document has unsaved changes. Closing it will discard them.",
        );
        let confirm = t(ui, "app.dialog.confirm", "Discard");
        let d = Dialog {
            title: &title,
            message: &message,
            extra_h: 0.0,
            confirm: &confirm,
            cancel: Cancel::Default,
        };
        if !self.closed {
            let r = ui.dialog("dialog", &d, &glass(), |_, _, _, _| {});
            self.closed = r.confirmed || r.cancelled;
        }
    }

    fn strings(&self) -> StringTable {
        if self.pseudo {
            pseudo_table()
        } else {
            StringTable::default()
        }
    }
}

const SCENES: &[UiScene] = &[UiScene {
    id: "locale-pseudo",
    scale: 2.0,
    tier: MotionTier::Full,
    state: pseudo,
    script: &[Step {
        at_ms: 300,
        events: &[],
    }],
    capture_ms: 700,
    expect: Expect::Settled,
}];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pseudo_table_covers_every_default_and_wraps_it() {
        let table = pseudo_table();
        for (key, english) in DEFAULTS {
            let text = table.get(key).unwrap_or_else(|| panic!("{key} missing"));
            assert!(
                text.starts_with('[') && text.ends_with(']'),
                "{key}: {text}"
            );
            assert!(text.contains(english), "{key}: {text}");
            let extra = text.chars().count() - english.chars().count() - 2;
            assert!(extra >= english.chars().count() * 3 / 10, "{key}: {text}");
        }
    }
}
