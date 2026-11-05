//! Split panes (`layout.split-panes` acceptance, D150): the layout of both axes and sides, the
//! pointer drag (size = press size + travel, clamped, `min` winning, the window clamping what is
//! drawn and not what is stored), the keys, the accessibility actions, Escape, hidden, persistence,
//! the 8 px hit over the 4 px strip, probes at scale 2, the focus-visible fill, the cursor, the
//! node and the disabled walk.

use lune_ui_core::access::{AccessAction, AccessValue, Action, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{
    CursorIcon, FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint,
};
use lune_ui_core::layout::Axis;
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Rect, Xform};
use lune_ui_core::scene::Scene;
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{UiHarness, disabled_violations};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::{ButtonKind, ButtonOptions};
use lune_ui_core::widgets::dialog::{Cancel, Dialog, DialogGlass};
use lune_ui_core::widgets::split_panes::{
    FOCUS, HIT, KEY_STEP, Pane, STRIP, SplitOptions, SplitResponse, SplitSide, bounds, geometry,
};

const WIN: (f32, f32) = (700.0, 400.0);
/// 500 wide, 300 high, at (20, 30): the room along either axis is the length less the strip.
const RECT: Rect = Rect::new(20.0, 30.0, 500.0, 300.0);
const P: PointerButton = PointerButton::Primary;
const GHOST: ButtonOptions<'static> = ButtonOptions {
    kind: ButtonKind::Ghost,
    small: false,
    icon: None,
};

fn split_id() -> WidgetId {
    WidgetId::ROOT.with("split")
}

#[derive(Clone)]
struct Cfg {
    rect: Rect,
    axis: Axis,
    sized: SplitSide,
    default: f32,
    min: f32,
    max: f32,
    min_other: f32,
    hidden: bool,
    enabled: bool,
    /// A button in each pane, against the strip.
    buttons: bool,
    /// The split sits in a keep-alive scope, drawn or only kept alive.
    keep: Keep,
    /// The split is the body of a dialog.
    dialog: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Keep {
    No,
    Drawn,
    Marked,
    /// Nothing at all.
    Absent,
}

fn cfg() -> Cfg {
    Cfg {
        rect: RECT,
        axis: Axis::Horizontal,
        sized: SplitSide::First,
        default: 200.0,
        min: 100.0,
        max: 400.0,
        min_other: 60.0,
        hidden: false,
        enabled: true,
        buttons: false,
        keep: Keep::No,
        dialog: false,
    }
}

fn with_buttons() -> Cfg {
    Cfg {
        buttons: true,
        ..cfg()
    }
}

fn rows() -> Cfg {
    Cfg {
        axis: Axis::Vertical,
        sized: SplitSide::Second,
        default: 100.0,
        min: 40.0,
        max: 200.0,
        min_other: 40.0,
        ..cfg()
    }
}

fn columns_second() -> Cfg {
    Cfg {
        sized: SplitSide::Second,
        ..cfg()
    }
}

impl Cfg {
    fn opts(&self) -> SplitOptions<'static> {
        SplitOptions {
            axis: self.axis,
            sized: self.sized,
            default: self.default,
            min: self.min,
            max: self.max,
            min_other: self.min_other,
            hidden: self.hidden,
            name: "Side panel width",
        }
    }
}

struct Out {
    scene: Scene,
    out: FrameOutput,
    split: Option<SplitResponse>,
    panes: Vec<(Pane, Rect)>,
    clicks: Vec<Pane>,
    stored: Option<Option<f32>>,
    /// Whether any frame of the call changed the size (set by `Clock::two`).
    changed_any: bool,
    /// The dialog around the split (if any) reported a cancel this frame.
    dialog_cancelled: bool,
}

impl Out {
    fn size(&self) -> f32 {
        self.split.as_ref().expect("drawn").size
    }

    fn pane(&self, which: Pane) -> Rect {
        self.panes
            .iter()
            .find(|p| p.0 == which)
            .expect("pane drawn")
            .1
    }
}

/// One frame; `write` is the app writing the stored size before the split is drawn.
fn frame_with(h: &mut UiHarness, c: &Cfg, write: Option<Option<f32>>) -> Out {
    let mut split = None;
    let mut panes = Vec::new();
    let mut clicks = Vec::new();
    let mut stored = None;
    let mut dialog_cancelled = false;
    let (scene, out) = h.frame(|ui| {
        let opts = c.opts();
        let mut run = |ui: &mut lune_ui_core::ui::Ui<'_>| {
            if let Some(w) = write {
                ui.split_state("split").size = w;
            }
            let r = ui.with_enabled(c.enabled, |ui| {
                ui.split_panes("split", c.rect, &opts, |ui, which, rect| {
                    panes.push((which, rect));
                    if c.buttons {
                        // Against the strip: the first pane's at its end, the second's at its start.
                        let b = match which {
                            Pane::First => Rect::new(rect.right() - 40.0, rect.y + 8.0, 40.0, 24.0),
                            Pane::Second => Rect::new(rect.x, rect.y + 8.0, 40.0, 24.0),
                        };
                        if ui.button_with(("b", which as u8), b, "B", &GHOST).clicked {
                            clicks.push(which);
                        }
                    }
                })
            });
            stored = Some(ui.split_state("split").size);
            split = Some(r);
        };
        match c.keep {
            Keep::No if c.dialog => {
                let d = Dialog {
                    title: "Panes",
                    message: "m",
                    extra_h: 100.0,
                    confirm: "OK",
                    cancel: Cancel::Default,
                };
                let glass = DialogGlass {
                    panel: [0.0; 4],
                    fill: [0.0; 4],
                    noise: false,
                };
                dialog_cancelled = ui
                    .dialog("dlg", &d, &glass, |ui, _, _, _| run(ui))
                    .cancelled;
            }
            Keep::No => run(ui),
            Keep::Drawn => ui.keep_alive("page", |ui| run(ui)),
            Keep::Marked => ui.mark_alive("page"),
            Keep::Absent => {}
        }
    });
    Out {
        scene,
        out,
        split,
        panes,
        clicks,
        stored,
        changed_any: false,
        dialog_cancelled,
    }
}

/// The clock of a test: frames 16 ms apart.
struct Clock(u64);

impl Clock {
    fn frame(&mut self, h: &mut UiHarness, c: &Cfg) -> Out {
        self.0 += 16;
        h.at_ms(self.0);
        frame_with(h, c, None)
    }

    /// Two frames: input lands at a frame's begin and a changed size asks for a catch-up.
    fn two(&mut self, h: &mut UiHarness, c: &Cfg) -> Out {
        let first = self.frame(h, c);
        let mut second = self.frame(h, c);
        // A click or a change lands on the first of the two.
        second.clicks.splice(0..0, first.clicks);
        second.changed_any = first.split.is_some_and(|s| s.changed)
            || second.split.as_ref().is_some_and(|s| s.changed);
        second
    }

