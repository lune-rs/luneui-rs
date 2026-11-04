//! The `tab-strip` page: the strip over a card, with the page playing the app. It owns the list and
//! the active tab, applies every [`TabEvent`], and crossfades the card's body on an activation
//! ([`CONTENT_SWAP`]): the strip only reports it.
//!
//! Titles are short, so every tab is the 108 px minimum and a scene can aim the pointer by number:
//! tab `i` starts at `TAB_X + i * 113`. Tabs are found by coordinates because a tab's id comes from
//! its key, which no string path names.

use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::tab_strip::{
    BusyCue, CONTENT_SWAP, GAP, HEIGHT, MIN_WIDTH, Tab, TabEvent, TabKey, TabStripOptions,
};
use lune_ui::widgets::tone::Tone;

use super::card;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "tab-strip",
    title: "Tab strip",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The inner card's offset in the page's card, and its size.
const INSET: f32 = 24.0;
const CARD_W: f32 = 720.0;
const CARD_H: f32 = 300.0;
/// The header's darker band, punched out by the outline. The toolkit's look has no token for it:
/// the header layer is a material of the preset, so the app chooses it.
const HEADER_TINT: [u8; 4] = [0x00, 0x00, 0x00, 0x24];

/// The inner card's top-left corner in the window: the page card's, plus [`INSET`].
const TOP: f32 = 34.0 + INSET;
const LEFT: f32 = 10.0 + INSET;
const TAB_STEP: f32 = MIN_WIDTH + GAP;
const MID_Y: f32 = TOP + HEIGHT / 2.0;

/// The middle of tab `i` at scroll offset 0.
const fn tab_mid(i: usize) -> f32 {
    LEFT + i as f32 * TAB_STEP + MIN_WIDTH / 2.0
}

/// One tab of the page's list.
#[derive(Debug, Clone)]
struct Item {
    key: u64,
    title: String,
    progress: Option<Tone>,
}

impl Item {
    fn new(key: u64, title: &str) -> Self {
        Item {
            key,
            title: title.to_owned(),
            progress: None,
        }
    }
}

/// The app's side of the strip.
struct Demo {
    items: Vec<Item>,
    active: Option<u64>,
    next: u64,
    panel_focused: bool,
    busy: Option<BusyCue>,
    tone: Tone,
}

fn numbered(n: u64) -> Vec<Item> {
    (1..=n).map(|k| Item::new(k, &format!("Tab {k}"))).collect()
}

impl Demo {
    fn new(items: Vec<Item>, active: Option<u64>) -> Self {
        let next = items.iter().map(|i| i.key).max().unwrap_or(0) + 1;
        Demo {
            items,
            active,
            next,
            panel_focused: true,
            busy: None,
            tone: Tone::Neutral,
        }
    }

    fn apply(&mut self, event: TabEvent) {
        match event {
            TabEvent::Activate(k) => self.active = Some(k.0),
            TabEvent::Close(k) => self.close(k),
            TabEvent::New => {
                let key = self.next;
                self.next += 1;
                self.items.push(Item::new(key, &format!("Tab {key}")));
                self.active = Some(key);
            }
            TabEvent::Reorder { from, to } => {
                let item = self.items.remove(from);
                self.items.insert(to, item);
            }
            TabEvent::CloseOthers(k) => {
                self.items.retain(|i| i.key == k.0);
                self.active = Some(k.0);
            }
            TabEvent::CloseAll => {
                self.items.clear();
                self.active = None;
            }
            TabEvent::Rename(_) | TabEvent::MenuExtra(..) => {}
        }
    }

    fn close(&mut self, k: TabKey) {
        let Some(at) = self.items.iter().position(|i| i.key == k.0) else {
            return;
        };
        self.items.remove(at);
        if self.active == Some(k.0) {
            let next = at.min(self.items.len().saturating_sub(1));
            self.active = self.items.get(next).map(|i| i.key);
        }
    }

