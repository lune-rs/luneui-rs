//! The code block (`text.code-block`, D10): the read-only code view in a frame with a copy button.
//!
//! The body is the [code editor](super::code_editor)'s frame, read-only, without a gutter and
//! scrolling only sideways, inside a faint box with a padding. Colours come from spans the app
//! passes by line (D45). The copy button sits in the top-right corner: it puts the whole text on
//! the clipboard and swaps its icon for a check mark for [`CodeBlockOptions::feedback_ms`], with a
//! polite `Status` node while the mark shows.

use std::hash::Hash;
use std::time::Duration;

use super::button::IconButton;
use super::code_editor::{CodeOptions, Look, TEXT_PAD};
use super::text_lines::{HighlightSpan, Highlighter};
use crate::access::{Live, Node, Role};
use crate::code_doc::CodeDoc;
use crate::color::Rgba;
use crate::icon;
use crate::motion::{Easing, Kind, Motion, Reduce, Time, Token};
use crate::paint::Rect;
use crate::ui::{Response, Ui};

/// The copy icon's size.
const ICON: f32 = 13.0;
/// Around the icon: the button is 21 px square.
const BUTTON_PAD: f32 = 4.0;
const BUTTON_RADIUS: f32 = 4.0;
/// Where the button sits from a bordered block's inner edge; an unbordered block puts it at the
/// padding.
const BORDERED_INSET: f32 = 6.0;
/// The ring of a bordered box.
const BORDER: f32 = 1.0;

/// The copy icon crossfading to the check mark and back: opacity, instant, linear; Reduced keeps
/// it.
pub const COPY_FADE: Motion = Motion {
    kind: Kind::Appear,
    token: Token::Instant,
    easing: Easing::Linear,
    reduced: Reduce::Keep,
    reduced_token: None,
};

/// What [`Ui::code_block`] and [`height`] are given besides the text. The defaults are the compact
/// look; the roomy one is `size: 13.0, line: 20.0, pad: (12.0, 10.0), border: true`.
#[derive(Debug, Clone, Copy)]
pub struct CodeBlockOptions<'a> {
    /// The accessible name of the text.
    pub label: &'a str,
    /// Highlight spans by line, 0-based, in chars of the line. Lines that do not exist are
    /// ignored, and so are empty and inverted spans; a span past the line's end is clamped.
    pub spans: &'a [(usize, HighlightSpan)],
    /// The mono size.
    pub size: f32,
    /// The line height.
    pub line: f32,
    /// The padding, horizontal then vertical.
    pub pad: (f32, f32),
    /// A 1 px `border_subtle` ring round the box; the padding then starts inside it.
    pub border: bool,
    /// How long the check mark shows after a copy, in milliseconds of app time.
    pub feedback_ms: u64,
    /// The text is a Tab stop. Off, a press still focuses it and Tab skips it.
    pub tab_stop: bool,
}

impl Default for CodeBlockOptions<'_> {
    fn default() -> Self {
        Self {
            label: "",
            spans: &[],
            size: 11.0,
            line: 14.0,
            pad: (10.0, 10.0),
            border: false,
            feedback_ms: 1_500,
            tab_stop: true,
        }
    }
}

/// What [`Ui::code_block`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlockResponse {
    /// The text's.
    pub response: Response,
    /// The copy button put the whole text on the clipboard this frame.
    pub copied: bool,
}

/// How many lines `text` has: a final terminator opens one more, and an empty text has one.
fn line_count(text: &str) -> usize {
    let b = text.as_bytes();
    let ends = b
        .iter()
        .enumerate()
        .filter(|&(i, &c)| c == b'\n' || (c == b'\r' && b.get(i + 1) != Some(&b'\n')))
        .count();
    ends + 1
}

/// The height of the block for `text`: its lines, the padding and the border. A host sizes the
/// rect with it. `ui` is taken so a later release can measure; today the lines are counted.
pub fn height(_ui: &Ui<'_>, text: &str, opts: &CodeBlockOptions<'_>) -> f32 {
    let edge = if opts.border { BORDER } else { 0.0 };
    line_count(text) as f32 * opts.line + 2.0 * (opts.pad.1 + edge)
}

/// The spans of the options, by line.
struct BlockSpans<'a>(&'a [(usize, HighlightSpan)]);

impl Highlighter for BlockSpans<'_> {
    fn spans(&self, line: usize, _text: &str) -> Vec<HighlightSpan> {
        self.0
            .iter()
            .filter(|(l, _)| *l == line)
            .map(|(_, s)| *s)
            .collect()
    }
}

