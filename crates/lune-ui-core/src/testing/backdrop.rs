//! The material seam's fake and the contract every backdrop meets.

use crate::backdrop::{
    Backdrop, BackdropKind, MacosMaterial, Material, MaterialState, OsMaterial, VibrancyEffect,
    WindowsMaterial,
};

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
/// - P3: `apply(Material::Opaque)` twice in a row is Ok;
/// - P4: a `Custom` material that names no OS (both arms `None`) is an Err on every backdrop.
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
            Material::Custom(custom_both()),
            Material::Custom(custom_macos_only()),
            Material::Custom(custom_windows_only()),
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
    let none = Material::Custom(OsMaterial {
        macos: None,
        windows: None,
    });
    if b.apply(none).is_ok() {
        return Err("P4: apply(Custom with no OS arm) succeeded".into());
    }
    Ok(())
}

fn custom_macos() -> MacosMaterial {
    MacosMaterial::Vibrancy {
        effect: VibrancyEffect::UltraDark,
        state: MaterialState::Active,
        appearance: None,
    }
}

fn custom_windows() -> WindowsMaterial {
    WindowsMaterial::Acrylic {
        tint: Some([0, 0, 0, 160]),
        dark: Some(true),
    }
}

fn custom_both() -> OsMaterial {
    OsMaterial {
        macos: Some(custom_macos()),
        windows: Some(custom_windows()),
    }
}

fn custom_macos_only() -> OsMaterial {
    OsMaterial {
        macos: Some(custom_macos()),
        windows: None,
    }
}

fn custom_windows_only() -> OsMaterial {
    OsMaterial {
        macos: None,
        windows: Some(custom_windows()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backdrop::LiquidStyle;

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

    /// A backdrop that accepts a `Custom` naming no OS fails P4 (and only P4: it refuses the
    /// glass P2 lists).
    #[test]
    fn contract_catches_a_custom_with_no_os() {
        struct Lax;
        impl Backdrop for Lax {
            fn best_supported(&self) -> BackdropKind {
                BackdropKind::Blur
            }
            fn apply(&mut self, _: Material) -> Result<(), String> {
                Ok(())
            }
        }
        assert!(backdrop_contract(&mut Lax).is_err_and(|e| e.contains("P4")));
    }

    #[test]
    fn named_materials_map_to_their_descriptors() {
        let acrylic = Some(WindowsMaterial::Acrylic {
            tint: None,
            dark: None,
        });
        let vibrancy = |state| {
            Some(MacosMaterial::Vibrancy {
                effect: VibrancyEffect::Light,
                state,
                appearance: None,
            })
        };
        let liquid = |style| {
            Some(MacosMaterial::Liquid {
                style,
                tint: None,
                opaque: None,
            })
        };
        let os = |macos, windows| Some(OsMaterial { macos, windows });
        assert_eq!(Material::Opaque.os_material(), None);
        assert_eq!(
            Material::Vibrancy.os_material(),
            os(vibrancy(MaterialState::Active), acrylic)
        );
        assert_eq!(
            Material::VibrancyFollowsWindow.os_material(),
            os(vibrancy(MaterialState::FollowsWindow), acrylic)
        );
        assert_eq!(
            Material::LiquidClear.os_material(),
            os(
                liquid(LiquidStyle::Clear),
                Some(WindowsMaterial::Mica { dark: Some(true) })
            )
        );
        assert_eq!(
            Material::LiquidRegular.os_material(),
            os(
                liquid(LiquidStyle::Regular),
                Some(WindowsMaterial::Tabbed { dark: Some(true) })
            )
        );
        let m = custom_both();
        assert_eq!(Material::Custom(m).os_material(), Some(m));
    }

    #[test]
    fn custom_is_glass() {
        assert!(Material::Custom(custom_both()).is_glass());
        assert!(!Material::Opaque.is_glass());
    }
}
