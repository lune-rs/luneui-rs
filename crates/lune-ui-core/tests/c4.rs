//! The C4 contract suite: cross-widget behaviour on the real widgets. Per-widget conformance
//! lives in the `widget_*` test files; this file checks what only holds when the widgets run
//! together in one frame: the semantic tree of a full page, accessibility actions, modal capture,
//! layered hover, Escape on the top overlay, the dialog's focus trap and restore, exit retention
//! per tier, snapping overlays reporting `Idle`, and determinism.

use lune_ui_core::access::{AccessAction, Action, Node, Role, State, Value};
use lune_ui_core::input::{FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{Easing, Kind, MOTION, Motion, MotionTier, Reduce, Token};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::{Ui, WidgetId};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::dialog::{Cancel, Dialog, DialogGlass, DialogResponse};
use lune_ui_core::widgets::dropdown::DropdownResponse;
use lune_ui_core::widgets::field::FieldState;
use lune_ui_core::widgets::label::LabelRole;
use lune_ui_core::widgets::menu::{MenuResponse, MenuRow};
use lune_ui_core::widgets::stepper::StepRange;

const WIN: (f32, f32) = (900.0, 700.0);
const P: PointerButton = PointerButton::Primary;

const PAGE_BTN: Rect = Rect::new(20.0, 20.0, 100.0, 32.0);
const TOGGLE: Rect = Rect::new(20.0, 70.0, 40.0, 24.0);
const CHECK: Rect = Rect::new(20.0, 110.0, 140.0, 24.0);
const STEPPER: Rect = Rect::new(20.0, 190.0, 140.0, 32.0);
const FIELD: Rect = Rect::new(20.0, 240.0, 210.0, 32.0);
const COMBO: Rect = Rect::new(20.0, 290.0, 210.0, 32.0);
const MENU_AT: (f32, f32) = (60.0, 30.0);

const RANGE: StepRange = StepRange {
    min: 0.0,
    max: 99.0,
    step: 1.0,
};

const OPTIONS: [(&str, bool); 3] = [("RznSans", true), ("Sans Serif", true), ("Soon", false)];

fn glass() -> DialogGlass {
    DialogGlass {
        panel: [0.1, 0.1, 0.12, 1.0],
        fill: [0.2, 0.2, 0.2, 0.62],
        noise: true,
    }
}

fn id(key: &str) -> WidgetId {
    WidgetId::ROOT.with(key)
}

/// The app side: widget values, which overlays are open, and what the frames reported.
#[derive(Debug, Default)]
struct App {
    toggle: bool,
    check: bool,
    value: u32,
    text: String,
    combo_open: bool,
    dialog_open: bool,
    menu_open: bool,
    // What the frames reported, summed.
    page_clicks: u32,
    page_hovered: bool,
    confirmed: u32,
    cancelled: u32,
    menu_picked: Vec<usize>,
    menu_dismissed: u32,
    dropdown_dismissed: u32,
    check_changed: u32,
}

impl App {
    fn new() -> Self {
        App {
            toggle: true,
            value: 12,
            text: "lune".to_owned(),
            ..App::default()
        }
    }
}

type Frame = (Scene, FrameOutput);

/// One frame of the page every test uses: a button, a toggle, a checkbox, a label, a stepper, a
/// text field, a combo and, while the app says so, a dropdown, a menu and a confirm dialog.
fn frame(h: &mut UiHarness, app: &mut App) -> Frame {
    h.frame(|ui: &mut Ui<'_>| {
        ui.painter().on(LayerId::Page);
        let r = ui.button("page-button", PAGE_BTN, "Page", ButtonKind::Secondary);
        app.page_clicks += u32::from(r.clicked);
        app.page_hovered = r.hovered;
        ui.toggle("toggle", TOGGLE, &mut app.toggle, "Notifications");
        let c = ui.checkbox("check", CHECK, &mut app.check, "Remember");
        app.check_changed += u32::from(c.changed);
        ui.label("label", (20.0, 150.0), "Hello", LabelRole::Base);
        let s = ui.stepper("stepper", STEPPER, app.value, RANGE, "Amount");
        if let Some(step) = s.step {
            app.value = app.value.saturating_add_signed(step);
        }
        ui.text_field(
            "field",
            FIELD,
            &mut app.text,
            "Search",
            FieldState::default(),
        );
        ui.combo("combo", COMBO, OPTIONS[0].0, &mut app.combo_open, "Font");
        if app.combo_open {
            let DropdownResponse { picked, dismissed } =
                ui.dropdown("dropdown", COMBO, id("combo"), &OPTIONS, 0);
            if picked.is_some() || dismissed {
                app.combo_open = false;
            }
            app.dropdown_dismissed += u32::from(dismissed);
        }
        if app.menu_open {
            let rows = [
                MenuRow::Item {
                    glyph: None,
                    text: "New Window",
                    shortcut: "Cmd+N",
                },
                MenuRow::Item {
                    glyph: None,
                    text: "Copy",
                    shortcut: "",
                },
            ];
            let MenuResponse { picked, dismissed } = ui.menu("menu", &rows, MENU_AT);
            app.menu_picked.extend(picked);
            app.menu_dismissed += u32::from(dismissed);
        }
        if app.dialog_open {
            let d = Dialog {
                title: "Discard changes",
                message: "The document has unsaved changes.",
                extra_h: 0.0,
                confirm: "Discard",
                cancel: Cancel::Default,
            };
            let DialogResponse {
                confirmed,
                cancelled,
            } = ui.dialog("dialog", &d, &glass(), |_, _, _, _| {});
            app.confirmed += u32::from(confirmed);
            app.cancelled += u32::from(cancelled);
        }
    })
}