    fn settle(&mut self, h: &mut UiHarness, c: &Cfg) -> Out {
        self.two(h, c);
        self.0 += 5000;
        h.at_ms(self.0);
        frame_with(h, c, None)
    }

    fn key(&mut self, h: &mut UiHarness, c: &Cfg, k: NamedKey) -> Out {
        h.input.key_down(Key::Named(k), None);
        h.input.key_up(Key::Named(k));
        self.two(h, c)
    }

    /// Tabs onto the strip (the config has no buttons, so it is the first stop).
    fn focus(&mut self, h: &mut UiHarness, c: &Cfg) -> Out {
        // One frame first: the tab order is the previous frame's.
        self.frame(h, c);
        self.key(h, c, NamedKey::Tab)
    }

    fn pointer(&mut self, h: &mut UiHarness, c: &Cfg, at: (f32, f32)) -> Out {
        h.input.pointer_to(at.0, at.1);
        self.two(h, c)
    }

    /// Presses at `from` and moves to `to`, holding.
    fn drag(&mut self, h: &mut UiHarness, c: &Cfg, from: (f32, f32), to: (f32, f32)) -> Out {
        self.pointer(h, c, from);
        h.input.press(P);
        self.frame(h, c);
        self.pointer(h, c, to)
    }

    fn release(&mut self, h: &mut UiHarness, c: &Cfg) -> Out {
        h.input.release(P);
        self.two(h, c)
    }

    fn access(&mut self, h: &mut UiHarness, c: &Cfg, a: Action, v: Option<AccessValue>) -> Out {
        h.input.access(AccessAction {
            target: split_id(),
            action: a,
            value: v,
        });
        self.two(h, c)
    }
}

fn start() -> (UiHarness, Clock) {
    (UiHarness::new(WIN), Clock(0))
}

/// The strip's centre for `c` at `size`.
fn strip_at(c: &Cfg, size: f32) -> (f32, f32) {
    let g = geometry(c.rect, c.axis, c.sized, size);
    (g.strip.x + g.strip.w / 2.0, g.strip.y + g.strip.h / 2.0)
}

/// `p` moved `d` along the axis.
fn moved(c: &Cfg, p: (f32, f32), d: f32) -> (f32, f32) {
    match c.axis {
        Axis::Horizontal => (p.0 + d, p.1),
        Axis::Vertical => (p.0, p.1 + d),
    }
}

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} vs {b}");
}

// ---- layout ----

#[test]
fn the_panes_tile_the_rect_around_a_4_px_strip_on_both_axes_and_sides() {
    let (mut h, mut clock) = start();
    let out = clock.settle(&mut h, &cfg());
    assert_eq!(out.pane(Pane::First), Rect::new(20.0, 30.0, 200.0, 300.0));
    assert_eq!(out.pane(Pane::Second), Rect::new(224.0, 30.0, 296.0, 300.0));
    let g = geometry(RECT, Axis::Horizontal, SplitSide::First, 200.0);
    assert_eq!(g.strip, Rect::new(220.0, 30.0, STRIP, 300.0));
    assert_eq!(g.hit, Rect::new(218.0, 30.0, HIT, 300.0));

    let (mut h, mut clock) = start();
    let out = clock.settle(&mut h, &columns_second());
    assert_eq!(out.pane(Pane::First), Rect::new(20.0, 30.0, 296.0, 300.0));
    assert_eq!(out.pane(Pane::Second), Rect::new(320.0, 30.0, 200.0, 300.0));
    let g = geometry(RECT, Axis::Horizontal, SplitSide::Second, 200.0);
    assert_eq!(g.strip, Rect::new(316.0, 30.0, STRIP, 300.0));

    let (mut h, mut clock) = start();
    let out = clock.settle(&mut h, &rows());
    assert_eq!(out.pane(Pane::First), Rect::new(20.0, 30.0, 500.0, 196.0));
    assert_eq!(out.pane(Pane::Second), Rect::new(20.0, 230.0, 500.0, 100.0));
    let g = geometry(RECT, Axis::Vertical, SplitSide::Second, 100.0);
    assert_eq!(g.strip, Rect::new(20.0, 226.0, 500.0, STRIP));
    assert_eq!(g.hit, Rect::new(20.0, 224.0, 500.0, HIT));

    let rows_first = Cfg {
        sized: SplitSide::First,
        ..rows()
    };
    let (mut h, mut clock) = start();
    let out = clock.settle(&mut h, &rows_first);
    assert_eq!(out.pane(Pane::First), Rect::new(20.0, 30.0, 500.0, 100.0));
    assert_eq!(out.pane(Pane::Second), Rect::new(20.0, 134.0, 500.0, 196.0));
}

#[test]
fn a_size_too_large_for_the_rect_never_takes_geometry_out_of_it() {
    let g = geometry(RECT, Axis::Horizontal, SplitSide::First, 9000.0);
    assert_eq!(g.first, Rect::new(20.0, 30.0, 496.0, 300.0));
    assert_eq!(g.strip, Rect::new(516.0, 30.0, STRIP, 300.0));
    assert_eq!(g.second.w, 0.0);
    let g = geometry(RECT, Axis::Horizontal, SplitSide::Second, f32::NAN);
    assert_eq!(g.second.w, 0.0);
    let tiny = geometry(
        Rect::new(0.0, 0.0, 2.0, 50.0),
        Axis::Horizontal,
        SplitSide::First,
        10.0,
    );
    assert!(tiny.strip.w <= 2.0 && tiny.hit.w <= 2.0);
}

#[test]
fn both_panes_clip_to_their_rects_and_have_scopes_of_their_own() {
    let (mut h, _) = start();
    let mut rects = Vec::new();
    h.frame(|ui| {
        ui.split_panes("split", RECT, &cfg().opts(), |ui, which, rect| {
            // A pane's own ids are not the other's.
            rects.push((which, rect, ui.id("x")));
            ui.painter().fill(
                Rect::new(rect.x - 50.0, rect.y, rect.w + 100.0, 10.0),
                0.0,
                [255, 0, 0, 255],
            );
        });
    });
    assert_ne!(rects[0].2, rects[1].2);
    let (scene, _) = h.frame(|ui| {
        ui.split_panes("split", RECT, &cfg().opts(), |ui, _, rect| {
            ui.painter().fill(
                Rect::new(rect.x - 50.0, rect.y, rect.w + 100.0, 10.0),
                0.0,
                [255, 0, 0, 255],
            );
        });
    });
    let red = premul([255, 0, 0, 255]);
    let fills: Vec<_> = scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.color == red)
        .collect();
    assert_eq!(fills.len(), 2);
    // The clip is the pane's rect (logical = physical at scale 1).
    assert_eq!(fills[0].clip, [20.0, 30.0, 200.0, 300.0]);
    assert_eq!(fills[1].clip, [224.0, 30.0, 296.0, 300.0]);
}

// ---- bounds ----

