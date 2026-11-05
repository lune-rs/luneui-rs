//! The markdown preview (`text.markdown-preview`, D12): a block model drawn in a window.
//!
//! The toolkit draws the model and interprets none of it: no parse, no HTML, no fetch. An image is
//! a picture the app decoded and registered ([`MdImage`]); a remote target is never asked for. A
//! link raises the app's [`MdResponse::link`] and the app opens it; a task box raises
//! [`MdResponse::task`] and the model keeps its value until the app changes it.
//!
//! The top-level blocks are the virtual rows of a [scroll area](super::scroll_area): heights are
//! estimated until a block is built, then what it measured, and a block above the first in view that
//! changes height moves the area by as much, so what is on screen stays. A block's inside is built
//! whole. One layout function serves measuring and drawing (`Cx::draw`), so the two cannot differ.
//!
//! Inline text is one rich [`Spec`] per block: bold, italic, links and inline code are runs. An
//! inline code run is padded with two U+2009 either side, which draws its pill's padding; the pads
//! are stripped from Copy and from the node by offset. The pills, strike bars and link underlines
//! are fills from the text's row boxes, because the text seam has no decoration of its own.

use std::hash::Hash;
use std::ops::Range;

use super::code_block::{self, CodeBlockOptions};
use super::focus_ring::{self, RingKind};
use super::scroll_area::{Bars, Extent, ScrollOptions};
use super::text_lines::{HighlightSpan, range_boxes};
use super::virtual_list::{Heights, RowHeights};
use crate::access::{Action, Node, Role, State};
use crate::color::{Rgba, premul};
use crate::icon;
use crate::image::{Anchor, Fit, ImageAlign, ImageId};
use crate::input::{CursorIcon, NamedKey};
use crate::paint::Rect;
use crate::text::{Font, Spec, Style};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui, WidgetId};

/// The margin round the document, in px.
pub const MARGIN: f32 = 12.0;
/// Between two blocks.
pub const GAP: f32 = 8.0;
/// The widest a block image is drawn.
pub const IMAGE_MAX: f32 = 640.0;

/// A block's height until it is built.
const ESTIMATE: f32 = 48.0;
/// The body text.
const BODY: f32 = 13.0;
/// The task box.
const TASK: f32 = 15.0;
/// Between a list marker and the item's blocks.
const MARKER_GAP: f32 = 6.0;
/// Between the items of a list, and between the blocks of an item.
const ITEM_GAP: f32 = 4.0;
/// Between the blocks of a quote.
const QUOTE_GAP: f32 = 6.0;
const QUOTE_BAR: f32 = 3.0;
const QUOTE_PAD: (f32, f32) = (10.0, 6.0);
const CELL_PAD: (f32, f32) = (8.0, 4.0);
const CELL_MIN: f32 = 80.0;
const HTML_PAD: f32 = 8.0;
const AREA: &str = "area";
/// The thin space an inline code run is padded with.
const PAD: &str = "\u{2009}\u{2009}";

/// A picture the app decoded and registered under `id`, with its size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MdImage {
    pub id: ImageId,
    pub width: u32,
    pub height: u32,
}

/// A GFM column's text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MdAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Inline content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MdInline<'a> {
    Text {
        text: &'a str,
        bold: bool,
        italic: bool,
        strike: bool,
    },
    /// Inline code: mono, on a pill.
    Code(&'a str),
    /// A link: its label and target. Whether it can be acted on is [`is_actionable`].
    Link { label: &'a str, url: &'a str },
    /// Inline maths, drawn as inline code: no engine ships.
    Math(&'a str),
}

/// A task item's state: the box shows `checked`, and a click reports `id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MdTask {
    pub checked: bool,
    pub id: u64,
}

/// One list item: its marker as shown (a bullet, `1.`), its blocks, and the task state when it has one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdItem<'a> {
    pub marker: &'a str,
    pub blocks: &'a [MdBlock<'a>],
    pub task: Option<MdTask>,
}

/// One table row, with one inline run per cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdRow<'a> {
    pub header: bool,
    pub cells: &'a [&'a [MdInline<'a>]],
}

