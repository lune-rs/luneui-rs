//! The text seam's contract on the glyphon `TextCache`, with the gallery's fonts and with none.

use lune_ui::testing::text_system_contract;
use lune_ui::text_cache::TextCache;

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