/// Runs a frame at `ms` of app time.
fn at(h: &mut UiHarness, app: &mut App, ms: u64) -> Frame {
    h.at_ms(ms);
    frame(h, app)
}

/// A settled page: frame at 0, then 1000 ms.
fn settled(app: &mut App) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    at(&mut h, app, 0);
    let (_, out) = at(&mut h, app, 1000);
    assert_eq!(out.repaint, Repaint::Idle);
    h
}

fn key(h: &mut UiHarness, k: NamedKey) {
    h.input.key_down(Key::Named(k), None);
}

fn shift_tab(h: &mut UiHarness) {
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    h.input
        .set_modifiers(shift)
        .key_down(Key::Named(NamedKey::Tab), None)
        .set_modifiers(Modifiers::default());
}

fn node(out: &FrameOutput, id: WidgetId) -> &Node {
    out.tree
        .get(id)
        .unwrap_or_else(|| panic!("no node for {id:?}"))
}

fn centre(r: Rect) -> (f32, f32) {
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

fn modal_is_empty(scene: &Scene) -> bool {
    let layer = &scene.layers[LayerId::Modal as usize];
    layer.geometry.under.is_empty()
        && layer.geometry.over.is_empty()
        && layer.geometry.acrylic.is_empty()
        && layer.texts.is_empty()
}

#[test]
fn a_scripted_frame_of_every_widget_yields_the_expected_nodes() {
    let mut app = App::new();
    let mut h = UiHarness::new(WIN);
    at(&mut h, &mut app, 0);
    let (_, out) = at(&mut h, &mut app, 100);

    // (key, role, name, checked, focusable, actions)
    #[allow(clippy::type_complexity)]
    let table: [(&str, Role, &str, bool, bool, &[Action]); 7] = [
        (
            "page-button",
            Role::Button,
            "Page",
            false,
            true,
            &[Action::Default],
        ),
        (
            "toggle",
            Role::Switch,
            "Notifications",
            true,
            true,
            &[Action::Default],
        ),
        (
            "check",
            Role::Checkbox,
            "Remember",
            false,
            true,
            &[Action::Default],
        ),
        ("label", Role::StaticText, "Hello", false, false, &[]),
        (
            "stepper",
            Role::SpinButton,
            "Amount",
            false,
            true,
            &[Action::Increment, Action::Decrement, Action::SetValue],
        ),
        (
            "field",
            Role::TextInput,
            "Search",
            false,
            true,
            &[Action::Focus, Action::SetValue],
        ),
        (
            "combo",
            Role::ComboBox,
            "Font",
            false,
            true,
            &[Action::Default, Action::Expand, Action::Collapse],
        ),
    ];
    for (k, role, name, checked, focusable, actions) in table {
        let n = node(&out, id(k));
        assert_eq!(n.role, role, "{k}");
        assert_eq!(n.name, name, "{k}");
        assert_eq!(n.states.contains(State::Checked), checked, "{k} checked");
        assert_eq!(n.states.contains(State::Focusable), focusable, "{k}");
        assert!(!n.states.contains(State::Disabled), "{k}");
        for a in actions {
            assert!(n.actions.contains(*a), "{k} lacks {a:?}");
        }
        assert!(n.bounds.w > 0.0 && n.bounds.h > 0.0, "{k} has bounds");
    }
    assert_eq!(
        node(&out, id("stepper")).value,
        Some(Value::Number {
            value: 12.0,
            min: 0.0,
            max: 99.0,
            step: 1.0
        })
    );
    assert_eq!(
        node(&out, id("field")).value,
        Some(Value::Text("lune".to_owned()))
    );
    assert_eq!(
        node(&out, id("combo")).value,
        Some(Value::Text("RznSans".to_owned()))
    );
    assert_eq!(
        node(&out, id("label")).value,
        Some(Value::Text("Hello".to_owned()))
    );

    // The overlays add theirs: a dialog (modal), its buttons; a menu and its items.
    app.dialog_open = true;
    app.menu_open = true;
    at(&mut h, &mut app, 2000);
    let (_, out) = at(&mut h, &mut app, 3000);
    let dialog = node(&out, id("dialog"));
    assert_eq!(dialog.role, Role::Dialog);
    assert!(dialog.states.contains(State::Modal));
    assert_eq!(dialog.name, "Discard changes");
    let buttons: Vec<&str> = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Button)
        .map(|n| n.name.as_str())
        .collect();
    assert!(buttons.contains(&"Cancel") && buttons.contains(&"Discard"));
    let items: Vec<&str> = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::MenuItem)
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(items, ["New Window", "Copy"]);
}

