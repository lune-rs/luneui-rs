//! The house look's dialog: a dim, blurred backdrop, a 448 px frosted glass card (600 for a form),
//! 24 px padding, darker footer block.
//!
//! The drawing is split into the scrim, the card (frost, ring, text), the app's body and the
//! footer, so one set of paint functions serves three callers: [`dialog`], the Painter-API
//! function (everything at rest, no transform), [`Ui::dialog`] and [`Ui::dialog_with`] (the same
//! on the overlay machinery, with the open fade and scale), and the exit painter of a closing
//! dialog (D8). The modal host (scrim, the card's motion, exit retention, the node) is
//! `modal_host`, which other modal surfaces share.

use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use super::button::{self, ButtonKind, ButtonLook, ButtonState};
use super::{button as button_fn, button_width};

/// The least width of a footer button (the source look's `MinWidth`, R4), so a two-letter confirm
/// is not a stub.
const BUTTON_MIN_WIDTH: f32 = 80.0;
use crate::access::{Action, Node, Role, State};
use crate::color::premul;
use crate::motion::{Easing, Kind, Motion, Reduce, Token};
use crate::paint::{LayerId, Ops, Painter, Rect, Xform};
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::{DISABLED_ALPHA, OverlayFrame, OverlayOptions, OverlayPhase, Sense, Ui, WidgetId};

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

/// The card's frost, from the app's glass policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogGlass {
    /// The card's own fill, premultiplied: frosted over the scrim's blurred picture.
    pub fill: [f32; 4],
    pub noise: bool,
}

impl DialogGlass {
    /// The glass every surface of the toolkit draws: `theme`'s `menu_glass`, premultiplied, with
    /// the noise on (D8, D9).
    pub fn of(theme: &Theme) -> DialogGlass {
        DialogGlass {
            fill: premul(theme.menu_glass),
            noise: true,
        }
    }
}

/// Hit regions of a dialog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogHits<H> {
    /// The backdrop and the card, so a click does not reach what is below.
    pub block: H,
    pub confirm: H,
    pub cancel: H,
}

