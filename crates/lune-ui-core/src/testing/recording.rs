//! The display-list seam's fake: a renderer that records draw calls instead of drawing.

use crate::scene::{
    BackdropRequest, CustomItem, FrameParams, ImageItem, Scene, Shape, StrokeVertex, TextItem,
};

/// `scene` with every overlay folded into its fixed layer ([`Scene::merged`]) and the backdrop's
/// tag cleared: the scene [`Painter::on`](crate::paint::Painter::on) would have built from the same
/// draws, to compare what an overlay draws with what the Painter API draws.
pub fn flattened(scene: &Scene) -> Scene {
    use crate::scene::LayerId::{Base, Modal, Page, Popup, Raised, Toast};
    Scene {
        layers: [Base, Page, Popup, Modal, Raised, Toast].map(|id| scene.merged(id)),
        overlays: Vec::new(),
        backdrop: scene.backdrop.map(|b| BackdropRequest { tag: 0, ..b }),
    }
}

/// One draw call, in the order a renderer issues it. `layer` is the index into the
/// list of [`Scene::in_order`].
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCall {
    /// One acrylic shape, blended in at `opacity` (1.0 replaces what is below).
    Acrylic {
        layer: usize,
        shape: Shape,
        opacity: f32,
    },
    /// One shape below the layer's text.
    Under { layer: usize, shape: Shape },
    /// The layer's path-fill vertices (one draw for the whole non-empty run), below the text.
    Fills {
        layer: usize,
        vertices: Vec<StrokeVertex>,
    },
    /// One app-owned draw.
    Custom { layer: usize, item: CustomItem },
    /// One image draw, after the customs and before the text.
    Image { layer: usize, item: ImageItem },
    /// One text placement.
    Text { layer: usize, item: TextItem },
    /// One shape above the layer's text.
    Over { layer: usize, shape: Shape },
    /// The layer's stroke vertices (one draw for the whole non-empty run).
    Strokes {
        layer: usize,
        vertices: Vec<StrokeVertex>,
    },
    /// The blurred backdrop, drawn after the layers below [`Scene::split`] and before the rest.
    Backdrop(BackdropRequest),
}

impl DrawCall {
    /// The layer the call belongs to, by its place in [`Scene::in_order`] (with no overlay, the
    /// [`LayerId`](crate::scene::LayerId)'s index); `None` for the backdrop.
    pub fn layer(&self) -> Option<usize> {
        match self {
            Self::Acrylic { layer, .. }
            | Self::Under { layer, .. }
            | Self::Fills { layer, .. }
            | Self::Custom { layer, .. }
            | Self::Image { layer, .. }
            | Self::Text { layer, .. }
            | Self::Over { layer, .. }
            | Self::Strokes { layer, .. } => Some(*layer),
            Self::Backdrop(_) => None,
        }
    }
}

/// Everything recorded for one [`RecordingRenderer::render`] call.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedFrame {
    pub params: FrameParams,
    pub calls: Vec<DrawCall>,
}

/// Consumes scenes like a renderer does, in the draw order the [`scene`](crate::scene) module
/// documents, and records one [`DrawCall`] per draw: one per acrylic, under and over shape, one
/// per non-empty stroke or path-fill run, one per text item, one per custom item, and the backdrop step when
/// the scene requests one.
#[derive(Debug, Default)]
pub struct RecordingRenderer {
    frames: Vec<RecordedFrame>,
}

impl RecordingRenderer {
    /// A recorder with no frame yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one frame of `scene` drawn at `params`.
    pub fn render(&mut self, scene: &Scene, params: &FrameParams) {
        let mut calls = Vec::new();
        for (i, layer) in scene.in_order().into_iter().enumerate() {
            if i == scene.split()
                && let Some(b) = scene.backdrop
            {
                calls.push(DrawCall::Backdrop(b));
            }
            let g = &layer.geometry;
            let shapes = |v: &[Shape], f: fn(usize, Shape) -> DrawCall| {
                v.iter().map(move |s| f(i, *s)).collect::<Vec<_>>()
            };
            calls.extend(g.acrylic.iter().map(|a| DrawCall::Acrylic {
                layer: i,
                shape: a.shape,
                opacity: a.opacity,
            }));
            calls.extend(shapes(&g.under, |layer, shape| DrawCall::Under {
                layer,
                shape,
            }));
            if !g.fills.is_empty() {
                calls.push(DrawCall::Fills {
                    layer: i,
                    vertices: g.fills.clone(),
                });
            }
            calls.extend(layer.customs.iter().map(|item| DrawCall::Custom {
                layer: i,
                item: *item,
            }));
            calls.extend(layer.images.iter().map(|item| DrawCall::Image {
                layer: i,
                item: *item,
            }));
            calls.extend(layer.texts.iter().map(|item| DrawCall::Text {
                layer: i,
                item: *item,
            }));
            calls.extend(shapes(&g.over, |layer, shape| DrawCall::Over {
                layer,
                shape,
            }));
            if !g.strokes.is_empty() {
                calls.push(DrawCall::Strokes {
                    layer: i,
                    vertices: g.strokes.clone(),
                });
            }
        }
        self.frames.push(RecordedFrame {
            params: *params,
            calls,
        });
    }

