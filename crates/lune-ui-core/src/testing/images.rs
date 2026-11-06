//! The image seam's fake and the contract every image store meets.

use std::collections::HashMap;

use crate::image::{ImageData, ImageError, ImageId, ImageStore};

/// An [`ImageStore`] that keeps only each picture's size, for tests. A side above `limit` is
/// refused with [`ImageError::TooLarge`]; `limit` is `u32::MAX` unless set by
/// [`FakeImages::with_limit`].
#[derive(Debug, Clone)]
pub struct FakeImages {
    limit: u32,
    sizes: HashMap<ImageId, (u32, u32)>,
    /// How many pictures were freed so far, by a release or a replacement.
    freed: usize,
}

impl Default for FakeImages {
    fn default() -> Self {
        Self::with_limit(u32::MAX)
    }
}

impl FakeImages {
    /// A store with no size limit.
    pub fn new() -> Self {
        Self::default()
    }

    /// A store that refuses a side above `limit`.
    pub fn with_limit(limit: u32) -> Self {
        Self {
            limit,
            sizes: HashMap::new(),
            freed: 0,
        }
    }

    /// How many pictures are registered.
    pub fn len(&self) -> usize {
        self.sizes.len()
    }

    /// Whether nothing is registered.
    pub fn is_empty(&self) -> bool {
        self.sizes.is_empty()
    }

    /// Whether `id` is registered.
    pub fn contains(&self, id: ImageId) -> bool {
        self.sizes.contains_key(&id)
    }

    /// The registered size of `id`.
    pub fn size(&self, id: ImageId) -> Option<(u32, u32)> {
        self.sizes.get(&id).copied()
    }

    /// How many pictures were freed so far, by a release or a replacement.
    pub fn freed(&self) -> usize {
        self.freed
    }
}

impl ImageStore for FakeImages {
    fn register(&mut self, id: ImageId, data: ImageData) -> Result<(), ImageError> {
        if !data.length_ok() {
            return Err(ImageError::BadLength);
        }
        if data.width.max(data.height) > self.limit {
            return Err(ImageError::TooLarge { limit: self.limit });
        }
        if self.sizes.insert(id, (data.width, data.height)).is_some() {
            self.freed += 1;
        }
        Ok(())
    }

    fn release(&mut self, id: ImageId) {
        if self.sizes.remove(&id).is_some() {
            self.freed += 1;
        }
    }
}

fn solid(width: u32, height: u32) -> ImageData {
    ImageData {
        width,
        height,
        rgba: vec![0x80; 4 * width as usize * height as usize],
    }
}

/// Checks the properties every [`ImageStore`] must have; the error names the first that fails.
/// `live` reads how many pictures the store holds now. `limit` is the most a side may be when the
/// store has a small enough one to probe (pass `None` to skip P6).
///
/// - P1: a valid picture registers, and `live` counts it;
/// - P2: a buffer that is not `4 * width * height` bytes is refused with `BadLength` and changes
///   nothing, also for an id that is registered;
/// - P3: registering an id again replaces the picture and frees the old one (`live` unchanged);
/// - P4: releasing frees the id (`live` drops), and releasing an unknown id does nothing;
/// - P5: an id registers again after its release;
/// - P6: a side above `limit` is refused with `TooLarge { limit }` and changes nothing.
pub fn image_store_contract<S: ImageStore>(
    store: &mut S,
    live: impl Fn(&S) -> usize,
    limit: Option<u32>,
) -> Result<(), String> {
    let (a, z) = (ImageId(u64::MAX - 1), ImageId(u64::MAX - 2));
    let base = live(store);
    let check = |what: &str, store: &S, want: usize| {
        let got = live(store);
        if got == base + want {
            Ok(())
        } else {
            Err(format!("{what}: {got} live, expected {}", base + want))
        }
    };
    store
        .register(a, solid(2, 2))
        .map_err(|e| format!("P1: a valid picture was refused: {e}"))?;
    check("P1", store, 1)?;

    let bad = ImageData {
        width: 2,
        height: 2,
        rgba: vec![0; 15],
    };
    if store.register(z, bad.clone()) != Err(ImageError::BadLength) {
        return Err("P2: a short buffer was not refused with BadLength".into());
    }
    check("P2 (new id)", store, 1)?;
    if store.register(a, bad) != Err(ImageError::BadLength) {
        return Err("P2: a short buffer for a registered id was not refused".into());
    }
    check("P2 (registered id)", store, 1)?;

    store
        .register(a, solid(1, 1))
        .map_err(|e| format!("P3: replacing was refused: {e}"))?;
    check("P3", store, 1)?;

    store.release(a);
    check("P4", store, 0)?;
    store.release(a);
    store.release(z);
    check("P4 (unknown id)", store, 0)?;

    store
        .register(a, solid(3, 1))
        .map_err(|e| format!("P5: registering after a release was refused: {e}"))?;
    check("P5", store, 1)?;
    store.release(a);
    check("P5 (release)", store, 0)?;

    if let Some(limit) = limit {
        let wide = solid(limit + 1, 1);
        if store.register(z, wide) != Err(ImageError::TooLarge { limit }) {
            return Err("P6: a side above the limit was not refused with TooLarge".into());
        }
        let tall = solid(1, limit + 1);
        if store.register(z, tall) != Err(ImageError::TooLarge { limit }) {
            return Err("P6: a height above the limit was not refused with TooLarge".into());
        }
        check("P6", store, 0)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_meets_the_contract() {
        let mut s = FakeImages::new();
        assert_eq!(image_store_contract(&mut s, FakeImages::len, None), Ok(()));
        let mut s = FakeImages::with_limit(8);
        assert_eq!(
            image_store_contract(&mut s, FakeImages::len, Some(8)),
            Ok(())
        );
    }

    #[test]
    fn the_fake_counts_frees() {
        let mut s = FakeImages::new();
        let id = ImageId(1);
        s.register(id, solid(1, 1)).expect("ok");
        s.register(id, solid(2, 2)).expect("ok");
        assert_eq!((s.len(), s.freed(), s.size(id)), (1, 1, Some((2, 2))));
        s.release(id);
        assert_eq!((s.len(), s.freed(), s.contains(id)), (0, 2, false));
    }

    /// The contract catches a store that keeps a refused picture.
    #[test]
    fn the_contract_catches_a_store_that_accepts_bad_data() {
        #[derive(Default)]
        struct Lax(usize);
        impl ImageStore for Lax {
            fn register(&mut self, _: ImageId, _: ImageData) -> Result<(), ImageError> {
                self.0 += 1;
                Ok(())
            }
            fn release(&mut self, _: ImageId) {
                self.0 = self.0.saturating_sub(1);
            }
        }
        let mut s = Lax::default();
        let err = image_store_contract(&mut s, |s| s.0, None).expect_err("must fail");
        assert!(err.starts_with("P2"), "{err}");
    }
}