/// The card's width: a confirm dialog's.
const CARD_W: f32 = 448.0;
/// The form dialog's card width.
const FORM_W: f32 = 600.0;
/// The scrim blur's Gaussian sigma and saturation (the window content behind the dialog).
const SCRIM_SIGMA: f32 = 8.0;
const SCRIM_SATURATION: f32 = 1.0;
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
    fn of<H: Copy + PartialEq>(
        p: &mut Painter<'_, H>,
        spec: &DialogSpec<'_>,
        card_w: f32,
    ) -> Layout {
        let t = p.theme;
        let win = p.window;
        let inner = card_w - 48.0;
        let title_style = Style::ui(t.font_m, t.fg_primary).medium();
        let body_style = Style::ui(t.font_base, t.fg_secondary);
        let title = Spec::plain(spec.title, title_style);
        let message = Spec::plain(spec.message, body_style).wrapped(inner);
        let (_, th) = p.measure(&title);
        let (_, mh) = p.measure(&message);
        let body_h = 24.0 + th + 10.0 + mh + spec.extra_h + 24.0;
        let footer_h = 18.0 + 32.0 + 18.0;
        let card = Rect::new(
            (win.w - card_w) / 2.0,
            ((win.h - body_h - footer_h) / 2.0).max(20.0),
            card_w,
            body_h + footer_h,
        );
        let footer = Rect::new(card.x, card.bottom() - footer_h, card_w, footer_h);
        let cw = button_width(p, spec.confirm).max(BUTTON_MIN_WIDTH);
        let confirm = Rect::new(footer.right() - 24.0 - cw, footer.y + 18.0, cw, 32.0);
        let cancel = spec.cancel.map(|label| {
            let xw = button_width(p, label).max(BUTTON_MIN_WIDTH);
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
}

/// A card's transform about its centre.
fn card_xform(card: Rect, scale: f32, alpha: f32) -> Xform {
    Xform {
        origin: (card.x + card.w / 2.0, card.y + card.h / 2.0),
        scale,
        dx: 0.0,
        dy: 0.0,
        alpha,
    }
}

/// The scrim over the window; `blur` first asks for what is drawn behind it, blurred (at sigma 8,
/// saturation 1), at the scrim's own opacity.
fn paint_scrim<H: Copy + PartialEq>(p: &mut Painter<'_, H>, blur: bool) {
    let win = p.window;
    let t = p.theme;
    if blur {
        p.backdrop_with(win, 0.0, SCRIM_SIGMA, SCRIM_SATURATION);
    }
    p.fill(win, 0.0, t.dialog_backdrop);
}

/// The card's frost and its ring. The scrim's blur (when there is one) is already under it, so
/// the card asks for none of its own.
fn paint_card<H: Copy + PartialEq>(p: &mut Painter<'_, H>, card: Rect, glass: &DialogGlass) {
    let t = p.theme;
    p.frost(card, 8.0, glass.fill, glass.noise);
    p.ring(card, 8.0, 1.0, t.menu_popup_border, false);
}

fn paint_text<H: Copy + PartialEq>(p: &mut Painter<'_, H>, l: &Layout) {
    p.text(&l.title, l.card.x + 24.0, l.title_y);
    p.text(&l.message, l.card.x + 24.0, l.message_y);
}

fn paint_footer<H: Copy + PartialEq>(p: &mut Painter<'_, H>, l: &Layout) {
    let t = p.theme;
    p.fill_radii(l.footer, [0.0, 0.0, 8.0, 8.0], t.dialog_footer);
}

/// Draws a dialog on the modal layer with [`Painter::on`], so it inherits the caller's clip and
/// transforms; call it with none (the `Ui` overlays start with none). `body` draws the app's part between the message and the
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
    paint_scrim(p, true);

    let l = Layout::of(p, spec, CARD_W);
    paint_card(p, l.card, glass);
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

/// What [`Ui::dialog`] and [`Ui::dialog_with`] show.
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

/// The card's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogWidth {
    /// 448 px: title, message, Cancel and one action.
    #[default]
    Confirm,
    /// 600 px: room for the app's body in two columns. The columns are the app's.
    Form,
}

impl DialogWidth {
    /// The card's width in logical px.
    pub const fn px(self) -> f32 {
        match self {
            DialogWidth::Confirm => CARD_W,
            DialogWidth::Form => FORM_W,
        }
    }
}

/// How [`Ui::dialog_with`] differs from [`Ui::dialog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogOptions {
    pub width: DialogWidth,
    /// Blur what is behind the scrim (sigma 8, saturation 1); the default. Off draws the plain
    /// scrim, and the card frosts the sharp picture.
    pub scrim_blur: bool,
    /// False draws the confirm button disabled: nothing activates it (Enter, Space, a click and
    /// the default action do nothing), and the open focus goes to Cancel, or nowhere.
    pub confirm_enabled: bool,
}

impl Default for DialogOptions {
    fn default() -> Self {
        Self {
            width: DialogWidth::Confirm,
            scrim_blur: true,
            confirm_enabled: true,
        }
    }
}

/// What the exit painter needs to draw a closing dialog from owned data (D8): the parts on the
/// card, after the host has drawn the scrim and the card.
struct Exit {
    layout: Layout,
    /// The app's body as recorded in the last called frame.
    body: Ops,
    /// Confirm first, then cancel: the draw order of the dialog. The flag is whether the button
    /// was enabled.
    buttons: Vec<(String, Rect, ButtonKind, ButtonState, bool)>,
}

impl Exit {
    /// The card's contents, under the transform the host already pushed.
    fn paint_parts(&self, p: &mut Painter<'_, WidgetId>) {
        paint_text(p, &self.layout);
        p.replay(&self.body);
        paint_footer(p, &self.layout);
        let theme = p.theme;
        for (label, rect, kind, state, enabled) in &self.buttons {
            let look = ButtonLook::of(&theme, *kind, *state);
            if !enabled {
                p.push_xform(fade(DISABLED_ALPHA));
            }
            button::paint(p, *rect, label, *kind, &look);
            if !enabled {
                p.pop_xform();
            }
        }
    }
}

fn fade(alpha: f32) -> Xform {
    Xform {
        origin: (0.0, 0.0),
        scale: 1.0,
        dx: 0.0,
        dy: 0.0,
        alpha,
    }
}

