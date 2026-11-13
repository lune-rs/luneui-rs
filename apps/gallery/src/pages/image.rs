//! The `image` page: the `image` primitive's gallery scenes (`core.image`). The pictures are
//! generated here from arithmetic (no bundled file, so no licence row). A scene's state returns the
//! pictures it draws; the runner registers them for the scene and releases them after.

use lune_ui::image::{Fit, ImageAlign, ImageData, ImageDraw, ImageId, ImagePart, Sampling};
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;

use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Expect, Page, PageKind, PageState, Step, UiPage, UiScene};
use crate::showcase::knobs::Knobs;

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
/// The 32 by 32 sprite sheet of [`sprite_sheet`], drawn by part.
const SPRITES: ImageId = ImageId(103);
/// The 16 by 16 checker of [`checker`], drawn at each sampling mode.
const CHECKER: ImageId = ImageId(104);
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

/// A 32 by 32 sheet of four 16 by 16 sprites, top left first: each has its own base colour, and
/// every other pixel is darker, so a draw that samples nearest or linear reads differently.
fn sprite_sheet() -> ImageData {
    const BASE: [[u32; 3]; 4] = [[220, 70, 70], [70, 190, 90], [70, 110, 230], [230, 180, 60]];
    let mut rgba = Vec::with_capacity(4 * 32 * 32);
    for y in 0..32u32 {
        for x in 0..32u32 {
            let base = BASE[((y / 16) * 2 + x / 16) as usize];
            let dark = (x + y) % 2 == 1;
            for c in base {
                let v = if dark { c * 3 / 5 } else { c };
                rgba.push(v as u8);
            }
            rgba.push(255);
        }
    }
    ImageData {
        width: 32,
        height: 32,
        rgba,
    }
}

/// The sprite at `i` (0 to 3) in [`sprite_sheet`]: a 16 by 16 part, top left first.
fn sprite(i: u32) -> ImagePart {
    ImagePart {
        x: (i % 2) * 16,
        y: (i / 2) * 16,
        w: 16,
        h: 16,
    }
}

/// A 16 by 16 checker of two strong colours, one pixel a cell: linear filtering blurs it and
/// nearest keeps it crisp.
fn checker() -> ImageData {
    let mut rgba = Vec::with_capacity(4 * 16 * 16);
    for y in 0..16u32 {
        for x in 0..16u32 {
            let px: [u8; 3] = if (x + y) % 2 == 0 {
                [230, 60, 60]
            } else {
                [40, 60, 230]
            };
            rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
        }
    }
    ImageData {
        width: 16,
        height: 16,
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
    PartSprite,
    PartNearest,
    Modes,
}

/// The sheet's grid. The part and sampling shots are not in it, so the sheet keeps its seven.
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
        Shot::PartSprite => "part sprite",
        Shot::PartNearest => "part nearest",
        Shot::Modes => "sampling",
    }
}

/// The pictures one shot draws.
fn pictures(which: Shot) -> Vec<(ImageId, ImageData)> {
    match which {
        Shot::Wide | Shot::AlignLeft | Shot::Alpha | Shot::Scale1 => {
            vec![(WIDE, pattern(200, 100))]
        }
        Shot::Tall => vec![(TALL, pattern(100, 200))],
        Shot::PartSprite | Shot::PartNearest => vec![(SPRITES, sprite_sheet())],
        Shot::Modes => vec![(CHECKER, checker())],
        Shot::Missing | Shot::TooLarge => Vec::new(),
    }
}

/// Draws the picture `id` into `dst` and outlines `dst`, so an empty draw still shows its place.
fn picture(ui: &mut Ui<'_>, id: ImageId, dst: Rect, align: ImageAlign, alpha: f32) {
    let p = ui.painter();
    p.image(dst, id, Fit::Contain, align, alpha);
    p.ring(dst, 0.0, 1.0, THEME.border, false);
}

