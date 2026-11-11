//! The `toast-notifications` page: a strip of buttons that push toasts of each kind, a loading
//! toast the page advances and finishes, a prompt whose actions answer with a toast, and the
//! notification history beside them (`overlay.toast-notifications`). The scenes show the stack,
//! a loading toast, a prompt, an expanded toast with its log, the history, and an arrival and a
//! removal in flight.

use std::time::Duration;

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::button::ButtonKind;
use lune_ui::widgets::toast::{ToastAction, ToastId, ToastKind, ToastSpec, Toasts};

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "toast-notifications",
    title: "Toasts and notification history",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// Which of the page's states this is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The window: buttons push toasts.
    Live,
    /// Info, warning and error stacked (the stack holds three).
    Stack,
    /// One loading toast at 40% that can be cancelled.
    Loading,
    /// One prompt with two actions.
    Prompt,
    /// One error toast, expanded to its log.
    Expanded,
    /// The history panel with three items.
    History,
    /// A success toast arriving over an error one, from 600 ms.
    Appearing,
    /// A warning toast leaving from 600 ms, over an error one.
    Leaving,
}

const LOG: [&str; 14] = [
    "$ git fetch --all --prune",
    "remote: Enumerating objects: 128, done.",
    "remote: Counting objects: 100% (128/128), done.",
    "remote: Compressing objects: 100% (74/74), done.",
    "Receiving objects: 100% (96/96), 41.20 KiB | 5.15 MiB/s, done.",
    "Resolving deltas: 100% (52/52), completed with 9 local objects.",
    "From github.com:example/project",
    "   3f1c2ab..91d4e07  main       -> origin/main",
    " * [new branch]      feature/x  -> origin/feature/x",
    " - [deleted]         (none)     -> origin/old",
    "error: cannot lock ref 'refs/remotes/origin/wip': is at 7a1 but expected 4c0",
    "fatal: could not fetch from origin",
    "hint: run 'git remote prune origin' and try again",
    "exit status 128",
];

fn failed() -> ToastSpec {
    ToastSpec {
        detail: Some("origin: could not lock refs/remotes/origin/wip".into()),
        log: LOG.iter().map(|l| (*l).to_owned()).collect(),
        ..ToastSpec::new(ToastKind::Error, "Fetch failed")
    }
}

fn prompt_spec() -> ToastSpec {
    ToastSpec {
        detail: Some("Version 2.4.0 is ready to install.".into()),
        actions: vec![
            ToastAction {
                label: "Install".into(),
                primary: true,
                id: 1,
            },
            ToastAction {
                label: "Later".into(),
                primary: false,
                id: 2,
            },
        ],
        ..ToastSpec::new(ToastKind::Info, "An update is available")
    }
}

fn loading_spec(progress: Option<f32>) -> ToastSpec {
    ToastSpec::new(
        ToastKind::Loading {
            progress,
            cancellable: true,
        },
        "Cloning the repository",
    )
}

/// A toast of each kind the buttons push.
fn spec_of(kind: &str, n: u32) -> ToastSpec {
    match kind {
        "info" => ToastSpec::new(ToastKind::Info, format!("Branch switched ({n})")),
        "success" => ToastSpec::new(ToastKind::Success, format!("Pushed 3 commits ({n})")),
        "warning" => ToastSpec::new(ToastKind::Warning, format!("Detached HEAD ({n})")),
        _ => failed(),
    }
}

const PUSHES: [(&str, &str); 6] = [
    ("push-info", "Info"),
    ("push-success", "Success"),
    ("push-warning", "Warning"),
    ("push-error", "Error"),
    ("push-loading", "Loading"),
    ("push-prompt", "Prompt"),
];

/// The page plays the app: it owns the model and answers the toasts.
struct State {
    mode: Mode,
    toasts: Toasts,
    started: bool,
    pushed: u32,
    /// The loading toast the page advances, and when it began.
    running: Option<(ToastId, Duration)>,
    /// The toast the later scenes act on at 600 ms.
    subject: Option<ToastId>,
    acted: bool,
    /// The toast the knobs push: its kind (an index into [`KINDS`]), title and detail.
    kind: usize,
    title: String,
    detail: String,
}

/// The kinds the knob offers, in the order shown.
const KINDS: [(&str, ToastKind); 4] = [
    ("Info", ToastKind::Info),
    ("Success", ToastKind::Success),
    ("Warning", ToastKind::Warning),
    ("Error", ToastKind::Error),
];

