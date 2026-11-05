//! The `command-palette` page: a button over the gallery background that opens the palette
//! (`Ui::command_palette`). The data is neutral, files and commands, and the filtering is the
//! page's own, as an app's would be: a case-insensitive match on the text, a `>` prefix for the
//! commands, a `date:` prefix that fails to parse, four rows a section and a "show more" row for
//! the rest. An empty query shows no rows. Activating the destructive command opens a dialog
//! above the palette; the rest close it and are named on the page.
//!
//! The scenes freeze the palette empty, with results, with one row selected by Down, with the
//! destructive row's tooltip, with the error line, and half way through the open.

use lune_ui::icon;
use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::command_palette::{Palette, PaletteRow, PaletteSection};
use lune_ui::widgets::dialog::{Cancel, Dialog, DialogGlass};

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "command-palette",
    title: "Command palette",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// One result the page can find.
#[derive(Debug, Clone, Copy)]
struct Hit {
    key: u64,
    text: &'static str,
    detail: Option<&'static str>,
    icon: Option<&'static str>,
    hint: Option<&'static str>,
    destructive: bool,
}

const fn file(key: u64, text: &'static str, detail: &'static str) -> Hit {
    Hit {
        key,
        text,
        detail: Some(detail),
        icon: None,
        hint: None,
        destructive: false,
    }
}

const FILES: [Hit; 6] = [
    file(1, "main.rs", "src/"),
    file(2, "lib.rs", "src/"),
    file(3, "render.rs", "src/gpu/"),
    file(4, "theme.rs", "src/"),
    file(5, "Cargo.toml", "./"),
    file(6, "README.md", "./"),
];

const COMMANDS: [Hit; 3] = [
    Hit {
        key: 10,
        text: "Reload window",
        detail: None,
        icon: Some(icon::SYNC),
        hint: Some("Cmd+R"),
        destructive: false,
    },
    Hit {
        key: 11,
        text: "Open settings",
        detail: None,
        icon: Some(icon::GEAR),
        hint: Some("Cmd+,"),
        destructive: false,
    },
    Hit {
        key: 12,
        text: "Clear cache",
        detail: None,
        icon: Some(icon::CLOSE),
        hint: None,
        destructive: true,
    },
];

/// The key of the destructive command.
const CLEAR_CACHE: u64 = 12;
/// Rows a section shows before its "show more".
const PER_SECTION: usize = 4;

/// The page with a palette that is open, or opens from its button.
struct Live {
    open: bool,
    query: String,
    /// Whether each section shows all its matches.
    expanded: [bool; 2],
    /// The confirm dialog the destructive command opens above the palette.
    confirming: bool,
    /// The text of the last row that finished its work.
    done: Option<&'static str>,
}

fn state(open: bool, query: &str) -> Box<dyn PageState> {
    Box::new(Live {
        open,
        query: query.to_owned(),
        expanded: [false; 2],
        confirming: false,
        done: None,
    })
}

fn live() -> Box<dyn PageState> {
    state(false, "")
}

fn empty() -> Box<dyn PageState> {
    state(true, "")
}

fn results() -> Box<dyn PageState> {
    state(true, "r")
}

fn clear_only() -> Box<dyn PageState> {
    state(true, "clear")
}

fn failing() -> Box<dyn PageState> {
    state(true, "date:7x")
}

fn glass() -> DialogGlass {
    DialogGlass {
        panel: lunegit_dark::PANEL.premultiplied(),
        fill: lunegit_dark::WINDOW.premultiplied(),
        noise: true,
    }
}

/// What the page finds for `query`: the rows of the files and of the commands, each with whether
/// more match than it shows, and the error line.
struct Found {
    files: Vec<PaletteRow<'static>>,
    commands: Vec<PaletteRow<'static>>,
    more: [bool; 2],
    error: Option<&'static str>,
}

fn row(hit: &Hit) -> PaletteRow<'static> {
    PaletteRow {
        key: hit.key,
        text: hit.text,
        detail: hit.detail,
        icon: hit.icon,
        hint: hit.hint,
        destructive: hit.destructive,
    }
}

