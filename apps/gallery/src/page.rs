//! Gallery pages: the registry types and the scene runner.
//!
//! A [`Page`] is one component's (or one subject's) screen. Legacy pages hold the fixed
//! Painter-API scenes of `scenes.rs` (D1); Ui pages draw through the [`Ui`] and carry scripted
//! scenes (a list of timed input events, then a capture frame) and the steps their bench timeline
//! replays (D16, D17). Adding a page is its own file under `pages/` plus one line in
//! `pages/mod.rs`.
//!
//! The runner is pure: input and time come from the script, text from whatever `TextSystem` the
//! caller lends, so dump, bench and the self-check tests all drive it the same way.

use lune_ui::access::{AccessAction, Action};
use lune_ui::image::{ImageData, ImageId, ImageStore};
use lune_ui::input::{
    FrameOutput, ImeEvent, Key, Keymap, Modifiers, NamedKey, PointerButton, RawInput, Repaint,
};
use lune_ui::lunegit_dark::THEME;
use lune_ui::material::Material;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::scene::{FrameParams, Scene};
use lune_ui::shortcuts::Shortcuts;
use lune_ui::strings::{DisplayFormat, StringTable};
use lune_ui::testing::ScriptedInput;
use lune_ui::text::TextSystem;
use lune_ui::theme::Theme;
use lune_ui::tray::TrayState;
use lune_ui::ui::{FrameConfig, Ui, UiStore, WidgetId};

use crate::scenes::LegacyScene;
use crate::scenes::{H, W};
use crate::showcase::knobs::Knobs;

/// One page of the gallery.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Page {
    /// Unique; also the id scope the page draws under, so it prefixes every key path.
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) kind: PageKind,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PageKind {
    /// Fixed Painter-API scenes.
    Legacy(&'static [LegacyScene]),
    /// A page drawn through the `Ui`.
    Ui(UiPage),
}

/// A Ui page: the state window mode shows, its scripted scenes and its bench steps.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UiPage {
    /// The state window mode and the bench draw with.
    pub(crate) state: fn() -> Box<dyn PageState>,
    pub(crate) scenes: &'static [UiScene],
    /// Extra input the bench replays after the scenes, times relative to its own start.
    pub(crate) bench: &'static [Step],
}

/// Where a page draws: the whole window (a scene, the bench, the page alone in a window) or a
/// rect of it (the showcase shell's content pane). A page gets its origin from the [`card`]
/// `card` returns and lays everything out relative to it, so one page serves both.
///
/// [`card`]: crate::pages::card
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Stage {
    rect: Option<Rect>,
    /// The clip the host holds over the page, in the space the page draws in. `Painter::on`
    /// drops the clip, so a page that changes layer gets it back through [`Stage::on`].
    clip: Option<Rect>,
}

impl Stage {
    /// The whole window: `card` paints the window glass, the title and the card, as always.
    pub(crate) const fn window() -> Self {
        Self {
            rect: None,
            clip: None,
        }
    }

    /// A rect of the window: `card` paints the card there and no window glass or title.
    #[allow(dead_code)] // the showcase shell's stage (step 2); the tests use it now
    pub(crate) const fn at(rect: Rect) -> Self {
        Self {
            rect: Some(rect),
            clip: None,
        }
    }

    /// The stage with the host's clip `clip`, in the space the page draws in (a scrolled host's
    /// is its pane moved by the offset).
    #[must_use]
    pub(crate) const fn clipped(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }

    /// `Painter::on`, then the host's clip again: a page that draws on more than one layer goes
    /// through here, or the clip around it would be lost at the first change.
    pub(crate) fn on<H: Copy + PartialEq>(
        &self,
        p: &mut lune_ui::paint::Painter<'_, H>,
        layer: lune_ui::scene::LayerId,
    ) {
        p.on(layer);
        if let Some(clip) = self.clip {
            p.set_clip(Some(clip));
        }
    }

    /// `r` cut to the host's clip: what of it shows. A request the clip cannot cut (a backdrop)
    /// is asked for at this rect, so it never reaches the host's other panes.
    pub(crate) fn visible(&self, r: Rect) -> Rect {
        self.clip.map_or(r, |clip| r.intersect(&clip))
    }

    /// The rect, when the page is not alone in the window.
    pub(crate) const fn rect(&self) -> Option<Rect> {
        self.rect
    }

