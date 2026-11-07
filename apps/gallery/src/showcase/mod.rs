//! The showcase page (D7): one Ui page that hosts every other page. A sidebar tree (groups, pages,
//! scenes) with a search field on the left, the chosen page on a scrolling canvas on the right.
//!
//! The shell is an ordinary page: it draws through the `Ui` and its scenes run headless like any
//! other. The hosted page is a child state drawn under its own id scope on a [`Stage`] (D5): it
//! lays itself out from the card it is given, at a fixed 1,020 x 616 canvas that scrolls inside
//! the content pane when the pane is narrower (D6). Choosing a page makes its live state fresh
//! (a new scope, so none of the old page's retained state is seen); choosing a scene makes the
//! scene's starting state and replays its script in real time, then leaves the page live.
//!
//! The host window holds three pane hooks of [`PageState`] for this page: `inject` (the replay's
//! input joins the frame's), `epoch` (the child's string table changed) and the cues `next-page`
//! and `prev-page` that the gallery's Cmd/Ctrl+] and Cmd/Ctrl+[ send.
//!
//! App code: it draws nothing of its own besides the chrome; no clock is read here (the frame's
//! time comes from the caller).

mod nav;
mod settings;

use std::collections::BTreeSet;
use std::time::Duration;

use lune_ui::image::{ImageData, ImageId};
use lune_ui::input::{Modifiers, RawInput};
use lune_ui::material::Material;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::strings::StringTable;
use lune_ui::testing::ScriptedInput;
use lune_ui::text::{Spec, Style};
use lune_ui::theme::Theme;
use lune_ui::tray::TrayState;
use lune_ui::ui::{Ui, UiStore, WidgetId};
use lune_ui::widgets::field::{FieldOptions, Variant};
use lune_ui::widgets::scroll_area::{Extent, ScrollOptions};
use lune_ui::widgets::tree_view::{ROW, TreeOptions, TreeRow};

use crate::page::{
    Ev, Expect, FrameTweaks, Page, PageKind, PageState, Scope, Stage, Step, UiPage, UiScene,
    apply_event, page_images,
};
use crate::pages::{PAGES, card, card_shift};
use nav::{HOST, Target};
use settings::Settings;

pub(crate) static PAGE: Page = Page {
    id: HOST,
    title: "Showcase",
    kind: PageKind::Ui(UiPage {
        state: home,
        scenes: SCENES,
        bench: &[],
    }),
};

fn home() -> Box<dyn PageState> {
    Box::new(Showcase::new())
}

/// The shell with settings other than the defaults, for the `showcase-settings` scene.
fn changed_settings() -> Box<dyn PageState> {
    let mut shell = Showcase::new();
    shell.settings.set_theme(2);
    shell.settings.tier = MotionTier::Reduced;
    shell.settings.speed = 0.5;
    shell.settings.sigma = 12.0;
    shell.settings.noise = false;
    Box::new(shell)
}

/// The gap around and between the panes.
const PAD: f32 = 12.0;
/// The sidebar's width, and the settings pane's.
const SIDE_W: f32 = 240.0;
const SETTINGS_W: f32 = 300.0;
/// The search field's height, and the gap under it.
const FIELD_H: f32 = 32.0;
const FIELD_GAP: f32 = 8.0;
/// The title line's height above the content.
const HEAD_H: f32 = 28.0;
/// The canvas a hosted page draws on: the card of a page alone in the window (1,040 x 660 less
/// its 10 px margin and its 34 px title band).
const CANVAS: (f32, f32) = (1020.0, 616.0);

/// Where the panes lie in the shell's card.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Layout {
    search: Rect,
    tree: Rect,
    head: Rect,
    pane: Rect,
    settings: Rect,
}

impl Layout {
    const fn of(card: Rect) -> Layout {
        let side_h = card.h - 2.0 * PAD;
        let x0 = card.x + PAD + SIDE_W + PAD;
        let w = card.w - (PAD + SIDE_W + PAD + PAD + SETTINGS_W + PAD);
        Layout {
            search: Rect::new(card.x + PAD, card.y + PAD, SIDE_W, FIELD_H),
            tree: Rect::new(
                card.x + PAD,
                card.y + PAD + FIELD_H + FIELD_GAP,
                SIDE_W,
                side_h - FIELD_H - FIELD_GAP,
            ),
            head: Rect::new(x0, card.y + PAD, w, HEAD_H),
            pane: Rect::new(x0, card.y + PAD + HEAD_H + 8.0, w, side_h - HEAD_H - 8.0),
            settings: Rect::new(
                card.x + card.w - PAD - SETTINGS_W,
                card.y + PAD,
                SETTINGS_W,
                side_h,
            ),
        }
    }