fn find(query: &str, expanded: [bool; 2]) -> Found {
    let mut found = Found {
        files: Vec::new(),
        commands: Vec::new(),
        more: [false; 2],
        error: None,
    };
    if query.starts_with("date:") {
        found.error = Some("Couldn't parse the date filter \u{2014} showing no results.");
        return found;
    }
    // A `>` prefix searches the commands only.
    let (only_commands, term) = match query.strip_prefix('>') {
        Some(rest) => (true, rest.trim()),
        None => (false, query.trim()),
    };
    if !only_commands && term.is_empty() {
        return found;
    }
    let term = term.to_lowercase();
    let matches = |hits: &'static [Hit]| -> Vec<&'static Hit> {
        hits.iter()
            .filter(|h| h.text.to_lowercase().contains(&term))
            .collect()
    };
    let take = |hits: Vec<&'static Hit>, all: bool| -> (Vec<PaletteRow<'static>>, bool) {
        let more = !all && hits.len() > PER_SECTION;
        let shown = if all { hits.len() } else { PER_SECTION };
        (hits.into_iter().take(shown).map(row).collect(), more)
    };
    if !only_commands {
        (found.files, found.more[0]) = take(matches(&FILES), expanded[0]);
    }
    (found.commands, found.more[1]) = take(matches(&COMMANDS), expanded[1]);
    found
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let c = card(ui, "Command palette");
        let (x, y) = (c.x + 24.0, c.y + 24.0);
        heading(ui, "A PALETTE OVER THE PAGE", x, y);
        let w = button_width(ui.painter(), "Open palette");
        let r = ui.button(
            "open",
            Rect::new(x, y + 20.0, w, 32.0),
            "Open palette",
            ButtonKind::Primary,
        );
        if r.clicked {
            self.open = true;
        }
        let said = match self.done {
            Some(text) => format!("Last run: {text}"),
            None => "Type to search; > for commands; date: for an error.".to_owned(),
        };
        caption(ui, &said, x, y + 68.0);
        caption(
            ui,
            "Up and Down select, Enter runs, Escape closes.",
            x,
            y + 88.0,
        );

        if self.open {
            let found = find(&self.query, self.expanded);
            let sections = [
                PaletteSection {
                    title: "FILES",
                    rows: &found.files,
                    more: found.more[0],
                },
                PaletteSection {
                    title: "COMMANDS",
                    rows: &found.commands,
                    more: found.more[1],
                },
            ];
            let palette = Palette {
                sections: &sections,
                placeholder: "Search files and commands...",
                footer_left: "Up/Down select    > commands    date:7d",
                error: found.error,
                name: "",
            };
            let out = ui.command_palette("palette", &mut self.query, &palette, &glass());
            if let Some(i) = out.show_more {
                self.expanded[i] = true;
            }
            if let Some(key) = out.activated {
                if key == CLEAR_CACHE {
                    self.confirming = true;
                } else {
                    self.finish(key);
                }
            }
            if out.dismissed {
                self.open = false;
            }
        }
        if self.confirming {
            let dialog = Dialog {
                title: "Clear the cache",
                message: "Cached files are rebuilt on the next run.",
                extra_h: 0.0,
                confirm: "Clear",
                cancel: Cancel::Default,
            };
            let d = ui.dialog("confirm", &dialog, &glass(), |_, _, _, _| {});
            if d.confirmed {
                self.finish(CLEAR_CACHE);
            }
            if d.confirmed || d.cancelled {
                self.confirming = false;
            }
        }
    }
}

impl Live {
    /// The row `key` ran: the palette closes.
    fn finish(&mut self, key: u64) {
        self.done = FILES
            .iter()
            .chain(&COMMANDS)
            .find(|h| h.key == key)
            .map(|h| h.text);
        self.open = false;
        self.query.clear();
        self.expanded = [false; 2];
    }
}

const OPEN: &[&str] = &["open"];

/// A frame after the open: the search box takes the focus after the first frame is drawn, and
/// its focus-in then plays from this one.
const FOCUSED: &[Step] = &[Step {
    at_ms: 300,
    events: &[],
}];

/// A scene open since the start and settled by `capture_ms` (the open is 250 ms).
const fn settled(id: &'static str, state: fn() -> Box<dyn PageState>) -> UiScene {
    UiScene {
        id,
        scale: 2,
        tier: MotionTier::Full,
        state,
        script: FOCUSED,
        capture_ms: 700,
        expect: Expect::Settled,
    }
}

/// The only result is the destructive command: with the card centred in the 1040 x 660 window,
/// its row is 29 px high, 18 px under the results' top (a header and its gap), and the results
/// start 68 px under the card's top (the 40 px bar in its padding).
const DESTRUCTIVE_AT: (f32, f32) = (520.0, 355.0);

