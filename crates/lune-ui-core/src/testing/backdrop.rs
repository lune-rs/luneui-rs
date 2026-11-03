//! The material seam's fake and the contract every backdrop meets.

use crate::backdrop::{Backdrop, BackdropKind, Material};

/// A [`Backdrop`] with no OS behind it, for tests: the semantics of a platform without glass.
/// `best_supported` is `Opaque`; `apply(Opaque)` succeeds, any glass material is an `Err`. It
/// records every material it accepted, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NullMaterial {
    /// The materials applied successfully so far.
    pub applied: Vec<Material>,
}

impl Backdrop for NullMaterial {
    fn best_supported(&self) -> BackdropKind {
        BackdropKind::Opaque
    }

    fn apply(&mut self, material: Material) -> Result<(), String> {
        if material.is_glass() {
            Err("no OS glass on this platform; use the opaque material".into())
        } else {
            self.applied.push(material);
            Ok(())
        }
    }
}

/// Checks the properties every [`Backdrop`] must have; the error names the first that fails.
///
/// - P1: `apply(Material::Opaque)` is Ok;
/// - P2: when `best_supported()` is `Opaque`, every glass material is an Err;
/// - P3: `apply(Material::Opaque)` twice in a row is Ok.
pub fn backdrop_contract(b: &mut dyn Backdrop) -> Result<(), String> {
    if let Err(e) = b.apply(Material::Opaque) {
        return Err(format!("P1: apply(Opaque) failed: {e}"));
    }
    if b.best_supported() == BackdropKind::Opaque {
        for m in [
            Material::Vibrancy,
            Material::VibrancyFollowsWindow,
            Material::LiquidClear,
            Material::LiquidRegular,
        ] {
            if b.apply(m).is_ok() {
                return Err(format!(
                    "P2: best_supported is Opaque but apply({m:?}) succeeded"
                ));
            }
        }
    }
    for n in 1..=2 {
        if let Err(e) = b.apply(Material::Opaque) {
            return Err(format!("P3: apply(Opaque) #{n} in a row failed: {e}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_material_meets_the_contract() {
        let mut b = NullMaterial::default();
        assert_eq!(backdrop_contract(&mut b), Ok(()));
        assert!(b.applied.iter().all(|m| *m == Material::Opaque));
    }

    /// A backdrop that claims `Opaque` but accepts glass must fail P2, and one that refuses
    /// `Opaque` must fail P1.
    #[test]
    fn contract_catches_broken_backdrops() {
        struct Liar;
        impl Backdrop for Liar {
            fn best_supported(&self) -> BackdropKind {
                BackdropKind::Opaque
            }
            fn apply(&mut self, _: Material) -> Result<(), String> {
                Ok(())
            }
        }
        struct Refuser;
        impl Backdrop for Refuser {
            fn best_supported(&self) -> BackdropKind {
                BackdropKind::Blur
            }
            fn apply(&mut self, _: Material) -> Result<(), String> {
                Err("no".into())
            }
        }
        assert!(backdrop_contract(&mut Liar).is_err_and(|e| e.starts_with("P2")));
        assert!(backdrop_contract(&mut Refuser).is_err_and(|e| e.starts_with("P1")));
    }
}
