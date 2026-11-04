//! The `drag-and-drop` page: a strip of tabs reordered by the pointer (`infra.drag-and-drop`),
//! through [`ReorderDrag`] and `Ui::drag_gesture`. The page plays the app: it owns the tab order
//! and applies a `Commit` to it. Tabs are 100, 200 and 100 px wide with a 5 px gap, the numbers of
//! the state machine's own tests, so a scene's offsets can be read against them.

use lune_ui::access::{Node, Role};
use lune_ui::drag::{DropResult, Metrics, ReorderDrag, Strip};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::{Sense, Ui};

use super::{card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "drag-and-drop",
    title: "Drag and drop",
    kind: PageKind::Ui(UiPage {
        state: tabs,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// The strip's left edge and top in window coordinates, inside the card under the heading.
const ORIGIN: f64 = 34.0;
const TOP: f32 = 84.0;
const HEIGHT: f32 = 32.0;
const SPACING: f64 = 5.0;
const VIEWPORT: f64 = 500.0;

/// The content x of each tab, unshifted.
fn layout_x(widths: &[f64]) -> Vec<f64> {
    let mut x = Vec::with_capacity(widths.len());
    let mut at = 0.0;
    for w in widths {
        x.push(at);
        at += w + SPACING;
    }
    x
}

/// The strip's page state: tab keys in order, each key's width, and the scroll offset.
struct Tabs {
    drag: ReorderDrag,
    keys: Vec<u32>,
    widths: Vec<f64>,
    offset: f64,
}

fn tabs() -> Box<dyn PageState> {
    Box::new(Tabs {
        drag: ReorderDrag::new(),
        keys: vec![0, 1, 2],
        widths: vec![100.0, 200.0, 100.0],
        offset: 0.0,
    })
}

impl PageState for Tabs {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Drag and drop");
        heading(ui, "TABS", card.x + 24.0, card.y + 24.0);
        let theme = *ui.theme();
        let widths: Vec<f64> = self.keys.iter().map(|k| self.widths[*k as usize]).collect();
        let x = layout_x(&widths);
        let extent = x.last().zip(widths.last()).map_or(0.0, |(x, w)| x + w);

        // The strip's track, then each tab's hit region and node.
        let track = Rect::new(
            ORIGIN as f32 - 4.0,
            TOP - 4.0,
            VIEWPORT as f32 + 8.0,
            HEIGHT + 8.0,
        );
        ui.painter().fill(track, 8.0, theme.surface_input);
        let mut resps = Vec::new();
        let mut rects = Vec::new();
        for (i, key) in self.keys.iter().enumerate() {
            let rect = Rect::new(
                (ORIGIN + x[i] - self.offset) as f32,
                TOP,
                widths[i] as f32,
                HEIGHT,
            );
            let id = ui.id(("tab", *key));
            let resp = ui.interact(id, rect, Sense::DRAG);
            ui.node(Node::new(id, Role::Tab, &format!("Tab {}", key + 1), rect));
            resps.push(resp);
            rects.push(rect);
        }

        // Every gesture before any offset, so the offsets are this frame's.
        let metrics = Metrics {
            widths: &widths,
            spacing: SPACING,
            viewport: VIEWPORT,
            offset: self.offset,
            extent,
        };
        let mut commit = None;
        let mut scroll = 0.0;
        for (i, key) in self.keys.iter().enumerate() {
            let strip = Strip {
                index: i,
                origin: ORIGIN,
                metrics,
            };
            let id = ui.id(("tab", *key));
            let frame = ui.drag_gesture(id, &resps[i], &mut self.drag, &strip);
            if let DropResult::Commit { from, to } = frame.result {
                commit = Some((from, to));
            }
            scroll += frame.scroll_by;
        }
        let mut shifts = Vec::new();
        for (i, key) in self.keys.iter().enumerate() {
            let id = ui.id(("tab", *key));
            shifts.push(ui.drag_offset(id, &self.drag, i, x[i] as f32));
        }

        // Paint: the dragged tab last, so it rides over its neighbours.
        let dragged = self.drag.dragged_index();
        let order = (0..self.keys.len())
            .filter(|i| Some(*i) != dragged)
            .chain(dragged);
        for i in order {
            let rect = Rect {
                x: rects[i].x + shifts[i],
                ..rects[i]
            };
            let lifted = Some(i) == dragged;
            let p = ui.painter();
            p.fill(
                rect,
                6.0,
                if lifted {
                    theme.surface_selected
                } else {
                    theme.surface_hover
                },
            );
            if lifted {
                p.ring(rect, 6.0, 1.0, theme.accent, false);
            }
            let spec = Spec::plain(
                format!("Tab {}", self.keys[i] + 1),
                Style::ui(theme.font_base, theme.fg_primary),
            );
            p.text_centered(&spec, rect);
        }

        if let Some((from, to)) = commit {
            let key = self.keys.remove(from);
            self.keys.insert(to, key);
        }
        if scroll != 0.0 {
            self.offset = (self.offset + scroll).clamp(0.0, (extent - VIEWPORT).max(0.0));
        }
    }
}

/// The window point `grab_in` px into the tab at content x `at`.
const fn grab(at: f32, grab_in: f32) -> (f32, f32) {
    (ORIGIN as f32 + at + grab_in, TOP + HEIGHT / 2.0)
}

const SCENES: &[UiScene] = &[
    // Tab 1 (100 wide) is pressed 50 px in, the pointer travels 4 px (the threshold, which also
    // fixes the grab point at 54 px into the tab: the machine re-takes it while armed) and then
    // on until the tab's left edge is at content x 300: the target slot is 2 and the other tabs
    // sit 105 px left of where they were. Captured after the neighbours' slide (180 ms) has ended;
    // the pointer stays down.
    UiScene {
        id: "dnd-dragging",
        scale: 2,
        tier: MotionTier::Full,
        state: tabs,
        script: &[
            Step {
                at_ms: 0,
                events: &[
                    Ev::PointerAt(grab(0.0, 50.0).0, grab(0.0, 50.0).1),
                    Ev::Press,
                ],
            },
            Step {
                at_ms: 10,
                events: &[Ev::PointerAt(grab(0.0, 54.0).0, grab(0.0, 54.0).1)],
            },
            Step {
                at_ms: 20,
                events: &[Ev::PointerAt(grab(300.0, 54.0).0, grab(300.0, 54.0).1)],
            },
        ],
        capture_ms: 400,
        expect: Expect::Settled,
    },
];

/// What the bench replays: a tab dragged across the strip and released.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[
            Ev::PointerAt(grab(0.0, 50.0).0, grab(0.0, 50.0).1),
            Ev::Press,
        ],
    },
    Step {
        at_ms: 50,
        events: &[Ev::PointerAt(grab(0.0, 54.0).0, grab(0.0, 54.0).1)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::PointerAt(grab(150.0, 54.0).0, grab(150.0, 54.0).1)],
    },
    Step {
        at_ms: 200,
        events: &[Ev::PointerAt(grab(300.0, 54.0).0, grab(300.0, 54.0).1)],
    },
    Step {
        at_ms: 400,
        events: &[Ev::Release],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::State;
    use lune_ui::testing::FakeText;
    use lune_ui::ui::WidgetId;

    #[test]
    fn the_dragging_scene_drags_tab_1_and_marks_its_node_pressed() {
        let scene = SCENES
            .iter()
            .find(|s| s.id == "dnd-dragging")
            .expect("scene");
        let (_, out) = run_scene(PAGE.id, scene, &mut FakeText).expect("runs");
        let id = WidgetId::ROOT.with(PAGE.id).with(("tab", 0u32));
        let node = out.tree.get(id).expect("tab 1 node");
        assert!(node.states.contains(State::Pressed));
    }
}