/// Draws `id` as `draw` says into `dst` and outlines `dst`, like [`picture`].
fn drawn(ui: &mut Ui<'_>, id: ImageId, dst: Rect, draw: &ImageDraw) {
    let p = ui.painter();
    p.image_with(dst, id, draw);
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
        Shot::PartSprite => {
            // Four sprites, 2 logical px per texel: a whole ratio, so Auto samples nearest.
            for i in 0..4 {
                let dst = Rect::new(area.x + i as f32 * 40.0, top, 32.0, 32.0);
                let draw = ImageDraw {
                    part: Some(sprite(i)),
                    ..ImageDraw::default()
                };
                drawn(ui, SPRITES, dst, &draw);
            }
        }
        Shot::PartNearest => {
            // Same part, same 2.5 logical px per texel: a fractional ratio, so Auto is linear.
            let part = Some(sprite(0));
            let auto = Rect::new(area.x, top, 40.0, 40.0);
            let nearest = Rect::new(area.x + 60.0, top, 40.0, 40.0);
            let draw = ImageDraw {
                part,
                ..ImageDraw::default()
            };
            drawn(ui, SPRITES, auto, &draw);
            let draw = ImageDraw {
                part,
                sampling: Sampling::Nearest,
                ..ImageDraw::default()
            };
            drawn(ui, SPRITES, nearest, &draw);
            caption(ui, "auto", auto.x, top + 44.0);
            caption(ui, "nearest", nearest.x, top + 44.0);
        }
        Shot::Modes => {
            // Three draws at a ratio of 3 device px per texel (1.5 logical px at scale 2).
            let modes = [
                (Sampling::Auto, "auto"),
                (Sampling::Nearest, "nearest"),
                (Sampling::Linear, "linear"),
            ];
            for (i, (sampling, label)) in modes.into_iter().enumerate() {
                let x = area.x + i as f32 * 56.0;
                let draw = ImageDraw {
                    sampling,
                    ..ImageDraw::default()
                };
                drawn(ui, CHECKER, Rect::new(x, top, 24.0, 24.0), &draw);
                caption(ui, label, x, top + 28.0);
            }
        }
    }
}

/// The card's heading, then the content area under it.
fn content(ui: &mut Ui<'_>, stage: &Stage) -> Rect {
    let card = card(ui, stage, "Image");
    let (x, y) = (card.x + 24.0, card.y + 24.0);
    heading(ui, "IMAGE", x, y);
    Rect::new(x, y + 24.0, card.w - 48.0, card.h - 48.0 - 24.0)
}

/// Every shot in a grid of three columns, or the one the knob picks, filling the area.
#[derive(Default)]
struct Sheet {
    /// 0 shows every shot; n shows `SHOTS[n - 1]`.
    pick: usize,
}

impl PageState for Sheet {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = content(ui, stage);
        if let Some(which) = self.pick.checked_sub(1).and_then(|i| SHOTS.get(i)) {
            shot(ui, *which, area);
            return;
        }
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

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let mut names = vec!["All"];
        names.extend(SHOTS.iter().map(|s| name(*s)));
        panel.choice(ui, "Show", &names, &mut self.pick);
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
    Box::new(Sheet::default())
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

fn part_sprite() -> Box<dyn PageState> {
    Box::new(Show(Shot::PartSprite))
}

fn part_nearest() -> Box<dyn PageState> {
    Box::new(Show(Shot::PartNearest))
}

fn modes() -> Box<dyn PageState> {
    Box::new(Show(Shot::Modes))
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
    UiScene {
        id: "image-part-sprite",
        scale: 2.0,
        tier: MotionTier::Full,
        state: part_sprite,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "image-part-nearest",
        scale: 1.5,
        tier: MotionTier::Full,
        state: part_nearest,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "image-sampling",
        scale: 2.0,
        tier: MotionTier::Full,
        state: modes,
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