#[test]
fn the_bounds_are_min_to_the_lesser_of_max_and_the_room_less_min_other() {
    let o = cfg().opts();
    // Room is 500 - 4; min_other 60 leaves 436, max 400 is the tighter.
    assert_eq!(bounds(500.0, &o), (100.0, 400.0));
    // A narrower split: the room less min_other (296 - 4 - 60 = 232) is tighter.
    assert_eq!(bounds(300.0, &o), (100.0, 236.0));
    // Nothing fits: min wins.
    let tight = SplitOptions {
        min: 250.0,
        min_other: 100.0,
        ..o
    };
    assert_eq!(bounds(300.0, &tight), (250.0, 250.0));
    // Unusable bounds count as none.
    let junk = SplitOptions {
        min: f32::NAN,
        max: f32::NAN,
        min_other: f32::INFINITY,
        ..o
    };
    assert_eq!(bounds(500.0, &junk), (0.0, 496.0));
    assert_eq!(bounds(f32::NAN, &junk), (0.0, 0.0));
}

// ---- pointer ----

#[test]
fn a_drag_of_n_changes_the_size_by_n_on_both_axes_and_sides() {
    // (config, the strip's travel along the axis, the size it leaves)
    let cases = [
        (cfg(), 50.0, 250.0),
        (cfg(), -50.0, 150.0),
        (columns_second(), 30.0, 170.0),
        (columns_second(), -30.0, 230.0),
        (rows(), 20.0, 80.0),
        (rows(), -20.0, 120.0),
        (
            Cfg {
                sized: SplitSide::First,
                default: 80.0,
                ..rows()
            },
            25.0,
            105.0,
        ),
    ];
    for (c, travel, want) in cases {
        let (mut h, mut clock) = start();
        let before = clock.settle(&mut h, &c);
        let from = strip_at(&c, before.size());
        let out = clock.drag(&mut h, &c, from, moved(&c, from, travel));
        near(out.size(), want);
        assert_eq!(out.stored, Some(Some(want)));
        // The panes show it on the catch-up frame.
        let g = geometry(c.rect, c.axis, c.sized, want);
        assert_eq!(out.pane(Pane::First), g.first);
        assert_eq!(out.pane(Pane::Second), g.second);
        let out = clock.release(&mut h, &c);
        near(out.size(), want);
        assert_eq!(out.out.repaint, Repaint::Idle);
    }
}

#[test]
fn a_press_follows_at_once_with_no_threshold() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.settle(&mut h, &c);
    let from = strip_at(&c, 200.0);
    clock.pointer(&mut h, &c, from);
    h.input.press(P);
    clock.frame(&mut h, &c);
    // One px of travel (less than the 4 px drag threshold) already moves it.
    h.input.pointer_to(from.0 + 1.0, from.1);
    let out = clock.frame(&mut h, &c);
    near(out.size(), 201.0);
    assert!(out.split.expect("drawn").changed);
}

#[test]
fn the_size_is_the_press_size_plus_the_travel_not_a_sum_of_clamped_steps() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.settle(&mut h, &c);
    let from = strip_at(&c, 200.0);
    // Far past the largest, then back to 30 px past the press: the clamp did not eat the travel.
    clock.drag(&mut h, &c, from, moved(&c, from, 600.0));
    let out = clock.pointer(&mut h, &c, moved(&c, from, 600.0));
    near(out.size(), 400.0);
    let out = clock.pointer(&mut h, &c, moved(&c, from, 30.0));
    near(out.size(), 230.0);
}

#[test]
fn the_drag_clamps_at_min_max_and_the_room_left_for_the_other_pane() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.settle(&mut h, &c);
    let from = strip_at(&c, 200.0);
    let out = clock.drag(&mut h, &c, from, moved(&c, from, -500.0));
    near(out.size(), 100.0);
    let out = clock.pointer(&mut h, &c, moved(&c, from, 500.0));
    near(out.size(), 400.0);
    clock.release(&mut h, &c);

    // min_other: a 300 wide split leaves 296 and 60 for the other: 236.
    let narrow = Cfg {
        rect: Rect::new(20.0, 30.0, 300.0, 300.0),
        ..cfg()
    };
    let (mut h, mut clock) = start();
    clock.settle(&mut h, &narrow);
    let from = strip_at(&narrow, 200.0);
    let out = clock.drag(&mut h, &narrow, from, moved(&narrow, from, 90.0));
    near(out.size(), 236.0);
    let g = geometry(narrow.rect, narrow.axis, narrow.sized, 236.0);
    near(g.second.w, 60.0);

    // Second-sized: dragging toward the start grows it up to its largest.
    let c2 = columns_second();
    let (mut h, mut clock) = start();
    clock.settle(&mut h, &c2);
    let from = strip_at(&c2, 200.0);
    let out = clock.drag(&mut h, &c2, from, moved(&c2, from, -900.0));
    near(out.size(), 400.0);
}

#[test]
fn min_wins_when_nothing_fits() {
    let c = Cfg {
        rect: Rect::new(20.0, 30.0, 300.0, 300.0),
        min: 250.0,
        min_other: 100.0,
        default: 260.0,
        ..cfg()
    };
    let (mut h, mut clock) = start();
    let out = clock.settle(&mut h, &c);
    near(out.size(), 250.0);
    let from = strip_at(&c, 250.0);
    let out = clock.drag(&mut h, &c, from, moved(&c, from, -40.0));
    near(out.size(), 250.0);
    let out = clock.pointer(&mut h, &c, moved(&c, from, 40.0));
    near(out.size(), 250.0);
    // The node reports the same bounds.
    let n = out.out.tree.get(split_id()).expect("node");
    let Some(Value::Number { min, max, .. }) = n.value else {
        panic!("a number");
    };
    assert_eq!((min, max), (250.0, 250.0));
}

#[test]
fn a_smaller_window_clamps_what_is_drawn_and_not_what_is_stored() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.settle(&mut h, &c);
    let from = strip_at(&c, 200.0);
    clock.drag(&mut h, &c, from, moved(&c, from, 150.0));
    let out = clock.release(&mut h, &c);
    near(out.size(), 350.0);
    // Shrink: the room less min_other is 236.
    let small = Cfg {
        rect: Rect::new(20.0, 30.0, 300.0, 300.0),
        ..cfg()
    };
    let out = clock.two(&mut h, &small);
    near(out.size(), 236.0);
    assert_eq!(out.stored, Some(Some(350.0)));
    assert!(!out.split.expect("drawn").changed);
    // Grow again: the stored size comes back.
    let out = clock.two(&mut h, &c);
    near(out.size(), 350.0);
    assert_eq!(out.out.repaint, Repaint::Idle);
    // Shrink below the smallest: min wins, still not stored.
    let tiny = Cfg {
        rect: Rect::new(20.0, 30.0, 120.0, 300.0),
        ..cfg()
    };
    let out = clock.two(&mut h, &tiny);
    near(out.size(), 100.0);
    assert_eq!(out.stored, Some(Some(350.0)));
    near(clock.two(&mut h, &c).size(), 350.0);
}