    /// The area the page owns: the stage's rect, or the painter's whole window. A page that
    /// anchors to the edges of "its window" (caption buttons, a resize border) reads it here
    /// instead of from `ui.painter().window`.
    pub(crate) fn area(&self, ui: &mut Ui<'_>) -> Rect {
        self.rect.unwrap_or(ui.painter().window)
    }
}

/// What a Ui page draws each frame. The runner calls `draw` under `ui.scope(page.id)`.
pub(crate) trait PageState {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage);

    /// The string table the page's window runs with (a test locale); the default is empty, so
    /// every lookup falls back to the toolkit's English.
    fn strings(&self) -> StringTable {
        StringTable::default()
    }

    /// The pictures the page draws, by id; the dump, the window and the bench register them with
    /// the renderer when the page or scene starts and release them after. Ids are unique across
    /// the gallery. The default is none.
    fn images(&self) -> Vec<(ImageId, ImageData)> {
        Vec::new()
    }

    /// The theme the page runs under, read at the start of every frame; a page that loads a
    /// theme file or swaps themes answers it, and the default is the preset's.
    fn theme(&self) -> Option<Theme> {
        None
    }

    /// A named cue from a scene's script (`Ev::Cue`), delivered before the frame it comes with.
    /// The default ignores it.
    fn cue(&mut self, _name: &str) {}

    /// What the page's tray control says, for the live tray in window mode; `None` on every page
    /// that has no tray.
    fn tray_state(&self) -> Option<TrayState> {
        None
    }

    /// Input the page adds to a frame's before it begins: the host of other pages replays a
    /// scene's script here. `store` holds the previous frame, for the rects a script aims at.
    /// The default adds nothing.
    fn inject(&mut self, _store: &UiStore, _input: &mut RawInput) {}

    /// Changes whenever the string table (`strings`) may have: the runner asks for the table
    /// again when it does. The default never changes.
    fn epoch(&self) -> u64 {
        0
    }

    /// The motion tier the page runs under, read at the start of every frame; `None` keeps the
    /// runner's own (a scene's tier, or Full in the window).
    fn tier(&self) -> Option<MotionTier> {
        None
    }

    /// How the frame is drawn: the backdrop blur and whether acrylic carries noise. `None`
    /// keeps the gallery's own values.
    fn frame_params(&self) -> Option<FrameTweaks> {
        None
    }

    /// The OS material the window should have, when the page chooses one. `None` leaves it.
    fn material(&self) -> Option<Material> {
        None
    }

    /// The window told the page whether its last `material` took.
    fn material_applied(&mut self, _result: Result<(), String>) {}

    /// How fast the page's clock runs against the real one (1.0 is real time). Window mode only:
    /// the app passes time in, so slow motion needs nothing of the toolkit.
    fn time_scale(&self) -> f32 {
        1.0
    }

    /// The page's zoom: the window draws and reads the pointer at this multiple of the
    /// display's scale. Window mode only.
    fn zoom(&self) -> f32 {
        1.0
    }

    /// The page's knobs: one row per option of the component the page shows, each bound to a
    /// field of the page, which the showcase draws at the top of its settings pane. A knob's
    /// default is what the page draws without it. The default has none.
    fn knobs(&mut self, _ui: &mut Ui<'_>, _panel: &mut Knobs<'_>) {}
}

/// What the page asks of the draw of a frame, besides the display list it makes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FrameTweaks {
    /// Gaussian sigma of the backdrop blur, in physical pixels.
    pub(crate) blur_sigma: f32,
    /// Saturation boost on the blurred backdrop.
    pub(crate) saturation: f32,
    /// Acrylic carries the noise texture.
    pub(crate) noise: bool,
}

impl FrameTweaks {
    /// The gallery's own values: the ones every scene and the pixel gate draw with.
    pub(crate) const DEFAULT: FrameTweaks = FrameTweaks {
        blur_sigma: crate::scenes::BLUR_SIGMA,
        saturation: crate::scenes::BLUR_SATURATION,
        noise: true,
    };

    /// The renderer's frame parameters for a target of `size` physical pixels at scale `scale`.
    pub(crate) fn params(&self, size: (u32, u32), scale: f32) -> FrameParams {
        FrameParams {
            size,
            scale,
            blur_sigma: self.blur_sigma,
            saturation: self.saturation,
        }
    }

    /// Takes the noise off every acrylic of `scene` when it is off here. The flag is the last of
    /// a shape's parameters, as `Painter::acrylic` wrote it.
    pub(crate) fn apply_noise(&self, scene: &mut Scene) {
        if self.noise {
            return;
        }
        for layer in &mut scene.layers {
            for acrylic in &mut layer.geometry.acrylic {
                acrylic.shape.params[3] = 0.0;
            }
        }
    }
}