    /// The middle of row `i` of the tree while it is not scrolled.
    const fn tree_row(&self, i: usize) -> (f32, f32) {
        (self.tree.x + 100.0, self.tree.y + (i as f32 + 0.5) * ROW)
    }
}

/// The scenes' card: the whole window of a scene, less its margin and title band.
const SCENE_CARD: Rect = Rect::new(10.0, 34.0, 1020.0, 616.0);
const SCENE_LAYOUT: Layout = Layout::of(SCENE_CARD);

const SEARCH: &[&str] = &["search"];

/// A scripted scene's pointer aims at the sidebar's second row, which holds the first match.
const SECOND_ROW: (f32, f32) = SCENE_LAYOUT.tree_row(1);

const SCENES: &[UiScene] = &[
    UiScene {
        id: "showcase-home",
        scale: 1.0,
        tier: MotionTier::Full,
        state: home,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Typing into the search narrows the tree to the pages and scenes that match.
    UiScene {
        id: "showcase-search",
        scale: 1.0,
        tier: MotionTier::Full,
        state: home,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(SEARCH)],
            },
            Step {
                at_ms: 10,
                events: &[Ev::Click],
            },
            Step {
                at_ms: 20,
                events: &[Ev::Type("tab")],
            },
        ],
        capture_ms: 300,
        expect: Expect::Settled,
    },
    // The settings pane under a changed theme, tier, speed, blur and noise.
    UiScene {
        id: "showcase-settings",
        scale: 1.0,
        tier: MotionTier::Full,
        state: changed_settings,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Search for the table, then press the page's row: it takes the content pane.
    UiScene {
        id: "showcase-table",
        scale: 1.0,
        tier: MotionTier::Full,
        state: home,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(SEARCH)],
            },
            Step {
                at_ms: 10,
                events: &[Ev::Click],
            },
            Step {
                at_ms: 20,
                events: &[Ev::Type("table")],
            },
            Step {
                at_ms: 100,
                events: &[Ev::PointerAt(SECOND_ROW.0, SECOND_ROW.1)],
            },
            Step {
                at_ms: 140,
                events: &[Ev::Click],
            },
            Step {
                at_ms: 200,
                events: &[],
            },
        ],
        capture_ms: 700,
        expect: Expect::Settled,
    },
];

/// A scene's script being played into the live window.
#[derive(Debug)]
struct Replay {
    script: &'static [Step],
    next: usize,
    /// The frame time the script's zero is; set by the first frame after the scene's first draw.
    start: Option<Duration>,
    /// The hosted state has drawn once, so the rects the script aims at exist.
    drawn: bool,
    held: Modifiers,
}

impl Replay {
    fn new(script: &'static [Step]) -> Self {
        Self {
            script,
            next: 0,
            start: None,
            drawn: false,
            held: Modifiers::default(),
        }
    }
}

/// The page that hosts the others.
struct Showcase {
    /// The search text.
    query: String,
    /// The first frame seeds the tree's expansion from the chosen page.
    seeded: bool,
    /// The sidebar must open the chosen page's group (a key moved the choice).
    reveal: bool,
    /// The chosen page, as an index into `PAGES`, and its scene when one was chosen.
    page: usize,
    scene: Option<usize>,
    child: Box<dyn PageState>,
    /// Bumped at every choice: it keys the child's id scope and the content's scroll.
    generation: u64,
    /// Bumped with it: the runner asks for the child's string table again.
    epoch: u64,
    replay: Option<Replay>,
    /// The id of the scope the child draws under, and the canvas it drew on, last frame.
    child_root: WidgetId,
    canvas: Rect,
    order: Vec<usize>,
    /// The global settings; they outlast a change of page.
    settings: Settings,
}

