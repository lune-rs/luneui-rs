//! A trigger that opens a searchable list and picks one entry (`widget.search-picker`): a
//! [`Trigger`] on the base layer and a popover holding a [filter bar](Ui::filter_bar) over a
//! scrolled list that builds only its visible rows.
//!
//! The popup's `open` flag is the picker's own. The Ui never closes an overlay, so every close is
//! `open` going false, and the popover is called exactly while it is true: a dismissal arrives on
//! the frame's return value and clears the flag on that frame, and the next frame does not call it.
//! A press on the open trigger is an outside press like any other: the Ui consumes it (D77), so it
//! closes through the dismissal and never reaches the trigger, which cannot reopen.
//!
//! Each open is a new overlay id. An overlay closes only at the end of a frame that did not call
//! it, so an open on the frame after a close would otherwise continue the old overlay: no initial
//! focus, and the old frame's dismissal.
//!
//! Only the search bar takes focus inside the popup (rows are `Sense::CLICK`), so its box is the
//! overlay's first stop and a row press moves no focus. There is no keyboard highlight (D124).

use std::hash::Hash;

use super::filter_bar::{self, FilterOptions};
use super::focus_ring::{self, RingKind};
use super::label;
use super::popover::{PopoverOptions, Side, Surface};
use super::popup_panel;
use super::scroll_area::{Bars, Extent, ScrollOptions};
use crate::access::{Action, Node, Role, State, Value};
use crate::icon;
use crate::input::NamedKey;
use crate::paint::Rect;
use crate::ui::{Response, Sense, Ui, WidgetId};

/// The popup's padding, least width and most height.
pub const POPUP_PADDING: f32 = 6.0;
pub const MIN_WIDTH: f32 = 220.0;
pub const MAX_HEIGHT: f32 = 320.0;
/// The search bar's height and the space under it.
pub const BAR_HEIGHT: f32 = 30.0;
pub const BAR_GAP: f32 = 6.0;
/// A row's height, corner radius and side padding.
pub const ROW_HEIGHT: f32 = 32.0;
pub const ROW_RADIUS: f32 = 4.0;
pub const ROW_PADDING: f32 = 8.0;
/// The field trigger's side padding, and the space between its text and its icon.
pub const FIELD_PADDING: f32 = 8.0;
pub const FIELD_ICON_GAP: f32 = 8.0;
/// The icon of both triggers, and the link's space between it and the text.
pub const ICON: f32 = 14.0;
pub const LINK_ICON_GAP: f32 = 6.0;
/// The link's underline: on and off runs of its dashes.
pub const DASH: (f32, f32) = (1.0, 2.0);

/// The space [`place`](super::popover::place) keeps from the window's edges, as
/// [`PopoverOptions::new`] sets it.
const WINDOW_MARGIN: f32 = 8.0;

/// What opens the popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Trigger {
    /// The filter bar's surface with the value and a search icon at the right. The popup takes its
    /// width when that is more than [`MIN_WIDTH`].
    #[default]
    Field,
    /// An icon and the value, underlined with dashes.
    Link,
}

/// What [`Ui::search_picker`] is given besides its rect, items and selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PickerOptions<'a> {
    pub trigger: Trigger,
    /// The link trigger's leading glyph; the field trigger's icon is always the search glyph.
    pub icon: Option<&'a str>,
    /// The search box's placeholder and accessible name.
    pub placeholder: &'a str,
    /// The accessible name of the trigger and the list, and of the box with no `placeholder`.
    pub name: &'a str,
}

/// What [`Ui::search_picker`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct PickerResponse {
    /// The trigger's region; `changed` tells that `selected` changed.
    pub response: Response,
    /// The index into `items` the user picked this frame, also when it was selected already.
    pub picked: Option<usize>,
}

/// The indices of `items` whose text contains `search`, trimmed and without regard to case; all of
/// them for a search of nothing but spaces.
pub fn matches<S: AsRef<str>>(items: &[S], search: &str) -> Vec<usize> {
    let needle = search.trim().to_lowercase();
    let all = needle.is_empty();
    let hit = |(_, s): &(usize, &S)| all || s.as_ref().to_lowercase().contains(&needle);
    items
        .iter()
        .enumerate()
        .filter(hit)
        .map(|(i, _)| i)
        .collect()
}

