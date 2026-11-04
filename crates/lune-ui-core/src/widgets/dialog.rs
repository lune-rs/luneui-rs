//! The house look's dialog: dim backdrop, 448 px acrylic card, 24 px padding, darker footer block.
//!
//! The drawing is split into the scrim, the card (acrylic, ring, text), the app's body and the
//! footer, so one set of paint functions serves three callers: [`dialog`], the Painter-API
//! function (everything at rest, no transform), [`Ui::dialog`] (the same on the overlay machinery,
//! with the open fade and scale), and the exit painter of a closing dialog (D8).

use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use super::button::{self, ButtonKind, ButtonLook, ButtonState};
use super::{button as button_fn, button_width};
use crate::access::{Action, Node, Role, State};
use crate::color::{over, premul};
use crate::motion::{Easing, Kind, Motion, Reduce, Token};
use crate::paint::{LayerId, Ops, Painter, Rect, Xform};
use crate::text::{Spec, Style};
use crate::ui::{OverlayOptions, OverlayPhase, Sense, Ui, WidgetId};

/// What the dialog says.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogSpec<'s> {
    pub title: &'s str,
    pub message: &'s str,
    /// Height of the body the app draws between the message and the footer.
    pub extra_h: f32,
    pub confirm: &'s str,
    /// The cancel button's label; `None` leaves only the confirm button.
    pub cancel: Option<&'s str>,
}

/// Acrylic colours for the card, from the app's glass policy (premultiplied).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogGlass {
    /// What the panel layer shows behind the dim backdrop (feeds the card's antialiased edge).
    pub panel: [f32; 4],
    /// The card's own fill.
    pub fill: [f32; 4],
    pub noise: bool,
}

/// Hit regions of a dialog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogHits<H> {
    /// The backdrop and the card, so a click does not reach what is below.
    pub block: H,
    pub confirm: H,
    pub cancel: H,
}

/// The card's width.
const CARD_W: f32 = 448.0;
/// The card's scale at the start of the open and the end of the close.
const SCALE_FROM: f32 = 0.96;
/// What the card's scale gains from `SCALE_FROM` to rest.
const SCALE_SPAN: f32 = 0.04;

/// Open: pop, base, quad-out; Reduced fades (no scale) at fast.
pub const OPEN: Motion = Motion::of(Kind::Pop);
/// Close: dismiss, fast, quad-out both ways (the house look ships quad-out);
/// Reduced fades at the mapped token.
pub const CLOSE: Motion = Motion {
    kind: Kind::Dismiss,
    token: Token::Fast,
    easing: Easing::QuadOut,
    reduced: Reduce::Fade,
    reduced_token: None,
};

/// Where everything of one dialog sits, measured once.
#[derive(Debug, Clone)]
struct Layout {
    title: Spec,
    message: Spec,
    card: Rect,
    footer: Rect,
    /// The title's and the message's y.
    title_y: f32,
    message_y: f32,
    /// Where the app's body starts: x, y and the content width.
    body: (f32, f32, f32),
    /// The confirm button, and the cancel button when there is one.
    confirm: Rect,
    cancel: Option<Rect>,
}

impl Layout {
    fn of<H: Copy + PartialEq>(p: &mut Painter<'_, H>, spec: &DialogSpec<'_>) -> Layout {
        let t = p.theme;
        let win = p.window;
        let inner = CARD_W - 48.0;
        let title_style = Style::ui(t.font_m, t.fg_primary).medium();
        let body_style = Style::ui(t.font_base, t.fg_secondary);
        let title = Spec::plain(spec.title, title_style);
        let message = Spec::plain(spec.message, body_style).wrapped(inner);
        let (_, th) = p.measure(&title);
        let (_, mh) = p.measure(&message);
        let body_h = 24.0 + th + 10.0 + mh + spec.extra_h + 24.0;
        let footer_h = 18.0 + 32.0 + 18.0;
        let card = Rect::new(
            (win.w - CARD_W) / 2.0,
            ((win.h - body_h - footer_h) / 2.0).max(20.0),
            CARD_W,
            body_h + footer_h,
        );
        let footer = Rect::new(card.x, card.bottom() - footer_h, CARD_W, footer_h);
        let cw = button_width(p, spec.confirm);
        let confirm = Rect::new(footer.right() - 24.0 - cw, footer.y + 18.0, cw, 32.0);
        let cancel = spec.cancel.map(|label| {
            let xw = button_width(p, label);
            Rect::new(confirm.x - 8.0 - xw, confirm.y, xw, 32.0)
        });
        // The same association as the moved code: `y += th + 10.0`, then `y += mh`.
        let title_y = card.y + 24.0;
        let message_y = title_y + (th + 10.0);
        let body_y = message_y + mh;
        Layout {
            title,
            message,
            card,
            footer,
            title_y,
            message_y,
            body: (card.x + 24.0, body_y, inner),
            confirm,
            cancel,
        }
    }

    /// The card's transform about its centre.
    fn xform(&self, scale: f32, alpha: f32) -> Xform {
        Xform {
            origin: (
                self.card.x + self.card.w / 2.0,
                self.card.y + self.card.h / 2.0,
            ),
            scale,
            dy: 0.0,
            alpha,
        }
    }
}