/// A block of the document.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MdBlock<'a> {
    /// `level` outside 1 to 6 clamps.
    Heading {
        level: u8,
        inlines: &'a [MdInline<'a>],
    },
    Paragraph(&'a [MdInline<'a>]),
    /// Code with highlight spans by 0-based line, as [`CodeBlockOptions::spans`].
    Code {
        text: &'a str,
        spans: &'a [(usize, HighlightSpan)],
    },
    /// A diagram's source and the picture the app drew of it, if it did.
    Mermaid {
        source: &'a str,
        rendered: Option<MdImage>,
    },
    /// Display maths: its source and the picture the app drew of it, if it did.
    Math {
        source: &'a str,
        rendered: Option<MdImage>,
    },
    Quote(&'a [MdBlock<'a>]),
    List {
        items: &'a [MdItem<'a>],
    },
    Table {
        rows: &'a [MdRow<'a>],
        align: &'a [MdAlign],
    },
    /// A picture alone on its line. `image` is what the app resolved `target` to.
    Image {
        target: &'a str,
        alt: &'a str,
        image: Option<MdImage>,
    },
    Rule,
    /// Raw HTML, shown as inert text.
    Html(&'a str),
}

/// What [`Ui::markdown`] is given besides its rect and blocks.
#[derive(Debug, Clone, Copy, Default)]
pub struct MdOptions<'a> {
    /// The accessible name of the document.
    pub label: &'a str,
    /// A click on a task box reports its id. Off, the boxes only show the model.
    pub tasks_live: bool,
    /// The app's "same document" key: a new one starts at the top and forgets the measured heights.
    pub identity: Option<u64>,
}

/// What [`Ui::markdown`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct MdResponse {
    /// The document's scroll region.
    pub response: Response,
    /// The target of the link activated this frame.
    pub link: Option<String>,
    /// The id of the task box clicked this frame, when `tasks_live`.
    pub task: Option<u64>,
}

/// Whether a link can be acted on: an absolute http or https URL, or a relative path. An empty
/// target, an in-page anchor and any other scheme (`mailto:`, `file:`) are inert.
pub fn is_actionable(url: &str) -> bool {
    if url.is_empty() || url.starts_with('#') {
        return false;
    }
    match scheme(url) {
        Some(s) => s.eq_ignore_ascii_case("http") || s.eq_ignore_ascii_case("https"),
        None => true,
    }
}

/// The URI scheme of `url`, if it starts with one.
fn scheme(url: &str) -> Option<&str> {
    let end = url.find(':')?;
    let s = &url[..end];
    let mut chars = s.chars();
    let first = chars.next()?;
    let valid = first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid.then_some(s)
}

/// Whether `target` is one the toolkit never asks the app to load.
fn is_remote(target: &str) -> bool {
    scheme(target)
        .is_some_and(|s| s.eq_ignore_ascii_case("http") || s.eq_ignore_ascii_case("https"))
}

fn heading_size(level: u8) -> f32 {
    [22.0, 18.0, 14.0, 13.0, 13.0, 13.0][usize::from(level) - 1]
}

fn heading_color(t: &Theme, level: u8) -> Rgba {
    match level {
        5 => t.fg_secondary,
        6 => t.fg_tertiary,
        _ => t.fg_primary,
    }
}

/// What a layout pass carries.
struct Cx {
    /// Draw, register and emit nodes. Off, only measure.
    draw: bool,
    tasks_live: bool,
    link: Option<String>,
    task: Option<u64>,
}

/// A link in a built run: chars `range` of the joined text.
#[derive(Debug)]
struct LinkRun {
    range: Range<usize>,
    label: String,
    url: String,
}

/// An inline run built into a spec, with where its marks are.
#[derive(Debug)]
struct Built {
    spec: Spec,
    /// Padding chars, which Copy and the node leave out.
    pads: Vec<Range<usize>>,
    pills: Vec<Range<usize>>,
    strikes: Vec<(Range<usize>, Rgba)>,
    links: Vec<LinkRun>,
    /// Chars so far.
    at: usize,
}

impl Built {
    fn run(&mut self, text: &str, style: Style) -> Range<usize> {
        let start = self.at;
        if !text.is_empty() {
            self.at += text.chars().count();
            self.spec.runs.push((text.to_owned(), style));
        }
        start..self.at
    }

    fn code(&mut self, text: &str, base: Style) {
        let pad = Style {
            font: Font::Ui,
            ..base
        };
        let first = self.run(PAD, pad);
        self.run(text, Style::mono(base.size, base.line, base.color));
        let last = self.run(PAD, pad);
        self.pills.push(first.start..last.end);
        self.pads.push(first);
        self.pads.push(last);
    }
}

