//! The `effects` page: glass, blur, chrome, acrylic and the custom-draw hook, drawn through the
//! Painter API. It also holds the drawing of the bench's cost probes (a fading label grid, a
//! scaling and fading dialog over a blurred stripe page), its bench timelines.

use lune_ui::color::Rgba;
use lune_ui::lunegit_dark::{self, THEME};
use lune_ui::paint::{LayerId, Rect, Xform};
use lune_ui::text::{Spec, Style};
use lune_ui::widgets::dialog::{DialogGlass, DialogHits, DialogSpec, dialog};

use crate::page::{Page, PageKind};
use crate::scenes::{EFFECTS, Hit, P};

pub(crate) static PAGE: Page = Page {
    id: "effects",
    title: "Effects",
    kind: PageKind::Legacy(EFFECTS),
};

/// Probe ramp: 250 ms at 60 Hz.
pub(crate) const RAMP_FRAMES: u64 = 15;

const FILL_TEXT: &str = "the quick brown fox jumps over the lazy dog ";

/// Label `i` of a fade probe: 12 to 40 characters, starting with the cycle number so each cycle
/// shapes text no earlier frame did.
fn label_text(i: usize, cycle: u64) -> String {
    let len = 12 + (i * 7) % 29;
    let mut s = format!("{cycle}-{i} ");
    s.extend(FILL_TEXT.chars().cycle().take(len));
    s.truncate(len);
    s
}

/// `n` base-size labels in a grid of 16 rows under an alpha-only transform that ramps 0 to 1.
pub(crate) fn label_fade(p: &mut P<'_>, n: usize, frame: u64) {
    let (phase, cycle) = (
        (frame % RAMP_FRAMES) as f32 / RAMP_FRAMES as f32,
        frame / RAMP_FRAMES,
    );
    p.on(LayerId::Base);
    let win = p.window;
    p.fill(win, 0.0, [0x1F, 0x1F, 0x1F, 0xFF]);
    p.set_xform(Some(Xform {
        origin: (0.0, 0.0),
        scale: 1.0,
        dx: 0.0,
        dy: 0.0,
        alpha: phase,
    }));
    let rows = 16;
    let cols = n / rows;
    let (cw, ch) = (win.w / cols as f32, win.h / rows as f32);
    for i in 0..n {
        let spec = Spec::plain(
            label_text(i, cycle),
            Style::ui(THEME.font_base, THEME.fg_primary),
        );
        p.text(
            &spec,
            (i % cols) as f32 * cw + 8.0,
            (i / cols) as f32 * ch + 8.0,
        );
    }
    p.set_xform(None);
}

/// The stripes of `blur_contrast_scene` on Base, a window-sized backdrop request, and a dialog
/// scaling 0.96 to 1 and fading 0 to 1 about the window centre (the card's centre) over 250 ms,
/// quad-out, repeating.
pub(crate) fn dialog_blur(p: &mut P<'_>, frame: u64) {
    p.on(LayerId::Base);
    let win = p.window;
    let stripes = (win.w / 16.0).ceil() as usize;
    for i in 0..stripes {
        let c: Rgba = if i % 2 == 0 {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        };
        p.fill(Rect::new(i as f32 * 16.0, 0.0, 16.0, win.h), 0.0, c);
    }
    p.on(LayerId::Popup);
    p.backdrop(win, 0.0);
    let phase = (frame % RAMP_FRAMES) as f32 / RAMP_FRAMES as f32;
    let e = 1.0 - (1.0 - phase) * (1.0 - phase);
    p.set_xform(Some(Xform {
        origin: (win.w / 2.0, win.h / 2.0),
        scale: 0.96 + 0.04 * e,
        dx: 0.0,
        dy: 0.0,
        alpha: e,
    }));
    let spec = DialogSpec {
        title: "Discard changes",
        message: "The document has unsaved changes. Closing it will discard them.",
        extra_h: 0.0,
        confirm: "Discard",
        cancel: Some("Cancel"),
    };
    let glass = DialogGlass {
        panel: lunegit_dark::PANEL.premultiplied(),
        fill: lunegit_dark::WINDOW.premultiplied(),
        noise: true,
    };
    let hits = DialogHits {
        block: Hit::Block,
        confirm: Hit::Confirm,
        cancel: Hit::Cancel,
    };
    dialog(p, &spec, &glass, &hits, |_, _, _, _| {});
    p.set_xform(None);
}
