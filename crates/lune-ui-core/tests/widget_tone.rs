//! The tone recipe (`widget.status-dot`, `widget.badge`): the five status hexes, fill at 1A,
//! border at 44, text in the tone with `Data` reading `data_hi`.

use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::tone::Tone;

const TONES: [Tone; 6] = [
    Tone::Neutral,
    Tone::Ok,
    Tone::Warn,
    Tone::Err,
    Tone::Data,
    Tone::Paused,
];

#[test]
fn the_dot_colours_are_the_five_status_hexes() {
    assert_eq!(Tone::Ok.color(&DARK), [0x73, 0xC9, 0x91, 0xFF]);
    assert_eq!(Tone::Warn.color(&DARK), [0xE2, 0xB9, 0x3D, 0xFF]);
    assert_eq!(Tone::Err.color(&DARK), [0xE0, 0x6C, 0x75, 0xFF]);
    assert_eq!(Tone::Data.color(&DARK), [0x5C, 0x8F, 0xE6, 0xFF]);
    assert_eq!(Tone::Paused.color(&DARK), [0x8A, 0x90, 0x99, 0xFF]);
}

#[test]
fn a_coloured_tone_fills_at_1a_and_borders_at_44_in_its_own_colour() {
    for tone in [Tone::Ok, Tone::Warn, Tone::Err, Tone::Data, Tone::Paused] {
        let c = tone.color(&DARK);
        assert_eq!(tone.fill(&DARK), [c[0], c[1], c[2], 0x1A], "{tone:?} fill");
        assert_eq!(
            tone.border(&DARK),
            [c[0], c[1], c[2], 0x44],
            "{tone:?} border"
        );
    }
}

#[test]
fn the_badge_recipe_of_each_tone() {
    assert_eq!(Tone::Ok.fill(&DARK), [0x73, 0xC9, 0x91, 0x1A]);
    assert_eq!(Tone::Warn.border(&DARK), [0xE2, 0xB9, 0x3D, 0x44]);
    assert_eq!(Tone::Err.text(&DARK), [0xE0, 0x6C, 0x75, 0xFF]);
    // `Data` text is `data_hi`, its fill and border stay `data`.
    assert_eq!(Tone::Data.text(&DARK), DARK.data_hi);
    assert_eq!(Tone::Data.text(&DARK), [0x8F, 0xBB, 0xFF, 0xFF]);
    assert_eq!(Tone::Data.fill(&DARK), [0x5C, 0x8F, 0xE6, 0x1A]);
    assert_eq!(Tone::Data.border(&DARK), [0x5C, 0x8F, 0xE6, 0x44]);
}

#[test]
fn neutral_is_the_overlay_fill_the_subtle_border_and_secondary_text() {
    assert_eq!(Tone::Neutral.fill(&DARK), DARK.surface_overlay);
    assert_eq!(Tone::Neutral.border(&DARK), DARK.border_subtle);
    assert_eq!(Tone::Neutral.text(&DARK), DARK.fg_secondary);
    assert_eq!(Tone::Neutral.color(&DARK), DARK.fg_secondary);
    assert_eq!(Tone::default(), Tone::Neutral);
}

#[test]
fn only_data_text_differs_from_the_colour() {
    for tone in TONES {
        if tone == Tone::Data {
            assert_ne!(tone.text(&DARK), tone.color(&DARK));
        } else {
            assert_eq!(tone.text(&DARK), tone.color(&DARK), "{tone:?}");
        }
    }
}