    /// Every recorded frame, oldest first.
    pub fn frames(&self) -> &[RecordedFrame] {
        &self.frames
    }

    /// The most recent frame.
    pub fn last(&self) -> Option<&RecordedFrame> {
        self.frames.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{Fit, ImageAlign, ImageId};
    use crate::paint::{LayerId, Painter, Rect, StrokeStyle};
    use crate::testing::FakeText;
    use crate::text::{Spec, Style};
    use crate::theme::lunegit_dark::THEME;

    /// What a call is, without its payload: (kind, layer).
    fn kind(c: &DrawCall) -> (&'static str, Option<usize>) {
        let k = match c {
            DrawCall::Acrylic { .. } => "acrylic",
            DrawCall::Under { shape, .. } if shape.params[0] == 1.0 => "under ring",
            DrawCall::Under { .. } => "under fill",
            DrawCall::Fills { .. } => "fills",
            DrawCall::Custom { item, .. } if item.slot == 0 => "custom 0",
            DrawCall::Custom { .. } => "custom",
            DrawCall::Image { .. } => "image",
            DrawCall::Text { .. } => "text",
            DrawCall::Over { shape, .. } if shape.params[0] == 1.0 => "over ring",
            DrawCall::Over { .. } => "over fill",
            DrawCall::Strokes { .. } => "strokes",
            DrawCall::Backdrop(_) => "backdrop",
        };
        (k, c.layer())
    }

    #[test]
    fn records_the_documented_draw_order() {
        let mut text = FakeText;
        let mut p: Painter<'_, u8> = Painter::new(
            1.0,
            Rect::new(0.0, 0.0, 200.0, 100.0),
            &mut text,
            None,
            THEME,
        );
        let r = Rect::new(10.0, 10.0, 50.0, 20.0);
        let spec = Spec::plain("label", Style::ui(13.0, THEME.fg_primary));
        p.fill(r, 4.0, THEME.fg_primary);
        p.acrylic(r, 4.0, [0.1; 4], [0.0; 4], true);
        p.ring(r, 4.0, 1.0, THEME.border, false);
        p.ring(r, 4.0, 1.0, THEME.border, true);
        p.stroke(
            &[[0.0, 0.0], [20.0, 20.0]],
            false,
            StrokeStyle::solid(THEME.fg_primary, 1.0),
        );
        p.fill_path(&[[0.0, 0.0], [20.0, 0.0], [20.0, 20.0]], THEME.fg_primary);
        p.text(&spec, 12.0, 12.0);
        p.custom(0, r);
        p.image(r, ImageId(1), Fit::Contain, ImageAlign::CENTER, 1.0);
        p.on(LayerId::Popup);
        p.backdrop(r, 4.0);
        p.fill(r, 4.0, THEME.fg_primary);
        p.text(&spec, 12.0, 12.0);
        let scene = p.scene;

        let params = FrameParams {
            size: (200, 100),
            scale: 1.0,
            blur_sigma: 4.0,
            saturation: 1.0,
        };
        let mut rec = RecordingRenderer::new();
        rec.render(&scene, &params);
        let frame = rec.last().expect("one frame");
        assert_eq!(frame.params, params);
        let got: Vec<_> = frame.calls.iter().map(kind).collect();
        assert_eq!(
            got,
            [
                ("acrylic", Some(0)),
                ("under fill", Some(0)),
                ("under ring", Some(0)),
                ("fills", Some(0)),
                ("custom 0", Some(0)),
                ("image", Some(0)),
                ("text", Some(0)),
                ("over ring", Some(0)),
                ("strokes", Some(0)),
                ("backdrop", None),
                ("under fill", Some(2)),
                ("text", Some(2)),
            ]
        );
    }
}
