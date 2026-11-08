//! Knobs: the per-component controls at the top of the showcase's settings pane (D3).
//!
//! A page that has options worth turning overrides `PageState::knobs` and declares one row per
//! option, each bound to a field of the page:
//!
//! ```ignore
//! fn knobs(&mut self, ui: &mut Ui<'_>, panel: &mut Knobs<'_>) {
//!     panel.choice(ui, "Kind", &["Primary", "Secondary"], &mut self.kind);
//!     panel.toggle(ui, "Enabled", &mut self.enabled);
//! }
//! ```
//!
//! The pane draws after the page, so a turned knob shows in the next frame: a row asks for one
//! when it changes. A knob's starting value is what the page draws without any knob, so a
//! scene (which never draws the pane) and the live page agree until a knob is turned.

use lune_ui::ui::Ui;
use lune_ui::widgets::field::FieldOptions;
use lune_ui::widgets::radio_group::RadioOption;
use lune_ui::widgets::tone::Tone;

use super::settings::Col;

/// The tones, in the order a knob lists them.
pub(crate) const TONES: [(&str, Tone); 6] = [
    ("Neutral", Tone::Neutral),
    ("Ok", Tone::Ok),
    ("Warn", Tone::Warn),
    ("Err", Tone::Err),
    ("Data", Tone::Data),
    ("Paused", Tone::Paused),
];

/// The names of [`TONES`].
pub(crate) fn tone_names() -> Vec<&'static str> {
    TONES.iter().map(|(n, _)| *n).collect()
}

/// The rows of the page's knobs, drawn down the settings column.
#[derive(Debug)]
pub(crate) struct Knobs<'c> {
    col: &'c mut Col,
    rows: usize,
}

impl<'c> Knobs<'c> {
    pub(crate) fn new(col: &'c mut Col) -> Self {
        Self { col, rows: 0 }
    }

    /// Whether the page declared no knob.
    pub(crate) fn is_empty(&self) -> bool {
        self.rows == 0
    }

    /// A frame after a change, for the page to draw it.
    fn turned(&mut self, ui: &mut Ui<'_>, changed: bool) -> bool {
        self.rows += 1;
        if changed {
            ui.request_repaint_at(ui.time());
        }
        changed
    }

    /// A switch for a yes-or-no option. True when it was turned.
    pub(crate) fn toggle(&mut self, ui: &mut Ui<'_>, label: &str, on: &mut bool) -> bool {
        let before = *on;
        self.col.toggle(ui, label, label, on);
        self.turned(ui, *on != before)
    }

    /// A button for an action the page takes at once (push a toast, say). True when pressed.
    pub(crate) fn button(&mut self, ui: &mut Ui<'_>, label: &str) -> bool {
        let pressed = self.col.button(ui, label, label);
        self.turned(ui, pressed)
    }

    /// One of `options`, by index into it. True when the choice changed.
    pub(crate) fn choice(
        &mut self,
        ui: &mut Ui<'_>,
        label: &str,
        options: &[&str],
        selected: &mut usize,
    ) -> bool {
        let rows: Vec<RadioOption<'_>> = options
            .iter()
            .enumerate()
            .map(|(i, text)| RadioOption {
                key: i as u64,
                label: text,
                enabled: true,
            })
            .collect();
        let mut chosen = (*selected).min(options.len().saturating_sub(1)) as u64;
        self.col.label(ui, label);
        let changed = self.col.radio(ui, label, &rows, &mut chosen, label);
        if changed {
            *selected = chosen as usize;
        }
        self.turned(ui, changed)
    }

    /// A number in `min..=max`; a `step` of 1 or more shows whole numbers. True when moved.
    pub(crate) fn number(
        &mut self,
        ui: &mut Ui<'_>,
        label: &str,
        value: &mut f32,
        (min, max, step): (f32, f32, f32),
    ) -> bool {
        let whole = step >= 1.0;
        let changed = self
            .col
            .slider(ui, label, label, value, (min, max, step), |v| {
                if whole {
                    format!("{v:.0}")
                } else {
                    format!("{v:.1}")
                }
            });
        self.turned(ui, changed)
    }