/// `items` as one rich spec in `base`, wrapped at `wrap`.
fn build(t: &Theme, items: &[MdInline<'_>], base: Style, wrap: Option<f32>) -> Built {
    let mut b = Built {
        spec: Spec {
            runs: Vec::new(),
            wrap,
        },
        pads: Vec::new(),
        pills: Vec::new(),
        strikes: Vec::new(),
        links: Vec::new(),
        at: 0,
    };
    for item in items {
        match *item {
            MdInline::Text {
                text,
                bold,
                italic,
                strike,
            } => {
                let mut style = base;
                if bold {
                    style = style.bold();
                }
                if italic {
                    style = style.italic();
                }
                let range = b.run(text, style);
                if strike && !range.is_empty() {
                    b.strikes.push((range, style.color));
                }
            }
            MdInline::Code(text) | MdInline::Math(text) => b.code(text, base),
            MdInline::Link { label, url } => {
                let style = Style {
                    color: t.link,
                    ..base
                };
                let range = b.run(label, style);
                if !range.is_empty() {
                    b.links.push(LinkRun {
                        range,
                        label: label.to_owned(),
                        url: url.to_owned(),
                    });
                }
            }
        }
    }
    b
}

/// The text of `items` as the node reads it.
fn plain(items: &[MdInline<'_>]) -> String {
    items
        .iter()
        .map(|i| match *i {
            MdInline::Text { text, .. } | MdInline::Code(text) | MdInline::Math(text) => text,
            MdInline::Link { label, .. } => label,
        })
        .collect()
}

/// A 1 px bar over the text: `r` is a row box of the text at (`x`, `y`).
fn bar(ui: &mut Ui<'_>, x: f32, y: f32, r: Rect, at: f32, color: Rgba) {
    let r = Rect::new(x + r.x, y + r.y + at, r.w, 1.0);
    ui.painter().fill_over(r, 0.0, premul(color));
}

/// Draws a built run at (`x`, `y`) in `w`, `h` tall: the pills under, the text selectable, the
/// strikes and underlines over, then the links on top of the text, so a press on one never starts
/// a selection.
fn draw_built(ui: &mut Ui<'_>, cx: &mut Cx, id: WidgetId, b: &Built, at: Rect, level: Option<u32>) {
    if !cx.draw || b.spec.runs.is_empty() {
        return;
    }
    let t = *ui.theme();
    for pill in &b.pills {
        for r in range_boxes(ui, &b.spec, pill) {
            let r = Rect::new(at.x + r.x, at.y + r.y + 1.0, r.w, (r.h - 2.0).max(0.0));
            ui.painter().fill(r, 4.0, t.code_inline);
        }
    }
    let sel = ui.selectable_padded(id, at, &b.spec, &b.pads);
    if let Some(level) = level {
        ui.patch_node(sel.id, |n| n.level = Some(level));
    }
    for (range, color) in &b.strikes {
        for r in range_boxes(ui, &b.spec, range) {
            bar(ui, at.x, at.y, r, r.h * 0.55, *color);
        }
    }
    for (i, link) in b.links.iter().enumerate() {
        let actionable = is_actionable(&link.url);
        let boxes = range_boxes(ui, &b.spec, &link.range);
        if actionable {
            for r in &boxes {
                bar(ui, at.x, at.y, *r, r.h - 3.0, t.link);
            }
        }
        if actionable {
            link_regions(ui, cx, id.with(("link", i)), link, &boxes, (at.x, at.y));
        }
    }
}

/// Whether a press on a link has raised it already, so its release does not raise it again.
#[derive(Debug, Default)]
struct Raised(bool);

/// The regions of an actionable link, one per row box; the first is the Tab stop.
fn link_regions(
    ui: &mut Ui<'_>,
    cx: &mut Cx,
    id: WidgetId,
    link: &LinkRun,
    boxes: &[Rect],
    origin: (f32, f32),
) {
    for (j, b) in boxes.iter().enumerate() {
        let at = Rect::new(origin.0 + b.x, origin.1 + b.y, b.w, b.h);
        let lid = id.with(j);
        let sense = Sense {
            skip_tab: j > 0,
            ..Sense::CLICK_FOCUS
        };
        let r = ui.interact(lid, at, sense);
        let key = ui.activation_keys(lid, r.enabled, &[NamedKey::Enter, NamedKey::Space]);
        let raised = ui.transient::<Raised>(lid);
        let mut act = key;
        if r.press_started {
            act = true;
            raised.0 = true;
        } else if r.clicked {
            act |= !raised.0;
        }
        if !r.pressed && !r.press_started {
            raised.0 = false;
        }
        if r.enabled && act && cx.link.is_none() {
            cx.link = Some(link.url.clone());
        }
        if r.hovered || r.pressed {
            ui.set_cursor(CursorIcon::Pointer);
        }
        if r.focus_visible {
            let kind = RingKind::Edge {
                radius: 2.0,
                on_accent: false,
            };
            focus_ring::paint(ui.painter(), at, kind, 1.0);
        }
        if j == 0 {
            let mut node = Node::new(lid, Role::Link, &link.label, at);
            node.description = Some(link.url.clone());
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Default);
            ui.node(node);
        }
    }
}

/// Inline `items` in `base` wrapped to `w`, with its top left at (`x`, `y`); returns the height.
#[allow(clippy::too_many_arguments)]
fn inlines(
    ui: &mut Ui<'_>,
    cx: &mut Cx,
    id: WidgetId,
    at: (f32, f32),
    w: f32,
    items: &[MdInline<'_>],
    base: Style,
    level: Option<u32>,
) -> f32 {
    let b = build(ui.theme(), items, base, Some(w));
    let h = ui.painter().measure(&b.spec).1.max(base.line);
    draw_built(ui, cx, id, &b, Rect::new(at.0, at.1, w, h), level);
    h
}

/// A `Group`-like node around what `f` emits, whose height is what `f` returns.
fn grouped(
    ui: &mut Ui<'_>,
    cx: &mut Cx,
    id: WidgetId,
    role: Role,
    name: &str,
    at: Rect,
    f: impl FnOnce(&mut Ui<'_>, &mut Cx) -> f32,
) -> f32 {
    if !cx.draw {
        return f(ui, cx);
    }
    ui.node_container(Node::new(id, role, name, at), |ui| {
        let h = f(ui, cx);
        ui.patch_node(id, |n| n.bounds.h = h);
        h
    })
}

/// The blocks of `blocks` one under another, `gap` apart; returns the height.
fn stack(
    ui: &mut Ui<'_>,
    cx: &mut Cx,
    id: WidgetId,
    blocks: &[MdBlock<'_>],
    (x, mut y): (f32, f32),
    w: f32,
    gap: f32,
) -> f32 {
    let top = y;
    for (i, b) in blocks.iter().enumerate() {
        y += block(ui, cx, id.with(i), b, (x, y), w) + gap;
    }
    (y - top - if blocks.is_empty() { 0.0 } else { gap }).max(0.0)
}

/// A line of small text in `fg_tertiary` wrapped to `w`; returns the height.
fn note(ui: &mut Ui<'_>, cx: &Cx, text: &str, (x, y): (f32, f32), w: f32) -> f32 {
    let t = *ui.theme();
    let spec = Spec::plain(text, Style::ui(t.font_s, t.fg_tertiary)).wrapped(w);
    let h = ui.painter().measure(&spec).1;
    if cx.draw {
        ui.painter().text(&spec, x, y);
    }
    h
}

/// A picture of `img` at most [`IMAGE_MAX`] wide, left or centred in `w`, its aspect kept.
/// `None` when the picture has no size.
#[allow(clippy::too_many_arguments)]
fn picture(
    ui: &mut Ui<'_>,
    cx: &Cx,
    id: WidgetId,
    img: MdImage,
    name: &str,
    centred: bool,
    (x, y): (f32, f32),
    w: f32,
) -> Option<f32> {
    if img.width == 0 || img.height == 0 {
        return None;
    }
    let iw = (img.width as f32).min(IMAGE_MAX).min(w).max(1.0);
    let ih = iw * img.height as f32 / img.width as f32;
    if cx.draw {
        let left = if centred { x + (w - iw) / 2.0 } else { x };
        let dst = Rect::new(left, y, iw, ih);
        let align = ImageAlign {
            x: Anchor::Start,
            y: Anchor::Start,
        };
        ui.painter().image(dst, img.id, Fit::Contain, align, 1.0);
        ui.node(Node::new(id, Role::Image, name, dst));
    }
    Some(ih)
}

/// A placeholder for a picture that is not drawn: a line of small text, and an image node.
fn placeholder(ui: &mut Ui<'_>, cx: &Cx, id: WidgetId, text: &str, at: (f32, f32), w: f32) -> f32 {
    let h = note(ui, cx, text, at, w);
    if cx.draw {
        ui.node(Node::new(
            id,
            Role::Image,
            text,
            Rect::new(at.0, at.1, w, h),
        ));
    }
    h
}

/// A fenced code block.
fn code(
    ui: &mut Ui<'_>,
    cx: &Cx,
    id: WidgetId,
    text: &str,
    spans: &[(usize, HighlightSpan)],
    (x, y): (f32, f32),
    w: f32,
) -> f32 {
    let opts = CodeBlockOptions {
        spans,
        tab_stop: false,
        ..CodeBlockOptions::default()
    };
    let h = code_block::height(ui, text, &opts);
    if cx.draw {
        ui.code_block(id, Rect::new(x, y, w, h), text, &opts);
    }
    h
}

/// Mermaid and display maths: the app's picture, else the source under a failure note.
#[allow(clippy::too_many_arguments)]
fn asset(
    ui: &mut Ui<'_>,
    cx: &Cx,
    id: WidgetId,
    source: &str,
    rendered: Option<MdImage>,
    failed: &str,
    centred: bool,
    at: (f32, f32),
    w: f32,
) -> f32 {
    if let Some(img) = rendered
        && let Some(h) = picture(ui, cx, id, img, source, centred, at, w)
    {
        return h;
    }
    let text = ui.tr(failed);
    let n = note(ui, cx, text, at, w);
    n + QUOTE_GAP
        + code(
            ui,
            cx,
            id.with("source"),
            source,
            &[],
            (at.0, at.1 + n + QUOTE_GAP),
            w,
        )
}

/// One block at (`x`, `y`) in `w`; returns its height.
fn block(
    ui: &mut Ui<'_>,
    cx: &mut Cx,
    id: WidgetId,
    b: &MdBlock<'_>,
    at: (f32, f32),
    w: f32,
) -> f32 {
    let t = *ui.theme();
    let (x, y) = at;
    match *b {
        MdBlock::Heading {
            level,
            inlines: items,
        } => {
            let level = level.clamp(1, 6);
            let style = Style::ui(heading_size(level), heading_color(&t, level)).bold();
            let h = inlines(
                ui,
                cx,
                id,
                (x, y + 8.0),
                w,
                items,
                style,
                Some(u32::from(level)),
            );
            8.0 + h + 4.0
        }
        MdBlock::Paragraph(items) => {
            let style = Style::ui(BODY, t.fg_primary);
            inlines(ui, cx, id, at, w, items, style, None)
        }
        MdBlock::Code { text, spans } => code(ui, cx, id, text, spans, at, w),
        MdBlock::Mermaid { source, rendered } => asset(
            ui,
            cx,
            id,
            source,
            rendered,
            "luneui.markdown.mermaid-failed",
            false,
            at,
            w,
        ),
        MdBlock::Math { source, rendered } => asset(
            ui,
            cx,
            id,
            source,
            rendered,
            "luneui.markdown.math-failed",
            true,
            at,
            w,
        ),
        MdBlock::Quote(blocks) => quote(ui, cx, id, blocks, at, w),
        MdBlock::List { items } => list(ui, cx, id, items, at, w),
        MdBlock::Table { rows, align } => table(ui, cx, id, rows, align, at, w),
        MdBlock::Image { target, alt, image } => {
            let name = if alt.is_empty() { target } else { alt };
            if is_remote(target) {
                let text = format!(
                    "{}: {target}",
                    ui.tr("luneui.markdown.image-remote-blocked")
                );
                return placeholder(ui, cx, id, &text, at, w);
            }
            match image.and_then(|img| picture(ui, cx, id, img, name, false, at, w)) {
                Some(h) => h,
                None => placeholder(ui, cx, id, name, at, w),
            }
        }
        MdBlock::Rule => {
            if cx.draw {
                ui.painter()
                    .fill(Rect::new(x, y + 6.0, w, 1.0), 0.0, t.surface_active);
                let bounds = Rect::new(x, y + 6.0, w, 1.0);
                ui.node(Node::new(id, Role::Separator, "", bounds));
            }
            13.0
        }
        MdBlock::Html(raw) => {
            let style = Style::mono(11.0, 14.0, t.fg_primary);
            let spec = Spec::plain(raw, style).wrapped((w - 2.0 * HTML_PAD).max(1.0));
            let h = ui.painter().measure(&spec).1.max(style.line) + 2.0 * HTML_PAD;
            if cx.draw {
                ui.painter()
                    .fill(Rect::new(x, y, w, h), t.item_radius, t.surface_input);
                let inner = Rect::new(x + HTML_PAD, y + HTML_PAD, w - 2.0 * HTML_PAD, h);
                ui.selectable_padded(id, inner, &spec, &[]);
            }
            h
        }
    }
}

/// A quote: a bar, a fill and the blocks inside, [`QUOTE_GAP`] apart.
fn quote(
    ui: &mut Ui<'_>,
    cx: &mut Cx,
    id: WidgetId,
    blocks: &[MdBlock<'_>],
    (x, y): (f32, f32),
    w: f32,
) -> f32 {
    let t = *ui.theme();
    let inner_x = x + QUOTE_BAR + QUOTE_PAD.0;
    let inner_w = (w - QUOTE_BAR - 2.0 * QUOTE_PAD.0).max(1.0);
    // The fill goes under what it holds, so its height is measured first.
    let drawing = std::mem::replace(&mut cx.draw, false);
    let inside = stack(
        ui,
        cx,
        id,
        blocks,
        (inner_x, y + QUOTE_PAD.1),
        inner_w,
        QUOTE_GAP,
    );
    cx.draw = drawing;
    let h = inside + 2.0 * QUOTE_PAD.1;
    if !cx.draw {
        return h;
    }
    let at = Rect::new(x, y, w, h);
    grouped(ui, cx, id, Role::Group, "", at, |ui, cx| {
        ui.painter()
            .fill_radii(at, [0.0, 4.0, 4.0, 0.0], t.surface_input);
        ui.painter()
            .fill(Rect::new(x, y, QUOTE_BAR, h), 0.0, t.link);
        stack(
            ui,
            cx,
            id,
            blocks,
            (inner_x, y + QUOTE_PAD.1),
            inner_w,
            QUOTE_GAP,
        );
        h
    })
}

/// The text of an item's first paragraph, for its task box's name.
fn first_text(blocks: &[MdBlock<'_>]) -> String {
    blocks
        .iter()
        .find_map(|b| match b {
            MdBlock::Paragraph(items) => Some(plain(items)),
            _ => None,
        })
        .unwrap_or_default()
}

/// The 15 px task box of `look`, at (`x`, `y`).
fn task_box(ui: &mut Ui<'_>, at: Rect, checked: bool, hovered: bool, focus: bool) {
    use super::checkbox::{CheckboxLook, CheckboxState};
    let t = *ui.theme();
    let look = CheckboxLook::of(
        &t,
        CheckboxState {
            checked,
            hovered,
            focus_visible: focus,
        },
    );
    let p = ui.painter();
    if look.hover_fill[3] > 0 {
        p.fill(at, 3.0, look.hover_fill);
    }
    if look.fill[3] > 0 {
        p.fill(at, 3.0, look.fill);
    }
    if look.ring[3] > 0 {
        p.ring(at, 3.0, 1.0, look.ring, false);
    }
    if look.check[3] > 0 {
        p.text_centered(&icon::glyph(icon::CHECK, 11.0, look.check), at);
    }
    focus_ring::paint(p, at, RingKind::Offset { radius: 3.0 }, look.focus);
}

/// A list: a marker column as wide as the widest marker, and each item's blocks beside it.
fn list(
    ui: &mut Ui<'_>,
    cx: &mut Cx,
    id: WidgetId,
    items: &[MdItem<'_>],
    (x, y): (f32, f32),
    w: f32,
) -> f32 {
    let t = *ui.theme();
    let style = Style::ui(BODY, t.fg_secondary);
    let mut column = 0.0f32;
    for item in items {
        let w = if item.task.is_some() {
            TASK
        } else {
            ui.painter().measure(&Spec::plain(item.marker, style)).0
        };
        column = column.max(w);
    }
    let column = column + MARKER_GAP;
    let inner_w = (w - column).max(1.0);
    grouped(
        ui,
        cx,
        id,
        Role::List,
        "",
        Rect::new(x, y, w, 0.0),
        |ui, cx| {
            let mut top = y;
            for (i, item) in items.iter().enumerate() {
                let iid = id.with(i);
                let from = top;
                let row = |ui: &mut Ui<'_>, cx: &mut Cx| {
                    let h = stack(
                        ui,
                        cx,
                        iid,
                        item.blocks,
                        (x + column, from),
                        inner_w,
                        ITEM_GAP,
                    );
                    if cx.draw {
                        marker(ui, cx, iid, item, style, (x, from));
                    }
                    h.max(style.line)
                };
                let at = Rect::new(x, from, w, 0.0);
                top += grouped(ui, cx, iid, Role::ListItem, "", at, row) + ITEM_GAP;
            }
            (top - y - if items.is_empty() { 0.0 } else { ITEM_GAP }).max(0.0)
        },
    )
}

/// An item's marker: a task box or the text.
fn marker(
    ui: &mut Ui<'_>,
    cx: &mut Cx,
    id: WidgetId,
    item: &MdItem<'_>,
    style: Style,
    (x, y): (f32, f32),
) {
    let Some(task) = item.task else {
        ui.painter().text(&Spec::plain(item.marker, style), x, y);
        return;
    };
    let line = style.line;
    let at = Rect::new(x, y + (line - TASK) / 2.0, TASK, TASK);
    let bid = id.with("task");
    let r = ui.interact(
        bid,
        Rect::new(x, y, TASK + MARKER_GAP, line),
        Sense::CLICK_FOCUS,
    );
    let key = ui.activation_keys(bid, r.enabled, &[NamedKey::Space]);
    if r.enabled && (r.clicked || key) && cx.tasks_live && cx.task.is_none() {
        cx.task = Some(task.id);
    }
    task_box(ui, at, task.checked, r.hovered, r.focus_visible);
    let mut node = Node::new(bid, Role::Checkbox, &first_text(item.blocks), at);
    if task.checked {
        node.states.insert(State::Checked);
    }
    node.states.insert(State::Focusable);
    node.actions.insert(Action::Default);
    ui.node(node);
}

/// A table: a border, columns as wide as their widest cell (80 at least), a header fill and a rule
/// under each row, scrolling sideways alone.
fn table(
    ui: &mut Ui<'_>,
    cx: &mut Cx,
    id: WidgetId,
    rows: &[MdRow<'_>],
    align: &[MdAlign],
    (x, y): (f32, f32),
    w: f32,
) -> f32 {
    let t = *ui.theme();
    let base = Style::ui(BODY, t.fg_primary);
    let cols = align
        .len()
        .max(rows.iter().map(|r| r.cells.len()).max().unwrap_or(0));
    let mut widths = vec![CELL_MIN; cols];
    let mut heights = Vec::with_capacity(rows.len());
    let mut built: Vec<Vec<(Built, f32)>> = Vec::with_capacity(rows.len());
    for row in rows {
        let (mut line, mut cells) = (base.line, Vec::new());
        for (c, cell) in row.cells.iter().take(cols).enumerate() {
            let b = build(&t, cell, base, None);
            let (tw, th) = ui.painter().measure(&b.spec);
            widths[c] = widths[c].max(tw + 2.0 * CELL_PAD.0);
            line = line.max(th);
            cells.push((b, tw));
        }
        heights.push(line + 2.0 * CELL_PAD.1 + 1.0);
        built.push(cells);
    }
    let rows_h: f32 = heights.iter().sum();
    let total = rows_h + 2.0;
    if !cx.draw || rows.is_empty() || cols == 0 {
        return if rows.is_empty() || cols == 0 {
            2.0
        } else {
            total
        };
    }
    let inner = Rect::new(x + 1.0, y + 1.0, (w - 2.0).max(1.0), rows_h);
    let sum: f32 = widths.iter().sum();
    let extent = f64::from(sum.max(inner.w));
    let scroll = ScrollOptions {
        bars: (
            Bars::Auto.unless_fits(f64::from(sum), f64::from(inner.w)),
            Bars::Disabled,
        ),
        extent: Some((extent, f64::from(rows_h))),
        ..ScrollOptions::default()
    };
    grouped(
        ui,
        cx,
        id,
        Role::Table,
        "",
        Rect::new(x, y, w, total),
        |ui, cx| {
            ui.scroll_area(id.with("x"), inner, &scroll, |ui, view| {
                let vp = view.viewport;
                let mut top = vp.y;
                for (r, row) in rows.iter().enumerate() {
                    let rh = heights[r];
                    let rid = id.with(("row", r));
                    let at = Rect::new(vp.x, top, extent as f32, rh);
                    let role = if row.header {
                        Role::ColumnHeader
                    } else {
                        Role::Cell
                    };
                    grouped(ui, cx, rid, Role::Row, "", at, |ui, cx| {
                        if row.header {
                            ui.painter().fill(at, 0.0, t.surface_hover);
                        }
                        let rule = Rect::new(vp.x, top + rh - 1.0, extent as f32, 1.0);
                        ui.painter().fill(rule, 0.0, t.surface_active);
                        let mut left = vp.x;
                        for (c, (b, tw)) in built[r].iter().enumerate() {
                            let cw = widths[c];
                            let slack = (cw - 2.0 * CELL_PAD.0 - tw).max(0.0);
                            let shift = match align.get(c).copied().unwrap_or_default() {
                                MdAlign::Left => 0.0,
                                MdAlign::Center => slack / 2.0,
                                MdAlign::Right => slack,
                            };
                            let cell = Rect::new(left, top, cw, rh);
                            let name = plain(row.cells[c]);
                            let cid = rid.with(c);
                            grouped(ui, cx, cid, role, &name, cell, |ui, cx| {
                                let h = rh - 2.0 * CELL_PAD.1 - 1.0;
                                let text =
                                    Rect::new(left + CELL_PAD.0 + shift, top + CELL_PAD.1, *tw, h);
                                draw_built(ui, cx, cid.with("text"), b, text, None);
                                rh
                            });
                            left += cw;
                        }
                        rh
                    });
                    top += rh;
                }
                ((), Extent::new(extent, f64::from(rows_h)))
            });
            ui.painter()
                .ring(Rect::new(x, y, w, total), 4.0, 1.0, t.surface_active, false);
            total
        },
    )
}

/// What a document keeps.
#[derive(Debug, Default)]
struct MdMem {
    heights: RowHeights,
    identity: Option<u64>,
    /// The width the heights were measured at.
    width: u32,
}

/// Heights changing while a frame measures them.
struct Tops {
    shift: f64,
    relaid: bool,
}

impl Tops {
    /// Records that row `i` measured `h` (with its gap); a row above `first` that changed height
    /// shifts the area by as much.
    fn note(&mut self, heights: &mut RowHeights, i: usize, first: usize, h: f32) {
        let change = heights.set(i, h + GAP);
        if change != 0.0 {
            self.relaid = true;
            if i < first {
                self.shift += change;
            }
        }
    }
}

impl Ui<'_> {
    /// A document under `key` over `rect`: the `blocks`, drawn in a scroll area. Only the blocks in
    /// view are built, so its frame costs the same at any block count. A link activated this frame
    /// is in [`MdResponse::link`] and a task box clicked, when `opts.tasks_live`, in
    /// [`MdResponse::task`]; the document opens nothing and changes nothing. The vertical wheel
    /// goes to the area around a table or code block that scrolls only sideways.
    pub fn markdown(
        &mut self,
        key: impl Hash,
        rect: Rect,
        blocks: &[MdBlock<'_>],
        opts: &MdOptions<'_>,
    ) -> MdResponse {
        self.scope(key, |ui| ui.md_frame(rect, blocks, opts))
    }

    fn md_frame(&mut self, rect: Rect, blocks: &[MdBlock<'_>], opts: &MdOptions<'_>) -> MdResponse {
        let id = self.scope_id();
        let usable = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
        let vp = Rect::new(rect.x, rect.y, usable(rect.w), usable(rect.h));
        let mut mem = std::mem::take(self.persistent::<MdMem>(id));
        let width = vp.w.to_bits();
        if mem.identity != opts.identity || mem.width != width {
            mem = MdMem {
                identity: opts.identity,
                width,
                ..MdMem::default()
            };
        }
        mem.heights
            .sync(blocks.len(), Heights::Measured { estimate: ESTIMATE });
        let content_w = (vp.w - 2.0 * MARGIN).max(1.0);
        let extent = (
            f64::from(vp.w),
            (mem.heights.total() + f64::from(2.0 * MARGIN - GAP)).max(0.0),
        );
        let scroll = ScrollOptions {
            bars: (
                Bars::Disabled,
                Bars::Auto.unless_fits(extent.1, f64::from(vp.h)),
            ),
            extent: Some(extent),
            identity: opts.identity,
            ..ScrollOptions::default()
        };
        let mut cx = Cx {
            draw: true,
            tasks_live: opts.tasks_live,
            link: None,
            task: None,
        };
        let mut tops = Tops {
            shift: 0.0,
            relaid: false,
        };
        let doc = Node::new(id, Role::Group, opts.label, vp);
        let out = self.node_container(doc, |ui| {
            ui.scroll_area_untranslated(AREA, vp, &scroll, |ui, view| {
                let offset = view.offset.1;
                let heights = &mut mem.heights;
                let near = (offset - f64::from(MARGIN)).max(0.0);
                let first = heights.row_at(near);
                let bottom = offset + f64::from(vp.h);
                let mut top = f64::from(MARGIN) + heights.top(first);
                let mut i = first;
                ui.painter().push_clip(vp);
                while i < blocks.len() && top < bottom {
                    let y = vp.y + (top - offset) as f32;
                    let h = block(
                        ui,
                        &mut cx,
                        id.with(i),
                        &blocks[i],
                        (vp.x + MARGIN, y),
                        content_w,
                    );
                    tops.note(heights, i, first, h);
                    top += heights.height(i);
                    i += 1;
                }
                ui.painter().pop_clip();
                // The blocks just outside are measured and not drawn, so one that comes into view
                // from above has shifted the area already.
                let drawing = std::mem::replace(&mut cx.draw, false);
                for n in heights.window(near, f64::from(vp.h)) {
                    if n < first || n >= i {
                        let h = block(ui, &mut cx, id.with(n), &blocks[n], (0.0, 0.0), content_w);
                        tops.note(heights, n, first, h);
                    }
                }
                cx.draw = drawing;
                if tops.shift != 0.0 {
                    ui.scroll_shift(AREA, std::mem::take(&mut tops.shift));
                }
                ((), Extent::default())
            })
        });
        if tops.relaid {
            self.request_catch_up();
        }
        *self.persistent::<MdMem>(id) = mem;
        MdResponse {
            response: out.response,
            link: cx.link,
            task: cx.task,
        }
    }
}
