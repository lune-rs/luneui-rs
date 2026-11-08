//! macOS: the effect view is inserted into the window's frame view *below* the content view (the
//! same place Avalonia puts its `_blurBehind`), so it can never cover what wgpu draws.

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2::ClassType;
use objc2::MainThreadMarker;
use objc2::runtime::NSObjectProtocol;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSView,
};
use raw_window_handle::{
    AppKitWindowHandle, HandleError, HasWindowHandle, RawWindowHandle, WindowHandle,
};
use window_vibrancy::{
    LiquidGlassOptions, NSGlassEffectViewStyle, NSVisualEffectMaterial, NSVisualEffectState,
    NSVisualEffectViewTagged, apply_liquid_glass, apply_vibrancy, clear_liquid_glass,
    clear_vibrancy,
};

use super::{
    LiquidStyle, MacosMaterial, Material, MaterialAppearance, MaterialState, VibrancyEffect,
};

/// The best backdrop kind this platform shows.
pub(super) const BEST_SUPPORTED: crate::backdrop::BackdropKind =
    crate::backdrop::BackdropKind::Blur;

pub(super) fn apply(window: &winit::window::Window, material: Material) -> Result<(), String> {
    apply_with_radius(window, material, None)
}

pub(super) fn apply_rounded(
    window: &winit::window::Window,
    material: Material,
    radius: f64,
) -> Result<(), String> {
    apply_with_radius(window, material, Some(radius))
}

fn apply_with_radius(
    window: &winit::window::Window,
    material: Material,
    radius: Option<f64>,
) -> Result<(), String> {
    let frame = FrameView::of(window).ok_or("no NSWindow frame view")?;
    // Clearing an effect that was never applied is a no-op (`Ok(false)`).
    clear_vibrancy(&frame).map_err(|e| e.to_string())?;
    clear_liquid_glass(&frame).map_err(|e| e.to_string())?;

    // `Opaque` stands for no descriptor: nothing to apply once the old effect is gone.
    let Some(os) = material.os_material() else {
        return Ok(());
    };
    let mac = os
        .macos
        .ok_or("this material has no macOS form (its `macos` is None)")?;
    match mac {
        MacosMaterial::Vibrancy {
            effect,
            state,
            appearance,
        } => {
            apply_vibrancy(&frame, effect_of(effect), Some(state_of(state)), radius)
                .map_err(|e| e.to_string())?;
            // `None` leaves the effect view's appearance alone: it is not touched at all.
            if let Some(appearance) = appearance {
                force_appearance(&frame, appearance)?;
            }
            Ok(())
        }
        MacosMaterial::Liquid {
            style,
            tint,
            opaque,
        } => apply_liquid_glass(&frame, liquid(style, tint, opaque, radius))
            .map_err(|e| e.to_string()),
    }
}

/// The `NSVisualEffectView` material for an effect. No wildcard arm: a new effect must be mapped.
// Several materials are deprecated by Apple; `UltraDark` and `Light` are the ones Avalonia uses.
#[allow(deprecated)]
fn effect_of(effect: VibrancyEffect) -> NSVisualEffectMaterial {
    match effect {
        VibrancyEffect::AppearanceBased => NSVisualEffectMaterial::AppearanceBased,
        VibrancyEffect::Light => NSVisualEffectMaterial::Light,
        VibrancyEffect::Dark => NSVisualEffectMaterial::Dark,
        VibrancyEffect::Titlebar => NSVisualEffectMaterial::Titlebar,
        VibrancyEffect::Selection => NSVisualEffectMaterial::Selection,
        VibrancyEffect::Menu => NSVisualEffectMaterial::Menu,
        VibrancyEffect::Popover => NSVisualEffectMaterial::Popover,
        VibrancyEffect::Sidebar => NSVisualEffectMaterial::Sidebar,
        VibrancyEffect::MediumLight => NSVisualEffectMaterial::MediumLight,
        VibrancyEffect::UltraDark => NSVisualEffectMaterial::UltraDark,
        VibrancyEffect::HeaderView => NSVisualEffectMaterial::HeaderView,
        VibrancyEffect::Sheet => NSVisualEffectMaterial::Sheet,
        VibrancyEffect::WindowBackground => NSVisualEffectMaterial::WindowBackground,
        VibrancyEffect::HudWindow => NSVisualEffectMaterial::HudWindow,
        VibrancyEffect::FullScreenUI => NSVisualEffectMaterial::FullScreenUI,
        VibrancyEffect::Tooltip => NSVisualEffectMaterial::Tooltip,
        VibrancyEffect::ContentBackground => NSVisualEffectMaterial::ContentBackground,
        VibrancyEffect::UnderWindowBackground => NSVisualEffectMaterial::UnderWindowBackground,
        VibrancyEffect::UnderPageBackground => NSVisualEffectMaterial::UnderPageBackground,
    }
}

/// The `NSVisualEffectView` state for a state. No wildcard arm.
fn state_of(state: MaterialState) -> NSVisualEffectState {
    match state {
        MaterialState::Active => NSVisualEffectState::Active,
        MaterialState::Inactive => NSVisualEffectState::Inactive,
        MaterialState::FollowsWindow => NSVisualEffectState::FollowsWindowActiveState,
    }
}