fn paint_scrim<H: Copy + PartialEq>(p: &mut Painter<'_, H>) {
    let win = p.window;
    let t = p.theme;
    p.fill(win, 0.0, t.dialog_backdrop);
}

/// The card's acrylic and its ring.
fn paint_card<H: Copy + PartialEq>(p: &mut Painter<'_, H>, l: &Layout, glass: &DialogGlass) {
    let t = p.theme;
    let under = over(premul(t.dialog_backdrop), glass.panel);
    p.acrylic(l.card, 8.0, glass.fill, under, glass.noise);
    p.ring(l.card, 8.0, 1.0, t.border_subtle, false);
}

fn paint_text<H: Copy + PartialEq>(p: &mut Painter<'_, H>, l: &Layout) {
    p.text(&l.title, l.card.x + 24.0, l.title_y);
    p.text(&l.message, l.card.x + 24.0, l.message_y);
}

fn paint_footer<H: Copy + PartialEq>(p: &mut Painter<'_, H>, l: &Layout) {
    let t = p.theme;
    p.fill_radii(l.footer, [0.0, 0.0, 8.0, 8.0], t.dialog_footer);
}

/// Draws a dialog on the modal layer. `body` draws the app's part between the message and the
/// footer; it gets the painter, the card's content x, the y just below the message and the
/// content width.
pub fn dialog<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    spec: &DialogSpec<'_>,
    glass: &DialogGlass,
    hits: &DialogHits<H>,
    body: impl FnOnce(&mut Painter<'_, H>, f32, f32, f32),
) {
    p.on(LayerId::Modal);
    let win = p.window;
    p.hit(win, hits.block);
    paint_scrim(p);

    let l = Layout::of(p, spec);
    paint_card(p, &l, glass);
    p.hit(l.card, hits.block);
    paint_text(p, &l);
    body(p, l.body.0, l.body.1, l.body.2);

    paint_footer(p, &l);
    button_fn(p, l.confirm, spec.confirm, true, hits.confirm);
    if let (Some(cancel), Some(r)) = (spec.cancel, l.cancel) {
        button_fn(p, r, cancel, false, hits.cancel);
    }
}

/// How the cancel button is labelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cancel<'s> {
    /// No cancel button.
    None,
    /// The string table's `luneui.dialog.cancel`.
    #[default]
    Default,
    Label(&'s str),
}

/// What [`Ui::dialog`] shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dialog<'s> {
    pub title: &'s str,
    pub message: &'s str,
    /// Height of the body the app draws between the message and the footer.
    pub extra_h: f32,
    pub confirm: &'s str,
    pub cancel: Cancel<'s>,
}

/// What a frame of [`Ui::dialog`] reports; at most one is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DialogResponse {
    /// The confirm button was activated (pointer, Enter or Space, or the default action).
    pub confirmed: bool,
    /// Cancel, Escape, a scrim click or an assistive technology's dismiss.
    pub cancelled: bool,
}

/// What the exit painter needs to draw a closing dialog from owned data (D8).
struct Exit {
    layout: Layout,
    glass: DialogGlass,
    /// The app's body as recorded in the last called frame.
    body: Ops,
    /// Confirm first, then cancel: the draw order of the dialog.
    buttons: Vec<(String, Rect, ButtonKind, ButtonState)>,
}

impl Exit {
    /// The card with everything on it, under the transform `x` already pushed.
    fn paint_card_parts(&self, p: &mut Painter<'_, WidgetId>) {
        paint_card(p, &self.layout, &self.glass);
        paint_text(p, &self.layout);
        p.replay(&self.body);
        paint_footer(p, &self.layout);
        let theme = p.theme;
        for (label, rect, kind, state) in &self.buttons {
            let look = ButtonLook::of(&theme, *kind, *state);
            button::paint(p, *rect, label, *kind, &look);
        }
    }
}

fn fade(alpha: f32) -> Xform {
    Xform {
        origin: (0.0, 0.0),
        scale: 1.0,
        dy: 0.0,
        alpha,
    }
}