/// Registers every picture of `images` with `store`; on a refusal releases the ones already
/// registered and names the id. Returns the ids to hand to [`release_images`].
pub(crate) fn register_images(
    store: &mut impl ImageStore,
    images: Vec<(ImageId, ImageData)>,
) -> Result<Vec<ImageId>, String> {
    let mut done = Vec::with_capacity(images.len());
    for (id, data) in images {
        if let Err(e) = store.register(id, data) {
            release_images(store, &done);
            return Err(format!("image {}: {e}", id.0));
        }
        done.push(id);
    }
    Ok(done)
}

/// Releases `ids`.
pub(crate) fn release_images(store: &mut impl ImageStore, ids: &[ImageId]) {
    for id in ids {
        store.release(*id);
    }
}

/// Every picture the states of `page` draw: its scenes' and its window state's. Empty for a
/// legacy page.
pub(crate) fn page_images(page: &Page) -> Vec<(ImageId, ImageData)> {
    let PageKind::Ui(ui) = page.kind else {
        return Vec::new();
    };
    let mut out = (ui.state)().images();
    for scene in ui.scenes {
        out.extend((scene.state)().images());
    }
    out
}

/// Whether a scene's capture frame still moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Expect {
    /// `Repaint::Idle` at the capture frame.
    Settled,
    /// `Repaint::Now` at the capture frame (a transition is in flight).
    Moving,
    /// `Repaint::At(_)` at the capture frame: nothing moves yet, a frame is due later (a pending
    /// tooltip).
    Waiting,
}

/// One scripted Ui scene: input events at times, then a capture frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UiScene {
    pub(crate) id: &'static str,
    /// Device pixels per logical pixel in dump mode.
    pub(crate) scale: f32,
    pub(crate) tier: MotionTier,
    /// The state the scene starts from.
    pub(crate) state: fn() -> Box<dyn PageState>,
    /// Steps in ascending time order. Frame 0 (time 0, no events) runs before them and
    /// establishes the hit regions.
    pub(crate) script: &'static [Step],
    pub(crate) capture_ms: u32,
    pub(crate) expect: Expect,
}

impl UiScene {
    /// `<id>@<scale>x`, the listing name.
    pub(crate) fn name(&self) -> String {
        format!("{}@{}x", self.id, self.scale)
    }

    /// The window the scene runs in, in logical pixels: the gallery's 1040 by 660, or for the
    /// showcase's own scenes (`showcase-*`) the 1600 by 900 its window opens at.
    pub(crate) fn window(&self) -> (u32, u32) {
        if self.id.starts_with("showcase-") {
            (1600, 900)
        } else {
            (W, H)
        }
    }

    /// The dump's size in physical pixels: the window times the scale, whole at 1.5 (1560 by 990).
    pub(crate) fn size(&self) -> (u32, u32) {
        let (w, h) = self.window();
        (
            (w as f32 * self.scale).round() as u32,
            (h as f32 * self.scale).round() as u32,
        )
    }
}

/// The events of one frame at `at_ms` of app time.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Step {
    pub(crate) at_ms: u32,
    pub(crate) events: &'static [Ev],
}

