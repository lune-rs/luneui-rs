//! The glass lab: the window, panel and card layers side by side, each over black and white
//! stripes and over bands of colour, so how much shows through is plain to see. The layers are
//! the theme's own (`Theme::glass`), so the settings pane's tint, tint opacity, material opacity
//! and acrylic level change them here at once. Below, each layer's numbers.

use lune_ui::color::premul;
use lune_ui::paint::{LayerId, Painter, Rect};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::{Ui, WidgetId};

use super::{hex, text};
use crate::page::{Page, PageKind, PageState, Stage, UiPage};
use crate::pages::{caption, card, heading};
use crate::showcase::knobs::Knobs;

pub(crate) static PAGE: Page = Page {
    id: "lab-glass",
    title: "Glass lab",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: &[],
        bench: &[],
    }),
};

fn live() -> Box<dyn PageState> {
    Box::new(Glass { panel_h: PANEL_H })
}

/// The width of a layer's panel and the gap between two: three panels and their margins make
/// 960, what the content pane shows of the canvas in a 1,600 window.
const COL_W: f32 = 296.0;
const GAP: f32 = 24.0;
/// The height of a band of content, and of a panel on it.
const BAND_H: f32 = 150.0;
const PANEL_H: f32 = 120.0;
/// The width of a stripe.
const STRIPE: f32 = 20.0;

/// Bands of colour, left to right.
const COLOURS: [[u8; 4]; 6] = [
    [0xE5, 0x48, 0x4D, 0xFF],
    [0xF2, 0xA6, 0x5A, 0xFF],
    [0x73, 0xC9, 0x91, 0xFF],
    [0x4D, 0xA3, 0xFF, 0xFF],
    [0xB0, 0x7A, 0xE8, 0xFF],
    [0xF0, 0xF0, 0xF0, 0xFF],
];

/// The three layers; the knob sets how tall the panels are.
struct Glass {
    panel_h: f32,
}

/// Black and white stripes across `band`.
fn stripes(p: &mut Painter<'_, WidgetId>, band: Rect) {
    let n = (band.w / STRIPE).ceil() as usize;
    for i in 0..n {
        let c = if i % 2 == 0 {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        };
        let w = STRIPE.min(band.w - i as f32 * STRIPE);
        p.fill(
            Rect::new(band.x + i as f32 * STRIPE, band.y, w, band.h),
            0.0,
            c,
        );
    }
}

/// Six bands of colour across `band`.
fn colours(p: &mut Painter<'_, WidgetId>, band: Rect) {
    let w = band.w / COLOURS.len() as f32;
    for (i, c) in COLOURS.iter().enumerate() {
        p.fill(Rect::new(band.x + i as f32 * w, band.y, w, band.h), 0.0, *c);
    }
}

/// Where the three bands of content and their panels lie for a card at (`x`, `y`): the stripes
/// and the colour, and the panel of each layer on each.
fn bands(x: f32, y: f32) -> [Rect; 2] {
    let w = 3.0 * COL_W + 2.0 * GAP + 24.0;
    [
        Rect::new(x, y + 20.0, w, BAND_H),
        Rect::new(x, y + 206.0, w, BAND_H),
    ]
}

/// The panel of layer `i` on `band`.
fn panel(band: Rect, i: usize, height: f32) -> Rect {
    Rect::new(
        band.x + 12.0 + i as f32 * (COL_W + GAP),
        band.y + (band.h - height) / 2.0,
        COL_W,
        height,
    )
}