fn state(mode: Mode) -> Box<dyn PageState> {
    Box::new(State {
        mode,
        toasts: Toasts::default(),
        started: false,
        pushed: 0,
        running: None,
        subject: None,
        acted: false,
        kind: 0,
        title: "A toast from the knobs".to_owned(),
        detail: String::new(),
    })
}

fn live() -> Box<dyn PageState> {
    state(Mode::Live)
}
fn stack() -> Box<dyn PageState> {
    state(Mode::Stack)
}
fn loading() -> Box<dyn PageState> {
    state(Mode::Loading)
}
fn prompt() -> Box<dyn PageState> {
    state(Mode::Prompt)
}
fn expanded() -> Box<dyn PageState> {
    state(Mode::Expanded)
}
fn history() -> Box<dyn PageState> {
    state(Mode::History)
}
fn appearing() -> Box<dyn PageState> {
    state(Mode::Appearing)
}
fn leaving() -> Box<dyn PageState> {
    state(Mode::Leaving)
}

impl State {
    /// What each scene starts with, at the page's first frame.
    fn seed(&mut self, now: Duration) {
        let t = &mut self.toasts;
        match self.mode {
            Mode::Live => {}
            Mode::Stack => {
                t.push(
                    ToastSpec {
                        detail: Some("Now on main, 3 ahead of origin.".into()),
                        ..ToastSpec::new(ToastKind::Info, "Branch switched")
                    },
                    now,
                );
                t.push(
                    ToastSpec::new(ToastKind::Warning, "Detached HEAD at 91d4e07"),
                    now,
                );
                t.push(
                    ToastSpec::new(ToastKind::Error, "Push rejected: the remote is ahead"),
                    now,
                );
            }
            Mode::Loading => {
                t.push(loading_spec(Some(0.4)), now);
            }
            Mode::Prompt => {
                t.push(prompt_spec(), now);
            }
            Mode::Expanded => {
                let id = t.push(failed(), now);
                t.set_expanded(id, true);
            }
            Mode::History => {
                for (kind, message) in [
                    (ToastKind::Success, "Pushed 3 commits"),
                    (ToastKind::Error, "Fetch failed"),
                    (ToastKind::Info, "Branch switched to main"),
                ] {
                    let id = t.push(ToastSpec::new(kind, message), now);
                    t.dismiss(id, now);
                }
            }
            Mode::Appearing | Mode::Leaving => {
                t.push(ToastSpec::new(ToastKind::Error, "Fetch failed"), now);
                if self.mode == Mode::Leaving {
                    self.subject = Some(t.push(
                        ToastSpec::new(ToastKind::Warning, "Detached HEAD at 91d4e07"),
                        now,
                    ));
                }
            }
        }
    }

    /// The later scenes' second act, at 600 ms.
    fn act(&mut self, now: Duration) {
        if self.acted || now < Duration::from_millis(600) {
            return;
        }
        match self.mode {
            Mode::Appearing => {
                self.toasts
                    .push(ToastSpec::new(ToastKind::Success, "Pushed 3 commits"), now);
            }
            Mode::Leaving => {
                if let Some(id) = self.subject {
                    self.toasts.dismiss(id, now);
                }
            }
            _ => {}
        }
        self.acted = true;
    }

    /// Advances the running loading toast: 6 s to finish, then it turns into a success.
    fn advance(&mut self, now: Duration) {
        let Some((id, began)) = self.running else {
            return;
        };
        let done = now.saturating_sub(began).as_secs_f32() / 6.0;
        if done >= 1.0 {
            self.toasts.update(
                id,
                ToastSpec::new(ToastKind::Success, "Cloned the repository"),
            );
            self.running = None;
        } else {
            self.toasts.update(id, loading_spec(Some(done)));
        }
    }
}

impl PageState for State {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let now = ui.time();
        if !self.started {
            self.started = true;
            self.seed(now);
        }
        self.act(now);
        self.advance(now);