/// The popup's size for `rows` matches under the trigger at `anchor`, inside `window`.
fn popup_size(trigger: Trigger, anchor: Rect, rows: usize, window: Rect) -> (f32, f32) {
    let width = match trigger {
        Trigger::Field => anchor.w.max(MIN_WIDTH),
        Trigger::Link => MIN_WIDTH,
    };
    let bare = 2.0 * POPUP_PADDING + BAR_HEIGHT;
    let list = if rows > 0 {
        BAR_GAP + rows as f32 * ROW_HEIGHT
    } else {
        0.0
    };
    let room = (window.h - 2.0 * WINDOW_MARGIN).max(bare);
    (width, (bare + list).min(MAX_HEIGHT).min(room))
}

/// The retained state of one picker.
#[derive(Debug, Default)]
struct PickerState {
    open: bool,
    /// Cleared on open, not on close: the closing popup replays what it drew.
    search: String,
    /// Counts the opens: the overlay's key.
    opens: u64,
    /// Counts the opens and the edits of the search: the list's scroll identity, so each new list
    /// starts at its top.
    lists: u64,
    /// The side the popup took on its first frame. It keeps it while the list narrows.
    side: Option<Side>,
}

/// Set on every frame that draws the picker: a fresh one means the last frame did not, so a live
/// keep-alive scope brought the state through a frame in which the Ui closed the popup.
#[derive(Debug, Default)]
struct Drawn(bool);

/// What the popup's content needs of the picker.
struct Content<'p, S> {
    items: &'p [S],
    /// The selected index, when it is in range.
    current: Option<usize>,
    name: &'p str,
}

/// Draws the rows of `shown` in `list` and returns the item of the one clicked.
fn rows<S: AsRef<str>>(
    ui: &mut Ui<'_>,
    list: Rect,
    shown: &[usize],
    identity: u64,
    c: &Content<'_, S>,
) -> Option<usize> {
    let t = *ui.theme();
    let list_id = ui.id("list");
    let height = shown.len() as f64 * f64::from(ROW_HEIGHT);
    let opts = ScrollOptions {
        bars: (Bars::Disabled, Bars::Auto),
        identity: Some(identity),
        extent: Some((f64::from(list.w), height)),
        ..ScrollOptions::default()
    };
    let area = ui.scroll_area_untranslated("rows", list, &opts, |ui, view| {
        let row = f64::from(ROW_HEIGHT);
        let top = view.offset.1;
        let first = ((top / row).floor() as usize).min(shown.len());
        let last = (((top + f64::from(list.h)) / row).ceil() as usize).min(shown.len());
        let node = Node::new(list_id, Role::List, c.name, list);
        let clicked = ui.node_container(node, |ui| {
            let mut clicked = None;
            for (n, &item) in shown.iter().enumerate().take(last).skip(first) {
                let y = list.y + (n as f64 * row - top) as f32;
                let r = Rect::new(list.x, y, list.w, ROW_HEIGHT);
                let id = list_id.with(item);
                let resp = ui.interact(id, r, Sense::CLICK);
                if resp.clicked {
                    clicked = Some(item);
                }
                let selected = c.current == Some(item);
                let fill = if selected {
                    Some(t.surface_selected)
                } else {
                    resp.hovered.then_some(t.surface_hover)
                };
                if let Some(fill) = fill {
                    ui.painter().fill(r, ROW_RADIUS, fill);
                }
                let text = c.items[item].as_ref();
                let p = ui.painter();
                let spec = p.elide(&label(&t, text, t.fg_primary), list.w - 2.0 * ROW_PADDING);
                p.text_in(&spec, r, ROW_PADDING);
                let mut node = Node::new(id, Role::ListItem, text, r);
                if selected {
                    node.states.insert(State::Selected);
                }
                node.actions.insert(Action::Default);
                ui.node(node);
            }
            clicked
        });
        (clicked, Extent::new(f64::from(list.w), height))
    });
    area.value
}

