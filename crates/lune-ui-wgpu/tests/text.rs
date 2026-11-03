//! The text seam's contract on the glyphon `TextCache`, with the gallery's fonts and with none.

use lune_ui_core::testing::text_system_contract;
use lune_ui_wgpu::text_cache::TextCache;

#[test]
fn text_cache_with_fonts_meets_the_contract() {
    let fonts: [&[u8]; 3] = [
        include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Medium.ttf"),
    ];
    let mut text = TextCache::new(&fonts, "RznSans", "RobotoMono Nerd Font Mono");
    assert_eq!(text_system_contract(&mut text), Ok(()));
}

#[test]
fn text_cache_without_fonts_meets_the_contract() {
    let mut text = TextCache::new(&[], "sans-serif", "monospace");
    assert_eq!(text_system_contract(&mut text), Ok(()));
}

#[test]
fn text_cache_without_fonts_lays_out_nothing() {
    use lune_ui_core::text::{Spec, Style, TextSystem};
    let mut text = TextCache::new(&[], "sans-serif", "monospace");
    let spec = Spec::plain("Hello", Style::ui(13.0, [255; 4]));
    assert_eq!(text.prepare(&spec, 2.0).1, (0.0, 0.0));
    assert_eq!(text.measure(&spec, 2.0), (0.0, 0.0));
}