/// What a modal surface needs of its host.
pub(crate) struct ModalHost<'s> {
    /// The card's rect: the transform's origin, the node's bounds, the frost.
    pub(crate) card: Rect,
    /// The node's name and description.
    pub(crate) name: &'s str,
    pub(crate) description: Option<&'s str>,
    /// Blur what is behind the scrim.
    pub(crate) scrim_blur: bool,
    pub(crate) glass: &'s DialogGlass,
    /// The widget focused on open, if it is a tab stop of the overlay; else its first tab stop.
    pub(crate) initial_focus: Option<WidgetId>,
    /// Whether the node offers the default action (and a report of it is honoured).
    pub(crate) default_action: bool,
}

/// What a frame of [`modal_host`] reports.
pub(crate) struct ModalReport<R> {
    pub(crate) frame: OverlayFrame,
    /// What the content returned.
    pub(crate) content: R,
    /// The scrim was clicked.
    pub(crate) scrim_clicked: bool,
    /// The default action reached the card (an assistive technology's; never when the host did
    /// not offer it).
    pub(crate) card_default: bool,
}

/// The modal host: the overlay on the `Modal` layer (Escape dismisses, focus trapped), the
/// scrim (blurred when asked) with its click, the card's frost with the open and close
/// transform, the node (role `Dialog`, state `Modal`, actions Dismiss and, when offered, Default),
/// and the exit painter of a closing surface. `content` draws on the card after the frost;
/// `exit_parts` draws the same on the card of a closing surface, from owned data, inside the
/// card's transform. Stop calling to close.
pub(crate) fn modal_host<'a, R>(
    ui: &mut Ui<'a>,
    key: impl Hash,
    host: &ModalHost<'_>,
    exit_parts: impl Fn(&mut Painter<'_, WidgetId>) + 'static,
    content: impl FnOnce(&mut Ui<'a>) -> R,
) -> ModalReport<R> {
    let id = ui.id(&key);
    let (card, glass, blur) = (host.card, *host.glass, host.scrim_blur);
    let options = OverlayOptions {
        layer: LayerId::Modal,
        modal: true,
        dismiss_on_escape: true,
        dismiss_on_outside_press: false,
        dismiss_on_focus_loss: false,
        anchor: None,
        initial_focus: host.initial_focus,
        open: OPEN,
        close: CLOSE,
    };
    let exit = move |p: &mut Painter<'_, WidgetId>, progress: f32, transforms: bool| {
        let e = CLOSE.easing.eval(progress);
        let alpha = 1.0 - e;
        p.push_xform(fade(alpha));
        paint_scrim(p, blur);
        p.pop_xform();
        let scale = if transforms {
            1.0 - SCALE_SPAN * e
        } else {
            1.0
        };
        p.push_xform(card_xform(card, scale, alpha));
        paint_card(p, card, &glass);
        exit_parts(p);
        p.pop_xform();
    };

    let (frame, (content, scrim_clicked, card_default)) =
        ui.overlay(&key, &options, exit, |ui, frame| {
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
            paint_scrim(ui.painter(), blur);
            if alpha.is_some() {
                ui.painter().pop_xform();
            }

            if let Some(a) = alpha {
                ui.painter().push_xform(card_xform(card, scale, a));
            }
            let hit = ui.interact(id, card, Sense::HOVER);
            let mut node = Node::new(id, Role::Dialog, host.name, card);
            node.description = host.description.map(str::to_owned);
            node.states.insert(State::Modal);
            if host.default_action {
                node.actions.insert(Action::Default);
            }
            node.actions.insert(Action::Dismiss);
            let r = ui.node_container(node, |ui| {
                paint_card(ui.painter(), card, &glass);
                content(ui)
            });
            if alpha.is_some() {
                ui.painter().pop_xform();
            }
            (r, scrim.clicked, hit.clicked && host.default_action)
        });
    ModalReport {
        frame,
        content,
        scrim_clicked,
        card_default,
    }
}