    /// A line of text. True when it was edited.
    pub(crate) fn text(&mut self, ui: &mut Ui<'_>, label: &str, value: &mut String) -> bool {
        self.col.label(ui, label);
        let rect = lune_ui::paint::Rect::new(self.col.x, self.col.y - 4.0, self.col.w, 28.0);
        let options = FieldOptions {
            label: Some(label),
            ..FieldOptions::default()
        };
        let changed = ui
            .text_field_with(label, rect, value, &options)
            .response
            .changed;
        self.col.y += 32.0;
        self.turned(ui, changed)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{HOST, SCENE_LAYOUT, Showcase};
    use crate::page::{Ev, Player};
    use lune_ui::access::Role;
    use lune_ui::input::{FrameOutput, Modifiers, Repaint};
    use lune_ui::motion::MotionTier;
    use lune_ui::paint::Rect;
    use lune_ui::scene::Scene;
    use lune_ui::testing::FakeText;

    /// The shell on a 1,600 x 900 window with one page chosen, driven the way a window drives it:
    /// the pointer moves and presses over the node a control draws.
    struct Shell {
        player: Player,
        ms: u32,
        last: (Scene, FrameOutput),
    }

    impl Shell {
        fn open(page: &str) -> Shell {
            let mut player = Player::new(
                HOST,
                Box::new(Showcase::new()),
                (1600.0, 900.0),
                1.0,
                MotionTier::Full,
            );
            let first = player.frame(&mut FakeText, 0, &[]).expect("frame");
            let mut shell = Shell {
                player,
                ms: 0,
                last: first,
            };
            let cue: &'static str = Box::leak(format!("page {page}").into_boxed_str());
            shell.frame(&[Ev::Cue(cue)]);
            shell.settle();
            shell
        }

        fn frame(&mut self, events: &[Ev]) {
            self.ms += 16;
            self.last = self
                .player
                .frame(&mut FakeText, self.ms, events)
                .expect("frame");
        }

        /// Frames for 600 ms: every motion of the page and the pane is done.
        fn settle(&mut self) {
            for _ in 0..38 {
                self.frame(&[]);
            }
        }

        fn centre(&self, role: Role, name: &str) -> Option<(f32, f32)> {
            self.last
                .1
                .tree
                .nodes
                .iter()
                .find(|n| n.role == role && n.name == name)
                .map(|n| (n.bounds.x + n.bounds.w / 2.0, n.bounds.y + n.bounds.h / 2.0))
        }

        /// Presses the control, leaving the frame of the press as the last but one; true when it
        /// is there.
        fn press(&mut self, role: Role, name: &str) -> bool {
            let Some((x, y)) = self.centre(role, name) else {
                return false;
            };
            self.frame(&[Ev::PointerAt(x, y)]);
            self.frame(&[Ev::Click]);
            true
        }

        /// Presses the control `fraction` of the way along it (a slider's value follows).
        fn press_along(&mut self, role: Role, name: &str, fraction: f32) -> bool {
            let Some(n) = self
                .last
                .1
                .tree
                .nodes
                .iter()
                .find(|n| n.role == role && n.name == name)
            else {
                return false;
            };
            let (x, y) = (
                n.bounds.x + n.bounds.w * fraction,
                n.bounds.y + n.bounds.h / 2.0,
            );
            self.frame(&[Ev::PointerAt(x, y)]);
            self.frame(&[Ev::Click]);
            true
        }

        /// Presses a control and lets the page draw what it made of it.
        fn turn(&mut self, role: Role, name: &str) {
            assert!(self.press(role, name), "no {role:?} named {name}");
            self.settle();
        }

        /// What the hosted page draws: the items of the frame that lie in the content pane,
        /// whatever the layer (a dialog is on another).
        fn page(&self) -> Vec<String> {
            let pane = SCENE_LAYOUT.pane;
            let inside = |r: Rect| pane.contains((r.x + r.w / 2.0, r.y + r.h / 2.0));
            let mut out = Vec::new();
            for layer in self.last.0.in_order() {
                let g = &layer.geometry;
                let acrylic = g.acrylic.iter().map(|a| &a.shape);
                for s in g.under.iter().chain(&g.over).chain(acrylic) {
                    let [x, y, w, h] = s.rect;
                    if inside(Rect::new(x, y, w, h)) {
                        out.push(format!("shape {:?} {:?} {:?}", s.rect, s.color, s.radii));
                    }
                }
                // Strokes and filled paths (a spinner's arc, a chart's line) by their vertices.
                for v in g.strokes.iter().chain(&g.fills) {
                    if pane.contains((v.pos[0], v.pos[1])) {
                        out.push(format!("vertex {:?} {:?}", v.pos, v.color));
                    }
                }
                // Pictures, and text drawn turned (a spinner's glyph).
                for i in &layer.images {
                    if inside(i.dst) {
                        out.push(format!("image {:?} {:?}", i.source, i.dst));
                    }
                }
                for t in &layer.texts {
                    if inside(Rect::new(t.x, t.y, 1.0, 1.0)) {
                        out.push(format!("text {:?} {} {}", t.key, t.x, t.y));
                    }
                }
            }
            out
        }
    }

    fn typed_into(shell: &mut Shell, field: &str, text: &'static str) {
        let (x, y) = shell.centre(Role::TextInput, field).expect("the field");
        shell.frame(&[Ev::PointerAt(x, y)]);
        shell.frame(&[Ev::Click]);
        let all = Modifiers {
            logo: true,
            ..Modifiers::default()
        };
        shell.frame(&[Ev::KeyChar("a", all)]);
        shell.frame(&[Ev::Type(text)]);
        shell.settle();
    }

    /// A page with knobs shows them at the top of the pane; a page with none shows none.
    #[test]
    fn a_page_with_knobs_shows_them_and_a_page_without_shows_none() {
        let with = Shell::open("button");
        for name in ["Small", "Icon", "Enabled"] {
            assert!(with.centre(Role::Switch, name).is_some(), "{name}");
        }
        assert!(with.centre(Role::Radio, "Ghost").is_some());
        assert!(with.centre(Role::TextInput, "Label").is_some());
        let without = Shell::open("label");
        assert!(without.centre(Role::Switch, "Small").is_none());
        assert!(without.centre(Role::Radio, "Ghost").is_none());
        // The global sections are still under the (empty) knobs.
        assert!(without.centre(Role::Radio, "Reduced").is_some());
    }

    /// A turn asks for a frame, so the page draws it at once, not at the next input.
    #[test]
    fn a_turned_knob_asks_for_a_frame() {
        let mut shell = Shell::open("button");
        assert!(shell.press(Role::Switch, "Small"));
        // The press is the last frame; the one before holds the click as well.
        assert_ne!(shell.last.1.repaint, Repaint::Idle);
    }

    #[test]
    fn the_button_knobs_change_the_button() {
        let mut shell = Shell::open("button");
        let rest = shell.page();
        assert!(!rest.is_empty());
        shell.turn(Role::Switch, "Small");
        let small = shell.page();
        assert_ne!(small, rest, "a small button");
        shell.turn(Role::Switch, "Small");
        assert_eq!(shell.page(), rest, "and back");
        shell.turn(Role::Radio, "Primary");
        assert_ne!(shell.page(), rest, "another kind");
        shell.turn(Role::Radio, "Secondary");
        shell.turn(Role::Switch, "Icon");
        assert_ne!(shell.page(), rest, "a glyph");
        shell.turn(Role::Switch, "Icon");
        shell.turn(Role::Switch, "Enabled");
        assert_ne!(shell.page(), rest, "disabled");
        shell.turn(Role::Switch, "Enabled");
        assert_eq!(shell.page(), rest);
        // The label is a text knob: its text is the button's.
        typed_into(&mut shell, "Label", "Go");
        assert_ne!(shell.page(), rest, "a label");
        assert!(shell.centre(Role::Button, "Go").is_some());
    }

    #[test]
    fn the_toggle_knobs_change_the_switches() {
        let mut shell = Shell::open("toggle");
        let rest = shell.page();
        shell.turn(Role::Switch, "Third enabled");
        assert_ne!(shell.page(), rest, "the third is no longer faded");
        shell.turn(Role::Switch, "Third enabled");
        assert_eq!(shell.page(), rest);
        shell.turn(Role::Switch, "First on");
        assert_ne!(shell.page(), rest, "the first switch is on");
        typed_into(&mut shell, "Second label", "Armed");
        assert!(
            shell.centre(Role::Switch, "Armed").is_some(),
            "its name too"
        );
    }

    #[test]
    fn the_table_knobs_change_the_columns_and_the_rows() {
        let mut shell = Shell::open("table");
        let rest = shell.page();
        assert!(shell.centre(Role::ColumnHeader, "Route").is_some() || !rest.is_empty());
        shell.turn(Role::Radio, "Tones");
        assert_ne!(shell.page(), rest, "other columns");
        assert!(
            shell.last.1.tree.nodes.iter().any(|n| n.name == "Strong"),
            "the tones' columns"
        );
        shell.turn(Role::Radio, "Route data");
        assert_eq!(shell.page(), rest, "back to the routes");
        shell.turn(Role::Switch, "Enabled");
        assert_ne!(shell.page(), rest, "a faded table");
    }

    /// A modal dialog blocks the whole window, the pane included: the variant is chosen while
    /// it is closed, then Open shows that variant.
    #[test]
    fn the_dialog_knobs_choose_the_variant_and_open_it() {
        let open_as = |variant: Option<&str>| {
            let mut shell = Shell::open("dialog");
            let closed = shell.page();
            if let Some(v) = variant {
                shell.turn(Role::Radio, v);
                assert_eq!(shell.page(), closed, "a closed dialog draws nothing of it");
            }
            shell.turn(Role::Switch, "Open");
            (closed, shell.page())
        };
        let (closed, confirm) = open_as(None);
        assert_ne!(confirm, closed, "the dialog is up");
        let (_, form) = open_as(Some("Form"));
        assert_ne!(form, confirm, "the form variant");
        let (_, disabled) = open_as(Some("Disabled confirm"));
        assert_ne!(disabled, confirm, "a confirm button that is faded");
    }

    #[test]
    fn the_sidebar_knobs_change_the_columns_the_footer_and_the_step() {
        let mut shell = Shell::open("sidebar");
        let all = shell.page();
        shell.turn(Role::Switch, "Status footer");
        assert_ne!(shell.page(), all, "no footers");
        shell.turn(Role::Switch, "Status footer");
        assert_eq!(shell.page(), all);
        shell.turn(Role::Radio, "Wizard");
        let wizard = shell.page();
        assert_ne!(wizard, all, "one column");
        assert!(wizard.len() < all.len());
    }

    /// One knob of a widget page turned in the shell.
    enum Turn {
        /// A press on the control.
        Press(Role, &'static str),
        /// A press on a slider, this far along it.
        Slide(&'static str, f32),
        /// Text typed into a field.
        Type(&'static str, &'static str),
        /// A press that stops the page's own motion first (so the page is still while it is
        /// measured), then the press that is measured.
        Still(Role, &'static str, Role, &'static str),
        /// A press that makes something the page draws outside the content pane (a toast at the
        /// window's corner): the semantic tree gets a node of this role and name.
        Spawns(Role, &'static str, Role, &'static str),
        /// Events fed to the page first (a file dropped on it), then the press that is measured.
        Fed(&'static [Ev], Role, &'static str),
    }

    use Turn::{Fed, Press, Slide, Spawns, Still, Type};

    /// Every widget page of this batch: turning the knob changes what the page draws.
    const WIDGET_TURNS: &[(&str, Turn)] = &[
        ("checkbox", Press(Role::Switch, "First checked")),
        ("label", Type("Sample text", "Hello")),
        ("stepper", Slide("Value", 0.9)),
        ("combo", Press(Role::Switch, "Open")),
        ("text-field", Press(Role::Radio, "Search")),
        ("scrollbar", Slide("Lines", 0.0)),
        ("badge", Press(Role::Switch, "Show a sample")),
        ("status-dot", Press(Role::Switch, "Show a sample")),
        ("verdict-row", Press(Role::Switch, "Show a sample")),
        (
            "spinner",
            Still(Role::Switch, "Busy", Role::Switch, "Last tinted"),
        ),
        ("callout", Press(Role::Switch, "Show a sample")),
        ("row", Press(Role::Switch, "Descriptions")),
        ("list-row", Press(Role::Switch, "Badge")),
        ("form-field", Press(Role::Switch, "Inline")),
        ("steps", Slide("Current step", 1.0)),
        ("nav-item", Press(Role::Switch, "Glyphs")),
        ("radio-group", Press(Role::Switch, "Last enabled")),
        ("star-rating", Slide("Value", 1.0)),
        ("slider", Press(Role::Switch, "First enabled")),
        ("filter-bar", Type("Placeholder", "Type here")),
        ("search-picker", Press(Role::Switch, "Disabled")),
        ("tab-strip", Press(Role::Switch, "Header tint")),
        ("toolbar", Press(Role::Switch, "Fetch enabled")),
        ("banner", Press(Role::Switch, "Picture")),
        // Overlays: the knobs that open one come last, since an open modal blocks the pane.
        ("dropdown", Press(Role::Switch, "Open")),
        ("menu", Press(Role::Radio, "Variants")),
        ("popover", Press(Role::Switch, "Open")),
        ("tooltip", Press(Role::Switch, "Enabled")),
        ("flyout", Press(Role::Switch, "Open")),
        (
            "toast-notifications",
            Spawns(
                Role::Button,
                "Push this toast",
                Role::Status,
                "A toast from the knobs",
            ),
        ),
        ("command-palette", Press(Role::Switch, "Open")),
        // Layout.
        ("stack", Press(Role::Radio, "Grid")),
        ("scroll-area", Slide("Lines", 0.0)),
        ("virtual-list", Slide("Rows", 0.0)),
        ("page", Press(Role::Switch, "Footer")),
        ("panel", Press(Role::Radio, "Strip")),
        ("side-panel", Press(Role::Radio, "Branches")),
        ("icon-rail", Press(Role::Switch, "Dot on Changes")),
        ("split-panes", Press(Role::Radio, "Second")),
        ("tree-view", Press(Role::Switch, "Indent guides")),
        ("settings-page", Press(Role::Switch, "Restart notice")),
        // Foundation.
        ("ui", Press(Role::Radio, "Menu")),
        ("focus", Press(Role::Switch, "Dialog over the controls")),
        ("image", Press(Role::Radio, "alpha 0.5")),
        ("primitives", Press(Role::Radio, "dash")),
        ("theme", Press(Role::Radio, "Blue")),
        ("motion", Press(Role::Switch, "Flip the swatches")),
        // Data.
        ("progress", Press(Role::Switch, "Show a sample")),
        ("stat", Press(Role::Switch, "Show a sample")),
        ("kv-list", Press(Role::Switch, "Show a sample")),
        ("bar-chart", Press(Role::Radio, "Empty")),
        ("pie-chart", Press(Role::Radio, "Half")),
        ("kanban-board", Press(Role::Switch, "Error view")),
        ("lane-graph", Slide("Commits", 0.0)),
        // Text.
        ("text-edit", Press(Role::Radio, "Error")),
        ("ime", Press(Role::Radio, "Area")),
        ("code-block", Press(Role::Radio, "Empty")),
        ("code-editor-view", Press(Role::Switch, "Line numbers")),
        ("markdown-preview", Press(Role::Radio, "Rule")),
        ("diff-view", Press(Role::Switch, "Enabled")),
        // Core, platform and infra.
        ("a11y", Press(Role::Switch, "Node outlines")),
        ("drag-and-drop", Slide("Tabs", 0.0)),
        ("tray", Press(Role::Radio, "degraded")),
        (
            "file-drop",
            Fed(
                &[Ev::FileDrop("/tmp/example.txt")],
                Role::Button,
                "Clear the list",
            ),
        ),
        ("window", Press(Role::Switch, "Maximized")),
        // Labs (the motion lab has a test of its own: its spinner never rests).
        ("lab-glass", Slide("Panel height", 0.0)),
        ("lab-blur", Slide("Panel width", 0.0)),
        ("lab-effects", Press(Role::Radio, "History")),
        ("lab-json", Press(Role::Radio, "A mistake")),
    ];

    /// Pages that have no knob, with the reason. A page not on this list must have one.
    const NO_KNOBS: &[(&str, &str)] = &[
        (
            "locale",
            "its dialog is modal and always open, so the pane could not be pressed; the pseudo-locale is the page's subject",
        ),
        (
            "shortcuts",
            "both menus are open from the start and a press outside dismisses them before it reaches the pane",
        ),
        (
            "lab-tokens",
            "it lists the running theme; the settings pane's theme and glass controls are its knobs",
        ),
        (
            "lab-clipboard",
            "its two buttons are the options: copy writes the first box, paste reads into the second",
        ),
    ];

    /// How many knob rows the pane of the shell shows for the hosted page: the nodes between the
    /// settings pane's scroll area and its first global section.
    fn knob_nodes(shell: &Shell) -> usize {
        let nodes = &shell.last.1.tree.nodes;
        let Some(theme) = nodes
            .iter()
            .position(|n| n.role == Role::RadioGroup && n.name == "Theme")
        else {
            return 0;
        };
        nodes[..theme]
            .iter()
            .rev()
            .take_while(|n| n.role != Role::ScrollArea)
            .count()
    }

    /// Every page of the sidebar declares a knob or is on [`NO_KNOBS`]; the list holds only pages
    /// that really have none, and names only pages of the sidebar.
    #[test]
    fn every_page_of_the_sidebar_has_a_knob_or_a_reason_not_to() {
        use super::super::nav;
        use crate::pages::PAGES;
        let mut missing = Vec::new();
        let mut stale = Vec::new();
        let mut listed = 0;
        for index in nav::order() {
            let id = PAGES[index].id;
            let shell = Shell::open(id);
            let knobs = knob_nodes(&shell);
            let exempt = NO_KNOBS.iter().any(|(page, _)| *page == id);
            listed += usize::from(exempt);
            match (knobs > 0, exempt) {
                (false, false) => missing.push(id),
                (true, true) => stale.push(id),
                _ => {}
            }
        }
        assert!(
            missing.is_empty(),
            "pages with no knob and no reason: {missing:?}"
        );
        assert!(
            stale.is_empty(),
            "pages on NO_KNOBS that have a knob: {stale:?}"
        );
        assert_eq!(
            listed,
            NO_KNOBS.len(),
            "NO_KNOBS names a page the sidebar lacks"
        );
        // The instrument: a page known to have knobs reads as having them, and a page known
        // to have none reads as having none.
        assert!(knob_nodes(&Shell::open("button")) >= 5);
        assert_eq!(knob_nodes(&Shell::open("locale")), 0);
    }

    #[test]
    fn a_knob_on_each_widget_page_changes_the_page() {
        let mut unchanged = Vec::new();
        for (page, turn) in WIDGET_TURNS {
            let mut shell = Shell::open(page);
            if let Still(role, name, ..) = turn {
                shell.turn(*role, name);
            }
            if let Fed(events, ..) = turn {
                shell.frame(events);
                shell.settle();
            }
            let rest = shell.page();
            assert!(!rest.is_empty(), "{page} draws");
            match turn {
                Still(_, _, role, name) => {
                    assert!(shell.press(*role, name), "{page}: no {name}");
                }
                Spawns(role, name, made_role, made) => {
                    assert!(shell.press(*role, name), "{page}: no {name}");
                    shell.settle();
                    assert!(
                        shell.centre(*made_role, made).is_some(),
                        "{page}: pressing {name} made no {made_role:?} {made}"
                    );
                    continue;
                }
                Fed(_, role, name) => assert!(shell.press(*role, name), "{page}: no {name}"),
                Press(role, name) => assert!(shell.press(*role, name), "{page}: no {name}"),
                Slide(name, at) => {
                    assert!(
                        shell.press_along(Role::Slider, name, *at),
                        "{page}: no {name}"
                    );
                }
                Type(field, text) => {
                    typed_into(&mut shell, field, text);
                }
            }
            shell.settle();
            if shell.page() == rest {
                unchanged.push(*page);
            }
        }
        assert!(
            unchanged.is_empty(),
            "a turned knob changed nothing on {unchanged:?}"
        );
    }
}