#[test]
fn an_injected_default_action_toggles_a_checkbox() {
    let mut app = App::new();
    let mut h = settled(&mut app);
    assert!(!app.check);
    h.input.access(AccessAction {
        target: id("check"),
        action: Action::Default,
        value: None,
    });
    let (_, out) = at(&mut h, &mut app, 1100);
    assert!(app.check, "the default action flipped the value");
    assert_eq!(app.check_changed, 1);
    let (_, out2) = at(&mut h, &mut app, 1200);
    assert!(node(&out2, id("check")).states.contains(State::Checked));
    assert!(!node(&out, id("check")).states.contains(State::Disabled));
}

#[test]
fn a_click_on_a_page_button_under_an_open_dialog_does_nothing() {
    let mut app = App::new();
    let mut h = settled(&mut app);

    // The instrument: with no dialog the same click is counted.
    let (x, y) = centre(PAGE_BTN);
    h.input.pointer_to(x, y).click(P);
    at(&mut h, &mut app, 1100);
    at(&mut h, &mut app, 1200);
    assert_eq!(app.page_clicks, 1, "the click lands without a dialog");

    app.dialog_open = true;
    at(&mut h, &mut app, 2000);
    at(&mut h, &mut app, 3000);
    app.page_clicks = 0;
    h.input.pointer_to(x + 1.0, y).click(P);
    at(&mut h, &mut app, 3100);
    assert!(!app.page_hovered, "no hover under a modal");
    at(&mut h, &mut app, 3200);
    assert_eq!(app.page_clicks, 0);
    // The click lands on the scrim, which cancels (and never on the page button below it).
    assert_eq!(app.confirmed, 0);
}