const SCENES: &[UiScene] = &[
    settled("command-palette-empty", empty),
    settled("command-palette-results", results),
    // A Down moves the selection from the first row to the second.
    UiScene {
        id: "command-palette-selected",
        scale: 2,
        tier: MotionTier::Full,
        state: results,
        script: &[Step {
            at_ms: 400,
            events: &[Ev::Key(NamedKey::ArrowDown)],
        }],
        capture_ms: 900,
        expect: Expect::Settled,
    },
    // The pointer rests on the destructive row from the start; the tooltip is due 800 ms later
    // and has settled by 1400 ms.
    UiScene {
        id: "command-palette-destructive",
        scale: 2,
        tier: MotionTier::Full,
        state: clear_only,
        script: &[
            Step {
                at_ms: 400,
                events: &[Ev::PointerAt(DESTRUCTIVE_AT.0, DESTRUCTIVE_AT.1)],
            },
            Step {
                at_ms: 1300,
                events: &[],
            },
        ],
        capture_ms: 1600,
        expect: Expect::Settled,
    },
    settled("command-palette-date-error", failing),
    // The open is `pop`, base (250 ms): half way at 125 ms.
    UiScene {
        id: "command-palette-opening",
        scale: 2,
        tier: MotionTier::Full,
        state: results,
        script: &[],
        capture_ms: 125,
        expect: Expect::Moving,
    },
];

/// What the bench replays: open from the button, type, walk down, run a row, reopen, type to the
/// destructive command, confirm its dialog away, close with Escape.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(OPEN)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Type("r")],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::ArrowDown)],
    },
    Step {
        at_ms: 900,
        events: &[Ev::Key(NamedKey::ArrowUp)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Key(NamedKey::Enter)],
    },
    Step {
        at_ms: 1500,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1900,
        events: &[Ev::Type("clear")],
    },
    Step {
        at_ms: 2100,
        events: &[Ev::Key(NamedKey::Enter)],
    },
    Step {
        at_ms: 2500,
        events: &[Ev::Key(NamedKey::Escape)],
    },
    Step {
        at_ms: 2900,
        events: &[Ev::Key(NamedKey::Escape)],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::{Role, State};
    use lune_ui::scene::LayerId;
    use lune_ui::testing::FakeText;

    fn run(id: &str) -> (lune_ui::scene::Scene, lune_ui::input::FrameOutput) {
        let scene = SCENES.iter().find(|s| s.id == id).expect("a scene");
        run_scene(PAGE.id, scene, &mut FakeText).expect("runs")
    }

    #[test]
    fn the_page_filters_as_an_app_would() {
        assert!(find("", [false; 2]).files.is_empty(), "empty shows nothing");
        let r = find("r", [false; 2]);
        assert_eq!((r.files.len(), r.more), (4, [true, false]));
        assert_eq!(find("r", [true, false]).files.len(), 6);
        let c = find(">", [false; 2]);
        assert!(c.files.is_empty() && c.commands.len() == 3);
        assert!(find("date:7x", [false; 2]).error.is_some());
        assert_eq!(find("clear", [false; 2]).commands.len(), 1);
    }

    #[test]
    fn the_scenes_show_what_they_are_named_for() {
        let (_, out) = run("command-palette-empty");
        assert!(out.tree.nodes.iter().all(|n| n.role != Role::ListItem));
        let (_, out) = run("command-palette-results");
        let rows = out
            .tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::ListItem)
            .count();
        assert_eq!(rows, 6, "four files and two commands");
        let (_, out) = run("command-palette-selected");
        let selected: Vec<_> = out
            .tree
            .nodes
            .iter()
            .filter(|n| n.states.contains(State::Selected))
            .map(|n| n.name.as_str())
            .collect();
        assert_eq!(
            selected,
            ["lib.rs"],
            "Down moved the selection to the second row"
        );
        let (scene, out) = run("command-palette-destructive");
        assert!(
            out.tree.nodes.iter().any(|n| n.role == Role::Tooltip),
            "the pointer is on the destructive row and the tooltip shows"
        );
        assert!(
            !scene.layers[LayerId::Raised as usize]
                .geometry
                .under
                .is_empty(),
            "above the card"
        );
        let (_, out) = run("command-palette-date-error");
        assert!(out.tree.nodes.iter().any(|n| n.role == Role::Status));
    }
}
