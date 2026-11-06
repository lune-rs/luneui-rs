//! The window's caption buttons through the `Ui` (`platform.window`, D7): minimize, maximize or
//! restore, close, right-aligned along the top of a borderless window.
//!
//! The look is the Painter form's, [`window::caption_buttons`](crate::window::caption_buttons):
//! 46 wide, a 10 px Codicon in `fg_primary`, `surface_hover` under the pointer, and the close
//! button `caption_close_hover` with a `caption_close_fg` glyph. On top of it the `Ui` form fades
//! the fill and the glyph on the `hover` kind (120 ms, linear; Reduced keeps it, Off snaps), emits
//! a semantic node per button and reads its names from the string table. At rest and active the
//! `Scene` is the Painter form's.
//!
//! The buttons are pointer targets, not tab stops: a press does not take the focus, and the keys
//! are the OS window's. The toolkit calls no window API: [`Ui::caption_buttons`] answers which
//! button was clicked and the app turns it into a [`WindowCommand`](crate::input::WindowCommand).

use std::hash::Hash;

use super::button::HOVER;
use crate::access::{Action, Node, Role, State};
use crate::color::Rgba;
use crate::icon;
use crate::paint::Rect;
use crate::ui::{Sense, Ui};
use crate::window::{CAPTION_GLYPH_SIZE, caption_rect};

/// One of the three caption buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionButton {
    Minimize,
    /// Maximize, or restore while the window is maximized.
    Maximize,
    Close,
}

/// What [`Ui::caption_buttons`] is given besides its key and the window rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptionOptions {
    /// The buttons' height; a title strip of 32 is the usual.
    pub height: f32,
    /// The middle button is "restore" and its glyph and name say so.
    pub maximized: bool,
    /// Whether the window is active; an inactive window's glyphs are `fg_quaternary`.
    pub active: bool,
}

impl Default for CaptionOptions {
    fn default() -> Self {
        CaptionOptions {
            height: 32.0,
            maximized: false,
            active: true,
        }
    }
}

impl Ui<'_> {
    /// The caption buttons of the window `win` (its width places them, from the window's own
    /// origin; see [`window::caption_buttons`](crate::window::caption_buttons)). Returns the
    /// button clicked this frame, if any. Each is a role-button node named `luneui.window.minimize`,
    /// `.maximize` (`.restore` while `opts.maximized`) or `.close`, whose default action clicks it.
    pub fn caption_buttons(
        &mut self,
        key: impl Hash,
        win: Rect,
        opts: CaptionOptions,
    ) -> Option<CaptionButton> {
        let base = self.id(key);
        let theme = *self.theme();
        let which = [
            CaptionButton::Minimize,
            CaptionButton::Maximize,
            CaptionButton::Close,
        ];
        let glyphs = [
            icon::CHROME_MINIMIZE,
            if opts.maximized {
                icon::CHROME_RESTORE
            } else {
                icon::CHROME_MAXIMIZE
            },
            icon::CHROME_CLOSE,
        ];
        let (names, strings) = (
            ["minimize", "maximize", "close"],
            [
                "luneui.window.minimize",
                if opts.maximized {
                    "luneui.window.restore"
                } else {
                    "luneui.window.maximize"
                },
                "luneui.window.close",
            ],
        );
        let mut clicked = None;
        for k in 0..3 {
            let is_close = k == 2;
            let r = caption_rect(win, opts.height, k);
            let id = base.with(names[k]);
            let resp = self.interact(id, r, Sense::CLICK);
            let hovered = resp.hovered;

            // Transparent in the hover colour, so the fade has no dark middle.
            let hover_fill = if is_close {
                theme.caption_close_hover
            } else {
                theme.surface_hover
            };
            let target_fill: Rgba = if hovered {
                hover_fill
            } else {
                [hover_fill[0], hover_fill[1], hover_fill[2], 0]
            };
            let rest_fg = if opts.active {
                theme.fg_primary
            } else {
                theme.fg_quaternary
            };
            let target_fg = if hovered && is_close {
                theme.caption_close_fg
            } else {
                rest_fg
            };
            let fill = self.animate_color(id.with("fill"), target_fill, &HOVER);
            let fg = self.animate_color(id.with("fg"), target_fg, &HOVER);

            let p = self.painter();
            if fill[3] > 0 {
                p.fill(r, 0.0, fill);
            }
            p.text_centered(&icon::glyph(glyphs[k], CAPTION_GLYPH_SIZE, fg), r);

            let name = self.tr(strings[k]).to_owned();
            let mut node = Node::new(id, Role::Button, &name, r);
            if resp.enabled {
                node.actions.insert(Action::Default);
            } else {
                node.states.insert(State::Disabled);
            }
            self.node(node);
            if resp.clicked {
                clicked = clicked.or(Some(which[k]));
            }
        }
        clicked
    }
}
