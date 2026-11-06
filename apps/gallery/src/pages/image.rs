//! The `image` page: the `image` primitive's gallery scenes (`core.image`). The pictures are
//! generated here from arithmetic (no bundled file, so no licence row). A scene's state returns the
//! pictures it draws; the runner registers them for the scene and releases them after.

use lune_ui::image::{Fit, ImageAlign, ImageData, ImageId};
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "image",
    title: "Image",
    kind: PageKind::Ui(UiPage {
        state: sheet,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// 200 by 100 test pattern.
const WIDE: ImageId = ImageId(101);
/// 100 by 200 test pattern.
const TALL: ImageId = ImageId(102);
/// Never registered: the `image-missing-id` scene draws it.
const MISSING: ImageId = ImageId(199);
/// Never registered by this page: the device refuses an image this wide, so registering it would
/// fail the dump (`dump_scenes` stops on a refused picture). The scene draws the id unregistered,
/// which is what a refused picture looks like on screen.
const TOO_LARGE: ImageId = ImageId(198);

/// A `width` by `height` pattern: red rises left to right, green top to bottom, blue checks every
/// 10 px. Straight alpha, fully opaque.
fn pattern(width: u32, height: u32) -> ImageData {
    let mut rgba = Vec::with_capacity(4 * width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let r = x * 255 / (width - 1).max(1);
            let g = y * 255 / (height - 1).max(1);
            let b = if (x / 10 + y / 10) % 2 == 0 { 255 } else { 0 };
            rgba.extend_from_slice(&[r as u8, g as u8, b, 255]);
        }
    }
    ImageData {
        width,
        height,
        rgba,
    }
}

#[derive(Debug, Clone, Copy)]
enum Shot {
    Wide,
    Tall,
    AlignLeft,
    Alpha,
    Scale1,
    Missing,
    TooLarge,
}

const SHOTS: [Shot; 7] = [
    Shot::Wide,
    Shot::Tall,
    Shot::AlignLeft,
    Shot::Alpha,
    Shot::Scale1,
    Shot::Missing,
    Shot::TooLarge,
];

fn name(which: Shot) -> &'static str {
    match which {
        Shot::Wide => "contain wide",
        Shot::Tall => "contain tall",
        Shot::AlignLeft => "align left",
        Shot::Alpha => "alpha 0.5",
        Shot::Scale1 => "scale 1",
        Shot::Missing => "missing id",
        Shot::TooLarge => "too large",
    }
}

/// The pictures one shot draws.
fn pictures(which: Shot) -> Vec<(ImageId, ImageData)> {
    match which {
        Shot::Wide | Shot::AlignLeft | Shot::Alpha | Shot::Scale1 => {
            vec![(WIDE, pattern(200, 100))]
        }
        Shot::Tall => vec![(TALL, pattern(100, 200))],
        Shot::Missing | Shot::TooLarge => Vec::new(),
    }
}

/// Draws the picture `id` into `dst` and outlines `dst`, so an empty draw still shows its place.
fn picture(ui: &mut Ui<'_>, id: ImageId, dst: Rect, align: ImageAlign, alpha: f32) {
    let p = ui.painter();
    p.image(dst, id, Fit::Contain, align, alpha);
    p.ring(dst, 0.0, 1.0, THEME.border, false);
}