impl Showcase {
    fn new() -> Self {
        let order = nav::order();
        let page = order.first().copied().unwrap_or(0);
        let mut me = Self {
            query: String::new(),
            seeded: false,
            reveal: false,
            page,
            scene: None,
            child: Box::new(Empty),
            generation: 0,
            epoch: 0,
            replay: None,
            child_root: WidgetId::ROOT,
            canvas: Rect::new(0.0, 0.0, CANVAS.0, CANVAS.1),
            order,
            settings: Settings::default(),
        };
        me.select(page, None);
        me.epoch = 0;
        me
    }

    /// Chooses `page` (live), or its `scene` (the scene's starting state, its script replayed).
    fn select(&mut self, page: usize, scene: Option<usize>) {
        let PageKind::Ui(ui_page) = PAGES[page].kind else {
            return;
        };
        self.page = page;
        self.generation += 1;
        self.epoch += 1;
        self.reveal = true;
        match scene.and_then(|s| ui_page.scenes.get(s).map(|scene| (s, scene))) {
            Some((s, scene)) => {
                self.scene = Some(s);
                self.child = (scene.state)();
                self.replay = Some(Replay::new(scene.script));
            }
            None => {
                self.scene = None;
                self.child = (ui_page.state)();
                self.replay = None;
            }
        }
    }

    /// The page `by` places on in the sidebar's order, wrapping.
    fn step(&mut self, by: isize) {
        let n = self.order.len();
        if n == 0 {
            return;
        }
        let at = self.order.iter().position(|p| *p == self.page).unwrap_or(0);
        let next = (at as isize + by).rem_euclid(n as isize) as usize;
        self.select(self.order[next], None);
    }

    /// The key of the tree row that stands for the choice.
    fn current_key(&self) -> u64 {
        match self.scene {
            Some(s) => Target::Scene(self.page, s).key(),
            None => Target::Page(self.page).key(),
        }
    }

    fn pick(&mut self, target: Option<Target>) {
        match target {
            Some(Target::Page(p)) => self.select(p, None),
            Some(Target::Scene(p, s)) => self.select(p, Some(s)),
            Some(Target::Group(_)) | None => {}
        }
    }

    /// The search field and the tree under it.
    fn sidebar(&mut self, ui: &mut Ui<'_>, lay: &Layout) {
        let opts = FieldOptions {
            placeholder: "Search components and scenes",
            variant: Variant::Search,
            ..FieldOptions::default()
        };
        let edited = ui
            .text_field_with("search", lay.search, &mut self.query, &opts)
            .response
            .changed;
        // Both trees keep their state (what is open) while the other one is on screen.
        ui.keep_alive("sidebar", |ui| self.tree(ui, lay, edited));
    }

    /// The tree: all the rows, or the ones a search keeps (each in a tree of its own).
    fn tree(&mut self, ui: &mut Ui<'_>, lay: &Layout, edited: bool) {
        let searching = !self.query.trim().is_empty();
        let key = if searching { "found" } else { "tree" };
        if edited && searching {
            ui.tree_state("found").expanded = nav::expanded_for(&self.query);
        }
        if !self.seeded {
            self.seeded = true;
            self.reveal = true;
        }
        if self.reveal {
            self.reveal = false;
            // The normal tree opens what the choice sits in; the search tree shows its matches.
            let st = ui.tree_state("tree");
            if let Some(g) = nav::group_of(self.page) {
                st.expanded.insert(Target::Group(g).key());
            }
            if self.scene.is_some() {
                st.expanded.insert(Target::Page(self.page).key());
            }
        }
        let rows = nav::rows(&self.query);
        if rows.is_empty() {
            let t = *ui.theme();
            let spec = Spec::plain("Nothing matches", Style::ui(t.font_s, t.fg_tertiary));
            ui.painter().text_centered(&spec, lay.tree);
            return;
        }
        let tree_rows: Vec<TreeRow<'_>> = rows
            .iter()
            .map(|r| TreeRow {
                key: r.target.key(),
                depth: r.depth,
                text: &r.text,
                glyph: None,
                has_children: r.has_children,
            })
            .collect();
        // The tree shows the choice; a press on another row is what changes it.
        let current = self.current_key();
        ui.tree_state(key).selected = BTreeSet::from([current]);
        let opts = TreeOptions {
            rows: &tree_rows,
            label: "Components",
            guides: true,
            toggle_on_row: true,
        };
        let out = ui.tree_view(key, lay.tree, &opts, |_, _| {});
        if out.response.changed {
            let picked = ui.tree_state(key).selected.iter().next().copied();
            self.pick(picked.and_then(Target::of_key));
        }
    }