/// Sets the appearance of the effect view `apply_vibrancy` just inserted, on the view and not the
/// window: it goes away with the view when the material is cleared, and the app's own window
/// appearance is never touched.
fn force_appearance(frame: &FrameView, appearance: MaterialAppearance) -> Result<(), String> {
    MainThreadMarker::new().ok_or("the effect view's appearance is set on the main thread only")?;
    // SAFETY: main thread (checked above); the frame view outlives this call (see
    // `FrameView::of`), and `subviews` only reads the view hierarchy.
    let subviews = unsafe { frame.view().subviews() };
    // The view `window-vibrancy` inserts is its own subclass of the effect view, so a class check
    // finds it without naming its private tag.
    let effect_view = subviews
        .iter()
        .find(|v| v.isKindOfClass(NSVisualEffectViewTagged::class()))
        .ok_or("the effect view was not found among the frame view's subviews")?;
    // SAFETY: both names are AppKit's constant strings, valid for the process lifetime.
    let name = unsafe {
        match appearance {
            MaterialAppearance::Dark => NSAppearanceNameDarkAqua,
            MaterialAppearance::Light => NSAppearanceNameAqua,
        }
    };
    let named =
        NSAppearance::appearanceNamed(name).ok_or("the system has no appearance of that name")?;
    // Main thread (checked above); `setAppearance` takes an appearance or nil.
    effect_view.setAppearance(Some(&named));
    Ok(())
}

fn liquid(
    style: LiquidStyle,
    tint: Option<[u8; 4]>,
    opaque: Option<bool>,
    radius: Option<f64>,
) -> LiquidGlassOptions<'static> {
    let style = match style {
        LiquidStyle::Clear => NSGlassEffectViewStyle::Clear,
        LiquidStyle::Regular => NSGlassEffectViewStyle::Regular,
    };
    let mut options = LiquidGlassOptions::new(style);
    // Each setter is called only for a `Some`, so the named forms make the same calls as before.
    if let Some([r, g, b, a]) = tint {
        options = options.tint_color((r, g, b, a));
    }
    if let Some(opaque) = opaque {
        options = options.opaque(opaque);
    }
    match radius {
        Some(radius) => options.radius(radius),
        None => options,
    }
}

/// The NSWindow's frame view (superview of winit's content view), exposed as a window handle
/// so `window-vibrancy` inserts its effect view there, below the content view.
struct FrameView(NonNull<c_void>);

impl FrameView {
    fn of(window: &winit::window::Window) -> Option<Self> {
        let RawWindowHandle::AppKit(handle) = window.window_handle().ok()?.as_raw() else {
            return None;
        };
        // SAFETY: winit guarantees `ns_view` is a live NSView on the main thread while the
        // window exists; we only call it from the event loop (main thread).
        let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
        // SAFETY: main thread (see above); `superview` only reads the view hierarchy.
        let frame = unsafe { view.superview() }?;
        // The frame view is owned by the NSWindow, which outlives this short-lived handle.
        Some(Self(NonNull::from(&*frame).cast()))
    }
}

impl FrameView {
    /// The frame view.
    ///
    /// # Safety
    ///
    /// Main thread only, as `of` states; the view is owned by the NSWindow, which outlives `self`.
    unsafe fn view(&self) -> &NSView {
        // SAFETY: the pointer is a valid NSView for the lifetime of `self` (see `of`).
        unsafe { self.0.cast().as_ref() }
    }
}

impl HasWindowHandle for FrameView {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = RawWindowHandle::AppKit(AppKitWindowHandle::new(self.0));
        // SAFETY: the pointer is a valid NSView for the lifetime of `self` (see `of`).
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

#[cfg(test)]
mod tests {
    use super::{effect_of, state_of};
    use crate::backdrop::{MacosMaterial, MaterialState, VibrancyEffect};
    use lune_ui_core::theme::near_black;
    use window_vibrancy::NSVisualEffectState;

    #[test]
    fn effects_map_to_the_appkit_raw_values() {
        assert_eq!(effect_of(VibrancyEffect::UltraDark) as u64, 9);
        assert_eq!(effect_of(VibrancyEffect::Light) as u64, 1);
        assert_eq!(effect_of(VibrancyEffect::HudWindow) as u64, 13);
    }

    #[test]
    fn states_map_to_appkit_states() {
        assert_eq!(state_of(MaterialState::Active), NSVisualEffectState::Active);
        assert_eq!(
            state_of(MaterialState::Inactive),
            NSVisualEffectState::Inactive
        );
        assert_eq!(
            state_of(MaterialState::FollowsWindow),
            NSVisualEffectState::FollowsWindowActiveState
        );
    }

    #[test]
    fn near_blacks_macos_arm_is_ultra_dark_active_with_no_appearance() {
        let Some(MacosMaterial::Vibrancy {
            effect,
            state,
            appearance,
        }) = near_black::MATERIAL.os_material().and_then(|m| m.macos)
        else {
            panic!("near-black's macOS arm is not a vibrancy");
        };
        assert_eq!(effect_of(effect) as u64, 9);
        assert_eq!(state_of(state), NSVisualEffectState::Active);
        assert_eq!(appearance, None);
    }
}
