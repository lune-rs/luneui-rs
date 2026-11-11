//! The `banner` page: the warning banner with its spinner (a switch starts and stops it) and the
//! empty view with its picture and two actions. The picture is a generated disc: the toolkit
//! bundles none and the app registers its own.

use lune_ui::image::{ImageData, ImageId};
use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::banner::EmptyState;

use super::{card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "banner",
    title: "Banner and empty state",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The empty view's picture; ids are unique across the gallery.
const LOGO: ImageId = ImageId(601);

/// A 100 by 100 disc, bright at the top left and fading out to the edge, straight alpha.
fn logo() -> ImageData {
    let size = 100u32;
    let mut rgba = Vec::with_capacity(4 * (size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f32 - 49.5, y as f32 - 49.5);
            let d = (dx * dx + dy * dy).sqrt();
            let a = ((50.0 - d).clamp(0.0, 1.0) * 255.0) as u8;
            let k = 1.0 - (x + y) as f32 / (2.0 * size as f32);
            let v = (90.0 + 120.0 * k) as u8;
            rgba.extend_from_slice(&[v, v.saturating_add(8), v.saturating_add(16), a]);
        }
    }
    ImageData {
        width: size,
        height: size,
        rgba,
    }
}

const INDEXING: &str = "Indexing the repository: history, branches and file changes are being read, and some views may be incomplete until it finishes.";
const SUBTITLE: &str =
    "Open a folder to begin, or create a new repository to start tracking your work.";
const ACTIONS: [&str; 2] = ["Open folder", "Create repository"];

fn live() -> Box<dyn PageState> {
    Box::new(Live::of(true, true, true, false))
}

/// The banner alone, its spinner running.
fn banner_only() -> Box<dyn PageState> {
    Box::new(Live::of(true, true, false, false))
}

/// The empty view alone: nothing moves.
fn view_only() -> Box<dyn PageState> {
    Box::new(Live::of(false, false, true, false))
}

/// The empty view with its actions disabled.
fn view_disabled() -> Box<dyn PageState> {
    Box::new(Live::of(false, false, true, true))
}

/// The banner and the empty view. The knobs say which show, the texts, the picture and whether
/// the actions are enabled.
struct Live {
    busy: bool,
    banner: bool,
    view: bool,
    disabled: bool,
    banner_text: String,
    title: String,
    subtitle: String,
    picture: bool,
}

impl Live {
    fn of(busy: bool, banner: bool, view: bool, disabled: bool) -> Self {
        Self {
            busy,
            banner,
            view,
            disabled,
            banner_text: INDEXING.to_owned(),
            title: "No repository open".to_owned(),
            subtitle: SUBTITLE.to_owned(),
            picture: true,
        }
    }
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Banner and empty state");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "BANNER", x, y);
        let mut bottom = y + 24.0;
        if self.banner {
            let h = ui.banner(
                "banner",
                Rect::new(x, bottom, (card.w - 48.0).min(640.0), 0.0),
                &self.banner_text,
                self.busy,
            );
            bottom += h + 12.0;
        }
        if self.banner {
            ui.toggle(
                "busy",
                Rect::new(x, bottom, 40.0, 24.0),
                &mut self.busy,
                "Busy",
            );
            let label = Spec::plain("Busy", Style::ui(THEME.font_base, THEME.fg_secondary));
            ui.painter().text(&label, x + 52.0, bottom + 3.0);
            bottom += 24.0 + 24.0;
        }
        if self.view {
            heading(ui, "EMPTY STATE", x, bottom);
            let area = Rect::new(
                card.x,
                bottom + 16.0,
                card.w,
                card.y + card.h - bottom - 16.0,
            );
            ui.with_enabled(!self.disabled, |ui| {
                ui.empty_state(
                    "empty",
                    area,
                    &EmptyState {
                        image: self.picture.then_some(LOGO),
                        title: &self.title,
                        subtitle: &self.subtitle,
                        actions: &ACTIONS,
                    },
                )
            });
        }
    }

    fn images(&self) -> Vec<(ImageId, ImageData)> {
        vec![(LOGO, logo())]
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.toggle(ui, "Banner", &mut self.banner);
        panel.text(ui, "Banner text", &mut self.banner_text);
        panel.toggle(ui, "Empty state", &mut self.view);
        panel.text(ui, "Title", &mut self.title);
        panel.text(ui, "Subtitle", &mut self.subtitle);
        panel.toggle(ui, "Picture", &mut self.picture);
        let mut enabled = !self.disabled;
        if panel.toggle(ui, "Actions enabled", &mut enabled) {
            self.disabled = !enabled;
        }
    }
}

const BUSY: &[&str] = &["busy"];
const OPEN: &[&str] = &["empty", "action-0"];
const CREATE: &[&str] = &["empty", "action-1"];

const SCENES: &[UiScene] = &[
    // The spinner runs from the first frame: a quarter turn at 300 ms.
    UiScene {
        id: "banner-shown",
        scale: 2.0,
        tier: MotionTier::Full,
        state: banner_only,
        script: &[],
        capture_ms: 300,
        expect: Expect::Waiting,
    },
    UiScene {
        id: "banner-empty-state",
        scale: 2.0,
        tier: MotionTier::Full,
        state: view_only,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The view is disabled (0.4 opacity, inert) and the pointer rests on an action: no hover fill.
    UiScene {
        id: "banner-empty-state-disabled",
        scale: 2.0,
        tier: MotionTier::Full,
        state: view_disabled,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(OPEN)],
        }],
        capture_ms: 400,
        expect: Expect::Settled,
    },
    // Press is `fast` (180 ms): half way at 90 ms.
    UiScene {
        id: "banner-empty-state-action-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: view_only,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(OPEN), Ev::Press],
        }],
        capture_ms: 90,
        expect: Expect::Moving,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "banner-empty-state-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: view_only,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(CREATE)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
];

/// What the bench replays: hover and click an action, then stop and start the banner's spinner.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(OPEN)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 400,
        events: &[Ev::PointerOver(BUSY), Ev::Click],
    },
    Step {
        at_ms: 800,
        events: &[Ev::Key(NamedKey::Space)],
    },
];