        let card = card(ui, stage, "Toasts and notification history");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "TOASTS", x, y);
        caption(
            ui,
            "Newest at the bottom, three at most. A press on a toast opens its log.",
            x,
            y + 20.0,
        );
        if self.mode == Mode::Live {
            for (n, (key, label)) in PUSHES.iter().enumerate() {
                let at = Rect::new(x + n as f32 * 104.0, y + 48.0, 96.0, 32.0);
                if ui.button(*key, at, label, ButtonKind::Secondary).clicked {
                    self.push(key, now);
                }
            }
        }
        let panel_top = if self.mode == Mode::Live {
            y + 104.0
        } else {
            y + 48.0
        };
        heading(ui, "HISTORY", x, panel_top);
        let area = Rect::new(x, panel_top + 20.0, 360.0, card.bottom() - panel_top - 44.0);
        ui.toast_history("history", area, &mut self.toasts);

        let resp = ui.toasts("toasts", &mut self.toasts);
        if let Some((_, action)) = resp.action {
            let said = if action == 1 {
                "Installing"
            } else {
                "Maybe later"
            };
            self.toasts.push(ToastSpec::new(ToastKind::Info, said), now);
        }
        if let Some(id) = resp.cancelled {
            self.toasts
                .update(id, ToastSpec::new(ToastKind::Warning, "Clone cancelled"));
            self.running = None;
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let names: Vec<&str> = KINDS.iter().map(|(n, _)| *n).collect();
        panel.choice(ui, "Kind", &names, &mut self.kind);
        panel.text(ui, "Title", &mut self.title);
        panel.text(ui, "Detail", &mut self.detail);
        if panel.button(ui, "Push this toast") {
            let spec = ToastSpec {
                detail: (!self.detail.is_empty()).then(|| self.detail.clone()),
                ..ToastSpec::new(KINDS[self.kind.min(KINDS.len() - 1)].1, self.title.clone())
            };
            self.toasts.push(spec, ui.time());
        }
    }
}

impl State {
    fn push(&mut self, key: &str, now: Duration) {
        self.pushed += 1;
        match key {
            "push-loading" => {
                let id = self.toasts.push(loading_spec(None), now);
                self.running = Some((id, now));
            }
            "push-prompt" => {
                self.toasts.push(prompt_spec(), now);
            }
            other => {
                let kind = other.trim_start_matches("push-");
                self.toasts.push(spec_of(kind, self.pushed), now);
            }
        }
    }
}

/// A frame at 600 ms, where `State::act` runs: the capture frame then falls part way into the
/// motion it starts, instead of drawing the act at progress 0.
const ACT: &[Step] = &[Step {
    at_ms: 600,
    events: &[],
}];

const fn scene(
    id: &'static str,
    state: fn() -> Box<dyn PageState>,
    script: &'static [Step],
    capture_ms: u32,
    expect: Expect,
) -> UiScene {
    UiScene {
        id,
        scale: 2.0,
        tier: MotionTier::Full,
        state,
        script,
        capture_ms,
        expect,
    }
}

const SCENES: &[UiScene] = &[
    // Timed toasts wait on their deadline: a frame is due, nothing moves.
    scene(
        "toast-notifications-stack",
        stack,
        &[],
        800,
        Expect::Waiting,
    ),
    // The turning glyph and the bar run.
    scene(
        "toast-notifications-loading",
        loading,
        &[],
        800,
        Expect::Waiting,
    ),
    scene(
        "toast-notifications-prompt",
        prompt,
        &[],
        800,
        Expect::Settled,
    ),
    scene(
        "toast-notifications-expanded",
        expanded,
        &[],
        800,
        Expect::Settled,
    ),
    scene(
        "toast-notifications-history",
        history,
        &[],
        400,
        Expect::Settled,
    ),
    // The arrival is `base` (250 ms) from the act at 600 ms: half way at 725 ms, with the toast
    // below it moving up.
    scene(
        "toast-notifications-appearing",
        appearing,
        ACT,
        725,
        Expect::Moving,
    ),
    // The fade is `fast` (180 ms) from the act at 600 ms: half way at 690 ms.
    scene(
        "toast-notifications-leaving",
        leaving,
        ACT,
        690,
        Expect::Moving,
    ),
];

const INFO: &[&str] = &["push-info"];
const ERROR: &[&str] = &["push-error"];
const LOADING: &[&str] = &["push-loading"];
const PROMPT: &[&str] = &["push-prompt"];

/// What the bench replays: push each kind, let the stack evict, finish the loading toast.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(INFO)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::PointerOver(ERROR)],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 900,
        events: &[Ev::PointerOver(LOADING)],
    },
    Step {
        at_ms: 1000,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1500,
        events: &[Ev::PointerOver(PROMPT)],
    },
    Step {
        at_ms: 1600,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 2000,
        events: &[Ev::PointerOver(INFO)],
    },
    Step {
        at_ms: 2100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 2200,
        events: &[Ev::Click],
    },
];
