//! The `markdown-preview` page (`text.markdown-preview`, D12): the block model drawn in a window.
//! The page plays the app: it owns the model (the toolkit never parses), the picture it resolved
//! and registered, and what a link does (it shows the target it was told to open). The live state
//! and the bench hold a long document; the scenes one block kind each.

use lune_ui::color::Rgba;
use lune_ui::image::{ImageData, ImageId};
use lune_ui::input::{Modifiers, NamedKey};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::markdown::{
    MdAlign, MdBlock, MdImage, MdInline, MdItem, MdOptions, MdRow, MdTask,
};
use lune_ui::widgets::text_lines::HighlightSpan;

use super::{caption, card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "markdown-preview",
    title: "Markdown preview",
    kind: PageKind::Ui(UiPage {
        state: long,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The document, in window coordinates: inside the card, under the heading.
const RECT: Rect = Rect::new(34.0, 90.0, 972.0, 500.0);
/// The document's text starts here: the 12 px margin in.
const TEXT_X: f32 = RECT.x + 12.0;
const TEXT_Y: f32 = RECT.y + 12.0;
const PIC: ImageId = ImageId(801);
/// A picture wider than the 640 px limit.
const PIC_SIZE: (u32, u32) = (800, 400);
const AWAY: Ev = Ev::PointerAt(5.0, 5.0);

const KEYWORD: Rgba = [0x56, 0x9C, 0xD6, 0xFF];
const STRING: Rgba = [0xCE, 0x91, 0x78, 0xFF];

const fn text(s: &'static str) -> MdInline<'static> {
    MdInline::Text {
        text: s,
        bold: false,
        italic: false,
        strike: false,
    }
}

const fn bold(s: &'static str) -> MdInline<'static> {
    MdInline::Text {
        text: s,
        bold: true,
        italic: false,
        strike: false,
    }
}

const fn italic(s: &'static str) -> MdInline<'static> {
    MdInline::Text {
        text: s,
        bold: false,
        italic: true,
        strike: false,
    }
}

const fn strike(s: &'static str) -> MdInline<'static> {
    MdInline::Text {
        text: s,
        bold: false,
        italic: false,
        strike: true,
    }
}

const fn link(label: &'static str, url: &'static str) -> MdInline<'static> {
    MdInline::Link { label, url }
}

const fn para(items: &'static [MdInline<'static>]) -> MdBlock<'static> {
    MdBlock::Paragraph(items)
}

const fn head(level: u8, s: &'static [MdInline<'static>]) -> MdBlock<'static> {
    MdBlock::Heading { level, inlines: s }
}

const fn item(
    marker: &'static str,
    blocks: &'static [MdBlock<'static>],
    task: Option<MdTask>,
) -> MdItem<'static> {
    MdItem {
        marker,
        blocks,
        task,
    }
}

const fn task(checked: bool, id: u64) -> Option<MdTask> {
    Some(MdTask { checked, id })
}

static HEADINGS: [MdBlock<'static>; 7] = [
    head(1, &[text("Heading level one")]),
    head(2, &[text("Heading level two")]),
    head(3, &[text("Heading level three")]),
    head(4, &[text("Heading level four")]),
    head(5, &[text("Heading level five")]),
    head(6, &[text("Heading level six")]),
    para(&[text(
        "Body text follows a heading at 13 px, wrapped to the column.",
    )]),
];

static INLINE: [MdBlock<'static>; 1] = [para(&[
    text("Plain text, "),
    bold("bold"),
    text(", "),
    italic("italic"),
    text(", "),
    strike("struck"),
    text(", "),
    MdInline::Code("inline_code()"),
    text(", an actionable "),
    link("link", "docs/guide.md"),
    text(" and an inert "),
    link("anchor", "#section"),
    text("."),
])];

static LINKS: [MdBlock<'static>; 1] = [para(&[
    text("Read the "),
    link("guide", "docs/guide.md"),
    text(" first, then the "),
    link("reference", "https://example.com/reference"),
    text("."),
])];

