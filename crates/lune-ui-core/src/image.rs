//! Images: the plain data an app registers once under an [`ImageId`], the [`ImageStore`] seam a
//! renderer implements to hold the pixels, and the pure fit geometry both sides share. The
//! toolkit decodes no file format and fetches nothing: the app hands over RGBA.

use crate::paint::Rect;

/// An app-chosen name for a registered picture. Pick ids that are unique across the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageId(pub u64);

/// A decoded picture: `rgba` is `width * height` pixels of 8-bit red, green, blue, alpha, row by
/// row from the top left, with straight (not premultiplied) alpha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl ImageData {
    /// Whether `rgba` holds exactly `4 * width * height` bytes.
    pub fn length_ok(&self) -> bool {
        self.rgba.len() as u128 == 4 * u128::from(self.width) * u128::from(self.height)
    }
}

/// Why a store refused a picture or an update. A refusal changes nothing: a replaced id keeps its
/// old picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageError {
    /// `rgba` is not `4 * width * height` bytes.
    BadLength,
    /// A side is above what the store can hold; `limit` is the most it takes.
    TooLarge { limit: u32 },
    /// An update named an id that is not registered.
    UnknownId,
    /// An update's rect does not lie inside the registered picture.
    OutOfBounds,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadLength => f.write_str("image data is not 4 * width * height bytes"),
            Self::TooLarge { limit } => write!(f, "image side above the limit of {limit} px"),
            Self::UnknownId => f.write_str("no picture is registered under this id"),
            Self::OutOfBounds => f.write_str("the update's rect is not inside the picture"),
        }
    }
}

impl std::error::Error for ImageError {}

/// Where registered pictures live. The renderer implements it; the painter only names ids, so an
/// id that is unknown or released draws nothing.
pub trait ImageStore {
    /// Stores `data` under `id`, replacing and freeing what the id held. A refusal leaves the
    /// store as it was.
    ///
    /// # Errors
    /// [`ImageError::BadLength`] when `data.rgba` is not `4 * width * height` bytes;
    /// [`ImageError::TooLarge`] when a side is above the store's limit.
    fn register(&mut self, id: ImageId, data: ImageData) -> Result<(), ImageError>;

    /// Replaces the pixels of the rect of the picture registered as `id` whose top-left corner is
    /// `at` (picture pixels) and whose size is `data`'s, in place: the picture's size, its id and
    /// the store's live count do not change, and the pixels outside the rect stay. `data` is
    /// straight alpha like [`ImageData`]. A `data` with no pixels does nothing. A refusal changes
    /// nothing.
    ///
    /// The next frame drawn after the call shows the new pixels; update a picture before drawing
    /// any window of the frame.
    ///
    /// # Errors
    /// Checked in this order: [`ImageError::BadLength`] when `data.rgba` is not
    /// `4 * width * height` bytes; [`ImageError::UnknownId`] when `id` is not registered;
    /// [`ImageError::OutOfBounds`] when the rect reaches past the picture's right or bottom edge.
    fn update(&mut self, id: ImageId, at: (u32, u32), data: &ImageData) -> Result<(), ImageError>;

    /// Frees `id`; does nothing for an id that is not registered. The id may be registered again.
    fn release(&mut self, id: ImageId);
}

/// How a picture is fitted into its destination rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// Scaled up or down, keeping the aspect ratio, to the largest size that fits.
    Contain,
}

/// A position along one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Center,
    End,
}

/// Where the fitted picture sits in the destination rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageAlign {
    pub x: Anchor,
    pub y: Anchor,
}

impl ImageAlign {
    /// Centred on both axes.
    pub const CENTER: Self = Self {
        x: Anchor::Center,
        y: Anchor::Center,
    };
    /// Against the left edge, centred vertically.
    pub const LEFT: Self = Self {
        x: Anchor::Start,
        y: Anchor::Center,
    };
}

/// A rect of a picture's own pixels, top-left origin: a sprite cut from a sheet. A part with no
/// area (`w` or `h` 0) draws nothing; a part past the picture's edge is dropped by the renderer,
/// which knows the picture's size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImagePart {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// How a draw samples its picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Sampling {
    /// The renderer's own rule: whole texels (nearest) when the drawn part is scaled up by an
    /// integer factor or drawn at its own size, linear filtering otherwise (D106), measured on
    /// the drawn part, not on the whole picture.
    #[default]
    Auto,
    /// Always the nearest texel: crisp pixel art at any scale.
    Nearest,
    /// Always linear filtering: smooth at any scale.
    Linear,
}

/// Everything about one picture draw except where and which picture. `Default` draws the whole
/// picture contained, centred, opaque, unturned, with [`Sampling::Auto`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageDraw {
    /// The part of the picture to draw; `None` is the whole picture.
    pub part: Option<ImagePart>,
    /// How the picture (or the part) is fitted into the destination rect.
    pub fit: Fit,
    /// Where the fitted picture sits in the destination rect.
    pub align: ImageAlign,
    /// Opacity multiplier, 0 to 1.
    pub alpha: f32,
    /// Radians, clockwise on screen, about the centre of the fitted picture.
    pub rotation: f32,
    /// How the picture is sampled.
    pub sampling: Sampling,
}

impl Default for ImageDraw {
    fn default() -> Self {
        Self {
            part: None,
            fit: Fit::Contain,
            align: ImageAlign::CENTER,
            alpha: 1.0,
            rotation: 0.0,
            sampling: Sampling::Auto,
        }
    }
}

/// The rect a picture of `size` (width, height in pixels) occupies when fitted into `dst`, or
/// `None` when the picture or `dst` has no area or a number is not finite.
pub fn image_rect(size: (u32, u32), dst: Rect, fit: Fit, align: ImageAlign) -> Option<Rect> {
    let finite = [dst.x, dst.y, dst.w, dst.h].iter().all(|v| v.is_finite());
    if size.0 == 0 || size.1 == 0 || !finite || dst.w <= 0.0 || dst.h <= 0.0 {
        return None;
    }
    let (iw, ih) = (size.0 as f32, size.1 as f32);
    let (w, h) = match fit {
        Fit::Contain => {
            let k = (dst.w / iw).min(dst.h / ih);
            (iw * k, ih * k)
        }
    };
    let at = |a: Anchor, start: f32, room: f32| match a {
        Anchor::Start => start,
        Anchor::Center => start + room / 2.0,
        Anchor::End => start + room,
    };
    let r = Rect::new(
        at(align.x, dst.x, dst.w - w),
        at(align.y, dst.y, dst.h - h),
        w,
        h,
    );
    (r.w > 0.0 && r.h > 0.0 && [r.x, r.y, r.w, r.h].iter().all(|v| v.is_finite())).then_some(r)
}
