//! The clipboard lab: text goes to the system clipboard when Copy is pressed and comes back
//! when Paste is. The page never touches the clipboard on its own: it writes on a Copy press,
//! asks for a read on a Paste press and takes only the answer to that ask. The window
//! executes the requests (`lune_ui::clipboard::Clipboard`); text is all it carries.

use lune_ui::input::Event;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::text_area::AreaOptions;

use super::text;
use crate::page::{Page, PageKind, PageState, Stage, UiPage};
use crate::pages::{card, heading};

pub(crate) static PAGE: Page = Page {
    id: "lab-clipboard",
    title: "Clipboard lab",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: &[],
        bench: &[],
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Clipboard::new())
}

/// Most characters either box holds: what is pasted past it is cut, and said so.
const MAX_CHARS: usize = 10_000;
const BOX: (f32, f32) = (560.0, 120.0);

struct Clipboard {
    /// The text Copy puts on the clipboard.
    copy: String,
    /// What the last Paste brought back.
    pasted: String,
    /// A read was asked for and has not been answered.
    waiting: bool,
    status: String,
}

impl Clipboard {
    fn new() -> Self {
        Self {
            copy: "Hello from luneui".to_owned(),
            pasted: String::new(),
            waiting: false,
            status: "Nothing copied or pasted yet".to_owned(),
        }
    }

    /// Takes the answer to a Paste, when one came this frame.
    fn take_answer(&mut self, ui: &Ui<'_>) {
        if !self.waiting {
            return;
        }
        for event in &ui.input().events {
            match event {
                Event::ClipboardText(t) => {
                    let total = t.chars().count();
                    self.pasted = t.chars().take(MAX_CHARS).collect();
                    self.status = if total > MAX_CHARS {
                        format!("Pasted {total} characters, the first {MAX_CHARS} shown")
                    } else {
                        format!("Pasted {total} characters")
                    };
                    self.waiting = false;
                }
                Event::ClipboardUnavailable => {
                    self.status = "The clipboard holds no text".to_owned();
                    self.waiting = false;
                }
                _ => {}
            }
        }
    }
}

impl PageState for Clipboard {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Clipboard lab");
        let t = *ui.theme();
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        self.take_answer(ui);

        heading(ui, "TEXT TO COPY", x, y);
        let source = AreaOptions {
            placeholder: "Type something to copy",
            label: Some("Text to copy"),
            wrap: true,
            max_chars: Some(MAX_CHARS),
            ..AreaOptions::default()
        };
        ui.text_area(
            "copy",
            Rect::new(x, y + 20.0, BOX.0, BOX.1),
            &mut self.copy,
            &source,
        );
        let w = button_width(ui.painter(), "Copy");
        let copy = Rect::new(x, y + 20.0 + BOX.1 + 12.0, w, 32.0);
        if ui
            .button("copy-button", copy, "Copy", ButtonKind::Primary)
            .clicked
        {
            ui.clipboard_write(self.copy.clone());
            self.status = format!("Copied {} characters", self.copy.chars().count());
        }

        heading(ui, "PASTED TEXT", x, y + 212.0);
        let shown = AreaOptions {
            placeholder: "Press Paste",
            label: Some("Pasted text"),
            wrap: true,
            read_only: true,
            ..AreaOptions::default()
        };
        ui.text_area(
            "pasted",
            Rect::new(x, y + 232.0, BOX.0, BOX.1),
            &mut self.pasted,
            &shown,
        );
        let w = button_width(ui.painter(), "Paste");
        let paste = Rect::new(x, y + 232.0 + BOX.1 + 12.0, w, 32.0);
        if ui
            .button("paste-button", paste, "Paste", ButtonKind::Secondary)
            .clicked
        {
            ui.clipboard_read();
            self.waiting = true;
            self.status = "Waiting for the clipboard".to_owned();
        }
        text(
            ui,
            &self.status,
            x,
            y + 232.0 + BOX.1 + 60.0,
            t.font_base,
            t.fg_secondary,
        );