impl<'a> Ui<'a> {
    /// A modal dialog under `key`; call it every frame it is open. `body` draws the app's part
    /// (content x, y just below the message, content width) and gets the `Ui`, so it can hold
    /// widgets; it is drawn live and recorded, and a closing dialog replays the recording. Stop
    /// calling to close: the Ui then draws the close motion from retained state (180 ms Full,
    /// 120 ms Reduced, none Off).
    ///
    /// Cancel is the first tab stop, Confirm the second and the initially focused one (D20), so
    /// Enter confirms at open. Escape, Cancel, a scrim click and an assistive technology's
    /// dismiss report `cancelled`; a click on the card does nothing. Overlays the body opens
    /// (a popover, a menu, a tooltip, a dropdown, a nested dialog) draw and take input above it.
    pub fn dialog(
        &mut self,
        key: impl Hash,
        dialog: &Dialog<'_>,
        glass: &DialogGlass,
        body: impl FnOnce(&mut Ui<'a>, f32, f32, f32),
    ) -> DialogResponse {
        self.dialog_with(key, dialog, &DialogOptions::default(), glass, body)
    }

    /// [`Self::dialog`] with `options`: the form width, the blurred scrim, a disabled confirm.
    ///
    /// A disabled confirm draws as the button's own disabled look and keeps its place; Enter,
    /// Space, a click and the default action do nothing, and the keys are consumed so no shortcut
    /// sees them. The open focus goes to Cancel, or nowhere when there is none.
    pub fn dialog_with(
        &mut self,
        key: impl Hash,
        dialog: &Dialog<'_>,
        options: &DialogOptions,
        glass: &DialogGlass,
        body: impl FnOnce(&mut Ui<'a>, f32, f32, f32),
    ) -> DialogResponse {
        let id = self.id(&key);
        let confirm_key = (id, "confirm");
        let cancel_key = (id, "cancel");
        let confirm_id = self.id(confirm_key);
        let cancel_id = self.id(cancel_key);
        let cancel_label: Option<String> = match dialog.cancel {
            Cancel::None => None,
            Cancel::Default => Some(self.tr("luneui.dialog.cancel").to_owned()),
            Cancel::Label(s) => Some(s.to_owned()),
        };
        let spec = DialogSpec {
            title: dialog.title,
            message: dialog.message,
            extra_h: dialog.extra_h,
            confirm: dialog.confirm,
            cancel: cancel_label.as_deref(),
        };
        let layout = Layout::of(self.painter(), &spec, options.width.px());
        let confirm_enabled = options.confirm_enabled;
        let initial_focus = if confirm_enabled {
            Some(confirm_id)
        } else {
            cancel_label.is_some().then_some(cancel_id)
        };
        let host = ModalHost {
            card: layout.card,
            name: dialog.title,
            description: Some(dialog.message),
            scrim_blur: options.scrim_blur,
            glass,
            initial_focus,
            default_action: confirm_enabled,
        };

        let exit_data: Rc<RefCell<Option<Exit>>> = Rc::default();
        let exit_parts = {
            let data = Rc::clone(&exit_data);
            move |p: &mut Painter<'_, WidgetId>| {
                if let Some(x) = data.borrow().as_ref() {
                    x.paint_parts(p);
                }
            }
        };
        let report = modal_host(self, &key, &host, exit_parts, |ui| {
            paint_text(ui.painter(), &layout);
            let (bx, by, bw) = layout.body;
            let ((), ops) = ui.record(|ui| body(ui, bx, by, bw));
            paint_footer(ui.painter(), &layout);

            // Confirm draws first (as the Painter API does) but Cancel is the first tab stop.
            if let Some(r) = layout.cancel {
                ui.interact(cancel_id, r, Sense::CLICK_FOCUS);
            }
            let confirm = ui.with_enabled(confirm_enabled, |ui| {
                ui.button(
                    confirm_key,
                    layout.confirm,
                    dialog.confirm,
                    ButtonKind::Primary,
                )
            });
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
                confirm_enabled,
            )];
            if let (Some((r, label)), Some(rect)) = (&cancel, layout.cancel) {
                buttons.push((
                    label.to_string(),
                    rect,
                    ButtonKind::Secondary,
                    state(r),
                    true,
                ));
            }
            *exit_data.borrow_mut() = Some(Exit {
                layout: layout.clone(),
                body: ops,
                buttons,
            });
            (confirm.clicked, cancel.is_some_and(|(r, _)| r.clicked))
        });
        let (confirm_clicked, cancel_clicked) = report.content;
        let confirmed = confirm_clicked || report.card_default;
        DialogResponse {
            confirmed,
            cancelled: !confirmed
                && (cancel_clicked || report.scrim_clicked || report.frame.dismissed.is_some()),
        }
    }
}
