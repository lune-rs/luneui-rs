//! The blur lab: a glass panel over content that moves, so what the backdrop blur does to edges
//! and colour is visible frame by frame. Sigma and saturation are the frame's, set in the
//! settings pane. The lab owns the frame's one backdrop; the shell asks for none.

use std::time::Duration;

use lune_ui::paint::{LayerId, Painter, Rect};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::{Ui, WidgetId};

use crate::page::{Page, PageKind, PageState, Stage, UiPage};
use crate::pages::{caption, card, heading};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "lab-blur",
    title: "Blur lab",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: &[],
        bench: &[],
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Blur::new())
}

/// The size of the area of content, and of the panel over it.
const AREA: (f32, f32) = (960.0, 440.0);
const PANEL: (f32, f32) = (360.0, 220.0);
/// A stripe's width; a pair (black, white) is twice that.
const STRIPE: f32 = 16.0;
/// How far the stripes travel in a second, and the circles' laps per second.
const SPEED: f32 = 60.0;
const LAP: f32 = 0.3;

const CIRCLES: [([u8; 4], f32, f32); 3] = [
    ([0xE5, 0x48, 0x4D, 0xFF], 44.0, 0.0),
    ([0x4D, 0xA3, 0xFF, 0xFF], 60.0, 0.37),
    ([0xF2, 0xA6, 0x5A, 0xFF], 36.0, 0.71),
];

struct Blur {
    /// The panel's size.
    panel: (f32, f32),
    /// The content moves.
    moving: bool,
    /// The panel goes where the pointer is.
    follow: bool,
    /// Seconds of motion so far; it stops with `moving` and goes on from there.
    phase: f32,
    last: Option<Duration>,
}

impl Blur {
    fn new() -> Self {
        Self {
            panel: PANEL,
            moving: true,
            follow: false,
            phase: 0.0,
            last: None,
        }
    }

    /// Takes the time since the last frame into the phase while the content moves.
    fn tick(&mut self, now: Duration) {
        if let Some(last) = self.last
            && self.moving
        {
            self.phase += now.saturating_sub(last).as_secs_f32();
        }
        self.last = Some(now);
    }
}

/// 0 to 1 and back, as `t` grows.
fn triangle(t: f32) -> f32 {
    let f = t.rem_euclid(2.0);
    if f < 1.0 { f } else { 2.0 - f }
}

/// The content in `area` at `phase`: stripes sliding right, circles going round, a line of text.
fn content(p: &mut Painter<'_, WidgetId>, area: Rect, phase: f32, ink: [u8; 4], size: f32) {
    p.push_clip(area);
    let pair = 2.0 * STRIPE;
    let shift = (phase * SPEED).rem_euclid(pair);
    let n = (area.w / STRIPE).ceil() as usize + 2;
    for i in 0..n {
        let c = if i % 2 == 0 {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        };
        let x = area.x - pair + shift + i as f32 * STRIPE;
        p.fill(Rect::new(x, area.y, STRIPE, area.h), 0.0, c);
    }
    for (colour, d, offset) in CIRCLES {
        let x = area.x + triangle(phase * LAP + offset) * (area.w - d);
        let y = area.y + triangle(phase * LAP * 1.7 + offset * 3.0) * (area.h - d);
        p.fill(Rect::new(x, y, d, d), d / 2.0, colour);
    }
    let line = Spec::plain(
        "Sharp text under a blurred panel",
        Style::ui(size, ink).medium(),
    );
    let x = area.x + (phase * 90.0).rem_euclid(area.w + 300.0) - 300.0;
    p.text(&line, x, area.y + 20.0);
    // The other way along the bottom.
    let back = area.right() - (phase * 90.0).rem_euclid(area.w + 300.0);
    p.text(&line, back, area.bottom() - 40.0);
    p.pop_clip();
}

/// Where the panel lies in `area`: its middle, or under the pointer, kept inside.
fn panel_in(area: Rect, pointer: Option<(f32, f32)>, size: (f32, f32)) -> Rect {
    let (cx, cy) = pointer.unwrap_or((area.x + area.w / 2.0, area.y + area.h / 2.0));
    let x = (cx - size.0 / 2.0).clamp(area.x, area.right() - size.0);
    let y = (cy - size.1 / 2.0).clamp(area.y, area.bottom() - size.1);
    Rect::new(x, y, size.0, size.1)
}