#[test]
fn a_popup_item_over_a_page_button_takes_the_pointer() {
    let mut app = App::new();
    let mut h = settled(&mut app);
    app.menu_open = true;
    at(&mut h, &mut app, 2000);
    let (_, out) = at(&mut h, &mut app, 3000);
    let item = node(
        &out,
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == Role::MenuItem)
            .expect("item")
            .id,
    )
    .bounds;
    // The overlap of the first item and the page button.
    let x0 = item.x.max(PAGE_BTN.x);
    let x1 = item.right().min(PAGE_BTN.right());
    let y0 = item.y.max(PAGE_BTN.y);
    let y1 = item.bottom().min(PAGE_BTN.bottom());
    assert!(x0 < x1 && y0 < y1, "the item overlaps the button");
    h.input.pointer_to((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    at(&mut h, &mut app, 3100);
    assert!(!app.page_hovered, "only the popup item is hovered");
    h.input.click(P);
    at(&mut h, &mut app, 3200);
    at(&mut h, &mut app, 3300);
    assert_eq!(app.page_clicks, 0);
    assert_eq!(app.menu_picked, [0]);
}

#[test]
fn escape_closes_only_the_top_overlay() {
    let mut app = App::new();
    let mut h = settled(&mut app);
    app.menu_open = true;
    at(&mut h, &mut app, 2000);
    app.dialog_open = true;
    at(&mut h, &mut app, 3000);
    at(&mut h, &mut app, 4000);

    key(&mut h, NamedKey::Escape);
    at(&mut h, &mut app, 4100);
    assert_eq!(
        (app.cancelled, app.menu_dismissed),
        (1, 0),
        "the dialog first"
    );

    // The app closes the dialog; once its close is over, Escape reaches the menu.
    app.dialog_open = false;
    at(&mut h, &mut app, 4200);
    at(&mut h, &mut app, 5000);
    key(&mut h, NamedKey::Escape);
    at(&mut h, &mut app, 5100);
    assert_eq!((app.cancelled, app.menu_dismissed), (1, 1), "then the menu");
}

#[test]
fn tab_inside_the_dialog_never_leaves_it() {
    let mut app = App::new();
    let mut h = settled(&mut app);
    app.dialog_open = true;
    at(&mut h, &mut app, 2000);
    let (_, out) = at(&mut h, &mut app, 3000);
    let in_dialog = |out: &FrameOutput| {
        let f = out.focused.expect("something in the dialog is focused");
        let n = node(out, f);
        n.role == Role::Button && (n.name == "Cancel" || n.name == "Discard")
    };
    assert!(in_dialog(&out), "focus starts in the dialog");
    let mut ms = 3000;
    for i in 0..20 {
        ms += 100;
        if i % 4 == 3 {
            shift_tab(&mut h);
        } else {
            key(&mut h, NamedKey::Tab);
        }
        let (_, out) = at(&mut h, &mut app, ms);
        assert!(in_dialog(&out), "press {i}");
    }
    assert_eq!(app.page_clicks, 0);
}

#[test]
fn closing_the_dialog_restores_focus_to_the_page_button() {
    let mut app = App::new();
    let mut h = settled(&mut app);
    key(&mut h, NamedKey::Tab);
    let (_, out) = at(&mut h, &mut app, 1100);
    assert_eq!(
        out.focused,
        Some(id("page-button")),
        "Tab reaches the first stop"
    );

    app.dialog_open = true;
    at(&mut h, &mut app, 2000);
    let (_, out) = at(&mut h, &mut app, 3000);
    assert_ne!(
        out.focused,
        Some(id("page-button")),
        "focus moved into the dialog"
    );

    app.dialog_open = false;
    let (_, out) = at(&mut h, &mut app, 4000);
    assert_eq!(out.focused, Some(id("page-button")));
}

#[test]
fn the_dialogs_exit_retention_per_tier_then_idle() {
    for (tier, close_ms) in [
        (MotionTier::Full, 180_u64),
        (MotionTier::Reduced, 120),
        (MotionTier::Off, 0),
    ] {
        let mut app = App::new();
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        app.dialog_open = true;
        at(&mut h, &mut app, 0);
        let (scene, out) = at(&mut h, &mut app, 2000);
        assert_eq!(out.repaint, Repaint::Idle, "{tier:?} open settles");
        assert!(!modal_is_empty(&scene), "{tier:?} draws the dialog");

        let t0 = 5000;
        app.dialog_open = false;
        let (scene, out) = at(&mut h, &mut app, t0);
        if close_ms == 0 {
            assert!(modal_is_empty(&scene), "{tier:?} snaps");
            assert_eq!(out.repaint, Repaint::Idle, "{tier:?}");
        } else {
            assert!(
                !modal_is_empty(&scene),
                "{tier:?} still draws at the first frame"
            );
            assert_eq!(out.repaint, Repaint::Now, "{tier:?}");
            let (scene, out) = at(&mut h, &mut app, t0 + close_ms - 1);
            assert!(
                !modal_is_empty(&scene),
                "{tier:?} draws until {close_ms} ms"
            );
            assert_eq!(out.repaint, Repaint::Now, "{tier:?}");
            let (scene, out) = at(&mut h, &mut app, t0 + close_ms);
            assert!(modal_is_empty(&scene), "{tier:?} is gone at {close_ms} ms");
            assert_eq!(out.repaint, Repaint::Idle, "{tier:?}");
        }
        let (scene, out) = at(&mut h, &mut app, t0 + 1000);
        assert!(
            modal_is_empty(&scene) && out.repaint == Repaint::Idle,
            "{tier:?}"
        );
    }
}

#[test]
fn a_zero_token_never_runs_in_any_tier() {
    for reduced in [Reduce::Keep, Reduce::Fade, Reduce::Snap] {
        let m = Motion {
            kind: Kind::Dismiss,
            token: Token::Zero,
            easing: Easing::Linear,
            reduced,
            reduced_token: None,
        };
        for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
            let ok = MOTION
                .resolve(&m, tier)
                .is_none_or(|r| r.duration.is_zero());
            assert!(ok, "{reduced:?} {tier:?}");
        }
    }
}