    /// The title line and the scrolling canvas the hosted page draws on.
    fn content(&mut self, ui: &mut Ui<'_>, lay: &Layout) {
        let t = *ui.theme();
        let page = PAGES[self.page];
        let group = nav::group_of(self.page).map_or("", |g| nav::GROUPS[g].name);
        let mut title = format!("{group}  /  {}", page.title);
        if let Some(s) = self.scene {
            if let Some(scene) = nav::scenes(page).get(s) {
                title = format!("{title}  /  {}", scene.name());
            }
        }
        let spec = Spec::plain(title, Style::ui(t.font_base, t.fg_primary).medium());
        ui.painter().text(&spec, lay.head.x, lay.head.y + 4.0);

        let canvas = Rect::new(lay.pane.x, lay.pane.y, CANVAS.0, CANVAS.1);
        self.canvas = canvas;
        let opts = ScrollOptions {
            extent: Some((f64::from(canvas.w), f64::from(canvas.h))),
            identity: Some(self.generation),
            ..ScrollOptions::default()
        };
        let (child, generation) = (&mut self.child, self.generation);
        let mut root = self.child_root;
        ui.scroll_area("content", lay.pane, &opts, |ui, _| {
            ui.scope(("page", generation), |ui| {
                ui.scope(page.id, |ui| {
                    root = ui.scope_id();
                    child.draw(ui, &Stage::at(canvas));
                });
            });
            ((), Extent::default())
        });
        self.child_root = root;
        if let Some(replay) = &mut self.replay {
            replay.drawn = true;
            // The next step is due at its time; before the script starts, on the next frame.
            let due = match (replay.start, replay.script.get(replay.next)) {
                (Some(start), Some(step)) => start + Duration::from_millis(u64::from(step.at_ms)),
                _ => ui.time() + Duration::from_millis(16),
            };
            ui.request_repaint_at(due);
        }
    }
}

/// What the shell holds before its first choice.
struct Empty;

impl PageState for Empty {
    fn draw(&mut self, _ui: &mut Ui<'_>, _stage: &Stage) {}
}

impl PageState for Showcase {
    fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
        let shell = card(ui, stage, "luneui gallery");
        let lay = Layout::of(shell);
        // The page first: its widgets then come first in the tab order, as a scene's script
        // (a Tab at its start) expects. The panes do not overlap, so the order draws the same.
        self.content(ui, &lay);
        self.sidebar(ui, &lay);
        self.settings.pane(ui, lay.settings);
        if self.replay.is_none() && self.scene.is_some() {
            let t = *ui.theme();
            let spec = Spec::plain("live", Style::ui(t.font_s, t.fg_tertiary));
            let w = ui.painter().measure(&spec).0;
            ui.painter()
                .text(&spec, lay.head.right() - w, lay.head.y + 6.0);
        }
    }

    fn strings(&self) -> StringTable {
        self.child.strings()
    }

