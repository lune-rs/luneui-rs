//! The `theme` page (`infra.theme-hot-reload`, M10): the controls composition under each of
//! Lunegit's five real theme files (bundled under `assets/lunegit/themes/`, see
//! `licenses/lunegit-fixtures.txt`), under a file that sets one colour, under the three acrylic
//! levels, and across a swap from one theme to another. Every scene's theme comes from
//! `Theme::from_json`, through `PageState::theme`, so what the scenes show is what an app that
//! loads a file gets. The window also lets you pick a file and a level.

use lune_ui::color::premul;
use lune_ui::icon;
use lune_ui::motion::MotionTier;
use lune_ui::paint::{LayerId, Rect};
use lune_ui::theme::{AcrylicLevel, DARK, Theme, ThemeFile};
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::menu::{MenuItem, MenuRow};

use super::controls_ui::Controls;
use super::{caption, card, heading};
use crate::page::Stage;
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "theme",
    title: "Theme files",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

const DARK_MODERN: &[u8] = include_bytes!("../../assets/lunegit/themes/dark-modern.json");
const BLUE: &[u8] = include_bytes!("../../assets/lunegit/themes/blue.json");
const GREEN: &[u8] = include_bytes!("../../assets/lunegit/themes/green.json");
const ORANGE: &[u8] = include_bytes!("../../assets/lunegit/themes/orange.json");
const PURPLE: &[u8] = include_bytes!("../../assets/lunegit/themes/purple.json");

/// A file that sets the accent and nothing else: every other field keeps the base theme's value
/// and the fields derived from the accent follow it.
const PARTIAL: &[u8] =
    br##"{"name":"Partial","author":"inline","version":1,"colors":{"AccentPrimaryColor":"#FFE5484D"},"acrylic":{}}"##;

/// The files the window offers, by the label of their button.
const FILES: [(&str, &[u8]); 6] = [
    ("Dark Modern", DARK_MODERN),
    ("Blue", BLUE),
    ("Green", GREEN),
    ("Orange", ORANGE),
    ("Purple", PURPLE),
    ("Partial", PARTIAL),
];

const LEVELS: [(&str, AcrylicLevel); 4] = [
    ("Default", AcrylicLevel::Default),
    ("Less opaque", AcrylicLevel::LessOpaque),
    ("More opaque", AcrylicLevel::MoreOpaque),
    ("Opaque", AcrylicLevel::Opaque),
];

/// Loads `bytes` over the preset. The files are bundled and a test loads each one, so a failure
/// is a broken build and stops the page.
fn load(bytes: &[u8]) -> ThemeFile {
    Theme::from_json(bytes, &DARK)
        .unwrap_or_else(|e| panic!("the bundled theme file does not load: {e:?}"))
}

/// What the file says about itself, for the caption.
fn describe(file: &ThemeFile) -> String {
    format!(
        "{} - {} - version {} - {} keys with no field here",
        file.name,
        file.author.as_deref().unwrap_or("no author"),
        file.version,
        file.skipped.len()
    )
}

/// The controls under a loaded theme.
struct Themed {
    controls: Controls,
    /// The theme as the file made it, before the acrylic level.
    theme: Theme,
    info: String,
    level: AcrylicLevel,
    /// The window's pickers; the scenes draw without them.
    pickers: bool,
}

impl Themed {
    fn of(bytes: &[u8], pickers: bool) -> Self {
        let file = load(bytes);
        Self {
            controls: Controls::new(),
            info: describe(&file),
            theme: file.theme,
            level: AcrylicLevel::Default,
            pickers,
        }
    }

    fn swap_to(&mut self, bytes: &[u8]) {
        let file = load(bytes);
        self.info = describe(&file);
        self.theme = file.theme;
    }
}

impl PageState for Themed {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let area = card(ui, stage, "luneui gallery");
        self.controls.draw(ui, area);
        let x = area.x + 24.0;
        if self.pickers {
            heading(ui, "THEME FILE", x, area.y + 380.0);
            let mut bx = x;
            for (i, (label, bytes)) in FILES.iter().enumerate() {
                let w = button_width(ui.painter(), label);
                let r = ui.button(
                    ("file", i),
                    Rect::new(bx, area.y + 400.0, w, 32.0),
                    label,
                    ButtonKind::Secondary,
                );
                if r.clicked {
                    self.swap_to(bytes);
                }
                bx += w + 8.0;
            }
            heading(ui, "ACRYLIC LEVEL", x, area.y + 450.0);
            let mut bx = x;
            for (i, (label, level)) in LEVELS.iter().enumerate() {
                let w = button_width(ui.painter(), label);
                let r = ui.button(
                    ("level", i),
                    Rect::new(bx, area.y + 470.0, w, 32.0),
                    label,
                    ButtonKind::Secondary,
                );
                if r.clicked {
                    self.level = *level;
                }
                bx += w + 8.0;
            }
        }
        caption(ui, &self.info, x, area.bottom() - 36.0);
    }

    fn theme(&self) -> Option<Theme> {
        Some(self.theme.with_acrylic(self.level))
    }

    fn cue(&mut self, name: &str) {
        match name {
            "swap" => self.swap_to(BLUE),
            "swap-back" => self.swap_to(DARK_MODERN),
            _ => {}
        }
    }
}