/// What a block keeps: its document, rebuilt only when the text differs, and when the copy
/// feedback began.
#[derive(Debug, Default)]
struct BlockMem {
    doc: CodeDoc,
    copied_at: Option<Time>,
}

/// `c` with its alpha scaled by `k` (0 to 1).
fn faded(c: Rgba, k: f32) -> Rgba {
    [
        c[0],
        c[1],
        c[2],
        (f32::from(c[3]) * k.clamp(0.0, 1.0)).round() as u8,
    ]
}

impl Ui<'_> {
    /// A code block under `key` over `rect` showing `text`: read-only mono text on a faint box,
    /// scrolling sideways when a line is wider than the block, with a copy button in the corner.
    /// Select, Select all and Copy work on the text as on a [code view](Self::code_editor); the
    /// button copies the whole text. Size `rect` with [`height`]. The vertical wheel always goes
    /// past the block, and the horizontal one only when a line overflows.
    pub fn code_block(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: &str,
        opts: &CodeBlockOptions<'_>,
    ) -> CodeBlockResponse {
        self.scope(key, |ui| ui.block_frame(rect, text, opts))
    }

    fn block_frame(
        &mut self,
        rect: Rect,
        text: &str,
        opts: &CodeBlockOptions<'_>,
    ) -> CodeBlockResponse {
        let t = *self.theme();
        let id = self.scope_id();
        let now = self.time();
        let hold = Duration::from_millis(opts.feedback_ms);

        let mut mem = std::mem::take(self.persistent::<BlockMem>(id));
        if mem.doc.text() != text {
            mem.doc.set_text(text);
            mem.copied_at = None;
        }
        if mem.copied_at.is_some_and(|at| now >= at + hold) {
            mem.copied_at = None;
        }

        // The box, then the text in it: the text starts `pad` from the box's edge, the
        // frame's own text pad being part of it.
        let radius = t.item_radius;
        self.painter().fill(rect, radius, t.surface_input);
        let edge = if opts.border {
            self.painter()
                .ring(rect, radius, BORDER, t.border_subtle, false);
            BORDER
        } else {
            0.0
        };
        let (px, py) = (opts.pad.0 + edge, opts.pad.1 + edge);
        let body = rect.inset((px - TEXT_PAD).max(0.0), py);
        let spans = BlockSpans(opts.spans);
        let code = CodeOptions {
            read_only: true,
            gutter: false,
            label: opts.label,
            highlighter: &spans,
            ..CodeOptions::default()
        };
        let look = Look {
            size: opts.size,
            line: opts.line,
            scroll_y: false,
            skip_tab: !opts.tab_stop,
            ring: Some(rect),
        };
        let text_view = self.scope("body", |ui| {
            ui.code_frame(body, &mut mem.doc, &code, None, &look)
        });

        // The corner button, over the text. Its icon fades out as the check mark fades in.
        let side = ICON + 2.0 * BUTTON_PAD;
        let (ix, iy) = if opts.border {
            (BORDER + BORDERED_INSET, BORDER + BORDERED_INSET)
        } else {
            opts.pad
        };
        let at = Rect::new(rect.right() - ix - side, rect.y + iy, side, side);
        let fade_id = id.with("fade");
        let showing = mem.copied_at.is_some();
        let mut mark = self.animate_f32(fade_id, f32::from(u8::from(showing)), &COPY_FADE);
        let has_text = !mem.doc.text().is_empty();
        let name = self.tr("luneui.code-block.copy");
        let icon_color = faded(t.fg_tertiary, 1.0 - mark);
        let pressed = self.with_enabled(has_text, |ui| {
            let button = IconButton {
                glyph: icon::COPY,
                glyph_size: ICON,
                color: Some(icon_color),
                radius: BUTTON_RADIUS,
                name,
            };
            ui.icon_button("copy", at, &button)
        });
        let copied = has_text && pressed.clicked;
        if copied {
            self.clipboard_write(mem.doc.text().to_owned());
            mem.copied_at = Some(now);
            mark = self.animate_f32(fade_id, 1.0, &COPY_FADE);
        }
        if mark > 0.0 {
            let check = icon::glyph(icon::CHECK, ICON, faded(t.fg_primary, mark));
            self.painter().text_centered(&check, at);
        }
        if let Some(from) = mem.copied_at {
            self.request_repaint_at(from + hold);
            let (status, said) = (self.id("copied"), self.tr("luneui.code-block.copied"));
            let mut node = Node::new(status, Role::Status, said, at);
            node.live = Some(Live::Polite);
            self.node(node);
        }

        *self.persistent::<BlockMem>(id) = mem;
        CodeBlockResponse {
            response: text_view.response,
            copied,
        }
    }
}