    /// Every page's pictures at once: a choice does not register anything, so each must be there.
    fn images(&self) -> Vec<(ImageId, ImageData)> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for page in PAGES.iter().filter(|p| p.id != HOST) {
            for (id, data) in page_images(page) {
                if seen.insert(id) {
                    out.push((id, data));
                }
            }
        }
        out
    }

    /// The chosen theme under the settings, or the hosted page's own under them.
    fn theme(&self) -> Option<Theme> {
        Some(match self.child.theme() {
            Some(page) => self.settings.compose(page),
            None => self.settings.theme(),
        })
    }

    fn tier(&self) -> Option<MotionTier> {
        Some(self.settings.tier)
    }

    fn frame_params(&self) -> Option<FrameTweaks> {
        Some(self.settings.tweaks())
    }

    fn material(&self) -> Option<Material> {
        Some(self.settings.material)
    }

    fn material_applied(&mut self, result: Result<(), String>) {
        self.settings.material_note = result.err().map(|e| format!("not applied: {e}"));
    }

    fn time_scale(&self) -> f32 {
        self.settings.speed
    }

    fn zoom(&self) -> f32 {
        self.settings.zoom
    }

    fn cue(&mut self, name: &str) {
        match name {
            "next-page" => self.step(1),
            "prev-page" => self.step(-1),
            _ => {
                if let Some(id) = name.strip_prefix("page ") {
                    if let Some(p) = nav::index_of(id) {
                        self.select(p, None);
                    }
                } else if let Some(scene) = name.strip_prefix("scene ") {
                    let found = PAGES.iter().enumerate().find_map(|(p, page)| {
                        nav::scenes(page)
                            .iter()
                            .position(|s| s.name() == scene)
                            .map(|s| (p, s))
                    });
                    if let Some((p, s)) = found {
                        self.select(p, Some(s));
                    }
                } else {
                    self.child.cue(name);
                }
            }
        }
    }

    fn tray_state(&self) -> Option<TrayState> {
        self.child.tray_state()
    }

    /// Adds the replayed script's events that are due to the frame's input.
    fn inject(&mut self, store: &UiStore, input: &mut RawInput) {
        let Some(mut replay) = self.replay.take() else {
            return;
        };
        if !replay.drawn {
            self.replay = Some(replay);
            return;
        }
        let start = *replay.start.get_or_insert(input.time);
        let elapsed = input.time.saturating_sub(start);
        let scope = Scope {
            name: PAGES[self.page].id,
            store,
            root: self.child_root,
        };
        // A script's absolute positions are those of a page alone in the window.
        let shift = card_shift(self.canvas);
        let mut scripted = ScriptedInput::new(input.size, input.scale);
        scripted.set_time(input.time);
        let child = &mut self.child;
        while let Some(step) = replay.script.get(replay.next) {
            if Duration::from_millis(u64::from(step.at_ms)) > elapsed {
                break;
            }
            for ev in step.events {
                let result = apply_event(
                    &mut scripted,
                    &mut replay.held,
                    ev,
                    shift,
                    &scope,
                    &mut |name| child.cue(name),
                );
                if let Err(e) = result {
                    eprintln!("replay: {e}");
                }
            }
            replay.next += 1;
        }
        input.events.extend(scripted.end_frame().events);
        if replay.next < replay.script.len() {
            self.replay = Some(replay);
        }
    }

    fn epoch(&self) -> u64 {
        self.epoch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lune_ui::access::State;
    use lune_ui::input::{FrameOutput, Translator};
    use lune_ui::motion::MotionTier;
    use lune_ui::shortcuts::Shortcuts;
    use lune_ui::strings::DisplayFormat;
    use lune_ui::testing::FakeText;
    use lune_ui::ui::FrameConfig;
    use std::cell::Cell;
    use std::rc::Rc;
    use winit::dpi::PhysicalSize;
    use winit::event::{ElementState, MouseButton};

    use crate::page::Player;

    fn player() -> Player {
        Player::new(
            HOST,
            Box::new(Showcase::new()),
            (1040.0, 660.0),
            1.0,
            MotionTier::Full,
        )
    }

    /// Frames at 16 ms steps from `from` to `to`, the first with `events`; the last frame's output.
    fn run(p: &mut Player, from: u32, to: u32, events: &[Ev]) -> FrameOutput {
        let mut last = None;
        let mut ms = from;
        let mut evs = events;
        while ms <= to {
            last = Some(p.frame(&mut FakeText, ms, evs).expect("frame").1);
            evs = &[];
            ms += 16;
        }
        last.expect("a frame")
    }

    fn named<'a>(out: &'a FrameOutput, name: &str) -> Option<&'a lune_ui::access::Node> {
        out.tree.nodes.iter().find(|n| n.name == name)
    }

    #[test]
    fn the_shell_opens_on_the_first_page_of_the_first_group() {
        let s = Showcase::new();
        assert_eq!(PAGES[s.page].id, "ui");
        assert_eq!(s.scene, None);
        assert_eq!(s.order.len(), nav::order().len());
    }

    #[test]
    fn page_keys_walk_the_sidebar_order_and_wrap() {
        let mut s = Showcase::new();
        let (first, last) = (s.order[0], s.order[s.order.len() - 1]);
        s.cue("next-page");
        assert_eq!(s.page, s.order[1]);
        s.cue("prev-page");
        s.cue("prev-page");
        assert_eq!(s.page, last);
        s.cue("next-page");
        assert_eq!(s.page, first);
        // A choice is a new child and the runner learns of it.
        let epoch = s.epoch;
        s.cue("next-page");
        assert_ne!(s.epoch, epoch);
    }

    #[test]
    fn the_images_of_every_page_are_listed_once() {
        let s = Showcase::new();
        let ids: Vec<ImageId> = s.images().into_iter().map(|(id, _)| id).collect();
        let unique: BTreeSet<ImageId> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len());
        assert!(!ids.is_empty());
    }

    /// The search narrows the tree; clearing it brings the tree back as it was, with its groups
    /// open as they were.
    #[test]
    fn a_search_narrows_the_tree_and_clearing_it_restores_the_expansion() {
        let mut p = player();
        let mut out = run(&mut p, 0, 0, &[]);
        assert!(named(&out, "Foundation").is_some());
        assert!(named(&out, "Foundation").is_some_and(|n| n.states.contains(State::Expanded)));
        assert!(named(&out, "Widgets").is_some_and(|n| !n.states.contains(State::Expanded)));
        run(
            &mut p,
            16,
            16,
            &[Ev::PointerOver(SEARCH), Ev::Click, Ev::Type("tab strip")],
        );
        out = run(&mut p, 32, 64, &[]);
        assert!(named(&out, "Tab strip").is_some());
        assert!(named(&out, "Foundation").is_none());
        assert!(named(&out, "Button").is_none());
        // Select it all and delete.
        let all = Modifiers {
            logo: true,
            ..Modifiers::default()
        };
        run(&mut p, 80, 80, &[Ev::KeyChar("a", all)]);
        out = run(
            &mut p,
            96,
            128,
            &[Ev::Key(lune_ui::input::NamedKey::Backspace)],
        );
        assert!(named(&out, "Foundation").is_some_and(|n| n.states.contains(State::Expanded)));
        assert!(named(&out, "Widgets").is_some_and(|n| !n.states.contains(State::Expanded)));
    }

    /// A scene's script plays in real time from the scene's own state, then the page stays live.
    #[test]
    fn a_chosen_scene_replays_its_script_then_stays_live() {
        let mut p = player();
        run(&mut p, 0, 0, &[]);
        let out = run(&mut p, 16, 48, &[Ev::Cue("scene toggle-switching@2x")]);
        // Tab at 0 and Space at 400 ms after the scene's first frame: not yet at ~100 ms.
        assert!(named(&out, "Off").is_some_and(|n| !n.states.contains(State::Checked)));
        let out = run(&mut p, 64, 800, &[]);
        assert!(
            named(&out, "Off").is_some_and(|n| n.states.contains(State::Checked)),
            "the script's Space flipped the switch"
        );
        // Over: the child is live, and a further Space flips nothing by itself.
        let out = run(&mut p, 816, 1200, &[]);
        assert!(named(&out, "Off").is_some_and(|n| n.states.contains(State::Checked)));
    }

    /// The `showcase-table` scene ends on the table page, chosen by the press on its row.
    #[test]
    fn the_table_scene_ends_on_the_table_page() {
        let PageKind::Ui(ui_page) = PAGE.kind else {
            panic!("the showcase is a Ui page");
        };
        let scene = ui_page
            .scenes
            .iter()
            .find(|s| s.id == "showcase-table")
            .expect("a scene");
        let (_, out) = crate::page::run_scene(HOST, scene, &mut FakeText).expect("runs");
        assert!(
            named(&out, "Table").is_some_and(|n| n.states.contains(State::Selected)),
            "the tree row is selected"
        );
        assert!(
            named(&out, "Routes").is_some(),
            "the table is on the canvas"
        );
    }

    /// The window's path: winit-shaped events through the `Translator`, into the shell.
    struct Rig {
        store: UiStore,
        shell: Showcase,
        translator: Translator,
        ms: u64,
    }

    impl Rig {
        fn new() -> Self {
            Self::sized(1040, 660)
        }

        /// A window of `w` x `h` physical pixels at scale 1.
        fn sized(w: u32, h: u32) -> Self {
            let mut translator = Translator::new(PhysicalSize::new(w, h), 1.0);
            translator.focused(true);
            Rig {
                store: UiStore::default(),
                shell: Showcase::new(),
                translator,
                ms: 0,
            }
        }

        fn frame(&mut self) -> FrameOutput {
            self.ms += 16;
            let mut input = self.translator.take(Duration::from_millis(self.ms));
            self.shell.inject(&self.store, &mut input);
            let strings = self.shell.strings();
            let shortcuts = Shortcuts::default();
            let config = FrameConfig {
                theme: self.shell.theme().unwrap_or(lune_ui::lunegit_dark::THEME),
                tier: MotionTier::Full,
                strings: &strings,
                arg_format: &DisplayFormat,
                keymap: lune_ui::input::Keymap::MAC,
                shortcuts: &shortcuts,
            };
            let mut text = FakeText;
            let mut ui = Ui::begin(&mut self.store, &input, config, &mut text);
            let shell = &mut self.shell;
            ui.scope(HOST, |ui| shell.draw(ui, &Stage::window()));
            ui.finish().1
        }

        /// Frames for 480 ms: the tree's expand motion is done.
        fn settle(&mut self) {
            for _ in 0..30 {
                self.frame();
            }
        }

        fn click_at(&mut self, (x, y): (f32, f32)) {
            self.translator.cursor_moved(f64::from(x), f64::from(y));
            self.frame();
            self.translator
                .mouse_input(ElementState::Pressed, MouseButton::Left);
            self.frame();
            self.translator
                .mouse_input(ElementState::Released, MouseButton::Left);
            self.frame();
        }
    }

    #[test]
    fn a_click_on_a_tree_row_selects_it() {
        let mut rig = Rig::new();
        rig.frame();
        let lay = SCENE_LAYOUT;
        // Open Widgets (row 7: the six Foundation pages sit under row 0), then press its Button row.
        let widgets = 7;
        rig.click_at(lay.tree_row(widgets));
        rig.settle();
        assert_eq!(
            PAGES[rig.shell.page].id, "ui",
            "a group row chooses nothing"
        );
        rig.click_at(lay.tree_row(widgets + 1));
        rig.settle();
        assert_eq!(PAGES[rig.shell.page].id, "button");
        let out = rig.frame();
        assert!(named(&out, "Button").is_some_and(|n| n.states.contains(State::Selected)));
    }

    /// The centre of the node of `role` named `name`, as the last frame left it.
    fn centre(out: &FrameOutput, role: lune_ui::access::Role, name: &str) -> (f32, f32) {
        let n = out
            .tree
            .nodes
            .iter()
            .find(|n| n.role == role && n.name == name)
            .unwrap_or_else(|| panic!("no {role:?} named {name}"));
        (n.bounds.x + n.bounds.w / 2.0, n.bounds.y + n.bounds.h / 2.0)
    }

    /// A tall window, so the whole settings pane is on screen.
    fn tall() -> Rig {
        let mut rig = Rig::sized(1600, 2600);
        rig.frame();
        rig
    }

    #[test]
    fn a_click_on_a_setting_changes_it_and_a_page_change_keeps_it() {
        use lune_ui::access::Role;
        let mut rig = tall();
        let out = rig.frame();
        rig.click_at(centre(&out, Role::Radio, "2x"));
        let out = rig.frame();
        assert!((rig.shell.settings.zoom - 2.0).abs() < f32::EPSILON);
        assert!(
            (rig.shell.zoom() - 2.0).abs() < f32::EPSILON,
            "the hook says it"
        );
        rig.click_at(centre(&out, Role::Radio, "Reduced"));
        let out = rig.frame();
        assert_eq!(rig.shell.settings.tier, MotionTier::Reduced);
        assert_eq!(rig.shell.tier(), Some(MotionTier::Reduced));
        rig.click_at(centre(&out, Role::Radio, "Blue"));
        assert_eq!(rig.shell.settings.theme, 2);
        // A change of page leaves them be.
        rig.shell.cue("next-page");
        rig.frame();
        assert!((rig.shell.settings.zoom - 2.0).abs() < f32::EPSILON);
        assert_eq!(rig.shell.settings.theme, 2);
        // Reset is the only way back.
        let out = rig.frame();
        rig.click_at(centre(&out, Role::Button, "Reset all settings"));
        assert_eq!(rig.shell.settings, Settings::default());
    }

    #[test]
    fn the_hooks_say_what_the_settings_say() {
        let mut s = Showcase::new();
        s.settings.speed = 0.5;
        s.settings.zoom = 1.5;
        s.settings.sigma = 12.0;
        s.settings.saturation = 2.5;
        s.settings.noise = false;
        s.settings.material = Material::Opaque;
        assert!((s.time_scale() - 0.5).abs() < f32::EPSILON);
        assert!((s.zoom() - 1.5).abs() < f32::EPSILON);
        assert_eq!(s.material(), Some(Material::Opaque));
        let tweaks = s.frame_params().expect("tweaks");
        let params = tweaks.params((200, 100), 2.0);
        assert_eq!((params.size, params.scale), ((200, 100), 2.0));
        assert!((params.blur_sigma - 12.0).abs() < f32::EPSILON);
        assert!((params.saturation - 2.5).abs() < f32::EPSILON);
        // The noise flag of every acrylic goes off.
        let mut scene = lune_ui::scene::Scene::default();
        for layer in &mut scene.layers {
            layer.geometry.acrylic.push(lune_ui::scene::Acrylic {
                shape: lune_ui::scene::Shape {
                    params: [0.0, 0.0, 1.0, 1.0],
                    ..Default::default()
                },
                opacity: 1.0,
            });
        }
        tweaks.apply_noise(&mut scene);
        assert!(
            scene
                .layers
                .iter()
                .all(|l| l.geometry.acrylic[0].shape.params[3] == 0.0)
        );
        assert_eq!(scene.layers[0].geometry.acrylic[0].shape.params[2], 1.0);
        // The material result lands in the pane's note.
        s.material_applied(Err("no compositor".to_owned()));
        assert!(
            s.settings
                .material_note
                .as_deref()
                .is_some_and(|n| n.contains("no compositor"))
        );
        s.material_applied(Ok(()));
        assert_eq!(s.settings.material_note, None);
    }

    /// Draws the hosted page's tier and theme.
    struct Probe {
        tier: Rc<Cell<Option<MotionTier>>>,
        window_fill: Rc<Cell<Option<[u8; 4]>>>,
    }

    impl PageState for Probe {
        fn draw(&mut self, ui: &mut Ui<'_>, _stage: &Stage) {
            self.tier.set(Some(ui.tier()));
            self.window_fill.set(Some(ui.theme().glass.panel.tint));
        }
    }

    #[test]
    fn the_tier_and_the_theme_reach_the_hosted_page() {
        let tier = Rc::new(Cell::new(None));
        let fill = Rc::new(Cell::new(None));
        let mut shell = Showcase::new();
        shell.child = Box::new(Probe {
            tier: tier.clone(),
            window_fill: fill.clone(),
        });
        shell.settings.tier = MotionTier::Off;
        shell.settings.layers[1].tint = Some([9, 8, 7, 255]);
        let mut p = Player::new(
            HOST,
            Box::new(shell),
            (1040.0, 660.0),
            1.0,
            MotionTier::Full,
        );
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        assert_eq!(tier.get(), Some(MotionTier::Off), "FrameConfig.tier");
        assert_eq!(fill.get(), Some([9, 8, 7, 255]), "FrameConfig.theme");
        // A page with no hook of its own runs under the scene's tier.
        let tier = Rc::new(Cell::new(None));
        let plain = Probe {
            tier: tier.clone(),
            window_fill: Rc::new(Cell::new(None)),
        };
        struct NoHooks(Probe);
        impl PageState for NoHooks {
            fn draw(&mut self, ui: &mut Ui<'_>, stage: &Stage) {
                self.0.draw(ui, stage);
            }
        }
        let mut p = Player::new(
            "p",
            Box::new(NoHooks(plain)),
            (100.0, 100.0),
            1.0,
            MotionTier::Reduced,
        );
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        assert_eq!(tier.get(), Some(MotionTier::Reduced));
    }

    #[test]
    fn a_click_on_a_hosted_button_fires_it() {
        let mut rig = Rig::new();
        rig.shell.cue("page toggle");
        rig.frame();
        let out = rig.frame();
        let off = named(&out, "Off").expect("the first switch");
        assert!(!off.states.contains(State::Checked));
        let at = (
            off.bounds.x + off.bounds.w / 2.0,
            off.bounds.y + off.bounds.h / 2.0,
        );
        rig.click_at(at);
        let out = rig.frame();
        assert!(
            named(&out, "Off").is_some_and(|n| n.states.contains(State::Checked)),
            "the hosted switch took the click"
        );
    }
}