#[test]
fn a_drag_after_a_shrink_starts_from_what_is_drawn() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.settle(&mut h, &c);
    clock.access(
        &mut h,
        &c,
        Action::SetValue,
        Some(AccessValue::Number(380.0)),
    );
    let small = Cfg {
        rect: Rect::new(20.0, 30.0, 300.0, 300.0),
        ..cfg()
    };
    clock.settle(&mut h, &small);
    let from = strip_at(&small, 236.0);
    let out = clock.drag(&mut h, &small, from, moved(&small, from, -36.0));
    near(out.size(), 200.0);
}

#[test]
fn a_drag_across_a_panes_button_clicks_nothing() {
    let (mut h, mut clock) = start();
    let c = with_buttons();
    clock.settle(&mut h, &c);
    let from = strip_at(&c, 200.0);
    // The size stops at its largest (400): the strip rests at 420 and the second pane's button at
    // [424, 464). The pointer goes on to its middle, past the strip, and releases there.
    let out = clock.drag(&mut h, &c, from, (444.0, 50.0));
    near(out.size(), 400.0);
    assert!(out.pane(Pane::Second).x == 424.0);
    let released = clock.release(&mut h, &c);
    assert!(released.clicks.is_empty(), "{:?}", released.clicks.len());
    near(released.size(), 400.0);
    // The button works for a click of its own.
    clock.pointer(&mut h, &c, (600.0, 380.0));
    clock.pointer(&mut h, &c, (444.0, 50.0));
    h.input.click(P);
    let out = clock.two(&mut h, &c);
    assert_eq!(out.clicks, vec![Pane::Second]);
}

// ---- hit ----

#[test]
fn the_hit_is_8_px_over_the_4_px_strip_and_a_pane_beside_it_keeps_its_own() {
    let (mut h, mut clock) = start();
    let c = with_buttons();
    clock.settle(&mut h, &c);
    // The strip is [220, 224); the hit [218, 226). Hover tells.
    let hovered = |h: &mut UiHarness, clock: &mut Clock, x: f32| {
        clock
            .pointer(h, &c, (x, 150.0))
            .split
            .expect("drawn")
            .response
            .hovered
    };
    assert!(!hovered(&mut h, &mut clock, 217.5));
    assert!(hovered(&mut h, &mut clock, 218.5));
    assert!(hovered(&mut h, &mut clock, 221.0));
    assert!(hovered(&mut h, &mut clock, 225.5));
    assert!(!hovered(&mut h, &mut clock, 226.5));

    // The first pane's button ends against the strip, at 220. A click at 217 reaches the button;
    // one at 219 (within the hit) reaches the strip.
    let click_at = |h: &mut UiHarness, clock: &mut Clock, x: f32| {
        clock.pointer(h, &c, (x, 50.0));
        h.input.click(P);
        clock.two(h, &c)
    };
    let out = click_at(&mut h, &mut clock, 217.0);
    assert_eq!(out.clicks, vec![Pane::First]);
    let out = click_at(&mut h, &mut clock, 219.0);
    assert!(out.clicks.is_empty());
    near(out.size(), 200.0);
}

#[test]
fn the_strip_blocks_what_is_under_its_hit_and_nothing_beyond_it() {
    // Pane content registered a full-bleed hover region: the strip still wins inside the hit.
    let (mut h, mut clock) = start();
    let mut press_started = false;
    for step in 0..4 {
        clock.0 += 16;
        h.at_ms(clock.0);
        if step == 1 {
            h.input.pointer_to(221.0, 100.0);
        }
        if step == 2 {
            h.input.press(P);
        }
        let (_, _) = h.frame(|ui| {
            let r = ui.split_panes("split", RECT, &cfg().opts(), |ui, which, rect| {
                let id = ui.id(("card", which as u8));
                ui.interact(id, rect, lune_ui_core::ui::Sense::CLICK);
            });
            press_started |= r.response.press_started;
        });
    }
    assert!(press_started);
}

// ---- keys ----

#[test]
fn the_arrows_along_the_axis_move_the_strip_16_px() {
    // (config, key, the size it leaves from the default)
    let cases = [
        (cfg(), NamedKey::ArrowRight, 216.0),
        (cfg(), NamedKey::ArrowLeft, 184.0),
        (columns_second(), NamedKey::ArrowRight, 184.0),
        (columns_second(), NamedKey::ArrowLeft, 216.0),
        (rows(), NamedKey::ArrowDown, 84.0),
        (rows(), NamedKey::ArrowUp, 116.0),
    ];
    for (c, key, want) in cases {
        let (mut h, mut clock) = start();
        clock.focus(&mut h, &c);
        let out = clock.key(&mut h, &c, key);
        near(out.size(), want);
        assert_eq!(out.stored, Some(Some(want)), "{key:?}");
        assert_eq!(KEY_STEP, 16.0);
    }
}

#[test]
fn up_and_down_on_columns_are_the_sized_panes_increment_and_decrement() {
    let (mut h, mut clock) = start();
    let c = columns_second();
    clock.focus(&mut h, &c);
    near(clock.key(&mut h, &c, NamedKey::ArrowUp).size(), 216.0);
    near(clock.key(&mut h, &c, NamedKey::ArrowDown).size(), 200.0);
    near(clock.key(&mut h, &c, NamedKey::ArrowDown).size(), 184.0);
}

#[test]
fn home_and_end_go_to_the_sized_panes_ends_and_keys_clamp() {
    for c in [cfg(), columns_second(), rows()] {
        let (mut h, mut clock) = start();
        clock.focus(&mut h, &c);
        let (lo, hi) = bounds(
            if c.axis == Axis::Horizontal {
                500.0
            } else {
                300.0
            },
            &c.opts(),
        );
        near(clock.key(&mut h, &c, NamedKey::End).size(), hi);
        near(clock.key(&mut h, &c, NamedKey::Home).size(), lo);
        // At an end an arrow that would go past it changes nothing.
        let past = if c.axis == Axis::Horizontal {
            if c.sized == SplitSide::First {
                NamedKey::ArrowLeft
            } else {
                NamedKey::ArrowRight
            }
        } else {
            NamedKey::ArrowDown
        };
        let out = clock.key(&mut h, &c, past);
        near(out.size(), lo);
        assert!(!out.split.expect("drawn").changed);
    }
}

