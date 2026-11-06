//! The `list-row` page: a list of rows with a status dot, a badge, a value and a sub-line on the
//! virtualized list (window mode and the mid-transition scenes), and a specimen sheet of every
//! state painted through the public `paint`.

use lune_ui::motion::MotionTier;
use lune_ui::paint::{Rect, Xform};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::focus_ring::{self, RingKind};
use lune_ui::widgets::list_row::{ListRow, SIDE_PANEL_PADDING, paint};
use lune_ui::widgets::tone::Tone;
use lune_ui::widgets::virtual_list::{
    HIGHLIGHT_INSET, Heights, ListOptions, RowKey, SelectionMode, row_fill,
};

use super::{card, heading, on_card};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "list-row",
    title: "List row",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: &[],
    }),
};

/// The list in window coordinates, under the card's heading.
const LIST: Rect = Rect::new(34.0, 82.0, 360.0, 440.0);

/// What row `i` shows: a name, a value, a sub-line, a meta, a badge and its tone.
const DATA: [(&str, &str, &str, &str, &str, Tone); 6] = [
    (
        "main",
        "a1b2c3d",
        "2 commits ahead",
        "3 h",
        "open",
        Tone::Ok,
    ),
    (
        "release/1.4",
        "9f8e7d6",
        "merge pending",
        "1 d",
        "review",
        Tone::Data,
    ),
    (
        "feature/glass",
        "4c3b2a1",
        "rebase needed",
        "2 d",
        "stale",
        Tone::Warn,
    ),
    (
        "fix/crash-on-start",
        "0d1e2f3",
        "checks failed",
        "5 d",
        "failed",
        Tone::Err,
    ),
    (
        "docs/readme",
        "7a6b5c4",
        "up to date",
        "1 w",
        "merged",
        Tone::Neutral,
    ),
    (
        "chore/deps",
        "b2c3d4e",
        "1 commit behind",
        "2 w",
        "paused",
        Tone::Paused,
    ),
];

fn row_of(i: usize) -> ListRow<'static> {
    let (name, value, sub, meta, badge, tone) = DATA[i % DATA.len()];
    ListRow {
        name,
        value: Some(value),
        sub: Some(sub),
        meta: Some(meta),
        badge: Some((badge, tone)),
        dot: Some(tone),
    }
}

/// The height of a row with every part: the inset 2, the padding 12, the three lines (head, sub, meta) and two gaps
/// (the scenes' pointer positions are consts, so the height is one too; a test pins it).
const ROW_H: f32 = 74.0;

fn row_height() -> f32 {
    ROW_H
}

fn key(i: usize) -> RowKey {
    RowKey(i as u64)
}

fn live() -> Box<dyn PageState> {
    Box::new(Live)
}

fn specimens() -> Box<dyn PageState> {
    Box::new(Specimens)
}

/// A list of 200 rows.
struct Live;

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "List row");
        let list = on_card(card, LIST);
        heading(ui, "LIST ROWS", card.x + 24.0, card.y + 24.0);
        let opts = ListOptions {
            count: 200,
            heights: Heights::Fixed(row_height()),
            selection: SelectionMode::Single,
            index_of: None,
            label: "Branches",
        };
        ui.virtual_list("list", list, &opts, key, |ui, ctx| {
            ui.list_row(ctx, &row_of(ctx.index), SIDE_PANEL_PADDING)
        });
    }
}

/// The states, each drawn with the list's own fill and ring around the public `paint`.
struct Specimens;

impl PageState for Specimens {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "List row states");
        heading(
            ui,
            "REST, HOVER, SELECTED, SELECTED + HOVER, FOCUS-VISIBLE, DISABLED",
            card.x + 24.0,
            card.y + 24.0,
        );
        let list = on_card(card, LIST);
        let theme = *ui.theme();
        let h = row_height();
        // Name, hovered, selected, focus-visible, disabled.
        let cases = [
            ("Rest", false, false, false, false),
            ("Hover", true, false, false, false),
            ("Selected", false, true, false, false),
            ("Selected + hover", true, true, false, false),
            ("Focus-visible", false, false, true, false),
            ("Disabled", false, false, false, true),
        ];
        for (i, (name, hovered, selected, focus, disabled)) in cases.into_iter().enumerate() {
            let slot = Rect::new(list.x, list.y + i as f32 * (h + 4.0), list.w, h);
            let p = ui.painter();
            if disabled {
                p.push_xform(Xform {
                    origin: (0.0, 0.0),
                    scale: 1.0,
                    dx: 0.0,
                    dy: 0.0,
                    alpha: 0.4,
                });
            }
            let highlight = slot.inset(HIGHLIGHT_INSET.0, HIGHLIGHT_INSET.1);
            let fill = row_fill(&theme, hovered, selected);
            if fill[3] != 0 {
                p.fill(highlight, theme.item_radius, fill);
            }
            let color = if selected {
                theme.fg_bright
            } else {
                theme.fg_secondary
            };
            paint(p, slot, &row_of(i), SIDE_PANEL_PADDING, color);
            if focus {
                focus_ring::paint(p, highlight, RingKind::Button, 1.0);
            }
            if disabled {
                p.pop_xform();
            }
            let caption = Spec::plain(name, Style::ui(theme.font_s, theme.fg_secondary));
            ui.painter()
                .text(&caption, slot.right() + 16.0, slot.y + h / 2.0 - 8.0);
        }
    }
}

/// The middle of row `i` while the list is at offset 0, clear of the badge.
const fn row_at(i: usize) -> (f32, f32) {
    (LIST.x + 160.0, LIST.y + (i as f32 + 0.5) * ROW_H)
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "list-row-badge-states",
        scale: 2.0,
        tier: MotionTier::Full,
        state: specimens,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Hover is `instant` (120 ms): half way at 60 ms.
    UiScene {
        id: "list-row-badge-hover-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(row_at(2).0, row_at(2).1)],
        }],
        capture_ms: 60,
        expect: Expect::Moving,
    },
    // Selecting is `selection` (180 ms): half way at 90 ms.
    UiScene {
        id: "list-row-badge-select-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(row_at(2).0, row_at(2).1), Ev::Click],
        }],
        capture_ms: 90,
        expect: Expect::Moving,
    },
    // Tab focuses the list and its ring shows on the first row. The list draws the ring at full
    // opacity at once (it has no fade of its own), so the frame at 90 ms is already settled.
    UiScene {
        id: "list-row-badge-focus-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::Tab],
        }],
        capture_ms: 90,
        expect: Expect::Settled,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_row_height_const_is_the_widgets() {
        assert_eq!(
            ROW_H,
            lune_ui::widgets::list_row::height(&row_of(0), SIDE_PANEL_PADDING)
        );
    }
}