impl PageState for Blur {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Blur lab");
        let t = *ui.theme();
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "BACKDROP BLUR OVER MOVING CONTENT", x, y);
        ui.checkbox(
            "move",
            Rect::new(x, y + 20.0, 200.0, 24.0),
            &mut self.moving,
            "Move the content",
        );
        ui.checkbox(
            "follow",
            Rect::new(x + 220.0, y + 20.0, 260.0, 24.0),
            &mut self.follow,
            "Panel follows the pointer",
        );
        let now = ui.time();
        self.tick(now);
        let area = Rect::new(x, y + 56.0, AREA.0, AREA.1);
        let pointer = if self.follow { ui.pointer_pos() } else { None };
        let panel = panel_in(area, pointer, self.panel);
        {
            let p = ui.painter();
            stage.on(p, LayerId::Page);
            content(p, area, self.phase, t.fg_primary, t.font_m);
            // The popup layer is the first above the scene layers: it blurs the page under it.
            stage.on(p, LayerId::Popup);
            p.backdrop(stage.visible(panel), 9.0);
            p.fill(panel, 9.0, t.menu_glass);
            p.ring(panel, 9.0, 1.0, t.menu_popup_border, false);
            let label = Spec::plain("Backdrop blur", Style::ui(t.font_m, t.menu_text_primary));
            p.text_centered(&label, panel);
            stage.on(p, LayerId::Base);
        }
        caption(
            ui,
            "Sigma and saturation: Blur in the settings pane.",
            x,
            area.bottom() + 12.0,
        );
        if self.moving {
            ui.request_repaint_at(now + Duration::from_millis(16));
        }
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.number(ui, "Panel width", &mut self.panel.0, (80.0, AREA.0, 1.0));
        panel.number(ui, "Panel height", &mut self.panel.1, (60.0, AREA.1, 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::super::rig::{CANVAS, player};
    use super::*;
    use crate::page::Ev;
    use lune_ui::testing::FakeText;

    #[test]
    fn the_panel_asks_for_the_frames_one_backdrop_over_the_page() {
        let mut p = player("lab-blur", live());
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        let (scene, out) = p.frame(&mut FakeText, 16, &[]).expect("frame");
        let b = scene.backdrop.expect("a backdrop");
        assert_eq!(
            b.layer,
            LayerId::Popup as usize,
            "everything below is blurred"
        );
        // The region is the panel, inside the canvas.
        let (x, y, w, h) = (b.rect[0], b.rect[1], b.rect[2], b.rect[3]);
        assert!(
            (w - PANEL.0).abs() < 0.5 && (h - PANEL.1).abs() < 0.5,
            "{w} x {h}"
        );
        assert!(x >= CANVAS.x && x + w <= CANVAS.right() + 0.5);
        assert!(y >= CANVAS.y && y + h <= CANVAS.bottom() + 0.5);
        // It moves: a frame is due.
        assert_ne!(out.repaint, lune_ui::input::Repaint::Idle);
    }

    /// The content slides while it moves and stops when it is turned off.
    #[test]
    fn the_content_moves_until_it_is_stopped() {
        let content_at = |p: &mut crate::page::Player, ms| {
            let (scene, _) = p.frame(&mut FakeText, ms, &[]).expect("frame");
            scene.layers[LayerId::Page as usize].geometry.under.clone()
        };
        let mut p = player("lab-blur", live());
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        let (a, b) = (content_at(&mut p, 100), content_at(&mut p, 400));
        assert_ne!(a, b, "moving");
        // Press the checkbox.
        p.frame(&mut FakeText, 450, &[Ev::PointerOver(&["move"]), Ev::Click])
            .expect("frame");
        p.frame(&mut FakeText, 470, &[]).expect("frame");
        let (c, d) = (content_at(&mut p, 600), content_at(&mut p, 900));
        assert_eq!(c, d, "stopped");
    }

    #[test]
    fn the_panel_follows_the_pointer_and_stays_in_the_area() {
        let area = Rect::new(100.0, 100.0, 960.0, 440.0);
        assert_eq!(
            panel_in(area, None, PANEL).x,
            100.0 + (960.0 - PANEL.0) / 2.0
        );
        let p = panel_in(area, Some((0.0, 0.0)), PANEL);
        assert_eq!((p.x, p.y), (100.0, 100.0));
        let p = panel_in(area, Some((5000.0, 5000.0)), PANEL);
        assert_eq!((p.right(), p.bottom()), (area.right(), area.bottom()));
        let p = panel_in(area, Some((500.0, 300.0)), PANEL);
        assert_eq!((p.x + PANEL.0 / 2.0, p.y + PANEL.1 / 2.0), (500.0, 300.0));
    }
}