fn live() -> Box<dyn PageState> {
    Box::new(Themed::of(DARK_MODERN, true))
}

fn dark_modern() -> Box<dyn PageState> {
    Box::new(Themed::of(DARK_MODERN, false))
}

fn blue() -> Box<dyn PageState> {
    Box::new(Themed::of(BLUE, false))
}

fn green() -> Box<dyn PageState> {
    Box::new(Themed::of(GREEN, false))
}

fn orange() -> Box<dyn PageState> {
    Box::new(Themed::of(ORANGE, false))
}

fn purple() -> Box<dyn PageState> {
    Box::new(Themed::of(PURPLE, false))
}

fn partial() -> Box<dyn PageState> {
    Box::new(Themed::of(PARTIAL, false))
}

/// The dark-modern file starts the swap scenes; the cue swaps to blue.
fn swapping() -> Box<dyn PageState> {
    dark_modern()
}

/// The glass of the preset at `level`: a panel over bright stripes and a glass menu over the
/// panel.
struct Acrylic {
    level: AcrylicLevel,
}

fn acrylic(level: AcrylicLevel) -> Box<dyn PageState> {
    Box::new(Acrylic { level })
}

fn acrylic_less() -> Box<dyn PageState> {
    acrylic(AcrylicLevel::LessOpaque)
}

fn acrylic_more() -> Box<dyn PageState> {
    acrylic(AcrylicLevel::MoreOpaque)
}

fn acrylic_opaque() -> Box<dyn PageState> {
    acrylic(AcrylicLevel::Opaque)
}

/// The stripes under the panel: acrylic shows through by its material opacity.
const STRIPES: [[u8; 4]; 6] = [
    [0xE5, 0x48, 0x4D, 0xFF],
    [0xF2, 0xA6, 0x5A, 0xFF],
    [0x73, 0xC9, 0x91, 0xFF],
    [0x4D, 0xA3, 0xFF, 0xFF],
    [0xB0, 0x7A, 0xE8, 0xFF],
    [0xF0, 0xF0, 0xF0, 0xFF],
];

const MENU: [MenuRow<'static>; 3] = [
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::ADD),
        text: "New Window",
        shortcut: "Cmd+N",
        disabled: false,
        danger: false,
        chevron: false,
    }),
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::COPY),
        text: "Copy",
        shortcut: "Cmd+C",
        disabled: false,
        danger: false,
        chevron: false,
    }),
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::GEAR),
        text: "Settings...",
        shortcut: "Cmd+,",
        disabled: false,
        danger: false,
        chevron: false,
    }),
];

impl PageState for Acrylic {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let theme = *ui.theme();
        let win = stage.area(ui);
        let band = win.w / STRIPES.len() as f32;
        let panel = Rect::new(win.x + 60.0, win.y + 70.0, 560.0, 480.0);
        let material = theme.glass.panel.material_opacity;
        let under = premul([0x70, 0x70, 0x70, 0xFF]);
        {
            let p = ui.painter();
            p.on(LayerId::Base);
            for (i, c) in STRIPES.iter().enumerate() {
                p.fill(
                    Rect::new(win.x + i as f32 * band, win.y, band, win.h),
                    0.0,
                    *c,
                );
            }
            // Acrylic replaces what is below it within a layer, so the panel is above the stripes.
            p.on(LayerId::Page);
            p.acrylic(panel, 8.0, theme.glass.panel.premultiplied(), under, true);
        }
        heading(ui, "PANEL GLASS", panel.x + 24.0, panel.y + 24.0);
        caption(
            ui,
            &format!("panel material opacity {material:.2}"),
            panel.x + 24.0,
            panel.y + 44.0,
        );
        ui.menu("menu", &MENU, (panel.x + 120.0, panel.y + 120.0));
    }

    fn theme(&self) -> Option<Theme> {
        Some(DARK.with_acrylic(self.level))
    }
}

const PRIMARY: &[&str] = &["primary"];
const TOGGLE: &[&str] = &["toggle-off"];

/// The pointer goes to the toggle and clicks it: the thumb starts across, which no theme drives.
/// [`SWAP`] is this, then the cue.
#[cfg(test)]
const CLICK: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(TOGGLE)],
    },
    Step {
        at_ms: 40,
        events: &[Ev::Click],
    },
];

