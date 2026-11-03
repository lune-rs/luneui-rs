//! The display-list seam's fake: a renderer that records draw calls instead of drawing.

use crate::scene::{
    BackdropRequest, CustomItem, FrameParams, SCENE_LAYERS, Scene, Shape, StrokeVertex, TextItem,
};

/// One draw call, in the order a renderer issues it. `layer` is the index into
/// [`Scene::layers`].
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCall {
    /// One acrylic shape (replace blend).
    Acrylic { layer: usize, shape: Shape },
    /// One shape below the layer's text.
    Under { layer: usize, shape: Shape },
    /// One app-owned draw.
    Custom { layer: usize, item: CustomItem },
    /// One text placement.
    Text { layer: usize, item: TextItem },
    /// One shape above the layer's text.
    Over { layer: usize, shape: Shape },
    /// The layer's stroke vertices (one draw for the whole non-empty run).
    Strokes {
        layer: usize,
        vertices: Vec<StrokeVertex>,
    },
    /// The blurred backdrop, drawn after the scene layers and before the rest.
    Backdrop(BackdropRequest),
}

impl DrawCall {
    /// The layer the call belongs to; `None` for the backdrop.
    pub fn layer(&self) -> Option<usize> {
        match self {
            Self::Acrylic { layer, .. }
            | Self::Under { layer, .. }
            | Self::Custom { layer, .. }
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
/// per non-empty stroke run, one per text item, one per custom item, and the backdrop step when
/// the scene requests one.
#[derive(Debug, Default)]
pub struct RecordingRenderer {
    frames: Vec<RecordedFrame>,
}

impl RecordingRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one frame of `scene` drawn at `params`.
    pub fn render(&mut self, scene: &Scene, params: &FrameParams) {
        let mut calls = Vec::new();
        for (i, layer) in scene.layers.iter().enumerate() {
            if i == SCENE_LAYERS
                && let Some(b) = scene.backdrop
            {
                calls.push(DrawCall::Backdrop(b));
            }
            let g = &layer.geometry;
            let shapes = |v: &[Shape], f: fn(usize, Shape) -> DrawCall| {
                v.iter().map(move |s| f(i, *s)).collect::<Vec<_>>()
            };
            calls.extend(shapes(&g.acrylic, |layer, shape| DrawCall::Acrylic {
                layer,
                shape,
            }));
            calls.extend(shapes(&g.under, |layer, shape| DrawCall::Under {
                layer,
                shape,
            }));
            calls.extend(layer.customs.iter().map(|item| DrawCall::Custom {
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
            DrawCall::Custom { item, .. } if item.slot == 0 => "custom 0",
            DrawCall::Custom { .. } => "custom",
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
        p.text(&spec, 12.0, 12.0);
        p.custom(0, r);
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
                ("custom 0", Some(0)),
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
