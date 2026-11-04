//! Anchored placement ([`place`]) and the popover ([`Ui::popover`]): a rect of known size next
//! to an anchor, flipped and shifted to stay inside the window, on the popup layer with light
//! dismiss and the menu's open and close motion (`overlay.popover`).

use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use super::menu::{CLOSE, OPEN, menu_close_anim, menu_open_anim, xform_of};
use super::popup_panel;
use crate::paint::{LayerId, Ops, Painter, Rect};
use crate::ui::{OverlayFrame, OverlayOptions, OverlayPhase, Ui, WidgetId};

/// The side of the anchor a popover prefers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bottom,
    Top,
    Left,
    Right,
}

/// How the popover lines up with the anchor along the other axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceAlign {
    /// Leading edges together (left edges for Bottom and Top, top edges for Left and Right).
    Start,
    Center,
    /// Trailing edges together.
    End,
}

/// What [`place`] needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaceRequest {
    pub anchor: Rect,
    /// The popover's size (width, height).
    pub size: (f32, f32),
    pub side: Side,
    pub align: PlaceAlign,
    /// The space between the anchor and the popover.
    pub gap: f32,
    /// The space kept between the popover and the window's edges.
    pub margin: f32,
    /// Clamp the popover into the window along the axis it is aligned on.
    pub shift: bool,
    pub window: Rect,
}

/// Where [`place`] put the popover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub rect: Rect,
    /// The preferred side overflowed the window, so the popover is on the opposite side.
    pub flipped: bool,
    /// The cross-axis clamp moved the popover.
    pub shifted: bool,
    /// The popover is larger than the window minus both margins on some axis: it keeps its size
    /// and sits on the margin, and the consumer should scroll its content.
    pub overflow: bool,
}

/// Places a popover next to an anchor: on the preferred side with `gap` between them, flipped to
/// the opposite side (and kept inside the margin) when the preferred one overflows the window
/// minus `margin`, then aligned and, with `shift`, clamped into the window along the other axis.
/// The clamp keeps the lower bound when the popover is larger than the window.
pub fn place(req: &PlaceRequest) -> Placement {
    let PlaceRequest {
        anchor: a,
        size: (w, h),
        side,
        align,
        gap,
        margin: m,
        shift,
        window: win,
    } = *req;
    let vertical = matches!(side, Side::Bottom | Side::Top);
    // The two axes as (start of the anchor, its length, the popover's length, the window's start
    // and end), main axis first. The bounds are `start + margin` and `end - margin`, and the
    // upper clamp is `end - len - margin`, in the order the moved code computed them.
    let (main, cross) = if vertical {
        (
            (a.y, a.h, h, win.y, win.bottom()),
            (a.x, a.w, w, win.x, win.right()),
        )
    } else {
        (
            (a.x, a.w, w, win.x, win.right()),
            (a.y, a.h, h, win.y, win.bottom()),
        )
    };
    let (a0, alen, len, start, end) = main;
    let (lo, hi) = (start + m, end - m);
    let after = a0 + alen + gap;
    let before = a0 - gap - len;
    let (main_pos, flipped) = match side {
        Side::Bottom | Side::Right => {
            if after + len > hi {
                (before.max(lo), true)
            } else {
                (after, false)
            }
        }
        Side::Top | Side::Left => {
            if before < lo {
                (after.min(end - len - m).max(lo), true)
            } else {
                (before, false)
            }
        }
    };
    let (a0, alen, len, start, end) = cross;
    let mut cross_pos = match align {
        PlaceAlign::Start => a0,
        PlaceAlign::Center => a0 + (alen - len) / 2.0,
        PlaceAlign::End => a0 + alen - len,
    };
    let mut shifted = false;
    if shift {
        let c = cross_pos.min(end - len - m).max(start + m);
        shifted = c != cross_pos;
        cross_pos = c;
    }
    let rect = if vertical {
        Rect::new(cross_pos, main_pos, w, h)
    } else {
        Rect::new(main_pos, cross_pos, w, h)
    };
    Placement {
        rect,
        flipped,
        shifted,
        overflow: w > win.w - 2.0 * m || h > win.h - 2.0 * m,
    }
}

/// What a popover draws behind its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Surface {
    /// [`popup_panel`]: radius 8, `dropdown_bg`, a 1 px `border` ring.
    #[default]
    Panel,
    /// Nothing: the content draws its own surface.
    None,
}

/// Where a popover goes and how it looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopoverOptions {
    pub anchor: Rect,
    /// The widget focus returns to when the popover closes, if the one focused at open is gone.
    pub anchor_id: Option<WidgetId>,
    pub size: (f32, f32),
    pub side: Side,
    pub align: PlaceAlign,
    pub gap: f32,
    pub margin: f32,
    pub shift: bool,
    pub surface: Surface,
}

impl PopoverOptions {
    /// Below `anchor`, left edges aligned, gap 4, margin 8, shifted, the default surface.
    pub fn new(anchor: Rect, size: (f32, f32)) -> Self {
        Self {
            anchor,
            anchor_id: None,
            size,
            side: Side::Bottom,
            align: PlaceAlign::Start,
            gap: 4.0,
            margin: 8.0,
            shift: true,
            surface: Surface::Panel,
        }
    }

    fn request(&self, window: Rect) -> PlaceRequest {
        PlaceRequest {
            anchor: self.anchor,
            size: self.size,
            side: self.side,
            align: self.align,
            gap: self.gap,
            margin: self.margin,
            shift: self.shift,
            window,
        }
    }
}