/// A scripted input event. Key paths are the keys under the page's scope, outermost first.
/// The whole vocabulary exists; a page's scenes use the subset they need.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum Ev {
    /// The pointer moves to the centre of the widget's previous-frame rect.
    PointerOver(&'static [&'static str]),
    PointerAt(f32, f32),
    Press,
    Release,
    Click,
    Key(NamedKey),
    Tab,
    ShiftTab,
    WindowFocus(bool),
    /// An accessibility action on the widget at the path.
    Access(&'static [&'static str], Action),
    /// A scroll gesture at the pointer; `precise` is a touchpad (pixels), else wheel lines.
    Scroll {
        x: f32,
        y: f32,
        precise: bool,
    },
    /// Types the text one key at a time.
    Type(&'static str),
    /// A character key with the modifiers held for it only (text attached, as a platform does).
    KeyChar(&'static str, Modifiers),
    /// A named key with the modifiers held for it only.
    KeyMod(NamedKey, Modifiers),
    /// Sets the modifiers held from now on.
    Mods(Modifiers),
    ImePreedit(&'static str, Option<(usize, usize)>),
    ImeCommit(&'static str),
    /// The clipboard's answer to a read.
    ClipboardText(&'static str),
    /// A cue for the page's state (`PageState::cue`): the script's way to ask for a change the
    /// pointer would otherwise have to make.
    Cue(&'static str),
    /// A file is dragged over the window, the pointer where it last was.
    FileHover(&'static str),
    /// The dragged file left the window.
    FileHoverCancelled,
    /// A file is dropped on the window.
    FileDrop(&'static str),
}

/// The id of the widget at `path` under page `page`.
#[cfg(test)]
pub(crate) fn path_id(page: &str, path: &[&str]) -> WidgetId {
    path.iter()
        .fold(WidgetId::ROOT.with(page), |id, key| id.with(*key))
}

/// Where a script's key paths resolve: the retained store of the page's frame, and the id of
/// the scope the page draws under (the paths are the keys under it, outermost first).
pub(crate) struct Scope<'a> {
    /// The page's name, for messages.
    pub(crate) name: &'a str,
    pub(crate) store: &'a UiStore,
    pub(crate) root: WidgetId,
}

impl Scope<'_> {
    /// The id of the widget at `path`.
    pub(crate) fn id(&self, path: &[&str]) -> WidgetId {
        path.iter().fold(self.root, |id, key| id.with(*key))
    }

    /// The previous frame's rect of the widget at `path`.
    pub(crate) fn rect_of(&self, path: &[&str]) -> Result<Rect, String> {
        self.store
            .last_rect(self.id(path))
            .ok_or_else(|| format!("{}: no widget at {path:?}", self.name))
    }
}

/// Applies one scripted event to `input`. `held` is the modifiers `Ev::Mods` holds; `shift`
/// moves the absolute positions of `Ev::PointerAt` (a page on a stage sits somewhere else than
/// a page alone in the window); `cue` receives an `Ev::Cue`.
pub(crate) fn apply_event(
    input: &mut ScriptedInput,
    held: &mut Modifiers,
    ev: &Ev,
    shift: (f32, f32),
    scope: &Scope<'_>,
    cue: &mut dyn FnMut(&str),
) -> Result<(), String> {
    let primary = PointerButton::Primary;
    match ev {
        Ev::PointerOver(path) => {
            let r = scope.rect_of(path)?;
            input.pointer_to(r.x + r.w / 2.0, r.y + r.h / 2.0);
        }
        Ev::PointerAt(x, y) => {
            input.pointer_to(*x + shift.0, *y + shift.1);
        }
        Ev::Press => {
            input.press(primary);
        }
        Ev::Release => {
            input.release(primary);
        }
        Ev::Click => {
            input.click(primary);
        }
        Ev::Key(k) => {
            input.key_down(Key::Named(*k), None);
        }
        Ev::Tab => {
            input.key_down(Key::Named(NamedKey::Tab), None);
        }
        Ev::ShiftTab => {
            let shift = Modifiers {
                shift: true,
                ..Modifiers::default()
            };
            input
                .set_modifiers(shift)
                .key_down(Key::Named(NamedKey::Tab), None)
                .set_modifiers(Modifiers::default());
        }
        Ev::WindowFocus(f) => {
            input.focus(*f);
        }
        Ev::Access(path, action) => {
            scope.rect_of(path)?;
            input.access(AccessAction {
                target: scope.id(path),
                action: *action,
                value: None,
            });
        }
        Ev::Scroll { x, y, precise } => {
            if *precise {
                input.scroll_pixels(*x, *y);
            } else {
                input.scroll_lines(*x, *y);
            }
        }
        Ev::Type(text) => {
            for c in text.chars() {
                let s = c.to_string();
                let key = if c == ' ' {
                    Key::Named(NamedKey::Space)
                } else {
                    Key::Character(s.clone())
                };
                input.key_down(key, Some(&s));
            }
        }
        Ev::KeyChar(s, mods) => {
            input
                .set_modifiers(*mods)
                .key_down(Key::Character((*s).to_owned()), Some(s))
                .set_modifiers(*held);
        }
        Ev::KeyMod(k, mods) => {
            input
                .set_modifiers(*mods)
                .key_down(Key::Named(*k), None)
                .set_modifiers(*held);
        }
        Ev::Mods(mods) => {
            *held = *mods;
            input.set_modifiers(*mods);
        }
        Ev::ImePreedit(text, cursor) => {
            input.ime(ImeEvent::Preedit {
                text: (*text).to_owned(),
                cursor: *cursor,
            });
        }
        Ev::ImeCommit(text) => {
            input.ime(ImeEvent::Commit((*text).to_owned()));
        }
        Ev::ClipboardText(text) => {
            input.clipboard_text(text);
        }
        Ev::Cue(name) => {
            cue(name);
        }
        Ev::FileHover(path) => {
            input.file_hover(*path);
        }
        Ev::FileHoverCancelled => {
            input.file_hover_cancelled();
        }
        Ev::FileDrop(path) => {
            input.file_drop(*path);
        }
    }
    Ok(())
}

/// One Ui page in flight: its retained store, state and scripted input.
pub(crate) struct Player {
    page: &'static str,
    store: UiStore,
    state: Box<dyn PageState>,
    input: ScriptedInput,
    strings: StringTable,
    tier: MotionTier,
    /// The modifiers `Ev::Mods` holds.
    held: Modifiers,
    shortcuts: Shortcuts,
    stage: Stage,
    /// `PageState::epoch` when `strings` was read.
    epoch: u64,
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("page", &self.page)
            .field("tier", &self.tier)
            .finish_non_exhaustive()
    }
}

impl Player {
    /// A focused window of `size` logical pixels at `scale`, drawing `state` under `page`.
    pub(crate) fn new(
        page: &'static str,
        state: Box<dyn PageState>,
        size: (f32, f32),
        scale: f32,
        tier: MotionTier,
    ) -> Self {
        let mut input = ScriptedInput::new(size, scale);
        input.focus(true);
        let strings = state.strings();
        Self {
            page,
            store: UiStore::default(),
            state,
            input,
            strings,
            tier,
            held: Modifiers::default(),
            shortcuts: Shortcuts::default(),
            stage: Stage::window(),
            epoch: 0,
        }
    }

    #[allow(dead_code)] // the showcase shell (step 2); the tests use it now
    /// Draws the page on `stage` from now on (the default is the whole window).
    pub(crate) fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
    }

    /// A player for the gallery's window size.
    pub(crate) fn for_scene(page: &'static str, scene: &UiScene) -> Self {
        let (w, h) = scene.window();
        Self::new(
            page,
            (scene.state)(),
            (w as f32, h as f32),
            scene.scale,
            scene.tier,
        )
    }

    /// Applies `ev` to the scripted input.
    fn apply(&mut self, ev: &Ev) -> Result<(), String> {
        let scope = Scope {
            name: self.page,
            store: &self.store,
            root: WidgetId::ROOT.with(self.page),
        };
        let state = &mut self.state;
        apply_event(
            &mut self.input,
            &mut self.held,
            ev,
            (0.0, 0.0),
            &scope,
            &mut |name| state.cue(name),
        )
    }

    /// Runs one frame at `ms` of app time with `events` and returns what `Ui::finish` does.
    pub(crate) fn frame(
        &mut self,
        text: &mut dyn TextSystem,
        ms: u32,
        events: &[Ev],
    ) -> Result<(Scene, FrameOutput), String> {
        self.input
            .set_time(std::time::Duration::from_millis(u64::from(ms)));
        for ev in events {
            self.apply(ev)?;
        }
        let mut raw = self.input.end_frame();
        self.state.inject(&self.store, &mut raw);
        if self.state.epoch() != self.epoch {
            self.epoch = self.state.epoch();
            self.strings = self.state.strings();
        }
        let config = FrameConfig {
            // Read after the events, so a cue's change shows in the frame it comes with.
            theme: self.state.theme().unwrap_or(THEME),
            tier: self.state.tier().unwrap_or(self.tier),
            strings: &self.strings,
            arg_format: &DisplayFormat,
            // Scripted scenes run the same on every host.
            keymap: Keymap::MAC,
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &raw, config, text);
        let page = self.page;
        let state = &mut self.state;
        let stage = self.stage;
        ui.scope(page, |ui| state.draw(ui, &stage));
        Ok(ui.finish())
    }
}

/// Whether a scene's capture frame reported the `Repaint` its `expect` names.
pub(crate) fn meets_expectation(expect: Expect, repaint: Repaint) -> bool {
    match expect {
        Expect::Settled => repaint == Repaint::Idle,
        Expect::Moving => repaint == Repaint::Now,
        Expect::Waiting => matches!(repaint, Repaint::At(_)),
    }
}

/// Runs `scene` of `page`: frame 0, the script, then the capture frame. Returns the capture
/// frame's scene and output.
pub(crate) fn run_scene(
    page: &'static str,
    scene: &UiScene,
    text: &mut dyn TextSystem,
) -> Result<(Scene, FrameOutput), String> {
    let mut player = Player::for_scene(page, scene);
    player.frame(text, 0, &[])?;
    for step in scene.script {
        player.frame(text, step.at_ms, step.events)?;
    }
    player.frame(text, scene.capture_ms, &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::PAGES;
    use lune_ui::access::{Actions, Node, Role, States};
    use lune_ui::motion::{Kind, Motion};
    use lune_ui::scene::LayerId;
    use lune_ui::testing::FakeText;
    use lune_ui::ui::Sense;
    use std::cell::Cell;
    use std::collections::BTreeSet;
    use std::rc::Rc;

    fn pic(w: u32, h: u32) -> ImageData {
        ImageData {
            width: w,
            height: h,
            rgba: vec![0; 4 * (w * h) as usize],
        }
    }

    #[test]
    fn images_register_and_release_as_a_set() {
        use lune_ui::testing::FakeImages;
        let mut store = FakeImages::new();
        let ids = register_images(
            &mut store,
            vec![(ImageId(1), pic(2, 2)), (ImageId(2), pic(1, 3))],
        )
        .expect("registers");
        assert_eq!((ids.len(), store.len()), (2, 2));
        release_images(&mut store, &ids);
        assert!(store.is_empty());
    }

    #[test]
    fn a_refused_picture_releases_the_ones_before_it() {
        use lune_ui::testing::FakeImages;
        let mut store = FakeImages::with_limit(4);
        let err = register_images(
            &mut store,
            vec![(ImageId(1), pic(2, 2)), (ImageId(2), pic(9, 1))],
        )
        .expect_err("refused");
        assert!(err.contains("image 2"), "{err}");
        assert!(store.is_empty());
    }

    #[test]
    fn a_page_without_pictures_has_no_images() {
        for page in PAGES {
            let n = page_images(page).len();
            assert!(n < 64, "{}: {n}", page.id);
        }
    }

    /// Every Ui scene runs, every key path resolves, and the capture frame's `Repaint` matches
    /// `expect`.
    #[test]
    fn every_ui_scene_runs_and_meets_its_expectation() {
        for page in PAGES {
            let PageKind::Ui(ui_page) = page.kind else {
                continue;
            };
            for scene in ui_page.scenes {
                let name = format!("{}/{}", page.id, scene.name());
                let (_, out) = run_scene(page.id, scene, &mut FakeText)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert!(
                    meets_expectation(scene.expect, out.repaint),
                    "{name}: {:?}, expected {:?}",
                    out.repaint,
                    scene.expect
                );
            }
        }
    }

    #[test]
    fn page_and_scene_names_are_unique() {
        let mut pages = BTreeSet::new();
        let mut scenes = BTreeSet::new();
        for page in PAGES {
            assert!(pages.insert(page.id), "page id {}", page.id);
            match page.kind {
                PageKind::Legacy(list) => {
                    for s in list {
                        assert!(scenes.insert(s.name()), "scene {}", s.name());
                    }
                }
                PageKind::Ui(p) => {
                    for s in p.scenes {
                        assert!(scenes.insert(s.name()), "scene {}", s.name());
                    }
                }
            }
        }
    }

    /// A page of two buttons for the runner's own checks.
    #[derive(Default)]
    struct Probe {
        clicks: Rc<Cell<u32>>,
        enter: Rc<Cell<u32>>,
    }

    const A: Rect = Rect::new(10.0, 10.0, 100.0, 40.0);
    const B: Rect = Rect::new(200.0, 10.0, 100.0, 40.0);

    impl PageState for Probe {
        fn draw(&mut self, ui: &mut Ui<'_>, _stage: &Stage) {
            let a = ui.id("a");
            let b = ui.id("b");
            let ra = ui.interact(a, A, Sense::CLICK_FOCUS);
            ui.interact(b, B, Sense::CLICK_FOCUS);
            self.clicks.set(self.clicks.get() + u32::from(ra.clicked));
            if ra.focused && ui.consume_key(a, NamedKey::Enter) {
                self.enter.set(self.enter.get() + 1);
            }
            ui.node(Node {
                id: a,
                role: Role::Button,
                name: "a".to_owned(),
                description: None,
                value: None,
                text: None,
                bounds: A,
                parent: None,
                children: Vec::new(),
                live: None,
                states: States::default(),
                actions: Actions::default(),
                sort: None,
                level: None,
            });
            let tint = ui.animate_f32(a, f32::from(ra.hovered as u8), &Motion::of(Kind::Hover));
            let p = ui.painter();
            p.fill(A, 0.0, [255, 255, 255, (tint * 255.0) as u8]);
        }
    }

    const PROBE_SCRIPT: &[Step] = &[
        Step {
            at_ms: 0,
            events: &[Ev::PointerOver(&["a"])],
        },
        Step {
            at_ms: 10,
            events: &[Ev::Click],
        },
    ];

    fn probe_scene(script: &'static [Step], capture_ms: u32, expect: Expect) -> UiScene {
        UiScene {
            id: "probe",
            scale: 1.0,
            tier: MotionTier::Full,
            state: || Box::new(Probe::default()),
            script,
            capture_ms,
            expect,
        }
    }

    #[test]
    fn the_runner_resolves_paths_and_moves_the_pointer_over_the_widget() {
        let moving = probe_scene(PROBE_SCRIPT, 60, Expect::Moving);
        let (_, out) = run_scene("p", &moving, &mut FakeText).expect("runs");
        assert_eq!(out.repaint, Repaint::Now);
        // Hover is instant (120 ms): settled by 200 ms.
        let settled = probe_scene(PROBE_SCRIPT, 200, Expect::Settled);
        let (_, out) = run_scene("p", &settled, &mut FakeText).expect("runs");
        assert_eq!(out.repaint, Repaint::Idle);
    }

    #[test]
    fn an_unknown_key_path_is_an_error_naming_it() {
        static BAD: &[Step] = &[Step {
            at_ms: 0,
            events: &[Ev::PointerOver(&["nope"])],
        }];
        let err = run_scene("p", &probe_scene(BAD, 10, Expect::Settled), &mut FakeText)
            .expect_err("no such widget");
        assert!(err.contains("nope"), "{err}");
    }

    #[test]
    fn click_and_key_events_reach_the_widget() {
        let probe = Probe::default();
        let (clicks, enter) = (probe.clicks.clone(), probe.enter.clone());
        let mut player = Player::new("p", Box::new(probe), (400.0, 300.0), 1.0, MotionTier::Full);
        let mut run = |ms, evs: &[Ev]| player.frame(&mut FakeText, ms, evs).expect("frame");
        run(0, &[]);
        run(10, &[Ev::PointerOver(&["a"]), Ev::Click]);
        run(20, &[]);
        assert_eq!((clicks.get(), enter.get()), (1, 0));
        // The click focused `a`; Enter is then consumable once.
        run(30, &[Ev::Key(NamedKey::Enter)]);
        run(40, &[]);
        assert_eq!((clicks.get(), enter.get()), (1, 1));
        // An access action on `a` is a click.
        run(50, &[Ev::Access(&["a"], Action::Default)]);
        assert_eq!(clicks.get(), 2);
        assert_eq!(path_id("p", &["a"]), WidgetId::ROOT.with("p").with("a"));
    }
    type StateFn = fn() -> Box<dyn PageState>;

    /// Nodes of overlays that centre themselves in the window, not in the stage (a modal dialog):
    /// the test cannot tell an overlay's node from a page's, so it names these `(page, name)`
    /// pairs, the name being a substring of the node's.
    const OVERLAY_NODES: &[(&str, &str)] =
        &[("locale", "Discard changes"), ("ui", "Discard changes")];

    /// Pages whose scenes open a modal or a popup list that fills the window (a dialog, a palette,
    /// a dropdown's scrolled rows): their nodes are an overlay's, so only their shapes and text
    /// on the base and page layers are checked.
    const OVERLAY_PAGES: &[&str] = &["dialog", "dropdown", "command-palette"];

    /// Where the page's content lies on a stage: its semantic nodes' bounds and the shapes and
    /// text it draws on the base and page layers (overlay layers are left out), in the live
    /// state and in each scene's starting state, when `drawn_on`. Names what, as far as a clip
    /// lets it show, lies left of `stage`'s x or above its y.
    fn off_stage(page: &Page, drawn_on: Stage, stage: Rect) -> Vec<String> {
        let PageKind::Ui(ui_page) = page.kind else {
            return Vec::new();
        };
        let mut states: Vec<(String, StateFn)> = vec![("live".to_owned(), ui_page.state)];
        states.extend(ui_page.scenes.iter().map(|s| (s.id.to_owned(), s.state)));
        let mut out = BTreeSet::new();
        for (which, state) in states {
            let mut player = Player::new(
                page.id,
                state(),
                (W as f32, H as f32),
                1.0,
                MotionTier::Full,
            );
            player.set_stage(drawn_on);
            let outside = |x: f32, y: f32| x < stage.x - 0.5 || y < stage.y - 0.5;
            for ms in [0, 16, 300] {
                let (scene, frame) = player.frame(&mut FakeText, ms, &[]).expect("frame");
                for n in &frame.tree.nodes {
                    let overlay = OVERLAY_NODES
                        .iter()
                        .any(|(page_id, name)| *page_id == page.id && n.name.contains(name));
                    let overlay = overlay || OVERLAY_PAGES.contains(&page.id);
                    if outside(n.bounds.x, n.bounds.y) && !overlay {
                        out.insert(format!("{which}: {} node {:?}", n.name, n.bounds));
                    }
                }
                for id in [LayerId::Base, LayerId::Page] {
                    let layer = &scene.layers[id as usize];
                    for t in &layer.texts {
                        if outside(t.x.max(t.clip.x), t.y.max(t.clip.y)) {
                            out.insert(format!("{which}: {id:?} text at ({}, {})", t.x, t.y));
                        }
                    }
                    let geometry = &layer.geometry;
                    let acrylic = geometry.acrylic.iter().map(|a| &a.shape);
                    for s in geometry.under.iter().chain(&geometry.over).chain(acrylic) {
                        let [mut x, mut y, w, h] = s.rect;
                        if s.clip[2] > 0.0 {
                            (x, y) = (x.max(s.clip[0]), y.max(s.clip[1]));
                        }
                        if w > 0.0 && h > 0.0 && outside(x, y) {
                            out.insert(format!("{which}: {id:?} shape {:?}", s.rect));
                        }
                    }
                }
            }
        }
        out.into_iter().collect()
    }

    /// Every Ui page's live state and scene states draw inside a content rect: nothing they make
    /// lies left of or above it. Names each offender.
    #[test]
    fn every_ui_page_draws_inside_its_stage() {
        let stage = Rect::new(300.0, 80.0, 1020.0, 592.0);
        let mut bad = Vec::new();
        for page in PAGES {
            let off = off_stage(page, Stage::at(stage), stage);
            if !off.is_empty() {
                bad.push(format!(
                    "{}: {} off the stage\n  {}",
                    page.id,
                    off.len(),
                    off.join("\n  ")
                ));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// The instrument finds what it should: the button page drawn on the whole window lies left
    /// of and above the stage rect, drawn on the stage it does not.
    #[test]
    fn the_stage_check_sees_an_offender() {
        let stage = Rect::new(300.0, 80.0, 1020.0, 592.0);
        let button = PAGES.iter().find(|p| p.id == "button").expect("a page");
        assert!(!off_stage(button, Stage::window(), stage).is_empty());
        assert!(off_stage(button, Stage::at(stage), stage).is_empty());
    }

    /// The `*-disabled` scenes of the D47 pass (M11 review round 1): each draws a scope in
    /// `with_enabled(false, ..)`, so a node is `Disabled`, and its pointer, click, Tab or key
    /// script leaves no widget focused.
    #[test]
    fn the_disabled_scenes_hold_a_disabled_node_and_no_focus() {
        use lune_ui::access::State;
        const SCENES: [(&str, &str); 8] = [
            ("toolbar", "toolbar-disabled"),
            ("search-picker", "search-picker-disabled"),
            ("banner", "banner-empty-state-disabled"),
            ("tree-view", "tree-view-disabled"),
            ("settings-page", "settings-page-disabled"),
            ("kanban-board", "kanban-board-disabled"),
            ("command-palette", "command-palette-disabled"),
            ("code-editor-view", "code-editor-view-disabled"),
        ];
        for (page, id) in SCENES {
            let PageKind::Ui(ui) = PAGES.iter().find(|p| p.id == page).expect("a page").kind else {
                panic!("{page} is a Ui page");
            };
            let scene = ui.scenes.iter().find(|s| s.id == id).expect("a scene");
            let (_, out) = run_scene(page, scene, &mut FakeText).expect("runs");
            assert!(
                out.tree
                    .nodes
                    .iter()
                    .any(|n| n.states.contains(State::Disabled)),
                "{id}: a Disabled node"
            );
            assert!(out.focused.is_none(), "{id}: nothing focused");
        }
    }
}