/// A frame of just a dropdown over an anchor: the combo's own open look is a visible fade, so it
/// is left out to isolate the list's repaint request.
fn dropdown_frame(h: &mut UiHarness, app: &mut App, ms: u64) -> Frame {
    h.at_ms(ms);
    h.frame(|ui: &mut Ui<'_>| {
        if app.combo_open {
            let r = ui.dropdown("dropdown", COMBO, id("combo"), &OPTIONS, 0);
            if r.picked.is_some() || r.dismissed {
                app.combo_open = false;
            }
            app.dropdown_dismissed += u32::from(r.dismissed);
        }
    })
}

#[test]
fn a_settled_dropdown_reports_idle_the_frame_after_the_change() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        let mut app = App::new();
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        dropdown_frame(&mut h, &mut app, 0);
        dropdown_frame(&mut h, &mut app, 1000);
        // Open: the first frame of the list is already still.
        app.combo_open = true;
        let (_, out) = dropdown_frame(&mut h, &mut app, 2000);
        assert_eq!(out.repaint, Repaint::Idle, "{tier:?} open frame");
        let (_, out) = dropdown_frame(&mut h, &mut app, 2016);
        assert_eq!(out.repaint, Repaint::Idle, "{tier:?} frame after open");
        // Close by Escape.
        key(&mut h, NamedKey::Escape);
        let (_, out) = dropdown_frame(&mut h, &mut app, 3000);
        assert!(!app.combo_open, "Escape dismissed the list");
        assert_eq!(app.dropdown_dismissed, 1);
        assert_eq!(out.repaint, Repaint::Idle, "{tier:?} close frame");
        let (_, out) = dropdown_frame(&mut h, &mut app, 3016);
        assert_eq!(out.repaint, Repaint::Idle, "{tier:?} frame after close");
    }
}