impl PageState for Glass {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let card = card(ui, stage, "Glass lab");
        let t = *ui.theme();
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        let layers = [
            ("Window", t.glass.window),
            ("Panel", t.glass.panel),
            ("Card", t.glass.card),
        ];
        let [stripe_band, colour_band] = bands(x, y);
        // A glass layer replaces what is under it within its own layer, so the content is on the
        // page layer (over the card's own glass) and the panels on the popup layer above it.
        let under = premul([0x70, 0x70, 0x70, 0xFF]);
        {
            let p = ui.painter();
            stage.on(p, LayerId::Page);
            stripes(p, stripe_band);
            colours(p, colour_band);
            stage.on(p, LayerId::Popup);
            for band in [stripe_band, colour_band] {
                for (i, (name, layer)) in layers.iter().enumerate() {
                    let r = panel(band, i, self.panel_h);
                    p.acrylic(r, 8.0, layer.premultiplied(), under, true);
                    let label = Spec::plain(*name, Style::ui(t.font_m, t.fg_primary).medium());
                    p.text_centered(&label, r);
                }
            }
            // Back on the layer the card is on, for the text and widgets that follow.
            stage.on(p, LayerId::Base);
        }
        heading(ui, "THE THREE LAYERS OVER BLACK AND WHITE", x, y);
        heading(ui, "AND OVER COLOUR", x, y + 186.0);
        heading(ui, "WHAT EACH LAYER IS MADE OF", x, y + 376.0);
        for (i, (name, layer)) in layers.iter().enumerate() {
            let cx = x + 12.0 + i as f32 * (COL_W + GAP);
            let c = layer.acrylic_colors();
            let result = layer.premultiplied();
            text(ui, name, cx, y + 398.0, t.font_base, t.fg_primary);
            let lines = [
                format!("tint {}", hex(layer.tint)),
                format!("tint opacity {:.2}", layer.tint_opacity),
                format!("material opacity {:.2}", layer.material_opacity),
                format!("tint over grey {}", hex(c.luminosity)),
                format!("covers {:.0}% of what is below", result[3] * 100.0),
            ];
            for (n, line) in lines.iter().enumerate() {
                text(
                    ui,
                    line,
                    cx,
                    y + 420.0 + n as f32 * 16.0,
                    t.font_s,
                    t.fg_secondary,
                );
            }
        }
        caption(
            ui,
            "Change the tint and the opacities in the settings pane.",
            x,
            y + 520.0,
        );
    }

    fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
        panel.number(ui, "Panel height", &mut self.panel_h, (40.0, BAND_H, 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::super::rig::{Themed, player};
    use super::*;
    use lune_ui::lunegit_dark::THEME;
    use lune_ui::scene::Scene;
    use lune_ui::testing::FakeText;

    fn scene_under(theme: lune_ui::theme::Theme) -> Scene {
        let mut p = player("lab-glass", Box::new(Themed(live(), theme)));
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        p.frame(&mut FakeText, 16, &[]).expect("frame").0
    }

    /// Six panels on the popup layer, in the three layers' colours, each over a band of content on
    /// the page layer.
    #[test]
    fn the_three_layers_are_drawn_over_content() {
        let scene = scene_under(THEME);
        let page = &scene.merged(LayerId::Popup).geometry.acrylic;
        let colours: Vec<[f32; 4]> = page.iter().map(|a| a.shape.color).collect();
        assert_eq!(colours.len(), 6);
        for (i, layer) in [THEME.glass.window, THEME.glass.panel, THEME.glass.card]
            .iter()
            .enumerate()
        {
            assert_eq!(colours[i], layer.premultiplied(), "the stripes' panel {i}");
            assert_eq!(
                colours[i + 3],
                layer.premultiplied(),
                "the colours' panel {i}"
            );
        }
        let content = &scene.layers[LayerId::Page as usize].geometry.under;
        assert!(content.len() > 50, "stripes and bands under the panels");
    }

    /// The glass is the theme's, so a changed layer changes the panel.
    #[test]
    fn the_panels_follow_the_themes_glass() {
        let mut theme = THEME;
        theme.glass.panel.tint = [9, 80, 200, 255];
        theme.glass.panel.material_opacity = 0.2;
        let scene = scene_under(theme);
        let page = &scene.merged(LayerId::Popup).geometry.acrylic;
        assert_eq!(page[1].shape.color, theme.glass.panel.premultiplied());
        assert_ne!(page[1].shape.color, THEME.glass.panel.premultiplied());
    }

    #[test]
    fn the_panels_lie_inside_the_card() {
        let [a, b] = bands(34.0, 58.0);
        for band in [a, b] {
            for i in 0..3 {
                let r = panel(band, i, PANEL_H);
                assert!(
                    r.x >= band.x && r.right() <= band.right(),
                    "{r:?} in {band:?}"
                );
                assert!(r.y >= band.y && r.bottom() <= band.bottom());
            }
        }
        // The bands do not touch, and the numbers (heading at 376 from the origin) start below.
        assert!(a.bottom() < b.y && b.bottom() < 58.0 + 376.0);
    }
}
