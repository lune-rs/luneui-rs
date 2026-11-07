//! The effects lab: what the window chrome does besides the glass layers, drawn live through
//! `ui.painter()`: the custom-draw slot (app-owned GPU drawing clipped to a rect), a window with
//! glass and with glass off, and the tab outline that wraps the active tab and the
//! card body.

use lune_ui::chrome::{self, TabSpan};
use lune_ui::paint::{LayerId, Rect, StrokeStyle};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, ButtonOptions};

use super::text;
use crate::page::{Page, PageKind, PageState, Stage, UiPage};
use crate::pages::{caption, card, heading};
use crate::scenes::card_in;
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "lab-effects",
    title: "Effects lab",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: &[],
        bench: &[],
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Effects { tab: 1 })
}

/// The tabs of the outline figure, left to right, and the width of each.
const TABS: [(&str, &str); 3] = [
    ("tab-0", "Overview"),
    ("tab-1", "Changes"),
    ("tab-2", "History"),
];
const TAB_W: f32 = 150.0;
/// The height of the tab header, and the size of the figure under it.
const SEAM: f32 = 32.0;
const FIGURE: (f32, f32) = (620.0, 180.0);
/// The two windows: the label, glass on.
const WINDOWS: [(&str, bool); 2] = [("Glass", true), ("Glass off", false)];
const WINDOW: (f32, f32) = (200.0, 140.0);
const WINDOW_GAP: f32 = 16.0;

struct Effects {
    /// The tab the outline wraps.
    tab: usize,
}

/// The span of tab `i` along the figure's top edge.
fn span(i: usize) -> TabSpan {
    TabSpan {
        left: i as f32 * TAB_W,
        right: (i + 1) as f32 * TAB_W,
    }
}

/// Where window `i` of the row lies.
fn window(x: f32, y: f32, i: usize) -> Rect {
    Rect::new(
        x + 300.0 + i as f32 * (WINDOW.0 + WINDOW_GAP),
        y + 20.0,
        WINDOW.0,
        WINDOW.1,
    )
}

impl PageState for Effects {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Effects lab");
        let t = *ui.theme();
        let (x, y) = (card.x + 24.0, card.y + 24.0);

        heading(ui, "CUSTOM DRAW SLOT", x, y);
        let slot = Rect::new(x, y + 20.0, 270.0, WINDOW.1);
        {
            // App-owned drawing: the fill lands inside the slot's clip, under the layer's text.
            let p = ui.painter();
            p.custom(0, slot);
            p.ring(slot.inset(-1.0, -1.0), 2.0, 1.0, t.border, true);
        }
        caption(
            ui,
            "Drawn by the app, clipped to the slot",
            x,
            slot.bottom() + 8.0,
        );

        heading(ui, "WINDOW GLASS", x + 300.0, y);
        // Stripes behind show the glass for what it is; the windows are on the popup layer.
        let stripes = Rect::new(
            x + 290.0,
            y + 14.0,
            3.0 * WINDOW.0 + 2.0 * WINDOW_GAP + 20.0,
            152.0,
        );
        {
            let p = ui.painter();
            stage.on(p, LayerId::Page);
            let n = (stripes.w / 20.0).ceil() as usize;
            for i in 0..n {
                let c = if i % 2 == 0 {
                    [0xF2, 0xA6, 0x5A, 0xFF]
                } else {
                    [0x4D, 0xA3, 0xFF, 0xFF]
                };
                let w = 20.0f32.min(stripes.w - i as f32 * 20.0);
                p.fill(
                    Rect::new(stripes.x + i as f32 * 20.0, stripes.y, w, stripes.h),
                    0.0,
                    c,
                );
            }
            stage.on(p, LayerId::Popup);
            for (i, (name, glass)) in WINDOWS.iter().enumerate() {
                let r = window(x, y, i);
                card_in(p, &t, *glass, r);
                let title = Spec::plain(*name, Style::ui(t.font_base, t.fg_primary).medium());
                p.text(&title, r.x + 14.0, r.y + 14.0);
                let note = Spec::plain(
                    "Window, panel and card",
                    Style::ui(t.font_s, t.fg_secondary),
                );
                p.text(&note, r.x + 14.0, r.y + 38.0);
            }
            stage.on(p, LayerId::Base);
        }