    /// The body: each tab's text at its own opacity, so an activation crossfades.
    fn body(&self, ui: &mut Ui<'_>, rect: Rect) {
        for item in &self.items {
            let shown = self.active == Some(item.key);
            let id = ui.id(("body", item.key));
            let alpha = ui.animate_f32(id, if shown { 1.0 } else { 0.0 }, &CONTENT_SWAP);
            if alpha <= 0.0 {
                continue;
            }
            let c = THEME.fg_secondary;
            let color = [c[0], c[1], c[2], (f32::from(c[3]) * alpha).round() as u8];
            let spec = Spec::plain(
                format!("The body of {}", item.title),
                Style::ui(THEME.font_base, color),
            );
            ui.painter().text_centered(&spec, rect);
        }
    }
}

impl PageState for Demo {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let page = card(ui, "Tab strip");
        let inner = Rect::new(page.x + INSET, page.y + INSET, CARD_W, CARD_H);
        let strip = Rect::new(inner.x, inner.y, inner.w, HEIGHT);
        let tabs: Vec<Tab<'_>> = self
            .items
            .iter()
            .map(|i| Tab {
                key: TabKey(i.key),
                title: &i.title,
                closable: true,
                progress: i.progress,
            })
            .collect();
        let opts = TabStripOptions {
            active: self.active.map(TabKey),
            panel_focused: self.panel_focused,
            busy: self.busy,
            busy_tone: self.tone,
            card: inner,
            menu_extra: &["Duplicate Tab"],
            name: "Sessions",
            header_tint: Some(HEADER_TINT),
        };
        let events = ui.tab_strip("strip", strip, &tabs, &opts).events;
        for event in events {
            self.apply(event);
        }
        let body = Rect::new(inner.x, inner.y + HEIGHT, inner.w, inner.h - HEIGHT);
        self.body(ui, body);
    }
}

fn three() -> Demo {
    Demo::new(numbered(3), Some(2))
}

fn live() -> Box<dyn PageState> {
    let mut demo = Demo::new(numbered(5), Some(2));
    demo.items[3].progress = Some(Tone::Neutral);
    Box::new(demo)
}

fn rest() -> Box<dyn PageState> {
    Box::new(three())
}

fn unfocused() -> Box<dyn PageState> {
    Box::new(Demo {
        panel_focused: false,
        ..three()
    })
}

fn progress() -> Box<dyn PageState> {
    let mut demo = three();
    demo.items[0].progress = Some(Tone::Neutral);
    demo.items[1].progress = Some(Tone::Err);
    demo.items[2].progress = Some(Tone::Warn);
    Box::new(demo)
}

fn busy(cue: BusyCue) -> Box<dyn PageState> {
    Box::new(Demo {
        busy: Some(cue),
        ..three()
    })
}

fn busy_sweep() -> Box<dyn PageState> {
    busy(BusyCue::Sweep)
}

fn busy_comet() -> Box<dyn PageState> {
    busy(BusyCue::Comet)
}

fn busy_breath() -> Box<dyn PageState> {
    busy(BusyCue::Breath)
}

/// Twelve tabs, the first active: scrolled 105 px, 3 px of it show at the left edge, too few for
/// a notch.
fn overflow() -> Box<dyn PageState> {
    Box::new(Demo::new(numbered(12), Some(1)))
}

fn long_title() -> Box<dyn PageState> {
    let mut items = numbered(3);
    items[1].title =
        "A title far longer than the widest tab the strip will ever draw for it".to_owned();
    Box::new(Demo::new(items, Some(2)))
}

fn empty() -> Box<dyn PageState> {
    Box::new(Demo::new(Vec::new(), None))
}

fn dragged() -> Box<dyn PageState> {
    Box::new(Demo::new(numbered(3), Some(1)))
}

/// Tab 1's press, 40 px into it.
const PRESS: Step = Step {
    at_ms: 0,
    events: &[Ev::PointerAt(LEFT + 40.0, MID_Y), Ev::Press],
};
/// The 4 px move that crosses the threshold.
const CROSS: Step = Step {
    at_ms: 10,
    events: &[Ev::PointerAt(LEFT + 44.0, MID_Y)],
};
/// Tab 1's left edge at 150: nearest slot 1, between tabs 2 and 3.
const FAR: Step = Step {
    at_ms: 20,
    events: &[Ev::PointerAt(LEFT + 44.0 + 150.0, MID_Y)],
};
const RELEASE: Step = Step {
    at_ms: 300,
    events: &[Ev::Release],
};

const HOVER_THIRD: Step = Step {
    at_ms: 0,
    events: &[Ev::PointerAt(tab_mid(2), MID_Y)],
};