static LISTS: [MdBlock<'static>; 3] = [
    MdBlock::List {
        items: &[
            item("\u{2022}", &[para(&[text("A bullet")])], None),
            item(
                "\u{2022}",
                &[
                    para(&[text("A bullet with a nested list")]),
                    MdBlock::List {
                        items: &[
                            item("\u{2022}", &[para(&[text("Nested one")])], None),
                            item("\u{2022}", &[para(&[text("Nested two")])], None),
                        ],
                    },
                ],
                None,
            ),
        ],
    },
    MdBlock::List {
        items: &[
            item("1.", &[para(&[text("First step")])], None),
            item("2.", &[para(&[text("Second step")])], None),
            item("3.", &[para(&[text("Third step")])], None),
        ],
    },
    MdBlock::List {
        items: &[
            item(
                "\u{2022}",
                &[para(&[text("Write the model")])],
                task(true, 1),
            ),
            item(
                "\u{2022}",
                &[para(&[text("Draw the preview")])],
                task(false, 2),
            ),
        ],
    },
];

static QUOTE: [MdBlock<'static>; 1] = [MdBlock::Quote(&[
    para(&[text("A quote holds blocks, spaced six apart.")]),
    MdBlock::Quote(&[para(&[text("And a quote inside it.")])]),
])];

static WIDE_TABLE: [MdBlock<'static>; 1] = [MdBlock::Table {
    rows: &[
        MdRow {
            header: true,
            cells: &[
                &[text("Left column")],
                &[text("Centre column")],
                &[text("Right column")],
            ],
        },
        MdRow {
            header: false,
            cells: &[
                &[text(
                    "The widest cell of this column sets its width, so the table is wider than the view",
                )],
                &[text(
                    "Centred text sits in the middle of an equally wide column of this table",
                )],
                &[text(
                    "Right aligned text ends at the column's right edge in this table too",
                )],
            ],
        },
        MdRow {
            header: false,
            cells: &[&[text("short")], &[text("short")], &[text("short")]],
        },
    ],
    align: &[MdAlign::Left, MdAlign::Center, MdAlign::Right],
}];

static NARROW_TABLE: [MdBlock<'static>; 1] = [MdBlock::Table {
    rows: &[
        MdRow {
            header: true,
            cells: &[&[text("Name")], &[text("Kind")], &[text("Size")]],
        },
        MdRow {
            header: false,
            cells: &[&[text("main.rs")], &[text("source")], &[text("4 KB")]],
        },
        MdRow {
            header: false,
            cells: &[&[text("notes")], &[text("text")], &[text("12 KB")]],
        },
    ],
    align: &[MdAlign::Left, MdAlign::Center, MdAlign::Right],
}];

const fn span(line: usize, start: usize, end: usize, color: Rgba) -> (usize, HighlightSpan) {
    (line, HighlightSpan { start, end, color })
}

static CODE: [MdBlock<'static>; 3] = [
    para(&[text("A fenced block draws through the code block:")]),
    MdBlock::Code {
        text: "fn run(queue: &Queue) -> Result<(), Error> {\n    let item = queue.next()?;\n    println!(\"{item}\");\n    Ok(())\n}",
        spans: &[
            span(0, 0, 2, KEYWORD),
            span(1, 4, 7, KEYWORD),
            span(2, 13, 21, STRING),
        ],
    },
    para(&[text(
        "Its copy button and its sideways scroll are the block's own.",
    )]),
];

static IMAGE: [MdBlock<'static>; 3] = [
    para(&[text("A resolved picture, scaled to 640 wide:")]),
    MdBlock::Image {
        target: "assets/overview.png",
        alt: "Overview",
        image: Some(MdImage {
            id: PIC,
            width: PIC_SIZE.0,
            height: PIC_SIZE.1,
        }),
    },
    para(&[text("The text goes on below it.")]),
];

static IMAGE_REMOTE: [MdBlock<'static>; 1] = [MdBlock::Image {
    target: "https://example.com/logo.png",
    alt: "Logo",
    image: None,
}];

static IMAGE_FAILED: [MdBlock<'static>; 1] = [MdBlock::Image {
    target: "assets/missing.png",
    alt: "The build graph",
    image: None,
}];

