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
    FrameOutput, ImeEvent, Key, Keymap, Modifiers, NamedKey, PointerButton, Repaint,
};
use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::MotionTier;
use lune_ui::scene::Scene;
use lune_ui::shortcuts::Shortcuts;
use lune_ui::strings::{DisplayFormat, StringTable};
use lune_ui::testing::ScriptedInput;
use lune_ui::text::TextSystem;
use lune_ui::theme::Theme;
use lune_ui::ui::{FrameConfig, Ui, UiStore, WidgetId};

use crate::scenes::LegacyScene;
use crate::scenes::{H, W};

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

/// What a Ui page draws each frame. The runner calls `draw` under `ui.scope(page.id)`.
pub(crate) trait PageState {
    fn draw(&mut self, ui: &mut Ui<'_>);

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
    pub(crate) scale: u32,
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
}

/// The id of the widget at `path` under page `page`.
pub(crate) fn path_id(page: &str, path: &[&str]) -> WidgetId {
    path.iter()
        .fold(WidgetId::ROOT.with(page), |id, key| id.with(*key))
}

/// One Ui page in flight: its retained store, state and scripted input.
pub(crate) struct Player {
    page: &'static str,
    store: UiStore,
    state: Box<dyn PageState>,
    input: ScriptedInput,
    strings: StringTable,
    tier: MotionTier,
    theme: Theme,
    /// The modifiers `Ev::Mods` holds.
    held: Modifiers,
    shortcuts: Shortcuts,
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
            theme: THEME,
            held: Modifiers::default(),
            shortcuts: Shortcuts::default(),
        }
    }

    /// A player for the gallery's window size.
    pub(crate) fn for_scene(page: &'static str, scene: &UiScene) -> Self {
        Self::new(
            page,
            (scene.state)(),
            (W as f32, H as f32),
            scene.scale as f32,
            scene.tier,
        )
    }

    /// The previous frame's rect of the widget at `path`.
    fn rect_of(&self, path: &[&str]) -> Result<lune_ui::paint::Rect, String> {
        self.store
            .last_rect(path_id(self.page, path))
            .ok_or_else(|| format!("{}: no widget at {path:?}", self.page))
    }

    fn apply(&mut self, ev: &Ev) -> Result<(), String> {
        let primary = PointerButton::Primary;
        match ev {
            Ev::PointerOver(path) => {
                let r = self.rect_of(path)?;
                self.input.pointer_to(r.x + r.w / 2.0, r.y + r.h / 2.0);
            }
            Ev::PointerAt(x, y) => {
                self.input.pointer_to(*x, *y);
            }
            Ev::Press => {
                self.input.press(primary);
            }
            Ev::Release => {
                self.input.release(primary);
            }
            Ev::Click => {
                self.input.click(primary);
            }
            Ev::Key(k) => {
                self.input.key_down(Key::Named(*k), None);
            }
            Ev::Tab => {
                self.input.key_down(Key::Named(NamedKey::Tab), None);
            }
            Ev::ShiftTab => {
                let shift = Modifiers {
                    shift: true,
                    ..Modifiers::default()
                };
                self.input
                    .set_modifiers(shift)
                    .key_down(Key::Named(NamedKey::Tab), None)
                    .set_modifiers(Modifiers::default());
            }
            Ev::WindowFocus(f) => {
                self.input.focus(*f);
            }
            Ev::Access(path, action) => {
                self.rect_of(path)?;
                self.input.access(AccessAction {
                    target: path_id(self.page, path),
                    action: *action,
                    value: None,
                });
            }
            Ev::Scroll { x, y, precise } => {
                if *precise {
                    self.input.scroll_pixels(*x, *y);
                } else {
                    self.input.scroll_lines(*x, *y);
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
                    self.input.key_down(key, Some(&s));
                }
            }
            Ev::KeyChar(s, mods) => {
                self.input
                    .set_modifiers(*mods)
                    .key_down(Key::Character((*s).to_owned()), Some(s))
                    .set_modifiers(self.held);
            }
            Ev::KeyMod(k, mods) => {
                self.input
                    .set_modifiers(*mods)
                    .key_down(Key::Named(*k), None)
                    .set_modifiers(self.held);
            }
            Ev::Mods(mods) => {
                self.held = *mods;
                self.input.set_modifiers(*mods);
            }
            Ev::ImePreedit(text, cursor) => {
                self.input.ime(ImeEvent::Preedit {
                    text: (*text).to_owned(),
                    cursor: *cursor,
                });
            }
            Ev::ImeCommit(text) => {
                self.input.ime(ImeEvent::Commit((*text).to_owned()));
            }
            Ev::ClipboardText(text) => {
                self.input.clipboard_text(text);
            }
        }
        Ok(())
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
        let raw = self.input.end_frame();
        let config = FrameConfig {
            theme: self.theme,
            tier: self.tier,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            // Scripted scenes run the same on every host.
            keymap: Keymap::MAC,
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &raw, config, text);
        let page = self.page;
        let state = &mut self.state;
        ui.scope(page, |ui| state.draw(ui));
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
    use lune_ui::paint::Rect;
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
        fn draw(&mut self, ui: &mut Ui<'_>) {
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
            scale: 1,
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
}