        let notes = [
            "Text only. The page reads or writes the clipboard",
            "only when Copy or Paste is pressed.",
            "",
            "Copy puts the first box on the clipboard; Paste asks",
            "for what is there and shows it in the second box.",
            "Cmd+C and Cmd+V inside a box work as in any field.",
        ];
        for (i, line) in notes.iter().enumerate() {
            text(
                ui,
                line,
                x + 620.0,
                y + 24.0 + i as f32 * 20.0,
                t.font_s,
                t.fg_tertiary,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::rig::player;
    use super::*;
    use crate::page::Ev;
    use lune_ui::access::Role;
    use lune_ui::input::{ClipboardRequest, FrameOutput};
    use lune_ui::testing::FakeText;

    fn pasted_text(out: &FrameOutput) -> String {
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == Role::MultilineText && n.name == "Pasted text")
            .and_then(|n| n.text.as_ref())
            .map(|t| t.text.clone())
            .expect("the pasted-text box")
    }

    fn frame(p: &mut crate::page::Player, ms: u32, events: &[Ev]) -> FrameOutput {
        p.frame(&mut FakeText, ms, events).expect("frame").1
    }

    /// The clipboard requests of the frames from `from` to `to`, 16 ms apart, the first with
    /// `events`.
    fn requests(
        p: &mut crate::page::Player,
        from: u32,
        to: u32,
        events: &[Ev],
    ) -> Vec<ClipboardRequest> {
        let mut out = Vec::new();
        let mut events = events;
        let mut ms = from;
        while ms <= to {
            out.extend(frame(p, ms, events).clipboard);
            events = &[];
            ms += 16;
        }
        out
    }

    /// Nothing happens to the clipboard until a button is pressed; Copy writes the box, Paste
    /// reads, and only the answer to that read lands in the second box.
    #[test]
    fn the_clipboard_is_touched_on_a_press_only() {
        let mut p = player("lab-clipboard", live());
        frame(&mut p, 0, &[]);
        // An answer nobody asked for is not taken, and frames alone ask for nothing.
        let idle = requests(&mut p, 16, 64, &[Ev::ClipboardText("unasked")]);
        assert!(idle.is_empty(), "{idle:?}");
        assert_eq!(pasted_text(&frame(&mut p, 80, &[])), "");
        // Copy.
        let copied = requests(
            &mut p,
            96,
            144,
            &[Ev::PointerOver(&["copy-button"]), Ev::Click],
        );
        assert_eq!(
            copied,
            [ClipboardRequest::Write("Hello from luneui".to_owned())]
        );
        // Paste, then the answer.
        let asked = requests(
            &mut p,
            160,
            208,
            &[Ev::PointerOver(&["paste-button"]), Ev::Click],
        );
        assert_eq!(asked, [ClipboardRequest::Read]);
        frame(&mut p, 224, &[Ev::ClipboardText("from elsewhere")]);
        assert_eq!(pasted_text(&frame(&mut p, 240, &[])), "from elsewhere");
        // A second answer without a second ask is left alone.
        frame(&mut p, 256, &[Ev::ClipboardText("again")]);
        assert_eq!(pasted_text(&frame(&mut p, 272, &[])), "from elsewhere");
    }

    #[test]
    fn a_long_paste_is_cut_to_what_the_box_holds() {
        let mut c = Clipboard::new();
        c.waiting = true;
        let long = "x".repeat(MAX_CHARS + 5);
        let mut input = lune_ui::testing::ScriptedInput::new((100.0, 100.0), 1.0);
        input.clipboard_text(&long);
        let raw = input.end_frame();
        let mut store = lune_ui::ui::UiStore::default();
        let strings = lune_ui::strings::StringTable::default();
        let shortcuts = lune_ui::shortcuts::Shortcuts::default();
        let config = lune_ui::ui::FrameConfig {
            theme: lune_ui::lunegit_dark::THEME,
            tier: lune_ui::motion::MotionTier::Full,
            strings: &strings,
            arg_format: &lune_ui::strings::DisplayFormat,
            keymap: lune_ui::input::Keymap::MAC,
            shortcuts: &shortcuts,
        };
        let mut text_system = FakeText;
        let ui = Ui::begin(&mut store, &raw, config, &mut text_system);
        c.take_answer(&ui);
        assert_eq!(c.pasted.chars().count(), MAX_CHARS);
        assert!(c.status.contains("first 10000"), "{}", c.status);
        assert!(!c.waiting);
    }
}