impl<'a> Ui<'a> {
    /// A popover on the popup layer: not modal; Escape, a press outside it (which reaches
    /// nothing) and focus loss dismiss it (`OverlayFrame::dismissed`), and focus returns to
    /// `anchor_id` when it closes. `content` draws inside the placed rect, after the surface. Call
    /// it every frame it is open; stop calling to close it: it fades out through an exit painter
    /// replaying what `content` last drew. Opens and closes with the menu's curves.
    pub fn popover<R>(
        &mut self,
        key: impl Hash,
        options: &PopoverOptions,
        content: impl FnOnce(&mut Ui<'a>, Rect) -> R,
    ) -> (OverlayFrame, Placement, R) {
        let window = self.painter().window;
        let placement = place(&options.request(window));
        let rect = placement.rect;
        let origin = (rect.x, rect.y);
        let surface = options.surface;
        let overlay = OverlayOptions {
            layer: LayerId::Popup,
            modal: false,
            dismiss_on_escape: true,
            dismiss_on_outside_press: true,
            dismiss_on_focus_loss: true,
            anchor: options.anchor_id,
            initial_focus: None,
            open: OPEN,
            close: CLOSE,
        };
        // The content runs after the exit painter is handed over, so the painter reads what it
        // drew through this cell.
        let drawn: Rc<RefCell<Ops>> = Rc::default();
        let exit_drawn = Rc::clone(&drawn);
        let exit = move |p: &mut Painter<'_, WidgetId>, progress: f32, transforms: bool| {
            let Some(anim) = menu_close_anim(progress) else {
                return;
            };
            p.push_xform(xform_of(anim, origin, transforms));
            p.replay(&exit_drawn.borrow());
            p.pop_xform();
        };
        let (frame, r) = self.overlay(key, &overlay, exit, |ui, frame| {
            let anim = match frame.phase {
                OverlayPhase::Opening(p) => menu_open_anim(p),
                OverlayPhase::Open => menu_open_anim(1.0),
            };
            ui.painter()
                .push_xform(xform_of(anim, origin, frame.transforms));
            let (r, ops) = ui.record(|ui| {
                if surface == Surface::Panel {
                    let t = *ui.theme();
                    popup_panel(ui.painter(), rect, t.dropdown_bg, t.border);
                }
                content(ui, rect)
            });
            ui.painter().pop_xform();
            *drawn.borrow_mut() = ops;
            r
        });
        (frame, placement, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIN: Rect = Rect::new(0.0, 0.0, 800.0, 600.0);

    fn req(anchor: Rect, size: (f32, f32)) -> PlaceRequest {
        PlaceRequest {
            anchor,
            size,
            side: Side::Bottom,
            align: PlaceAlign::Start,
            gap: 4.0,
            margin: 8.0,
            shift: true,
            window: WIN,
        }
    }

    #[test]
    fn known_answers_from_the_moved_code() {
        let below = place(&req(Rect::new(100.0, 100.0, 200.0, 32.0), (220.0, 120.0)));
        assert_eq!((below.rect.y, below.flipped), (136.0, false));
        let flipped = place(&req(Rect::new(100.0, 500.0, 200.0, 32.0), (220.0, 120.0)));
        assert_eq!((flipped.rect.y, flipped.flipped), (376.0, true));
        let tall = place(&req(Rect::new(100.0, 20.0, 200.0, 32.0), (220.0, 700.0)));
        assert_eq!((tall.rect.y, tall.overflow), (8.0, true));
        let shifted = place(&req(Rect::new(790.0, 100.0, 200.0, 32.0), (220.0, 120.0)));
        assert_eq!((shifted.rect.x, shifted.shifted), (572.0, true));
    }

    #[test]
    fn sides_flip_to_the_opposite_one_and_align_on_the_other_axis() {
        let a = Rect::new(400.0, 300.0, 100.0, 40.0);
        let right = PlaceRequest {
            side: Side::Right,
            align: PlaceAlign::Center,
            ..req(a, (150.0, 80.0))
        };
        let p = place(&right);
        assert_eq!(p.rect, Rect::new(504.0, 280.0, 150.0, 80.0));
        assert!(!p.flipped);
        let near_edge = PlaceRequest {
            anchor: Rect::new(700.0, 300.0, 80.0, 40.0),
            ..right
        };
        let p = place(&near_edge);
        assert_eq!((p.rect.x, p.flipped), (700.0 - 4.0 - 150.0, true));
        let top = PlaceRequest {
            side: Side::Top,
            align: PlaceAlign::End,
            ..req(a, (150.0, 80.0))
        };
        let p = place(&top);
        assert_eq!(p.rect, Rect::new(350.0, 216.0, 150.0, 80.0));
        let top_flips = PlaceRequest {
            anchor: Rect::new(400.0, 30.0, 100.0, 40.0),
            ..top
        };
        let p = place(&top_flips);
        assert_eq!((p.rect.y, p.flipped), (74.0, true));
    }

    #[test]
    fn a_popover_larger_than_the_window_keeps_its_size_on_the_margin() {
        let p = place(&req(Rect::new(100.0, 100.0, 50.0, 30.0), (900.0, 700.0)));
        assert_eq!(
            (p.rect.x, p.rect.y, p.rect.w, p.rect.h),
            (8.0, 8.0, 900.0, 700.0)
        );
        assert!(p.overflow);
    }
}