/// [`CLICK`], and at 60 ms the cue swaps dark modern for blue with the thumb in flight.
const SWAP: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(TOGGLE)],
    },
    Step {
        at_ms: 40,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 60,
        events: &[Ev::Cue("swap")],
    },
];

const fn scene(
    id: &'static str,
    state: fn() -> Box<dyn PageState>,
    script: &'static [Step],
    capture_ms: u32,
    expect: Expect,
) -> UiScene {
    UiScene {
        id,
        scale: 2.0,
        tier: MotionTier::Full,
        state,
        script,
        capture_ms,
        expect,
    }
}

const SCENES: &[UiScene] = &[
    scene("theme-dark-modern", dark_modern, &[], 0, Expect::Settled),
    scene("theme-blue", blue, &[], 0, Expect::Settled),
    scene("theme-green", green, &[], 0, Expect::Settled),
    scene("theme-orange", orange, &[], 0, Expect::Settled),
    scene("theme-purple", purple, &[], 0, Expect::Settled),
    scene("theme-partial", partial, &[], 0, Expect::Settled),
    scene(
        "theme-acrylic-less",
        acrylic_less,
        &[],
        400,
        Expect::Settled,
    ),
    scene(
        "theme-acrylic-more",
        acrylic_more,
        &[],
        400,
        Expect::Settled,
    ),
    scene(
        "theme-acrylic-opaque",
        acrylic_opaque,
        &[],
        400,
        Expect::Settled,
    ),
    // 20 ms after the swap: every colour is already blue (the swap is a hard cut, D5) and only
    // the toggle's thumb, which no theme drives, is still moving.
    scene("theme-swap", swapping, SWAP, 80, Expect::Moving),
    // Settled: the same frame the blue theme gives after the same click.
    scene("theme-swap-b", swapping, SWAP, 600, Expect::Settled),
];

/// What the bench replays: the pointer on the primary button, a swap to blue and back.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(PRIMARY)],
    },
    Step {
        at_ms: 600,
        events: &[Ev::Cue("swap")],
    },
    Step {
        at_ms: 1200,
        events: &[Ev::Cue("swap-back")],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Player, run_scene};
    use lune_ui::testing::FakeText;

    #[test]
    fn every_bundled_file_loads_with_a_name() {
        for (label, bytes) in FILES {
            let file = load(bytes);
            assert!(!file.name.trim().is_empty(), "{label}");
        }
    }

    #[test]
    fn the_partial_file_sets_the_accent_and_what_follows_it() {
        let file = load(PARTIAL);
        assert_eq!(file.theme.accent, [0xE5, 0x48, 0x4D, 0xFF]);
        assert_ne!(file.theme.accent_hover, DARK.accent_hover);
        assert_eq!(file.theme.fg_primary, DARK.fg_primary);
        assert!(file.skipped.is_empty(), "{:?}", file.skipped);
    }

    /// The frame at `capture_ms` of a player on `state` that runs `script`.
    fn solo(
        state: fn() -> Box<dyn PageState>,
        script: &[Step],
        capture_ms: u32,
    ) -> lune_ui::scene::Scene {
        let mut player = Player::new(
            "theme",
            state(),
            (crate::scenes::W as f32, crate::scenes::H as f32),
            2.0,
            MotionTier::Full,
        );
        player.frame(&mut FakeText, 0, &[]).expect("frame 0");
        for step in script {
            player
                .frame(&mut FakeText, step.at_ms, step.events)
                .expect("step");
        }
        player
            .frame(&mut FakeText, capture_ms, &[])
            .expect("capture")
            .0
    }

    fn scene_named(id: &str) -> &'static UiScene {
        SCENES
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("no scene {id}"))
    }

    /// The settled swap frame is the frame the blue theme gives from the start after the same
    /// click: a swap leaves nothing of the theme it came from.
    #[test]
    fn the_settled_swap_equals_the_blue_theme_for_the_same_composition() {
        let (swapped, _) =
            run_scene("theme", scene_named("theme-swap-b"), &mut FakeText).expect("scene runs");
        let blue_solo = solo(blue, CLICK, 600);
        assert!(
            swapped == blue_solo,
            "the swapped frame differs from blue's"
        );
        // The probe can tell the themes apart: the dark-modern frame is not blue's.
        assert!(
            solo(dark_modern, CLICK, 600) != blue_solo,
            "the probe cannot tell two themes apart"
        );
    }

    /// The frame after the swap holds blue's colours at once while the thumb is still moving: it
    /// is not the frame the old theme draws at that time.
    #[test]
    fn the_frame_after_the_swap_has_left_the_old_theme() {
        let (swapped, _) =
            run_scene("theme", scene_named("theme-swap"), &mut FakeText).expect("scene runs");
        assert!(
            swapped != solo(dark_modern, CLICK, 80),
            "the swap changed nothing"
        );
    }
}