#[test]
fn a_key_is_taken_and_a_key_the_strip_does_not_use_is_left_alone() {
    let (mut h, mut clock) = start();
    let c = cfg();
    for k in [NamedKey::ArrowRight, NamedKey::ArrowLeft, NamedKey::Home] {
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
    }
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::Enter)),
        "enter",
    ));
    clock.focus(&mut h, &c);
    h.input.key_down(Key::Named(NamedKey::ArrowRight), None);
    assert!(clock.frame(&mut h, &c).out.fired.is_empty());
    h.input.key_down(Key::Named(NamedKey::Home), None);
    assert!(clock.frame(&mut h, &c).out.fired.is_empty());
    // Enter is not the strip's: it reaches the binding.
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    assert_eq!(clock.frame(&mut h, &c).out.fired.len(), 1);
    // On rows Left and Right are not the strip's either.
    let (mut h, mut clock) = start();
    let r = rows();
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::ArrowLeft)),
        "left",
    ));
    clock.focus(&mut h, &r);
    h.input.key_down(Key::Named(NamedKey::ArrowLeft), None);
    assert_eq!(clock.frame(&mut h, &r).out.fired.len(), 1);
}

#[test]
fn keys_while_the_pointer_holds_the_strip_are_dropped() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.focus(&mut h, &c);
    let from = strip_at(&c, 200.0);
    clock.drag(&mut h, &c, from, moved(&c, from, 20.0));
    // Checked on the frame the key lands: the next frame's pointer would hide a move.
    h.input.key_down(Key::Named(NamedKey::End), None);
    let out = clock.frame(&mut h, &c);
    near(out.size(), 220.0);
    assert_eq!(out.stored, Some(Some(220.0)));
    assert!(!out.split.as_ref().expect("drawn").changed);
    h.input.access(AccessAction {
        target: split_id(),
        action: Action::Increment,
        value: None,
    });
    near(clock.frame(&mut h, &c).size(), 220.0);
    let out = clock.release(&mut h, &c);
    near(out.size(), 220.0);
}

#[test]
fn a_chord_on_the_strips_keys_is_left_for_the_shortcuts() {
    let held = |control, alt, logo| Modifiers {
        control,
        alt,
        logo,
        ..Modifiers::default()
    };
    let chords = [
        (held(true, false, false), Chord::ctrl as fn(Chord) -> Chord),
        (held(false, true, false), Chord::alt),
        (held(false, false, true), Chord::logo),
    ];
    let keys = [
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::Home,
        NamedKey::End,
    ];
    for (m, chord) in chords {
        for k in keys {
            let (mut h, mut clock) = start();
            let c = cfg();
            h.shortcuts.add(Binding::new(
                Scope::Global,
                chord(Chord::key(Key::Named(k))),
                "chord",
            ));
            clock.focus(&mut h, &c);
            h.input.set_modifiers(m);
            h.input.key_down(Key::Named(k), None);
            let out = clock.frame(&mut h, &c);
            near(out.size(), 200.0);
            assert_eq!(out.out.fired.len(), 1, "{m:?} {k:?}");
        }
    }
}

#[test]
fn the_arrows_on_a_row_split_with_the_first_pane_sized_follow_the_strip() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        sized: SplitSide::First,
        ..rows()
    };
    clock.focus(&mut h, &c);
    near(clock.key(&mut h, &c, NamedKey::ArrowDown).size(), 116.0);
    near(clock.key(&mut h, &c, NamedKey::ArrowUp).size(), 100.0);
    near(clock.key(&mut h, &c, NamedKey::ArrowUp).size(), 84.0);
}

// ---- the panes catch up ----

#[test]
fn a_move_below_the_drag_threshold_asks_for_the_frame_that_shows_it_in_the_panes() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.settle(&mut h, &c);
    let from = strip_at(&c, 200.0);
    clock.pointer(&mut h, &c, from);
    h.input.press(P);
    clock.settle(&mut h, &c);
    h.input.pointer_to(from.0 + 1.0, from.1);
    let moved = clock.frame(&mut h, &c);
    near(moved.size(), 201.0);
    near(moved.pane(Pane::First).w, 200.0);
    assert_eq!(moved.out.repaint, Repaint::Now);
    let next = clock.frame(&mut h, &c);
    near(next.pane(Pane::First).w, 201.0);
}

// ---- accessibility ----

#[test]
fn increment_raises_and_decrement_lowers_the_value_on_all_four_configurations() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for sized in [SplitSide::First, SplitSide::Second] {
            let (mut h, mut clock) = start();
            let c = Cfg {
                rect: Rect::new(20.0, 30.0, 500.0, 340.0),
                axis,
                sized,
                max: 250.0,
                ..cfg()
            };
            clock.settle(&mut h, &c);
            let up = clock.access(&mut h, &c, Action::Increment, None);
            near(up.size(), 216.0);
            let node = up.out.tree.get(split_id()).expect("strip");
            assert!(
                matches!(node.value, Some(Value::Number { value, .. }) if value == 216.0),
                "{axis:?} {sized:?}"
            );
            near(
                clock.access(&mut h, &c, Action::Decrement, None).size(),
                200.0,
            );
            near(
                clock.access(&mut h, &c, Action::Decrement, None).size(),
                184.0,
            );
        }
    }
}

#[test]
fn increment_decrement_and_set_value_move_the_size_and_a_nan_or_infinity_changes_nothing() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.settle(&mut h, &c);
    near(
        clock.access(&mut h, &c, Action::Increment, None).size(),
        216.0,
    );
    near(
        clock.access(&mut h, &c, Action::Decrement, None).size(),
        200.0,
    );
    let out = clock.access(
        &mut h,
        &c,
        Action::SetValue,
        Some(AccessValue::Number(321.0)),
    );
    near(out.size(), 321.0);
    assert!(out.changed_any);
    // Past the bounds clamps.
    near(
        clock
            .access(&mut h, &c, Action::SetValue, Some(AccessValue::Number(9e9)))
            .size(),
        400.0,
    );
    near(
        clock
            .access(
                &mut h,
                &c,
                Action::SetValue,
                Some(AccessValue::Number(-5.0)),
            )
            .size(),
        100.0,
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let out = clock.access(&mut h, &c, Action::SetValue, Some(AccessValue::Number(bad)));
        near(out.size(), 100.0);
        assert_eq!(out.stored, Some(Some(100.0)), "{bad}");
    }
    // A set value of the wrong kind is ignored.
    let out = clock.access(
        &mut h,
        &c,
        Action::SetValue,
        Some(AccessValue::Text("x".into())),
    );
    near(out.size(), 100.0);
}

