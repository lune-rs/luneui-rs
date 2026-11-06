//! The `file-drop` page (`core.input`, `platform.input`, M10): files dragged over the window and
//! dropped on it reach the page as `Event::FileHover`, `FileHoverCancelled` and `FileDrop` in
//! `ui.input()`. The toolkit only carries the path (D20); this page shows the one it read. In the
//! window the events are the OS's, so dragging a file from the Finder or Explorer over the window
//! lights the zone, and dropping lists the path.

use std::path::PathBuf;

use lune_ui::input::Event;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "file-drop",
    title: "File drop",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// How many dropped paths the page lists.
const KEPT: usize = 8;

/// The files dropped so far and the one being dragged over the window.
#[derive(Default)]
struct Drops {
    hover: Option<PathBuf>,
    dropped: Vec<PathBuf>,
}

impl Drops {
    /// Reads the frame's file events, in arrival order.
    fn read(&mut self, events: &[Event]) {
        for event in events {
            match event {
                Event::FileHover { path, .. } => self.hover = Some(path.clone()),
                Event::FileHoverCancelled => self.hover = None,
                Event::FileDrop { path, .. } => {
                    self.hover = None;
                    self.dropped.push(path.clone());
                    if self.dropped.len() > KEPT {
                        self.dropped.remove(0);
                    }
                }
                _ => {}
            }
        }
    }
}

impl PageState for Drops {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        self.read(&ui.input().events);
        let t = *ui.theme();
        let area = card(ui, "File drop");
        let (x, y) = (area.x + 24.0, area.y + 24.0);
        heading(ui, "DROP ZONE", x, y);
        caption(
            ui,
            "Drag a file over the window and let go. The toolkit hands the page the path and opens nothing.",
            x,
            y + 20.0,
        );
        let zone = Rect::new(x, y + 52.0, area.w - 48.0, 150.0);
        let lit = self.hover.is_some();
        {
            let p = ui.painter();
            if lit {
                p.fill(zone, t.item_radius, t.surface_hover);
            }
            let edge = if lit { t.accent } else { t.border };
            p.ring(zone, t.item_radius, 2.0, edge, false);
            let (title, color) = if lit {
                ("Release to drop", t.fg_primary)
            } else {
                ("Drop files here", t.fg_secondary)
            };
            let spec = Spec::plain(title, Style::ui(t.font_m, color).medium());
            p.text_centered(&spec, Rect::new(zone.x, zone.y + 24.0, zone.w, 30.0));
        }
        if let Some(path) = &self.hover {
            let text = format!("over the window: {}", path.display());
            let spec = Spec::plain(text, Style::ui(t.font_base, t.fg_primary));
            let spec = ui.painter().elide(&spec, zone.w - 32.0);
            ui.painter().text(&spec, zone.x + 16.0, zone.y + 76.0);
        }
        heading(ui, "DROPPED", x, zone.bottom() + 24.0);
        if self.dropped.is_empty() {
            caption(ui, "nothing yet", x, zone.bottom() + 46.0);
        }
        for (i, path) in self.dropped.iter().enumerate() {
            let spec = Spec::plain(
                path.display().to_string(),
                Style::ui(t.font_base, t.fg_primary),
            );
            let spec = ui.painter().elide(&spec, area.w - 48.0);
            ui.painter()
                .text(&spec, x, zone.bottom() + 46.0 + i as f32 * 22.0);
        }
    }
}

fn live() -> Box<dyn PageState> {
    Box::new(Drops::default())
}

const FILE: &str = "/Users/demo/notes/report.md";

const SCENES: &[UiScene] = &[
    // A file is over the window, the pointer at (300, 200).
    UiScene {
        id: "file-drop-hover",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::PointerAt(300.0, 200.0), Ev::FileHover(FILE)],
        }],
        capture_ms: 100,
        expect: Expect::Settled,
    },
    // Hovered, then dropped.
    UiScene {
        id: "file-drop-dropped",
        scale: 2.0,
        tier: MotionTier::Full,
        state: live,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerAt(300.0, 200.0), Ev::FileHover(FILE)],
            },
            Step {
                at_ms: 100,
                events: &[Ev::FileDrop(FILE)],
            },
        ],
        capture_ms: 200,
        expect: Expect::Settled,
    },
];

/// What the bench replays: a hover, a cancel, a hover and a drop.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerAt(300.0, 200.0), Ev::FileHover(FILE)],
    },
    Step {
        at_ms: 300,
        events: &[Ev::FileHoverCancelled],
    },
    Step {
        at_ms: 600,
        events: &[Ev::FileHover(FILE)],
    },
    Step {
        at_ms: 900,
        events: &[Ev::FileDrop(FILE)],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::testing::FakeText;

    fn events(list: &[Event]) -> Drops {
        let mut d = Drops::default();
        d.read(list);
        d
    }

    #[test]
    fn a_hover_is_kept_until_it_is_cancelled_or_dropped() {
        let over = Event::FileHover {
            pos: (1.0, 2.0),
            path: PathBuf::from(FILE),
        };
        assert_eq!(
            events(std::slice::from_ref(&over)).hover,
            Some(PathBuf::from(FILE))
        );
        let cancelled = events(&[over.clone(), Event::FileHoverCancelled]);
        assert_eq!(cancelled.hover, None);
        let dropped = events(&[
            over,
            Event::FileDrop {
                pos: (1.0, 2.0),
                path: PathBuf::from(FILE),
            },
        ]);
        assert_eq!((dropped.hover, dropped.dropped), (None, vec![FILE.into()]));
    }

    #[test]
    fn the_list_keeps_the_last_few() {
        let mut d = Drops::default();
        for i in 0..KEPT + 3 {
            d.read(&[Event::FileDrop {
                pos: (0.0, 0.0),
                path: PathBuf::from(format!("/f{i}")),
            }]);
        }
        assert_eq!(d.dropped.len(), KEPT);
        assert_eq!(d.dropped[0], PathBuf::from("/f3"));
    }

    /// The path the scene shows is the one the script sent, read from `ui.input()`: the dropped
    /// scene's frame differs from the hover scene's, and both differ from an empty page.
    #[test]
    fn the_scenes_show_what_the_input_carried() {
        let frame = |scene: &UiScene| {
            run_scene("file-drop", scene, &mut FakeText)
                .expect("runs")
                .0
        };
        let (hover, dropped) = (frame(&SCENES[0]), frame(&SCENES[1]));
        let empty = frame(&UiScene {
            script: &[],
            ..SCENES[0]
        });
        assert!(hover != dropped && hover != empty && dropped != empty);
    }
}
