//! The `tooltip` page: an icon button with a tooltip (`overlay.tooltip`). The tooltip shows after
//! 800 ms of hover; the scenes script the pointer and the time. A shown tooltip begins its fade on
//! the first frame that draws it, so every scene that wants one on screen runs a frame at 800 ms.
//! The button is drawn here from the public pieces (a region, a node, a fill and a glyph) because
//! the toolkit has no icon button.

use lune_ui::access::{Node, Role, State};
use lune_ui::icon;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::{Sense, Ui};

use super::{card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "tooltip",
    title: "Tooltip",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The icon button, in window coordinates: inside the card, under the heading.
const ANCHOR: Rect = Rect::new(34.0, 82.0, 32.0, 32.0);
/// Near the bottom edge (window height 660): a tooltip below would overflow, so it flips above.
const LOW: Rect = Rect::new(34.0, 600.0, 32.0, 32.0);
/// Near the right edge (window width 1040): the centred tooltip is shifted inside.
const RIGHT: Rect = Rect::new(1000.0, 82.0, 32.0, 32.0);
const TEXT: &str = "Rename";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    Rich,
}

fn base() -> Live {
    Live {
        anchor: ANCHOR,
        kind: Kind::Text,
        enabled: true,
        text: TEXT.to_owned(),
    }
}

fn live() -> Box<dyn PageState> {
    Box::new(base())
}

fn rich() -> Box<dyn PageState> {
    Box::new(Live {
        kind: Kind::Rich,
        ..base()
    })
}

fn flipped() -> Box<dyn PageState> {
    Box::new(Live {
        anchor: LOW,
        ..base()
    })
}

fn shifted() -> Box<dyn PageState> {
    Box::new(Live {
        anchor: RIGHT,
        ..base()
    })
}

fn disabled() -> Box<dyn PageState> {
    Box::new(Live {
        enabled: false,
        ..base()
    })
}

/// One icon button with a tooltip: one line, or rich content.
struct Live {
    anchor: Rect,
    kind: Kind,
    enabled: bool,
    /// The tooltip's text, and the rich tooltip's bold line.
    text: String,
}

/// The anchors the knob offers, in the order shown.
const ANCHORS: [(&str, Rect); 3] = [
    ("Top left", ANCHOR),
    ("Bottom edge", LOW),
    ("Right edge", RIGHT),
];

/// The rich tooltip's content size (before padding): a bold line, a secondary line, a separator
/// and a row of three icons.
const RICH: (f32, f32) = (190.0, 76.0);

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Tooltip");
        heading(ui, "TOOLTIP", card.x + 24.0, card.y + 24.0);
        let id = ui.id("icon");
        let anchor = on_card(card, self.anchor);
        let r = ui.with_enabled(self.enabled, |ui| {
            let r = ui.interact(id, anchor, Sense::CLICK_FOCUS);
            let mut node = Node::new(id, Role::Button, "", anchor);
            node.states.insert(State::Focusable);
            ui.node(node);
            r
        });
        let ink = if self.enabled {
            THEME.fg_primary
        } else {
            THEME.fg_dim
        };
        let p = ui.painter();
        if r.hovered {
            p.fill(anchor, 4.0, THEME.surface_hover);
        }
        p.text_centered(&icon::glyph(icon::EDIT, 16.0, ink), anchor);
        match self.kind {
            Kind::Text => ui.tooltip(&r, &self.text),
            Kind::Rich => ui.tooltip_with(&r, RICH, |ui, at| {
                let bold = Spec::plain(
                    self.text.as_str(),
                    Style::ui(THEME.font_s, THEME.fg_primary).bold(),
                );
                let note = Spec::plain(
                    "Change the file name",
                    Style::ui(THEME.font_s, THEME.fg_secondary),
                );
                let p = ui.painter();
                p.text(&bold, at.x, at.y);
                p.text(&note, at.x, at.y + 20.0);
                p.fill(Rect::new(at.x, at.y + 42.0, at.w, 1.0), 0.0, THEME.border);
                for (i, glyph) in [icon::EDIT, icon::COPY, icon::INFO].into_iter().enumerate() {
                    let cell = Rect::new(at.x + i as f32 * 28.0, at.y + 50.0, 24.0, 24.0);
                    p.text_centered(&icon::glyph(glyph, 14.0, THEME.fg_secondary), cell);
                }
            }),
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let mut rich = usize::from(self.kind == Kind::Rich);
        if panel.choice(ui, "Content", &["Text", "Rich"], &mut rich) {
            self.kind = if rich == 1 { Kind::Rich } else { Kind::Text };
        }
        panel.text(ui, "Text", &mut self.text);
        panel.toggle(ui, "Enabled", &mut self.enabled);
        let names: Vec<&str> = ANCHORS.iter().map(|(n, _)| *n).collect();
        let mut at = ANCHORS
            .iter()
            .position(|(_, r)| *r == self.anchor)
            .unwrap_or(0);
        if panel.choice(ui, "Anchor", &names, &mut at) {
            self.anchor = ANCHORS[at].1;
        }
    }
}

const ICON: &[&str] = &["icon"];
/// Where the pointer rests when it is not on the button.
const AWAY: Ev = Ev::PointerAt(600.0, 360.0);

/// The pointer comes onto the button at 0; the tooltip is due at 800 ms, so a frame there starts
/// its fade (`fast`, 180 ms); at 1100 ms it has settled.
const SHOWN: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(ICON)],
    },
    Step {
        at_ms: 800,
        events: &[],
    },
];

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
    scene("tooltip-text", live, SHOWN, 1100, Expect::Settled),
    scene("tooltip-rich", rich, SHOWN, 1100, Expect::Settled),
    scene("tooltip-flipped", flipped, SHOWN, 1100, Expect::Settled),
    scene("tooltip-shifted", shifted, SHOWN, 1100, Expect::Settled),
    scene(
        "tooltip-disabled-anchor",
        disabled,
        SHOWN,
        1100,
        Expect::Settled,
    ),
    // 700 ms of hover: nothing is drawn, and the frame asks to be redrawn at 800 ms.
    scene(
        "tooltip-pending",
        live,
        &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(ICON)],
        }],
        700,
        Expect::Waiting,
    ),
    // Due at 800 ms, where a frame starts the fade; half way at 890 ms, captured at 900 ms.
    scene(
        "tooltip-appearing",
        live,
        &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(ICON)],
            },
            Step {
                at_ms: 800,
                events: &[],
            },
        ],
        900,
        Expect::Moving,
    ),
    // Shown and settled by 1100 ms; the pointer leaves at 1200 ms and the dismiss is `instant`
    // (120 ms): half way at 1260 ms.
    scene(
        "tooltip-dismissing",
        live,
        &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(ICON)],
            },
            Step {
                at_ms: 800,
                events: &[],
            },
            Step {
                at_ms: 1100,
                events: &[],
            },
            Step {
                at_ms: 1200,
                events: &[AWAY],
            },
        ],
        1260,
        Expect::Moving,
    ),
];

/// What the bench replays: the pointer arrives, the tooltip shows, the pointer leaves.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(ICON)],
    },
    Step {
        at_ms: 800,
        events: &[],
    },
    Step {
        at_ms: 1600,
        events: &[AWAY],
    },
];