#[test]
fn the_node_is_a_separator_with_the_sizes_as_a_number_and_the_three_actions() {
    let (mut h, mut clock) = start();
    let c = cfg();
    let out = clock.settle(&mut h, &c);
    let n = out.out.tree.get(split_id()).expect("a node");
    assert_eq!(n.role, Role::Separator);
    assert_eq!(n.name, "Side panel width");
    assert_eq!(
        n.value,
        Some(Value::Number {
            value: 200.0,
            min: 100.0,
            max: 400.0,
            step: 16.0,
        })
    );
    assert!(n.states.contains(State::Focusable));
    assert!(!n.states.contains(State::Disabled));
    for a in [Action::Increment, Action::Decrement, Action::SetValue] {
        assert!(n.actions.contains(a), "{a:?}");
    }
    assert_eq!(n.bounds, Rect::new(218.0, 30.0, 8.0, 300.0));
    // The value follows the size.
    clock.key(&mut h, &c, NamedKey::Tab);
    let out = clock.key(&mut h, &c, NamedKey::ArrowRight);
    let n = out.out.tree.get(split_id()).expect("a node");
    assert!(matches!(n.value, Some(Value::Number { value, .. }) if value == 216.0));
    // The node's numbers are finite whatever the options.
    let (mut h, mut clock) = start();
    let junk = Cfg {
        min: f32::NAN,
        max: f32::INFINITY,
        min_other: f32::NAN,
        default: f32::NAN,
        ..cfg()
    };
    let out = clock.settle(&mut h, &junk);
    let n = out.out.tree.get(split_id()).expect("a node");
    let Some(Value::Number {
        value, min, max, ..
    }) = n.value
    else {
        panic!("a number");
    };
    assert!(value.is_finite() && min.is_finite() && max.is_finite());
}

// ---- escape ----

#[test]
fn escape_during_a_drag_restores_the_press_size_and_is_consumed_and_a_release_ends_it() {
    let (mut h, mut clock) = start();
    let c = cfg();
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::Escape)),
        "esc",
    ));
    clock.settle(&mut h, &c);
    let from = strip_at(&c, 200.0);
    let out = clock.drag(&mut h, &c, from, moved(&c, from, 80.0));
    near(out.size(), 280.0);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let out = clock.frame(&mut h, &c);
    assert!(out.out.fired.is_empty(), "consumed");
    near(out.size(), 200.0);
    assert!(out.split.as_ref().expect("drawn").changed);
    let out = clock.frame(&mut h, &c);
    near(out.size(), 200.0);
    // The pointer is still down but no longer moves it.
    let out = clock.pointer(&mut h, &c, moved(&c, from, 150.0));
    near(out.size(), 200.0);
    assert_eq!(out.stored, Some(None), "back to never stored");
    let out = clock.release(&mut h, &c);
    near(out.size(), 200.0);
    // With nothing in flight Escape is not the strip's.
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    assert_eq!(clock.frame(&mut h, &c).out.fired.len(), 1);

    // A new press after that follows again.
    let out = clock.drag(&mut h, &c, from, moved(&c, from, 40.0));
    near(out.size(), 240.0);
    clock.release(&mut h, &c);
}

#[test]
fn in_a_dialog_the_first_escape_restores_the_size_and_only_the_second_closes_the_dialog() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        dialog: true,
        ..cfg()
    };
    // The dialog opens, then the strip is dragged.
    clock.settle(&mut h, &c);
    let from = strip_at(&c, 200.0);
    let out = clock.drag(&mut h, &c, from, moved(&c, from, 80.0));
    near(out.size(), 280.0);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    h.input.key_up(Key::Named(NamedKey::Escape));
    let out = clock.frame(&mut h, &c);
    assert!(!out.dialog_cancelled, "the first Escape is the drag's");
    near(out.size(), 200.0);
    // The drag is over (the button is still down): the next Escape closes the dialog.
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    h.input.key_up(Key::Named(NamedKey::Escape));
    let out = clock.frame(&mut h, &c);
    assert!(out.dialog_cancelled, "the second Escape is the dialog's");
}

#[test]
fn escape_puts_back_a_size_that_was_never_stored() {
    let (mut h, mut clock) = start();
    let c = cfg();
    let out = clock.settle(&mut h, &c);
    assert_eq!(out.stored, Some(None));
    let from = strip_at(&c, 200.0);
    clock.drag(&mut h, &c, from, moved(&c, from, 60.0));
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let out = clock.two(&mut h, &c);
    near(out.size(), 200.0);
    assert_eq!(out.stored, Some(None), "the default holds again");
    // A default the app then changes is followed.
    let d = Cfg {
        default: 150.0,
        ..cfg()
    };
    clock.release(&mut h, &d);
    near(clock.two(&mut h, &d).size(), 150.0);
}

// ---- hidden ----

#[test]
fn hidden_gives_the_other_pane_the_whole_rect_with_no_strip_no_node_and_no_hit() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        let (mut h, mut clock) = start();
        h.tier = tier;
        let shown = cfg();
        let out = clock.settle(&mut h, &shown);
        assert_eq!(out.panes.len(), 2);
        assert!(out.out.tree.get(split_id()).is_some());
        // The very next frame after hiding shows the final layout.
        let hidden = Cfg {
            hidden: true,
            ..cfg()
        };
        let out = clock.frame(&mut h, &hidden);
        assert_eq!(out.panes, vec![(Pane::Second, RECT)], "{tier:?}");
        assert!(out.out.tree.get(split_id()).is_none());
        let r = out.split.expect("a response");
        assert!(!r.changed && !r.response.hovered && !r.response.pressed);
        // Hiding the second-sized one leaves the first.
        let hidden2 = Cfg {
            hidden: true,
            ..columns_second()
        };
        let out = clock.frame(&mut h, &hidden2);
        assert_eq!(out.panes, vec![(Pane::First, RECT)]);
        // And showing again snaps to the stored layout.
        let out = clock.frame(&mut h, &shown);
        assert_eq!(out.pane(Pane::First), Rect::new(20.0, 30.0, 200.0, 300.0));
    }
}

#[test]
fn a_hidden_split_hits_nothing_and_keeps_its_size() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.focus(&mut h, &c);
    clock.key(&mut h, &c, NamedKey::ArrowRight);
    let hidden = Cfg {
        hidden: true,
        ..cfg()
    };
    clock.settle(&mut h, &hidden);
    // A press where the strip was and a drag move nothing.
    let at = strip_at(&c, 216.0);
    clock.pointer(&mut h, &hidden, at);
    h.input.press(P);
    clock.frame(&mut h, &hidden);
    let out = clock.pointer(&mut h, &hidden, moved(&hidden, at, 80.0));
    assert!(!out.split.expect("a response").response.pressed);
    h.input.release(P);
    clock.two(&mut h, &hidden);
    // Keys do nothing either: no stop to hold them.
    h.input.key_down(Key::Named(NamedKey::End), None);
    let out = clock.two(&mut h, &hidden);
    assert_eq!(out.stored, Some(Some(216.0)));
    // Shown again: the size it had.
    near(clock.settle(&mut h, &c).size(), 216.0);
}

// ---- persistence ----