static MERMAID: [MdBlock<'static>; 2] = [
    MdBlock::Mermaid {
        source: "graph TD\n  A[Parse] --> B[Draw]\n  B --> C[Copy]",
        rendered: None,
    },
    MdBlock::Math {
        source: "e^{i\\pi} + 1 = 0",
        rendered: None,
    },
];

static HTML: [MdBlock<'static>; 2] = [
    para(&[text("Raw HTML is never rendered:")]),
    MdBlock::Html("<div class=\"note\">raw <b>html</b> is shown as text</div>"),
];

static RULE: [MdBlock<'static>; 3] = [
    para(&[text("Above the rule.")]),
    MdBlock::Rule,
    para(&[text("Below the rule.")]),
];

/// A section of the long document: every block kind once.
fn section(_: usize) -> Vec<MdBlock<'static>> {
    let mut out = vec![head(2, &SECTION), para(&INLINE_TEXT)];
    out.extend_from_slice(&LISTS);
    out.extend_from_slice(&QUOTE);
    out.extend_from_slice(&NARROW_TABLE);
    out.extend_from_slice(&CODE[1..2]);
    out.push(MdBlock::Rule);
    out
}

const SECTION: [MdInline<'static>; 1] = [text("Section")];

const INLINE_TEXT: [MdInline<'static>; 4] = [
    text("Each section repeats the same blocks so the document is long: "),
    bold("bold"),
    text(" and "),
    link("a link", "docs/guide.md"),
];

/// What the page lays out: the model, whether tasks are live, and whether it holds a picture.
struct Live {
    blocks: Vec<MdBlock<'static>>,
    tasks_live: bool,
    pic: bool,
    /// The last link raised, shown above the document.
    opened: Option<String>,
}

fn state(blocks: &[MdBlock<'static>]) -> Live {
    Live {
        blocks: blocks.to_vec(),
        tasks_live: true,
        pic: false,
        opened: None,
    }
}

fn long() -> Box<dyn PageState> {
    let blocks: Vec<MdBlock<'static>> = (0..400).flat_map(section).collect();
    Box::new(Live {
        blocks,
        ..state(&[])
    })
}

macro_rules! scene_state {
    ($name:ident, $blocks:expr) => {
        fn $name() -> Box<dyn PageState> {
            Box::new(state(&$blocks))
        }
    };
}

scene_state!(headings, HEADINGS);
scene_state!(inline, INLINE);
scene_state!(links, LINKS);
scene_state!(lists, LISTS);
scene_state!(quote, QUOTE);
scene_state!(wide_table, WIDE_TABLE);
scene_state!(narrow_table, NARROW_TABLE);
scene_state!(code, CODE);
scene_state!(image_remote, IMAGE_REMOTE);
scene_state!(image_failed, IMAGE_FAILED);
scene_state!(mermaid, MERMAID);
scene_state!(html, HTML);
scene_state!(rule, RULE);

fn image() -> Box<dyn PageState> {
    Box::new(Live {
        pic: true,
        ..state(&IMAGE)
    })
}

fn readonly_tasks() -> Box<dyn PageState> {
    Box::new(Live {
        tasks_live: false,
        ..state(&LISTS[2..3])
    })
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Markdown preview");
        heading(ui, "MARKDOWN PREVIEW", card.x + 24.0, card.y + 24.0);
        if let Some(url) = &self.opened {
            let t = *ui.theme();
            let spec = Spec::plain(
                format!("Opened: {url}"),
                Style::ui(t.font_s, t.fg_secondary),
            );
            ui.painter().text(&spec, card.x + 300.0, card.y + 24.0);
        } else {
            caption(ui, "Press a link to open it", card.x + 300.0, card.y + 24.0);
        }
        let opts = MdOptions {
            label: "Document",
            tasks_live: self.tasks_live,
            identity: Some(1),
        };
        let out = ui.markdown("doc", on_card(card, RECT), &self.blocks, &opts);
        if let Some(url) = out.link {
            self.opened = Some(url);
        }
    }

    fn images(&self) -> Vec<(ImageId, ImageData)> {
        if self.pic {
            vec![(PIC, pattern(PIC_SIZE.0, PIC_SIZE.1))]
        } else {
            Vec::new()
        }
    }
}

/// A `width` by `height` pattern: red rises left to right, green top to bottom, blue checks every
/// 20 px. Opaque.
fn pattern(width: u32, height: u32) -> ImageData {
    let mut rgba = Vec::with_capacity(4 * width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let r = x * 255 / (width - 1).max(1);
            let g = y * 255 / (height - 1).max(1);
            let b = if (x / 20 + y / 20) % 2 == 0 { 255 } else { 0 };
            rgba.extend_from_slice(&[r as u8, g as u8, b, 255]);
        }
    }
    ImageData {
        width,
        height,
        rgba,
    }
}

