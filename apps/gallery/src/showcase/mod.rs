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

pub(crate) mod knobs;
pub(crate) mod labs;
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
use lune_ui::widgets::tree_view::{ChevronStyle, GuideStub, GuideStyle, TreeOptions, TreeRow};

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
    shell.settings.noise = 0.0;
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
    #[cfg(test)]
    const fn tree_row(&self, i: usize) -> (f32, f32) {
        (
            self.tree.x + 100.0,
            self.tree.y + (i as f32 + 0.5) * lune_ui::widgets::tree_view::ROW,
        )
    }
}

/// The scenes' card: the 1600 x 900 window of a `showcase-*` scene, less its margin and title band.
#[cfg(test)]
const SCENE_CARD: Rect = Rect::new(10.0, 34.0, 1580.0, 856.0);
#[cfg(test)]
const SCENE_LAYOUT: Layout = Layout::of(SCENE_CARD);

const SEARCH: &[&str] = &["search"];

/// The search tree's scope under the sidebar: its rows are the numbers under it.
const FOUND_ROWS: &[&str] = &["sidebar", "found"];
/// The row key of the table page in the sidebar: `Target::Page(index of "table").key()`, which
/// `the_table_scenes_row_key_is_the_first_match_of_the_search` pins.
const TABLE_ROW: u64 = 100 + 61;

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
    // The motion lab, chosen by a cue: every easing and kind at rest, the spinner turning (a
    // frame is always due).
    UiScene {
        id: "showcase-lab-motion",
        scale: 1.0,
        tier: MotionTier::Full,
        state: home,
        script: &[Step {
            at_ms: 0,
            events: &[Ev::Cue("page lab-motion")],
        }],
        capture_ms: 300,
        expect: Expect::Waiting,
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
                events: &[Ev::PointerOverKey(FOUND_ROWS, TABLE_ROW)],
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
            guide_style: GuideStyle::Elbow,
            chevron: ChevronStyle::Triangle,
            stub: GuideStub::Label,
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
            title = format!("{title}  /  {}", nav::scene_label(page, s));
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
        ui.scroll_area("content", lay.pane, &opts, |ui, view| {
            // The pane clips the page, whichever layer the page draws on.
            let clip = Rect::new(
                lay.pane.x + view.offset.0 as f32,
                lay.pane.y + view.offset.1 as f32,
                lay.pane.w,
                lay.pane.h,
            );
            ui.scope(("page", generation), |ui| {
                ui.scope(page.id, |ui| {
                    root = ui.scope_id();
                    child.draw(ui, &Stage::at(canvas).clipped(clip));
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
        // Tab visits the hosted page first, as a scene's script (a Tab at its start) expects, then
        // the sidebar, then the settings: tab scopes say so, whatever the panes' draw order.
        ui.tab_scope(0, |ui| self.content(ui, &lay));
        // The panes stay outside the page's modals: a hit on `Outside` is above a modal's layer
        // and stays live, so a hosted dialog that never closes cannot take the way out with it.
        ui.outside_modals(|ui| {
            ui.tab_scope(1, |ui| self.sidebar(ui, &lay));
            // The settings pane has a fill of its own and is drawn after the page.
            let fill = ui.theme().surface_input;
            ui.painter().fill(lay.settings, 8.0, fill);
            ui.tab_scope(2, |ui| {
                self.settings
                    .pane(ui, lay.settings, &mut *self.child, self.generation);
            });
            if self.replay.is_none() && self.scene.is_some() {
                let t = *ui.theme();
                let spec = Spec::plain("live", Style::ui(t.font_s, t.fg_tertiary));
                let w = ui.painter().measure(&spec).0;
                ui.painter()
                    .text(&spec, lay.head.right() - w, lay.head.y + 6.0);
            }
        });
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
        s.settings.noise = 0.0;
        s.settings.material = Material::Opaque;
        assert!((s.time_scale() - 0.5).abs() < f32::EPSILON);
        assert!((s.zoom() - 1.5).abs() < f32::EPSILON);
        assert_eq!(s.material(), Some(Material::Opaque));
        let tweaks = s.frame_params().expect("tweaks");
        let params = tweaks.params((200, 100), 2.0);
        assert_eq!((params.size, params.scale), ((200, 100), 2.0));
        assert!((params.blur_sigma - 12.0).abs() < f32::EPSILON);
        assert!((params.saturation - 2.5).abs() < f32::EPSILON);
        // Noise off is a strength of 0, which the renderer reads.
        assert_eq!(params.noise_opacity, 0.0);
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

    /// A page that draws nothing and keeps its knobs.
    struct Silent(Box<dyn PageState>);

    impl PageState for Silent {
        fn draw(&mut self, _ui: &mut Ui<'_>, _stage: &Stage) {}

        fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut knobs::Knobs<'_>) {
            self.0.knobs(ui, panel);
        }
    }

    /// What the base and page layers of `scene` paint, each with the rect it can show in: a
    /// shape's own rect cut by its clip; a text's clip (its extent is not known here).
    fn painted(scene: &lune_ui::scene::Scene) -> Vec<(String, Rect)> {
        let mut out = Vec::new();
        let rect = |r: [f32; 4]| Rect::new(r[0], r[1], r[2], r[3]);
        for layer in &scene.layers[..2] {
            for t in &layer.texts {
                out.push((
                    format!("text {:.1},{:.1} in {:?}", t.x, t.y, t.clip),
                    t.clip,
                ));
            }
            let g = &layer.geometry;
            let acrylic = g.acrylic.iter().map(|a| &a.shape);
            for s in g.under.iter().chain(&g.over).chain(acrylic) {
                let shown = if s.clip[2] > 0.0 {
                    rect(s.rect).intersect(&rect(s.clip))
                } else {
                    rect(s.rect)
                };
                out.push((format!("shape {:?} in {:?}", s.rect, s.clip), shown));
            }
        }
        out
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.w > 0.0
            && a.h > 0.0
            && a.x < b.right()
            && b.x < a.right()
            && a.y < b.bottom()
            && b.y < a.bottom()
    }

    /// At a window as narrow as 1,040 the canvas is wider than its pane: nothing the hosted page
    /// paints may show over the settings pane, whatever page it is and whatever it does with the
    /// layers. A hosted page's items are what a frame holds beyond the same frame with the page
    /// drawing nothing.
    #[test]
    fn a_hosted_page_never_paints_over_the_settings_pane() {
        let window = (1040.0, 660.0);
        let settings = Layout::of(Rect::new(10.0, 34.0, window.0 - 20.0, window.1 - 44.0)).settings;
        let frame = |child: Box<dyn PageState>, page: usize| {
            let mut shell = Showcase::new();
            shell.page = page;
            shell.child = child;
            let mut p = Player::new(HOST, Box::new(shell), window, 1.0, MotionTier::Full);
            p.frame(&mut FakeText, 0, &[]).expect("frame");
            painted(&p.frame(&mut FakeText, 16, &[]).expect("frame").0)
        };
        let mut bad = Vec::new();
        for (page_index, page) in PAGES.iter().enumerate() {
            let PageKind::Ui(ui_page) = page.kind else {
                continue;
            };
            if page.id == HOST {
                continue;
            }
            let states = std::iter::once(("live", ui_page.state))
                .chain(ui_page.scenes.iter().map(|s| (s.id, s.state)));
            for (which, state) in states {
                // The same page with its drawing left out: what it leaves is the panes, with the
                // page's knobs in the settings pane, which belong there.
                let mut rest = frame(Box::new(Silent(state())), page_index);
                for (key, shown) in frame(state(), page_index) {
                    if let Some(at) = rest.iter().position(|(k, _)| *k == key) {
                        rest.swap_remove(at);
                    } else if overlaps(shown, settings) {
                        bad.push(format!("{}/{which}: {key}", page.id));
                    }
                }
            }
        }
        assert!(
            bad.is_empty(),
            "{} items over the settings pane, e.g.\n  {}",
            bad.len(),
            bad.iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }

    /// A Tab at the start of a shell scene lands on the hosted page's first stop, not on the
    /// sidebar's search field, which draws after the page and is scoped after it.
    #[test]
    fn the_first_tab_reaches_the_hosted_page_before_the_sidebar_and_the_settings() {
        let window = (1600.0, 900.0);
        let lay = Layout::of(Rect::new(10.0, 34.0, window.0 - 20.0, window.1 - 44.0));
        let mut shell = Showcase::new();
        shell.cue("page button");
        let mut p = Player::new(HOST, Box::new(shell), window, 1.0, MotionTier::Full);
        p.frame(&mut FakeText, 0, &[]).expect("frame");
        p.frame(&mut FakeText, 16, &[]).expect("frame");
        let mut ms = 32;
        let mut at = |p: &mut Player, evs: &[Ev]| {
            ms += 16;
            p.frame(&mut FakeText, ms, evs).expect("frame").1
        };
        let mut focused_node = |p: &mut Player, evs: &[Ev]| {
            let out = at(p, evs);
            let f = out.focused?;
            out.tree.nodes.iter().find(|n| n.id == f).map(|n| n.bounds)
        };
        let first = focused_node(&mut p, &[Ev::Tab]).expect("a stop takes the first Tab");
        assert!(
            overlaps(first, lay.pane),
            "{first:?} is in the hosted page, {:?}",
            lay.pane
        );
        // Walk on: the sidebar's stops come after the page's, the settings' after those.
        let mut seen_sidebar = false;
        let mut seen_settings = false;
        for _ in 0..200 {
            let Some(b) = focused_node(&mut p, &[Ev::Tab]) else {
                continue;
            };
            if overlaps(b, lay.tree) || overlaps(b, lay.search) {
                assert!(!seen_settings, "the sidebar comes before the settings");
                seen_sidebar = true;
            }
            if overlaps(b, lay.settings) {
                seen_settings = true;
            }
            if seen_settings {
                break;
            }
        }
        assert!(
            seen_sidebar && seen_settings,
            "{seen_sidebar} {seen_settings}"
        );
    }

    /// The `showcase-table` script aims at the first match of "table" by its row key, so the key
    /// must be the one the sidebar gives that row.
    #[test]
    fn the_table_scenes_row_key_is_the_first_match_of_the_search() {
        let rows = nav::rows("table");
        let first = rows.get(1).expect("a match");
        assert_eq!(first.target.key(), TABLE_ROW, "{:?}", first.target);
        let table = nav::index_of("table").expect("a page");
        assert_eq!(first.target, Target::Page(table));
    }

    /// A hosted page that changes layer (the glass lab does) must not leave its clip on the
    /// panes drawn after it: the sidebar's text is clipped to the sidebar, not to the content.
    #[test]
    fn a_page_that_changes_layer_leaves_the_sidebar_and_settings_unclipped() {
        let window = (1600.0, 900.0);
        let lay = Layout::of(Rect::new(10.0, 34.0, window.0 - 20.0, window.1 - 44.0));
        let texts_in = |page: &str, region: Rect| {
            let mut shell = Showcase::new();
            shell.cue(&format!("page {page}"));
            let mut p = Player::new(HOST, Box::new(shell), window, 1.0, MotionTier::Full);
            p.frame(&mut FakeText, 0, &[]).expect("frame");
            let scene = p.frame(&mut FakeText, 16, &[]).expect("frame").0;
            let texts = &scene.merged(lune_ui::scene::LayerId::Outside).texts;
            let inside: Vec<_> = texts
                .iter()
                .filter(|t| region.contains((t.x + 2.0, t.y + 2.0)))
                .collect();
            let shown = inside.iter().filter(|t| overlaps(t.clip, region)).count();
            (inside.len(), shown)
        };
        for region in [lay.tree, lay.settings] {
            let (n, shown) = texts_in("lab-glass", region);
            assert!(n > 3, "texts in {region:?}");
            assert_eq!(n, shown, "a clip left on cuts the pane at {region:?}");
            // The same on a page that stays on one layer, as the instrument's check.
            let (n, shown) = texts_in("button", region);
            assert_eq!(n, shown);
        }
    }

    /// Cueing `dialog-popover@2x` over `dialog-combo-open@2x` replaces the page's dialog in one
    /// frame. Once the first one's exit is over, the dialog's text is on `Modal` and the
    /// popover's, opened in it, on `Raised`: the same split as the scene cued on a fresh shell.
    #[test]
    fn a_dialog_replaced_by_another_returns_to_modal_and_its_popover_stays_raised() {
        use lune_ui::scene::LayerId;
        // The text counts of `Modal` and `Raised` after the cues, each settled for 4 s.
        let counts = |cues: &[&'static str]| {
            let mut p = player();
            run(&mut p, 0, 0, &[]);
            let mut ms = 16;
            for cue in cues {
                p.frame(&mut FakeText, ms, &[Ev::Cue(cue)]).expect("frame");
                ms += 16;
                for _ in 0..250 {
                    p.frame(&mut FakeText, ms, &[]).expect("frame");
                    ms += 16;
                }
            }
            let scene = p.frame(&mut FakeText, ms, &[]).expect("frame").0;
            let n = |l: LayerId| scene.merged(l).texts.len();
            (n(LayerId::Modal), n(LayerId::Raised))
        };
        let replaced = counts(&["scene dialog-combo-open@2x", "scene dialog-popover@2x"]);
        let fresh = counts(&["scene dialog-popover@2x"]);
        assert!(replaced.0 > 0, "the dialog's text is on Modal");
        assert_eq!(replaced, fresh);
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

    /// A hosted page's modal must not take the shell with it: `locale` opens one that never
    /// closes, so the sidebar has to stay the way out.
    #[test]
    fn a_hosted_modal_leaves_the_sidebar_live() {
        let mut rig = Rig::new();
        rig.shell.cue("page locale");
        rig.frame();
        rig.settle();
        let lay = SCENE_LAYOUT;
        let widgets = 7;
        rig.click_at(lay.tree_row(widgets));
        rig.settle();
        rig.click_at(lay.tree_row(widgets + 1));
        rig.settle();
        assert_ne!(
            PAGES[rig.shell.page].id, "locale",
            "the sidebar took the click"
        );
    }

    /// The toolkit never closes an overlay: the page must stop calling it. The `ui` page's dialog
    /// closes on its own Cancel.
    #[test]
    fn a_hosted_dialog_closes_on_its_cancel() {
        let mut rig = Rig::new();
        rig.shell.cue("page ui");
        rig.frame();
        let out = rig.frame();
        let mid = |n: &lune_ui::access::Node| {
            (n.bounds.x + n.bounds.w / 2.0, n.bounds.y + n.bounds.h / 2.0)
        };
        rig.click_at(mid(named(&out, "Dialog").expect("the knob")));
        rig.settle();
        let out = rig.frame();
        rig.click_at(mid(named(&out, "Cancel").expect("the dialog's Cancel")));
        rig.settle();
        let out = rig.frame();
        assert!(named(&out, "Cancel").is_none(), "the dialog closed");
    }

    /// A press outside a page's menu closes it, in the page and in the panes above it.
    #[test]
    fn a_hosted_menu_closes_on_a_press_outside() {
        let mid = |n: &lune_ui::access::Node| {
            (n.bounds.x + n.bounds.w / 2.0, n.bounds.y + n.bounds.h / 2.0)
        };
        for outside in ["Off", "Green"] {
            let mut rig = Rig::new();
            rig.shell.cue("page ui");
            rig.frame();
            let out = rig.frame();
            rig.click_at(mid(named(&out, "Menu").expect("the knob")));
            rig.settle();
            let out = rig.frame();
            assert!(named(&out, "New Window").is_some(), "the menu is open");
            rig.click_at(mid(named(&out, outside).expect("a target outside")));
            rig.settle();
            let out = rig.frame();
            assert!(
                named(&out, "New Window").is_none(),
                "a press on {outside} closed it"
            );
        }
    }

    /// No page or scene can trap the shell: whatever it leaves open, the settings pane still
    /// takes a click (a press may only close a menu).
    #[test]
    fn no_scene_traps_the_shell() {
        let mid = |n: &lune_ui::access::Node| {
            (n.bounds.x + n.bounds.w / 2.0, n.bounds.y + n.bounds.h / 2.0)
        };
        let mut stuck = Vec::new();
        for page in PAGES.iter().filter(|p| p.id != HOST) {
            for scene in nav::scenes(page) {
                let mut rig = Rig::sized(1600, 900);
                rig.shell.cue(&format!("scene {}", scene.name()));
                for _ in 0..(scene.capture_ms / 16 + 20) {
                    rig.frame();
                }
                let mut took = false;
                // Up to one press per open menu, then the one that lands.
                for _ in 0..3 {
                    let out = rig.frame();
                    // Two targets: a page with its own theme or tier knob hides one of them.
                    for name in ["Purple", "Reduced"] {
                        if let Some(n) = named(&out, name) {
                            rig.click_at(mid(n));
                        }
                    }
                    rig.settle();
                    took |= rig.shell.settings.theme != 0
                        || rig.shell.settings.tier == MotionTier::Reduced;
                }
                if !took {
                    stuck.push(scene.name());
                }
            }
        }
        assert!(stuck.is_empty(), "scenes that trap the shell: {stuck:#?}");
    }
}