/// One shot in `area`: its name, then the picture from the area's top.
fn shot(ui: &mut Ui<'_>, which: Shot, area: Rect) {
    caption(ui, name(which), area.x, area.y);
    let top = area.y + 22.0;
    match which {
        Shot::Wide => picture(
            ui,
            WIDE,
            Rect::new(area.x, top, 100.0, 100.0),
            ImageAlign::CENTER,
            1.0,
        ),
        Shot::Tall => picture(
            ui,
            TALL,
            Rect::new(area.x, top, 100.0, 100.0),
            ImageAlign::CENTER,
            1.0,
        ),
        Shot::AlignLeft => picture(
            ui,
            WIDE,
            Rect::new(area.x, top, 300.0, 100.0),
            ImageAlign::LEFT,
            1.0,
        ),
        Shot::Alpha => {
            let dst = Rect::new(area.x, top, 200.0, 100.0);
            ui.painter().fill(dst, 0.0, THEME.accent);
            picture(ui, WIDE, dst, ImageAlign::CENTER, 0.5);
        }
        Shot::Scale1 => picture(
            ui,
            WIDE,
            Rect::new(area.x, top, 200.0, 100.0),
            ImageAlign::CENTER,
            1.0,
        ),
        Shot::Missing => picture(
            ui,
            MISSING,
            Rect::new(area.x, top, 100.0, 100.0),
            ImageAlign::CENTER,
            1.0,
        ),
        Shot::TooLarge => picture(
            ui,
            TOO_LARGE,
            Rect::new(area.x, top, 100.0, 100.0),
            ImageAlign::CENTER,
            1.0,
        ),
    }
}

/// The card's heading, then the content area under it.
fn content(ui: &mut Ui<'_>, stage: &Stage) -> Rect {
    let card = card(ui, stage, "Image");
    let (x, y) = (card.x + 24.0, card.y + 24.0);
    heading(ui, "IMAGE", x, y);
    Rect::new(x, y + 24.0, card.w - 48.0, card.h - 48.0 - 24.0)
}

/// Every shot in a grid of three columns.
struct Sheet;

impl PageState for Sheet {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = content(ui, stage);
        for (i, which) in SHOTS.iter().enumerate() {
            let cell = Rect::new(
                area.x + (i % 3) as f32 * 300.0,
                area.y + (i / 3) as f32 * 170.0,
                300.0,
                170.0,
            );
            shot(ui, *which, cell);
        }
    }

    fn images(&self) -> Vec<(ImageId, ImageData)> {
        vec![(WIDE, pattern(200, 100)), (TALL, pattern(100, 200))]
    }
}

/// One shot filling the content area.
struct Show(Shot);

impl PageState for Show {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = content(ui, stage);
        shot(ui, self.0, area);
    }

    fn images(&self) -> Vec<(ImageId, ImageData)> {
        pictures(self.0)
    }
}

fn sheet() -> Box<dyn PageState> {
    Box::new(Sheet)
}

fn wide() -> Box<dyn PageState> {
    Box::new(Show(Shot::Wide))
}

fn tall() -> Box<dyn PageState> {
    Box::new(Show(Shot::Tall))
}

fn align_left() -> Box<dyn PageState> {
    Box::new(Show(Shot::AlignLeft))
}

fn alpha() -> Box<dyn PageState> {
    Box::new(Show(Shot::Alpha))
}

fn scale_1() -> Box<dyn PageState> {
    Box::new(Show(Shot::Scale1))
}

fn missing() -> Box<dyn PageState> {
    Box::new(Show(Shot::Missing))
}

fn too_large() -> Box<dyn PageState> {
    Box::new(Show(Shot::TooLarge))
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "image-contain-wide",
        scale: 2.0,
        tier: MotionTier::Full,
        state: wide,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "image-contain-tall",
        scale: 2.0,
        tier: MotionTier::Full,
        state: tall,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "image-align-left",
        scale: 2.0,
        tier: MotionTier::Full,
        state: align_left,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "image-alpha",
        scale: 2.0,
        tier: MotionTier::Full,
        state: alpha,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "image-scale-1",
        scale: 1.0,
        tier: MotionTier::Full,
        state: scale_1,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "image-missing-id",
        scale: 2.0,
        tier: MotionTier::Full,
        state: missing,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "image-too-large",
        scale: 2.0,
        tier: MotionTier::Full,
        state: too_large,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];

/// A static page: the bench moves the pointer across it.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[crate::page::Ev::PointerAt(120.0, 120.0)],
    },
    Step {
        at_ms: 200,
        events: &[crate::page::Ev::PointerAt(480.0, 360.0)],
    },
];