#[test]
fn the_size_survives_keep_alive_and_frames_it_was_not_drawn() {
    let (mut h, mut clock) = start();
    let drawn = Cfg {
        keep: Keep::Drawn,
        ..cfg()
    };
    clock.focus(&mut h, &drawn);
    near(clock.key(&mut h, &drawn, NamedKey::End).size(), 400.0);
    let marked = Cfg {
        keep: Keep::Marked,
        ..cfg()
    };
    for _ in 0..5 {
        clock.frame(&mut h, &marked);
    }
    let out = clock.two(&mut h, &drawn);
    near(out.size(), 400.0);
    assert_eq!(out.stored, Some(Some(400.0)));

    // With no keep-alive scope open, an undrawn frame drops the state: the default holds.
    let (mut h, mut clock) = start();
    let plain = cfg();
    clock.focus(&mut h, &plain);
    near(clock.key(&mut h, &plain, NamedKey::End).size(), 400.0);
    let absent = Cfg {
        keep: Keep::Absent,
        ..cfg()
    };
    clock.two(&mut h, &absent);
    near(clock.two(&mut h, &plain).size(), 200.0);
}

#[test]
fn the_app_reads_and_writes_the_stored_size_and_a_bad_write_falls_to_the_default() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.settle(&mut h, &c);
    // The app loads a saved size: drawn at once, nothing reported as changed.
    clock.0 += 16;
    h.at_ms(clock.0);
    let out = frame_with(&mut h, &c, Some(Some(333.0)));
    near(out.size(), 333.0);
    assert!(!out.split.expect("drawn").changed);
    assert_eq!(out.stored, Some(Some(333.0)));
    // Out of range: drawn clamped, stored as written.
    clock.0 += 16;
    h.at_ms(clock.0);
    let out = frame_with(&mut h, &c, Some(Some(5000.0)));
    near(out.size(), 400.0);
    assert_eq!(out.stored, Some(Some(5000.0)));
    // NaN and infinity count as unset.
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        clock.0 += 16;
        h.at_ms(clock.0);
        let out = frame_with(&mut h, &c, Some(Some(bad)));
        near(out.size(), 200.0);
    }
    // None puts the default back.
    clock.0 += 16;
    h.at_ms(clock.0);
    near(frame_with(&mut h, &c, Some(None)).size(), 200.0);
}

// ---- cursor ----

#[test]
fn the_cursor_is_a_resize_cursor_over_and_while_holding_the_strip_only() {
    for (c, want) in [
        (cfg(), CursorIcon::ColResize),
        (rows(), CursorIcon::RowResize),
    ] {
        let (mut h, mut clock) = start();
        let out = clock.settle(&mut h, &c);
        assert_eq!(out.out.cursor, CursorIcon::Default);
        let at = strip_at(&c, out.size());
        let out = clock.pointer(&mut h, &c, at);
        assert_eq!(out.out.cursor, want);
        // Held: the cursor stays while the pointer is away from the strip.
        h.input.press(P);
        clock.frame(&mut h, &c);
        let out = clock.pointer(&mut h, &c, moved(&c, at, 100.0));
        assert_eq!(out.out.cursor, want);
        clock.release(&mut h, &c);
        // Away from the strip again.
        let out = clock.pointer(&mut h, &c, (650.0, 380.0));
        assert_eq!(out.out.cursor, CursorIcon::Default);
    }
}

// ---- focus and probes ----

#[test]
fn focus_visible_fills_the_4_px_strip_with_accent_and_nothing_else_does() {
    let (mut h, mut clock) = start();
    let c = cfg();
    let out = clock.settle(&mut h, &c);
    let accent = premul(DARK.accent);
    let count = |s: &Scene| {
        let g = &s.layers[0].geometry;
        g.under
            .iter()
            .chain(&g.over)
            .filter(|f| f.color == accent)
            .count()
    };
    // At rest, hovered and pressed the strip draws nothing.
    assert!(out.scene.layers[0].geometry.under.is_empty());
    let at = strip_at(&c, 200.0);
    let hov = clock.pointer(&mut h, &c, at);
    assert_eq!(count(&hov.scene), 0);
    h.input.press(P);
    let held = clock.two(&mut h, &c);
    assert_eq!(count(&held.scene), 0);
    clock.release(&mut h, &c);
    // A pointer focus is no focus-visible; Tab is.
    let (mut h, mut clock) = start();
    let out = clock.focus(&mut h, &c);
    assert_eq!(out.out.focused, Some(split_id()));
    let out = clock.settle(&mut h, &c);
    assert_eq!(count(&out.scene), 1);
    let fill = out.scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|f| f.color == accent)
        .expect("a fill");
    assert_eq!(fill.rect, [220.0, 30.0, 4.0, 300.0]);
    assert_eq!(out.out.repaint, Repaint::Idle);
}

#[test]
fn the_focus_fill_conforms_in_three_tiers() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&FOCUS, &DARK.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("focus {tier:?}: {e}"));
    }
    // Half way through the fade the fill is part way.
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.focus(&mut h, &c);
    let t = clock.0;
    h.at_ms(t + 90);
    let out = frame_with(&mut h, &c, None);
    assert_eq!(out.out.repaint, Repaint::Now);
    let full = premul(DARK.accent)[3];
    let alpha = out.scene.layers[0]
        .geometry
        .under
        .first()
        .expect("a fill")
        .color[3];
    assert!(alpha > 0.0 && alpha < full, "{alpha} of {full}");
    // Reduced keeps the fade, Off snaps.
    let (mut h, mut clock) = start();
    h.tier = MotionTier::Off;
    clock.focus(&mut h, &c);
    assert_eq!(
        frame_with(&mut h, &c, None).scene.layers[0].geometry.under[0].color,
        premul(DARK.accent)
    );
}

#[test]
fn pixel_probes_at_scale_two() {
    let (mut h, mut clock) = start();
    h.input.set_scale(2.0);
    let c = cfg();
    clock.focus(&mut h, &c);
    let out = clock.settle(&mut h, &c);
    let accent = premul(DARK.accent);
    let fill = out.scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|f| f.color == accent)
        .expect("the focus fill");
    // The strip is 4 logical px: 8 physical; the rect's height 300 logical: 600.
    assert_eq!(fill.rect, [440.0, 60.0, 8.0, 600.0]);
    // The hit is 8 logical px: the node's bounds are logical.
    let n = out.out.tree.get(split_id()).expect("node");
    assert_eq!(n.bounds.w, 8.0);
    // The panes' clips are physical.
    let (mut h, _) = start();
    h.input.set_scale(2.0);
    let (scene, _) = h.frame(|ui| {
        ui.split_panes("split", RECT, &c.opts(), |ui, _, rect| {
            ui.painter().fill(rect, 0.0, [255, 0, 0, 255]);
        });
    });
    let red = premul([255, 0, 0, 255]);
    let f: Vec<_> = scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.color == red)
        .collect();
    assert_eq!(f[0].rect, [40.0, 60.0, 400.0, 600.0]);
    assert_eq!(f[1].rect, [448.0, 60.0, 592.0, 600.0]);
}