        heading(ui, "TAB OUTLINE", x, y + 196.0);
        let figure = Rect::new(x, y + 220.0, FIGURE.0, FIGURE.1);
        let outline = chrome::outline(figure.w, figure.h, SEAM, Some(span(self.tab)));
        let points: Vec<[f32; 2]> = outline
            .iter()
            .map(|q| [q[0] + figure.x, q[1] + figure.y])
            .collect();
        ui.painter()
            .stroke(&points, true, StrokeStyle::solid(t.border, chrome::STROKE));
        for (i, (key, name)) in TABS.iter().enumerate() {
            let r = Rect::new(figure.x + i as f32 * TAB_W, figure.y, TAB_W, SEAM);
            let kind = if i == self.tab {
                ButtonKind::Secondary
            } else {
                ButtonKind::Ghost
            };
            let options = ButtonOptions {
                kind,
                small: false,
                icon: None,
            };
            if ui.button_with(key, r, name, &options).clicked {
                self.tab = i;
            }
        }
        text(
            ui,
            "One outline wraps the active tab and the body;",
            figure.x + 24.0,
            figure.y + SEAM + 24.0,
            t.font_base,
            t.fg_secondary,
        );
        text(
            ui,
            "press a tab to move the gap under it.",
            figure.x + 24.0,
            figure.y + SEAM + 46.0,
            t.font_base,
            t.fg_secondary,
        );
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        let names: Vec<&str> = TABS.iter().map(|(_, n)| *n).collect();
        panel.choice(ui, "Active tab", &names, &mut self.tab);
    }
}

#[cfg(test)]
mod tests {
    use super::super::rig::{Themed, player};
    use super::*;
    use crate::page::Ev;
    use lune_ui::lunegit_dark::THEME;
    use lune_ui::testing::FakeText;

    #[test]
    fn the_slot_and_the_two_windows_are_drawn() {
        let mut p = player("lab-effects", Box::new(Themed(live(), THEME)));
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        let (scene, _) = p.frame(&mut FakeText, 16, &[]).expect("frame");
        let customs: usize = scene.in_order().into_iter().map(|l| l.customs.len()).sum();
        assert_eq!(customs, 1, "the custom-draw slot");
        let popup = scene.merged(LayerId::Popup);
        let windows: Vec<_> = popup
            .geometry
            .under
            .iter()
            .filter(|s| s.is_frost())
            .collect();
        assert_eq!(windows.len(), 2);
        // Glass on shows the glass layer; glass off is opaque.
        assert_eq!(windows[0].color, THEME.glass.card.premultiplied());
        assert!(
            (windows[1].color[3] - 1.0).abs() < 1e-4,
            "{:?}",
            windows[1].color
        );
        assert_eq!(windows[0].params[3], 1.0);
    }

    /// A press on a tab moves the outline's gap under it.
    #[test]
    fn a_tab_moves_the_outline() {
        let strokes = |scene: &lune_ui::scene::Scene| {
            scene
                .in_order()
                .into_iter()
                .flat_map(|l| l.geometry.strokes.clone())
                .collect::<Vec<_>>()
        };
        let mut p = player("lab-effects", live());
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        let before = strokes(&p.frame(&mut FakeText, 16, &[]).expect("frame").0);
        assert!(!before.is_empty(), "the outline is drawn");
        p.frame(&mut FakeText, 32, &[Ev::PointerOver(&["tab-2"]), Ev::Click])
            .expect("frame");
        let after = strokes(&p.frame(&mut FakeText, 48, &[]).expect("frame").0);
        assert_ne!(before, after, "the notch moved under the third tab");
        // Pressing the same tab again changes nothing.
        p.frame(&mut FakeText, 64, &[Ev::PointerOver(&["tab-2"]), Ev::Click])
            .expect("frame");
        assert_eq!(
            after,
            strokes(&p.frame(&mut FakeText, 80, &[]).expect("frame").0)
        );
    }
}