const fn scene(
    id: &'static str,
    state: fn() -> Box<dyn PageState>,
    script: &'static [Step],
    capture_ms: u32,
) -> UiScene {
    UiScene {
        id,
        scale: 2.0,
        tier: MotionTier::Full,
        state,
        script,
        capture_ms,
        expect: Expect::Settled,
    }
}

const fn key(at_ms: u32, events: &'static [Ev]) -> Step {
    Step { at_ms, events }
}

const NO_MODS: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: false,
};

/// Over the word "guide" of the links scene: after "Read the " at 13 px.
const OVER_GUIDE: Ev = Ev::PointerAt(TEXT_X + 66.0, TEXT_Y + 8.0);

const SCENES: &[UiScene] = &[
    scene("markdown-headings", headings, &[], 600),
    scene("markdown-inline", inline, &[], 600),
    scene("markdown-link-hover", links, &[key(0, &[OVER_GUIDE])], 600),
    // Tab to the first link: its ring.
    scene(
        "markdown-link-focus",
        links,
        &[key(0, &[Ev::Tab, AWAY])],
        600,
    ),
    scene("markdown-lists", lists, &[], 600),
    scene("markdown-quote", quote, &[], 600),
    // Wider than the view, the pointer over it: the thumb.
    scene(
        "markdown-table",
        wide_table,
        &[key(0, &[Ev::PointerAt(TEXT_X + 200.0, TEXT_Y + 40.0)])],
        600,
    ),
    scene("markdown-table-narrow", narrow_table, &[], 600),
    scene("markdown-code", code, &[], 600),
    scene("markdown-image", image, &[], 600),
    scene("markdown-image-remote", image_remote, &[], 600),
    scene("markdown-image-failed", image_failed, &[], 600),
    scene("markdown-mermaid-fallback", mermaid, &[], 600),
    scene("markdown-html-inert", html, &[], 600),
    scene("markdown-rule", rule, &[], 600),
    scene("markdown-readonly-tasks", readonly_tasks, &[], 600),
];

const WHEEL: Ev = Ev::Scroll {
    x: 0.0,
    y: -5.0,
    precise: false,
};
const WHEEL_BACK: Ev = Ev::Scroll {
    x: 0.0,
    y: 5.0,
    precise: false,
};

/// What the bench replays on the long document: wheel steps down and back, page keys, a click on
/// a link and the end of the document.
const BENCH: &[Step] = &[
    key(0, &[Ev::PointerAt(TEXT_X + 200.0, TEXT_Y + 100.0)]),
    key(100, &[WHEEL]),
    key(160, &[WHEEL]),
    key(220, &[WHEEL]),
    key(280, &[WHEEL]),
    key(340, &[WHEEL_BACK]),
    key(500, &[Ev::Click]),
    key(700, &[Ev::Tab, Ev::Mods(NO_MODS)]),
    key(800, &[Ev::Key(NamedKey::PageDown)]),
    key(900, &[Ev::Key(NamedKey::PageDown)]),
    key(1_000, &[Ev::Key(NamedKey::PageUp)]),
    key(1_100, &[Ev::Key(NamedKey::End)]),
    key(1_300, &[Ev::Key(NamedKey::Home)]),
];