/// Draws the field trigger: the filter bar's surface at the radius of its look, the value cut to
/// the room left by the icon, the icon.
fn paint_field(ui: &mut Ui<'_>, rect: Rect, value: &str) {
    let t = *ui.theme();
    let p = ui.painter();
    p.fill(rect, t.item_radius, t.surface_input);
    let room = rect.w - 2.0 * FIELD_PADDING - ICON - FIELD_ICON_GAP;
    let spec = p.elide(&label(&t, value, t.fg_primary), room);
    p.text_in(&spec, rect, FIELD_PADDING);
    let at = Rect::new(rect.right() - FIELD_PADDING - ICON, rect.y, ICON, rect.h);
    p.text_centered(&icon::glyph(icon::SEARCH, ICON, t.fg_dim), at);
}

/// Draws the link trigger: the icon, then the value on its dashed underline, as wide as the text.
fn paint_link(ui: &mut Ui<'_>, rect: Rect, value: &str, glyph: Option<&str>, hovered: bool) {
    let t = *ui.theme();
    let ink = if hovered {
        t.fg_primary
    } else {
        t.fg_secondary
    };
    let p = ui.painter();
    let mut left = 0.0;
    if let Some(glyph) = glyph {
        let at = Rect::new(rect.x, rect.y, ICON, rect.h);
        p.text_centered(&icon::glyph(glyph, ICON, ink), at);
        left = ICON + LINK_ICON_GAP;
    }
    let spec = p.elide(&label(&t, value, ink), rect.w - left);
    let (w, h) = p.text_in(&spec, rect, left);
    if w > 0.0 {
        let x = rect.x + left;
        // A 1 px line on the last pixel row of the text's line box.
        let y = rect.y + (rect.h + h) / 2.0 - 0.5;
        p.dash(&[[x, y], [x + w, y]], false, 1.0, DASH.0, DASH.1, ink);
    }
}

impl<'a> Ui<'a> {
    /// A search picker under `key` in `rect` over `items`. A click, Enter or Space on the trigger,
    /// or an accessibility default, expand or collapse action opens or closes the popup; opening
    /// clears the search and focuses the box. In the box, Enter picks the first match and closes;
    /// with no match it does nothing. A click on a row picks it and closes. Escape, a press
    /// outside, focus loss and disabling close. A pick writes `selected`, an index into `items`.
    /// Every close gives focus back to the trigger, unless focus went to another widget. A
    /// `selected` out of range reads as none, and is left as it is.
    pub fn search_picker<S: AsRef<str>>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        items: &[S],
        selected: &mut Option<usize>,
        opts: &PickerOptions<'_>,
    ) -> PickerResponse {
        let id = self.id(&key);
        let drawn = std::mem::replace(&mut self.transient::<Drawn>(id).0, true);
        let mut st = std::mem::take(self.persistent::<PickerState>(id));
        st.open &= drawn;
        let was_open = st.open;

        let mut r = self.interact(id, rect, Sense::CLICK_FOCUS);
        if r.enabled {
            let enter = self.consume_key(id, NamedKey::Enter);
            let space = self.consume_key(id, NamedKey::Space);
            r.clicked |= enter || space;
            if r.clicked {
                st.open = !st.open;
                if st.open {
                    st.search.clear();
                    st.opens += 1;
                    st.lists += 1;
                    st.side = None;
                }
            }
        } else {
            st.open = false;
        }

        let current = selected.filter(|i| *i < items.len());
        let value = current.map_or("", |i| items[i].as_ref());
        match opts.trigger {
            Trigger::Field => paint_field(self, rect, value),
            Trigger::Link => paint_link(self, rect, value, opts.icon, r.hovered),
        }
        let ring = match opts.trigger {
            Trigger::Field => RingKind::Edge {
                radius: self.theme().item_radius,
                on_accent: false,
            },
            Trigger::Link => RingKind::Button,
        };
        let opacity = if r.focus_visible { 1.0 } else { 0.0 };
        focus_ring::paint(self.painter(), rect, ring, opacity);

        let mut node = Node::new(id, Role::ComboBox, opts.name, rect);
        node.value = current.map(|_| Value::Text(value.to_owned()));
        if st.open {
            node.states.insert(State::Expanded);
        }
        if r.enabled {
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Default);
            node.actions.insert(Action::Expand);
            node.actions.insert(Action::Collapse);
        } else {
            node.states.insert(State::Disabled);
        }
        self.node(node);

        let mut picked = None;
        if st.open {
            let content = Content {
                items,
                current,
                name: opts.name,
            };
            let (dismissed, chosen) =
                self.scope(&key, |ui| popup(ui, rect, &mut st, &content, opts));
            picked = chosen;
            st.open &= !dismissed && picked.is_none();
        }
        if let Some(i) = picked
            && *selected != Some(i)
        {
            *selected = Some(i);
            r.mark_changed();
        }
        if was_open && !st.open {
            self.give_focus_back(id);
        }
        if !st.open {
            self.patch_node(id, |n| {
                n.states.remove(State::Expanded);
            });
        }
        *self.persistent::<PickerState>(id) = st;
        PickerResponse {
            response: r,
            picked,
        }
    }

    /// Focuses the trigger `id` when focus is in its popup or gone: focus that went to another
    /// widget is that widget's, and is what closed the popup. The ring stays as it was. The
    /// overlay's own restore goes to what was focused at open, which is not the trigger when an
    /// accessibility action opened it.
    fn give_focus_back(&mut self, id: WidgetId) {
        let (focused, ring) = self.focus_state();
        let bar = id.with("search");
        if focused.is_none_or(|f| f == bar || f == filter_bar::clear_id(bar)) {
            if ring {
                self.rove_focus(id);
            } else {
                self.request_focus(id);
            }
        }
    }
}