impl<'a> Ui<'a> {
    /// A modal dialog under `key`; call it every frame it is open. `body` draws the app's part
    /// (content x, y just below the message, content width) and gets the `Ui`, so it can hold
    /// widgets; it is drawn live and recorded, and a closing dialog replays the recording. Stop
    /// calling it to close: the Ui then draws the close motion from retained state (180 ms Full,
    /// 120 ms Reduced, none Off).
    ///
    /// Cancel is the first tab stop, Confirm the second and the initially focused one (D20), so
    /// Enter confirms at open. Escape, Cancel, a scrim click and an assistive technology's
    /// dismiss report `cancelled`; a click on the card does nothing.
    pub fn dialog(
        &mut self,
        key: impl Hash,
        dialog: &Dialog<'_>,
        glass: &DialogGlass,
        body: impl FnOnce(&mut Ui<'a>, f32, f32, f32),
    ) -> DialogResponse {
        let id = self.id(&key);
        let confirm_key = (id, "confirm");
        let cancel_key = (id, "cancel");
        let confirm_id = self.id(confirm_key);
        let cancel_label: Option<String> = match dialog.cancel {
            Cancel::None => None,
            Cancel::Default => Some(self.tr("luneui.dialog.cancel").to_owned()),
            Cancel::Label(s) => Some(s.to_owned()),
        };
        let options = OverlayOptions {
            layer: LayerId::Modal,
            modal: true,
            dismiss_on_escape: true,
            dismiss_on_outside_press: false,
            dismiss_on_focus_loss: false,
            anchor: None,
            initial_focus: Some(confirm_id),
            open: OPEN,
            close: CLOSE,
        };
        let exit_data: Rc<RefCell<Option<Exit>>> = Rc::default();
        let exit = {
            let data = Rc::clone(&exit_data);
            move |p: &mut Painter<'_, WidgetId>, progress: f32, transforms: bool| {
                let data = data.borrow();
                let Some(x) = data.as_ref() else { return };
                let e = CLOSE.easing.eval(progress);
                let alpha = 1.0 - e;
                p.push_xform(fade(alpha));
                paint_scrim(p);
                p.pop_xform();
                let scale = if transforms {
                    1.0 - SCALE_SPAN * e
                } else {
                    1.0
                };
                p.push_xform(x.layout.xform(scale, alpha));
                x.paint_card_parts(p);
                p.pop_xform();
            }
        };
        let spec = DialogSpec {
            title: dialog.title,
            message: dialog.message,
            extra_h: dialog.extra_h,
            confirm: dialog.confirm,
            cancel: cancel_label.as_deref(),
        };

        let (frame, (confirmed, cancelled)) = self.overlay(&key, &options, exit, |ui, frame| {
            // Opening: the scrim and the card fade by the eased progress, the card also scales
            // (unless Reduced). Open pushes nothing, so the settled scene is the Painter API's.
            let (alpha, scale) = match frame.phase {
                OverlayPhase::Opening(p) => {
                    let e = OPEN.easing.eval(p);
                    let scale = if frame.transforms {
                        SCALE_FROM + SCALE_SPAN * e
                    } else {
                        1.0
                    };
                    (Some(e), scale)
                }
                OverlayPhase::Open => (None, 1.0),
            };
            let win = ui.painter().window;
            let scrim = ui.interact(id.with("scrim"), win, Sense::CLICK);
            if let Some(a) = alpha {
                ui.painter().push_xform(fade(a));
            }
            paint_scrim(ui.painter());
            if alpha.is_some() {
                ui.painter().pop_xform();
            }

            let layout = Layout::of(ui.painter(), &spec);
            if let Some(a) = alpha {
                ui.painter().push_xform(layout.xform(scale, a));
            }
            let card = ui.interact(id, layout.card, Sense::HOVER);
            let mut node = Node::new(id, Role::Dialog, dialog.title, layout.card);
            node.description = Some(dialog.message.to_owned());
            node.states.insert(State::Modal);
            node.actions.insert(Action::Default);
            node.actions.insert(Action::Dismiss);
            let results = ui.node_container(node, |ui| {
                paint_card(ui.painter(), &layout, glass);
                paint_text(ui.painter(), &layout);
                let (bx, by, bw) = layout.body;
                let ((), ops) = ui.record(|ui| body(ui, bx, by, bw));
                paint_footer(ui.painter(), &layout);

                // Confirm draws first (as the Painter API does) but Cancel is the first tab stop.
                if let Some(r) = layout.cancel {
                    ui.interact(ui.id(cancel_key), r, Sense::CLICK_FOCUS);
                }
                let confirm = ui.button(
                    confirm_key,
                    layout.confirm,
                    dialog.confirm,
                    ButtonKind::Primary,
                );
                let cancel = match (layout.cancel, cancel_label.as_deref()) {
                    (Some(r), Some(label)) => Some((
                        ui.button(cancel_key, r, label, ButtonKind::Secondary),
                        label,
                    )),
                    _ => None,
                };
                let state = |r: &crate::ui::Response| ButtonState {
                    hovered: r.hovered,
                    pressed: r.pressed,
                    focus_visible: r.focus_visible,
                };
                let mut buttons = vec![(
                    dialog.confirm.to_owned(),
                    layout.confirm,
                    ButtonKind::Primary,
                    state(&confirm),
                )];
                if let (Some((r, label)), Some(rect)) = (&cancel, layout.cancel) {
                    buttons.push((label.to_string(), rect, ButtonKind::Secondary, state(r)));
                }
                *exit_data.borrow_mut() = Some(Exit {
                    layout: layout.clone(),
                    glass: *glass,
                    body: ops,
                    buttons,
                });
                (confirm.clicked, cancel.is_some_and(|(r, _)| r.clicked))
            });
            if alpha.is_some() {
                ui.painter().pop_xform();
            }
            let (confirm_clicked, cancel_clicked) = results;
            (
                confirm_clicked || card.clicked,
                cancel_clicked || scrim.clicked,
            )
        });
        DialogResponse {
            confirmed,
            cancelled: !confirmed && (cancelled || frame.dismissed.is_some()),
        }
    }
}