const SCENES: &[UiScene] = &[
    UiScene {
        id: "tab-strip-rest",
        scale: 2,
        tier: MotionTier::Full,
        state: rest,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Hovered and settled, with the tooltip's 800 ms still to run.
    UiScene {
        id: "tab-strip-hover",
        scale: 2,
        tier: MotionTier::Full,
        state: rest,
        script: &[HOVER_THIRD],
        capture_ms: 300,
        expect: Expect::Waiting,
    },
    UiScene {
        id: "tab-strip-unfocused",
        scale: 2,
        tier: MotionTier::Full,
        state: unfocused,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Each spinner turned a quarter.
    UiScene {
        id: "tab-strip-progress",
        scale: 2,
        tier: MotionTier::Full,
        state: progress,
        script: &[],
        capture_ms: 300,
        expect: Expect::Moving,
    },
    // Phase 0.25 of the 2400 ms loop.
    UiScene {
        id: "tab-strip-busy-sweep",
        scale: 2,
        tier: MotionTier::Full,
        state: busy_sweep,
        script: &[],
        capture_ms: 600,
        expect: Expect::Moving,
    },
    UiScene {
        id: "tab-strip-busy-comet",
        scale: 2,
        tier: MotionTier::Full,
        state: busy_comet,
        script: &[],
        capture_ms: 600,
        expect: Expect::Moving,
    },
    // The brightest point of the 1800 ms breath.
    UiScene {
        id: "tab-strip-busy-breath",
        scale: 2,
        tier: MotionTier::Full,
        state: busy_breath,
        script: &[],
        capture_ms: 900,
        expect: Expect::Moving,
    },
    UiScene {
        id: "tab-strip-overflow",
        scale: 2,
        tier: MotionTier::Full,
        state: overflow,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(tab_mid(3), MID_Y)],
            },
            Step {
                at_ms: 10,
                events: &[Ev::Scroll {
                    x: 0.0,
                    y: -105.0,
                    precise: true,
                }],
            },
            Step {
                at_ms: 300,
                events: &[Ev::PointerAt(LEFT + 300.0, TOP + 200.0)],
            },
        ],
        capture_ms: 600,
        expect: Expect::Settled,
    },
    UiScene {
        id: "tab-strip-long-title",
        scale: 2,
        tier: MotionTier::Full,
        state: long_title,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "tab-strip-empty",
        scale: 2,
        tier: MotionTier::Full,
        state: empty,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "tab-strip-dragging",
        scale: 2,
        tier: MotionTier::Full,
        state: dragged,
        script: &[PRESS, CROSS, FAR],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Half of `instant`.
    UiScene {
        id: "tab-strip-hover-fade",
        scale: 2,
        tier: MotionTier::Full,
        state: rest,
        script: &[HOVER_THIRD],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // Half of `fast`, from the frame the neighbours started.
    UiScene {
        id: "tab-strip-slide",
        scale: 2,
        tier: MotionTier::Full,
        state: dragged,
        script: &[PRESS, CROSS, FAR],
        capture_ms: 110,
        expect: Expect::Moving,
    },
    // Half of `fast` after the release.
    UiScene {
        id: "tab-strip-settle",
        scale: 2,
        tier: MotionTier::Full,
        state: dragged,
        script: &[PRESS, CROSS, FAR, RELEASE],
        capture_ms: 390,
        expect: Expect::Moving,
    },
    // Half of `base` after the activation.
    UiScene {
        id: "tab-strip-content-swap",
        scale: 2,
        tier: MotionTier::Full,
        state: rest,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(tab_mid(2), MID_Y), Ev::Click],
        }],
        capture_ms: 125,
        expect: Expect::Moving,
    },
];

/// Hover, a click on each end, keys, a drag and a drop.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(tab_mid(2), MID_Y), Ev::Click],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Key(NamedKey::ArrowLeft)],
    },
    Step {
        at_ms: 400,
        events: &[Ev::PointerAt(tab_mid(0), MID_Y), Ev::Press],
    },
    Step {
        at_ms: 420,
        events: &[Ev::PointerAt(tab_mid(0) + 4.0, MID_Y)],
    },
    Step {
        at_ms: 440,
        events: &[Ev::PointerAt(tab_mid(0) + 150.0, MID_Y)],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Release],
    },
];