/// One frame of the popup. Returns whether it was dismissed and what was picked.
fn popup<S: AsRef<str>>(
    ui: &mut Ui<'_>,
    rect: Rect,
    st: &mut PickerState,
    c: &Content<'_, S>,
    opts: &PickerOptions<'_>,
) -> (bool, Option<usize>) {
    let mut shown = matches(c.items, &st.search);
    let anchor = ui.painter().transformed(rect);
    let window = ui.painter().window;
    let size = popup_size(opts.trigger, anchor, shown.len(), window);
    let id = ui.scope_id();
    let options = PopoverOptions {
        anchor_id: Some(id),
        side: st.side.unwrap_or(Side::Bottom),
        surface: Surface::None,
        ..PopoverOptions::new(anchor, size)
    };
    let search = &mut st.search;
    let lists = &mut st.lists;
    let (frame, placement, picked) = ui.popover(("popup", st.opens), &options, |ui, at| {
        let t = *ui.theme();
        popup_panel(ui.painter(), at, t.dropdown_bg, t.border);
        let bar_w = at.w - 2.0 * POPUP_PADDING;
        let bar = Rect::new(
            at.x + POPUP_PADDING,
            at.y + POPUP_PADDING,
            bar_w,
            BAR_HEIGHT,
        );
        let filter = FilterOptions {
            placeholder: opts.placeholder,
            label: opts.placeholder.is_empty().then_some(opts.name),
            clear_unfocusable: false,
        };
        let edit = ui.filter_bar("search", bar, search, &filter);
        if edit.response.changed {
            shown = matches(c.items, search);
            *lists += 1;
        }
        let top = bar.bottom() + BAR_GAP;
        let height = (at.bottom() - POPUP_PADDING - top).max(0.0);
        let list = Rect::new(bar.x, top, bar_w, height);
        let clicked = rows(ui, list, &shown, *lists, c);
        clicked.or(shown.first().copied().filter(|_| edit.submitted))
    });
    if st.side.is_none() {
        // `flipped` says the popup sits opposite the side it asked for, which was the bottom.
        st.side = Some(if placement.flipped {
            Side::Top
        } else {
            Side::Bottom
        });
    }
    (frame.dismissed.is_some(), picked)
}