/// Input steps that touch every widget and both overlays.
type Step = Box<dyn Fn(&mut UiHarness, &mut App)>;

fn click_at(r: Rect) -> Step {
    let (x, y) = centre(r);
    Box::new(move |h, _| {
        h.input.pointer_to(x, y).click(P);
    })
}

fn steps() -> Vec<Step> {
    vec![
        Box::new(|h, _| {
            let (x, y) = centre(PAGE_BTN);
            h.input.pointer_to(x, y);
        }),
        click_at(PAGE_BTN),
        click_at(TOGGLE),
        click_at(CHECK),
        click_at(Rect::new(20.0, 190.0, 40.0, 32.0)),
        click_at(FIELD),
        Box::new(|h, _| key(h, NamedKey::Tab)),
        Box::new(|h, _| key(h, NamedKey::Tab)),
        Box::new(|h, _| shift_tab(h)),
        click_at(COMBO),
        Box::new(|h, _| key(h, NamedKey::Escape)),
        Box::new(|_, app| app.menu_open = true),
        Box::new(|h, _| {
            let (x, y) = centre(Rect::new(70.0, 40.0, 100.0, 20.0));
            h.input.pointer_to(x, y);
        }),
        Box::new(|_, app| app.dialog_open = true),
        Box::new(|h, _| key(h, NamedKey::Tab)),
        Box::new(|h, _| key(h, NamedKey::Enter)),
        Box::new(|_, app| app.dialog_open = false),
        Box::new(|_, app| app.menu_open = false),
        Box::new(|h, _| {
            h.input.access(AccessAction {
                target: id("toggle"),
                action: Action::Default,
                value: None,
            });
        }),
    ]
}

/// Runs the whole script on a fresh page, 150 ms between steps and a long settle after each, and
/// returns every frame.
fn run_script(tier: MotionTier) -> Vec<Frame> {
    let mut app = App::new();
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let mut frames = vec![at(&mut h, &mut app, 0)];
    let mut ms = 0;
    for step in steps() {
        ms += 150;
        step(&mut h, &mut app);
        frames.push(at(&mut h, &mut app, ms));
        // A frame later (focus lands at the end of a frame), then a long settle.
        ms += 50;
        frames.push(at(&mut h, &mut app, ms));
        ms += 1500;
        frames.push(at(&mut h, &mut app, ms));
    }
    frames
}

#[test]
fn the_script_is_deterministic() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        let a = run_script(tier);
        let b = run_script(tier);
        assert_eq!(a.len(), b.len());
        for (i, (x, y)) in a.iter().zip(&b).enumerate() {
            assert!(x == y, "{tier:?}: frame {i} differs between two runs");
        }
    }
}

#[test]
fn nothing_reports_now_while_nothing_moves() {
    // Off: nothing ever animates, so every frame of the script is idle.
    for (i, (_, out)) in run_script(MotionTier::Off).iter().enumerate() {
        assert_eq!(out.repaint, Repaint::Idle, "Off: frame {i}");
    }
    // Full and Reduced: every settle frame (1.5 s after a step) is idle.
    for tier in [MotionTier::Full, MotionTier::Reduced] {
        for (i, (_, out)) in run_script(tier).iter().enumerate() {
            if i > 0 && i % 3 == 0 {
                assert_eq!(out.repaint, Repaint::Idle, "{tier:?}: settle frame {i}");
            }
        }
    }
}