#[test]
fn a_split_in_a_scaled_drawing_space_follows_the_pointer_one_to_one() {
    // The split is drawn in a space scaled 2 around the origin: a window travel of 40 is 20 local.
    let (mut h, mut clock) = start();
    let local = Rect::new(10.0, 15.0, 250.0, 150.0);
    let opts = SplitOptions {
        default: 100.0,
        min: 20.0,
        max: 200.0,
        min_other: 20.0,
        ..cfg().opts()
    };
    let mut sizes = Vec::new();
    let mut run = |h: &mut UiHarness, clock: &mut Clock| {
        clock.0 += 16;
        h.at_ms(clock.0);
        let (_, _) = h.frame(|ui| {
            ui.painter().push_xform(Xform {
                origin: (0.0, 0.0),
                scale: 2.0,
                dx: 0.0,
                dy: 0.0,
                alpha: 1.0,
            });
            let r = ui.split_panes("split", local, &opts, |_, _, _| {});
            sizes.push(r.size);
            ui.painter().pop_xform();
        });
    };
    run(&mut h, &mut clock);
    run(&mut h, &mut clock);
    // Strip local x 110..114, centre 112: window 224; y local 90: window 180.
    h.input.pointer_to(224.0, 180.0);
    run(&mut h, &mut clock);
    run(&mut h, &mut clock);
    h.input.press(P);
    run(&mut h, &mut clock);
    h.input.pointer_to(264.0, 180.0);
    run(&mut h, &mut clock);
    run(&mut h, &mut clock);
    near(*sizes.last().expect("sizes"), 120.0);
}

// ---- disabled ----

#[test]
fn a_disabled_split_does_not_drag_or_move_and_the_walk_holds() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        enabled: false,
        ..cfg()
    };
    let out = clock.settle(&mut h, &c);
    let bad = disabled_violations(&out.out.tree, split_id());
    assert!(bad.is_empty(), "{bad:?}");
    let n = out.out.tree.get(split_id()).expect("a node");
    assert!(n.states.contains(State::Disabled));
    // Hovering the strip: no resize cursor, no hover.
    let at = strip_at(&c, 200.0);
    let out = clock.pointer(&mut h, &c, at);
    assert_eq!(out.out.cursor, CursorIcon::Default);
    // A drag moves nothing.
    h.input.press(P);
    clock.frame(&mut h, &c);
    let out = clock.pointer(&mut h, &c, moved(&c, at, 80.0));
    near(out.size(), 200.0);
    assert_eq!(out.stored, Some(None));
    clock.release(&mut h, &c);
    // The strip is no tab stop.
    for _ in 0..3 {
        let out = clock.key(&mut h, &c, NamedKey::Tab);
        assert_ne!(out.out.focused, Some(split_id()));
    }
    // Accessibility actions do nothing.
    near(
        clock.access(&mut h, &c, Action::Increment, None).size(),
        200.0,
    );
    let out = clock.access(
        &mut h,
        &c,
        Action::SetValue,
        Some(AccessValue::Number(300.0)),
    );
    near(out.size(), 200.0);
    // An enabled split is not marked disabled.
    let (mut h, mut clock) = start();
    let out = clock.settle(&mut h, &cfg());
    assert!(
        !out.out
            .tree
            .get(split_id())
            .expect("present")
            .states
            .contains(State::Disabled)
    );
}

#[test]
fn a_global_binding_on_the_strips_keys_still_fires_while_it_is_disabled() {
    let c = Cfg {
        enabled: false,
        ..cfg()
    };
    for k in [
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::Home,
        NamedKey::End,
        NamedKey::Escape,
    ] {
        let (mut h, mut clock) = start();
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        clock.settle(&mut h, &c);
        h.input.key_down(Key::Named(k), None);
        let out = clock.frame(&mut h, &c);
        assert_eq!(out.out.fired.len(), 1, "{k:?}");
        near(out.size(), 200.0);
    }
}

#[test]
fn a_strip_that_was_focused_and_is_then_disabled_still_takes_its_keys_and_acts_on_none() {
    let (mut h, mut clock) = start();
    let c = cfg();
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::ArrowRight)),
        "go",
    ));
    clock.focus(&mut h, &c);
    let off = Cfg {
        enabled: false,
        ..cfg()
    };
    let out = clock.frame(&mut h, &off);
    near(out.size(), 200.0);
    h.input.key_down(Key::Named(NamedKey::ArrowRight), None);
    let out = clock.frame(&mut h, &off);
    near(out.size(), 200.0);
    // The focus held through the disabling, so the key was the strip's: taken, not acted on.
    assert_eq!(out.out.focused, Some(split_id()));
    assert!(out.out.fired.is_empty());
}

// ---- state of the frame ----

#[test]
fn a_change_asks_for_one_catch_up_frame_and_a_settled_split_is_idle() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.focus(&mut h, &c);
    h.input.key_down(Key::Named(NamedKey::ArrowRight), None);
    let out = clock.frame(&mut h, &c);
    assert!(out.split.as_ref().expect("drawn").changed);
    assert_eq!(out.out.repaint, Repaint::Now);
    // The panes show the new size one frame later.
    assert_eq!(out.pane(Pane::First).w, 200.0);
    let out = clock.frame(&mut h, &c);
    assert_eq!(out.pane(Pane::First).w, 216.0);
    let out = clock.settle(&mut h, &c);
    assert_eq!(out.out.repaint, Repaint::Idle);
}

#[test]
fn the_split_draws_only_on_the_layer_it_is_called_on() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.focus(&mut h, &c);
    let out = clock.settle(&mut h, &c);
    for (i, l) in out.scene.layers.iter().enumerate() {
        if i != LayerId::Base as usize {
            assert!(
                l.texts.is_empty() && l.geometry.under.is_empty(),
                "layer {i}"
            );
        }
    }
}

#[test]
fn degenerate_rects_and_options_never_panic() {
    let rects = [
        Rect::new(0.0, 0.0, 0.0, 0.0),
        Rect::new(0.0, 0.0, 3.0, 3.0),
        Rect::new(f32::NAN, 0.0, f32::NAN, 10.0),
        Rect::new(0.0, 0.0, f32::INFINITY, f32::INFINITY),
        Rect::new(-50.0, -50.0, -10.0, 100.0),
    ];
    for rect in rects {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            for sized in [SplitSide::First, SplitSide::Second] {
                let (mut h, mut clock) = start();
                let c = Cfg {
                    rect,
                    axis,
                    sized,
                    min: f32::NAN,
                    max: f32::INFINITY,
                    default: f32::NEG_INFINITY,
                    ..cfg()
                };
                let out = clock.focus(&mut h, &c);
                assert!(out.size().is_finite());
                h.input.pointer_to(1.0, 1.0);
                h.input.press(P);
                clock.two(&mut h, &c);
                h.input.pointer_to(30.0, 30.0);
                let out = clock.two(&mut h, &c);
                assert!(out.size().is_finite());
                let g = geometry(rect, axis, sized, f32::NAN);
                for r in [g.first, g.second, g.strip, g.hit] {
                    assert!(r.w >= 0.0 && r.h >= 0.0, "{r:?}");
                }
            }
        }
    }
}
