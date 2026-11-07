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
fn page(ui: &mut Ui<'_>, app: &mut App) {
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
}

fn frame(h: &mut UiHarness, app: &mut App) -> Frame {
    h.frame(|ui: &mut Ui<'_>| page(ui, app))
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
    let layer = &scene.merged(LayerId::Modal);
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
            &[Action::Default, Action::Expand],
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
            // Focus returned at the end of the frame, after the page drew (D81).
            assert_eq!(out.repaint, Repaint::Now, "{tier:?}");
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
        // Open: the list is still from its first frame; that frame asks for one more only
        // because focus moved into the list after it drew (D81).
        app.combo_open = true;
        let (_, out) = dropdown_frame(&mut h, &mut app, 2000);
        assert_eq!(
            out.repaint,
            Repaint::Now,
            "{tier:?} open frame (focus moved)"
        );
        let (_, out) = dropdown_frame(&mut h, &mut app, 2016);
        assert_eq!(out.repaint, Repaint::Idle, "{tier:?} frame after open");
        // Close by Escape: the dismissal frame still drew the list, so it asks for the next
        // frame (D81); that frame drops the list and moves focus back, asking once more; then
        // nothing moves.
        key(&mut h, NamedKey::Escape);
        let (_, out) = dropdown_frame(&mut h, &mut app, 3000);
        assert!(!app.combo_open, "Escape dismissed the list");
        assert_eq!(app.dropdown_dismissed, 1);
        assert_eq!(out.repaint, Repaint::Now, "{tier:?} dismissal frame");
        let (scene, out) = dropdown_frame(&mut h, &mut app, 3016);
        assert!(
            scene.merged(LayerId::Popup).geometry.under.is_empty(),
            "{tier:?} the list is gone at once (D80)"
        );
        assert_eq!(
            out.repaint,
            Repaint::Now,
            "{tier:?} close frame (focus moved)"
        );
        let (_, out) = dropdown_frame(&mut h, &mut app, 3032);
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
    // Off: nothing ever animates. A step's frame may ask for one more frame when its input acted
    // or focus moved after the widgets drew (D81); every other frame is idle, except the frame
    // after Escape (step 10), where the dismissed list stops and focus returns to the combo.
    for (i, (_, out)) in run_script(MotionTier::Off).iter().enumerate() {
        let step_frame = i % 3 == 1;
        let after_escape = i == 3 * 10 + 2;
        if !step_frame && !after_escape {
            assert_eq!(out.repaint, Repaint::Idle, "Off: frame {i}");
        }
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

#[test]
fn a_settled_frame_has_the_same_tree_and_look_in_every_tier() {
    // The tier changes motion, never the resting look, the semantic tree or the strings in it.
    let runs = [MotionTier::Full, MotionTier::Reduced, MotionTier::Off].map(run_script);
    for i in (0..runs[0].len()).filter(|i| i % 3 == 0) {
        for other in &runs[1..] {
            assert!(runs[0][i].1.tree == other[i].1.tree, "tree, frame {i}");
            assert!(runs[0][i].0 == other[i].0, "scene, frame {i}");
        }
    }
}

fn has_focused_state(out: &FrameOutput, k: &str) -> bool {
    node(out, id(k)).states.contains(State::Focused)
}

#[test]
fn a_menu_pick_that_opens_the_dialog_keeps_focus_in_the_dialog_then_restores_the_page() {
    // Review F1, D83: the menu stops under the dialog it opened.
    let mut app = App::new();
    let mut h = settled(&mut app);
    key(&mut h, NamedKey::Tab);
    let (_, out) = at(&mut h, &mut app, 1100);
    assert_eq!(out.focused, Some(id("page-button")));
    app.menu_open = true;
    at(&mut h, &mut app, 1200);
    let (_, out) = at(&mut h, &mut app, 1300);
    assert!(
        out.focused.is_some() && out.focused != Some(id("page-button")),
        "focus moved into the menu"
    );
    // The pick: the app opens the dialog in the menu's frame, then stops calling the menu.
    app.dialog_open = true;
    at(&mut h, &mut app, 1400);
    app.menu_open = false;
    let confirm = WidgetId::ROOT.with((id("dialog"), "confirm"));
    for ms in [1500, 1600, 3000] {
        let (_, out) = at(&mut h, &mut app, ms);
        assert_eq!(out.focused, Some(confirm), "{ms}: Confirm is focused");
        assert!(!has_focused_state(&out, "page-button"), "{ms}");
    }
    // Enter still confirms (D20).
    key(&mut h, NamedKey::Enter);
    at(&mut h, &mut app, 3100);
    assert_eq!(app.confirmed, 1);
    app.dialog_open = false;
    at(&mut h, &mut app, 3200);
    let (_, out) = at(&mut h, &mut app, 3300);
    assert_eq!(
        out.focused,
        Some(id("page-button")),
        "focus back on the page button"
    );
}

#[test]
fn a_dropdown_pick_frame_asks_for_the_frame_that_drops_the_list() {
    // Review F3 (b), D81: the pick frame still drew the open list.
    for tier in [MotionTier::Full, MotionTier::Off] {
        let mut app = App::new();
        let mut h = settled(&mut app);
        h.tier = tier;
        let (x, y) = centre(COMBO);
        h.input.pointer_to(x, y).click(P);
        at(&mut h, &mut app, 1100);
        assert!(app.combo_open);
        at(&mut h, &mut app, 2000);
        // Row 1 sits under the combo's panel; press and release on it.
        let row = h
            .store
            .last_rect(id("dropdown").with("list").with("1".to_string()))
            .or_else(|| {
                let p = h.store.last_rect(id("dropdown").with("list"))?;
                Some(Rect::new(p.x, p.y + p.h / 2.0 - 4.0, p.w, 8.0))
            })
            .expect("the list registered");
        h.input
            .pointer_to(row.x + 8.0, row.y + row.h / 2.0)
            .click(P);
        let (scene, out) = at(&mut h, &mut app, 2100);
        assert!(!app.combo_open, "{tier:?}: picked");
        assert!(
            !scene.merged(LayerId::Popup).geometry.under.is_empty(),
            "{tier:?}: the pick frame still drew the list"
        );
        assert_eq!(out.repaint, Repaint::Now, "{tier:?}: pick frame");
        let (scene, _) = at(&mut h, &mut app, 2116);
        assert!(
            scene.merged(LayerId::Popup).geometry.under.is_empty(),
            "{tier:?}: the list is gone"
        );
        // Then it settles (Full: once the combo's closed look has faded in).
        let (_, out) = at(&mut h, &mut app, 3500);
        assert_eq!(out.repaint, Repaint::Idle, "{tier:?}: settles");
    }
}

// ---------------------------------------------------------------------------------------------
// M6: the control and chrome set together. One page holds every interactive M6 widget; the
// per-widget suites own the behaviour, this section owns what only holds side by side: the
// semantic tree, accessibility actions, one tab stop per composite and the idle report.
// ---------------------------------------------------------------------------------------------

mod m6 {
    use super::*;
    use lune_ui_core::icon;
    use lune_ui_core::text::{Spec, Style};
    use lune_ui_core::theme::DARK;
    use lune_ui_core::widgets::button::{ButtonOptions, IconButton};
    use lune_ui_core::widgets::filter_bar::FilterOptions;
    use lune_ui_core::widgets::nav_item::NavEntry;
    use lune_ui_core::widgets::progress::ProgressOptions;
    use lune_ui_core::widgets::radio_group::RadioOption;
    use lune_ui_core::widgets::search_picker::PickerOptions;
    use lune_ui_core::widgets::slider::SliderOptions;
    use lune_ui_core::widgets::spinner::SpinnerOptions;
    use lune_ui_core::widgets::tab_strip::{Tab, TabEvent, TabKey, TabStripOptions};
    use lune_ui_core::widgets::tone::Tone;

    const GHOST: Rect = Rect::new(20.0, 10.0, 90.0, 32.0);
    const ICON_BTN: Rect = Rect::new(120.0, 10.0, 32.0, 32.0);
    const NAV: Rect = Rect::new(20.0, 60.0, 180.0, 0.0);
    const RADIO: Rect = Rect::new(240.0, 60.0, 180.0, 0.0);
    const STAR_AT: (f32, f32) = (240.0, 160.0);
    const SLIDER: Rect = Rect::new(240.0, 220.0, 180.0, 24.0);
    const STRIP: Rect = Rect::new(20.0, 280.0, 600.0, 32.0);
    const CARD: Rect = Rect::new(20.0, 280.0, 600.0, 160.0);
    const FILTER: Rect = Rect::new(20.0, 470.0, 220.0, 32.0);
    const PICKER: Rect = Rect::new(260.0, 470.0, 220.0, 32.0);
    const SELECT: Rect = Rect::new(500.0, 60.0, 200.0, 20.0);
    const PROGRESS: Rect = Rect::new(500.0, 100.0, 200.0, 4.0);

    const NAVS: [NavEntry<'static>; 3] = [
        NavEntry {
            key: 1,
            label: "General",
            icon: None,
            enabled: true,
        },
        NavEntry {
            key: 2,
            label: "Network",
            icon: None,
            enabled: true,
        },
        NavEntry {
            key: 3,
            label: "About",
            icon: None,
            enabled: true,
        },
    ];
    const RADIOS: [RadioOption<'static>; 3] = [
        RadioOption {
            key: 1,
            label: "Small",
            enabled: true,
        },
        RadioOption {
            key: 2,
            label: "Medium",
            enabled: true,
        },
        RadioOption {
            key: 3,
            label: "Large",
            enabled: true,
        },
    ];
    const SLIDER_OPTS: SliderOptions<'static> = SliderOptions {
        min: 0.0,
        max: 100.0,
        step: 10.0,
        name: "Volume",
    };
    const ITEMS: [&str; 3] = ["main", "develop", "release"];

    #[derive(Debug)]
    struct Page {
        nav: u64,
        radio: u64,
        stars: u32,
        volume: f32,
        tabs: Vec<(u64, &'static str)>,
        active: u64,
        tab_events: Vec<TabEvent>,
        filter: String,
        picked: Option<usize>,
        ghost_clicks: u32,
        busy: bool,
    }

    impl Page {
        fn new() -> Self {
            Page {
                nav: 1,
                radio: 2,
                stars: 2,
                volume: 40.0,
                tabs: vec![(1, "alpha"), (2, "beta"), (3, "gamma")],
                active: 1,
                tab_events: Vec::new(),
                filter: String::new(),
                picked: Some(0),
                ghost_clicks: 0,
                busy: false,
            }
        }
    }

    fn page_frame(h: &mut UiHarness, p: &mut Page) -> FrameOutput {
        let (_, out) = h.frame(|ui: &mut Ui<'_>| {
            ui.painter().on(LayerId::Page);
            let r = ui.button_with(
                "ghost",
                GHOST,
                "Cancel",
                &ButtonOptions {
                    kind: ButtonKind::Ghost,
                    small: true,
                    icon: None,
                },
            );
            p.ghost_clicks += u32::from(r.clicked);
            ui.icon_button(
                "icon-button",
                ICON_BTN,
                &IconButton {
                    glyph: icon::SEARCH,
                    glyph_size: 16.0,
                    color: None,
                    radius: 4.0,
                    name: "Search",
                },
            );
            ui.nav_list("nav", NAV, &NAVS, &mut p.nav, "Settings");
            ui.radio_group("radio", RADIO, &RADIOS, &mut p.radio, "Size");
            ui.star_rating("stars", STAR_AT, &mut p.stars, 5, "Rating");
            ui.slider("slider", SLIDER, &mut p.volume, &SLIDER_OPTS);
            let tabs: Vec<Tab<'_>> = p
                .tabs
                .iter()
                .map(|&(k, title)| Tab {
                    key: TabKey(k),
                    title,
                    closable: true,
                    progress: None,
                })
                .collect();
            let events = ui
                .tab_strip(
                    "strip",
                    STRIP,
                    &tabs,
                    &TabStripOptions {
                        active: Some(TabKey(p.active)),
                        panel_focused: true,
                        busy: None,
                        busy_tone: Tone::Neutral,
                        card: CARD,
                        menu_extra: &[],
                        name: "Documents",
                        header_tint: None,
                    },
                )
                .events;
            for e in &events {
                if let TabEvent::Activate(k) = e {
                    p.active = k.0;
                }
            }
            p.tab_events.extend(events);
            ui.filter_bar(
                "filter",
                FILTER,
                &mut p.filter,
                &FilterOptions {
                    placeholder: "Filter",
                    label: None,
                    clear_unfocusable: false,
                },
            );
            ui.search_picker(
                "picker",
                PICKER,
                &ITEMS,
                &mut p.picked,
                &PickerOptions {
                    placeholder: "Branch",
                    name: "Branch",
                    ..PickerOptions::default()
                },
            );
            let spec = Spec::plain("v1.2.3", Style::ui(13.0, DARK.fg_primary));
            ui.selectable_text("select", SELECT, &spec, "v1.2.3");
            ui.progress(
                "progress",
                PROGRESS,
                &ProgressOptions {
                    value: Some(0.5),
                    name: "Upload",
                    ..ProgressOptions::default()
                },
            );
            ui.spinner(
                "spinner",
                (560.0, 140.0),
                &SpinnerOptions {
                    busy: p.busy,
                    size: 16.0,
                    color: None,
                    name: "Loading",
                },
            );
        });
        out
    }

    fn run(h: &mut UiHarness, p: &mut Page, ms: u64) -> FrameOutput {
        h.at_ms(ms);
        page_frame(h, p)
    }

    fn by_name<'a>(out: &'a FrameOutput, role: Role, name: &str) -> &'a Node {
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == role && n.name == name)
            .unwrap_or_else(|| panic!("no {role:?} named {name:?}"))
    }

    fn settled_page() -> (UiHarness, Page) {
        let mut p = Page::new();
        let mut h = UiHarness::new(WIN);
        run(&mut h, &mut p, 0);
        let out = run(&mut h, &mut p, 1000);
        assert_eq!(out.repaint, Repaint::Idle, "the page settles");
        (h, p)
    }

    #[test]
    fn a_frame_of_every_m6_widget_yields_the_expected_roles_names_and_states() {
        let (mut h, mut p) = settled_page();
        let out = run(&mut h, &mut p, 1100);
        let checked =
            |role: Role, name: &str| by_name(&out, role, name).states.contains(State::Checked);

        // Buttons.
        for (role, name) in [(Role::Button, "Cancel"), (Role::Button, "Search")] {
            let n = by_name(&out, role, name);
            assert!(n.states.contains(State::Focusable), "{name}");
            assert!(n.actions.contains(Action::Default), "{name}");
        }
        // The nav list: a list of items, the current one selected and current.
        assert_eq!(by_name(&out, Role::List, "Settings").id, id("nav"));
        for (name, on) in [("General", true), ("Network", false), ("About", false)] {
            let n = by_name(&out, Role::ListItem, name);
            assert_eq!(n.states.contains(State::Selected), on, "{name}");
            assert_eq!(n.states.contains(State::Current), on, "{name}");
            assert!(n.actions.contains(Action::Default), "{name}");
        }
        // The radio group: a radio-group of radios, the selected one checked.
        assert_eq!(by_name(&out, Role::RadioGroup, "Size").id, id("radio"));
        for (name, on) in [("Small", false), ("Medium", true), ("Large", false)] {
            assert_eq!(checked(Role::Radio, name), on, "{name}");
            assert!(
                by_name(&out, Role::Radio, name)
                    .actions
                    .contains(Action::Default)
            );
        }
        // The star rating: a radio-group of "N of 5" radios, the Nth checked.
        assert_eq!(by_name(&out, Role::RadioGroup, "Rating").id, id("stars"));
        for n in 1..=5 {
            let name = format!("{n} of 5");
            assert_eq!(checked(Role::Radio, &name), n == 2, "{name}");
        }
        // The slider: a number value in range with increment and decrement.
        let s = by_name(&out, Role::Slider, "Volume");
        assert!(
            matches!(s.value, Some(Value::Number { value, min, max, step })
            if (value - 40.0).abs() < 1e-4 && min == 0.0 && max == 100.0 && step == 10.0)
        );
        assert!(s.actions.contains(Action::Increment));
        assert!(s.actions.contains(Action::Decrement));
        // The tab list: tabs, the active one selected.
        assert_eq!(by_name(&out, Role::TabList, "Documents").id, id("strip"));
        for (name, on) in [("alpha", true), ("beta", false), ("gamma", false)] {
            let n = by_name(&out, Role::Tab, name);
            assert_eq!(n.states.contains(State::Selected), on, "{name}");
            assert!(n.actions.contains(Action::Default), "{name}");
        }
        // The progress bar and the static text.
        assert!(
            out.tree
                .nodes
                .iter()
                .any(|n| n.role == Role::Progress && n.name == "Upload")
        );
        assert_eq!(by_name(&out, Role::StaticText, "v1.2.3").name, "v1.2.3");
    }

    #[test]
    fn an_injected_default_action_works_on_radio_star_nav_item_and_tab() {
        let (mut h, mut p) = settled_page();
        let out = run(&mut h, &mut p, 1100);
        for (role, name) in [
            (Role::Radio, "Large"),
            (Role::Radio, "4 of 5"),
            (Role::ListItem, "About"),
            (Role::Tab, "gamma"),
            (Role::Button, "Cancel"),
        ] {
            h.input.access(AccessAction {
                target: by_name(&out, role, name).id,
                action: Action::Default,
                value: None,
            });
        }
        run(&mut h, &mut p, 1200);
        assert_eq!(p.radio, 3, "radio");
        assert_eq!(p.stars, 4, "star");
        assert_eq!(p.nav, 3, "nav item");
        assert_eq!(p.active, 3, "tab");
        assert_eq!(p.tab_events, [TabEvent::Activate(TabKey(3))]);
        assert_eq!(p.ghost_clicks, 1, "button");
        // The default action on the current star clears it, as a click does.
        let out = run(&mut h, &mut p, 1300);
        h.input.access(AccessAction {
            target: by_name(&out, Role::Radio, "4 of 5").id,
            action: Action::Default,
            value: None,
        });
        run(&mut h, &mut p, 1400);
        assert_eq!(p.stars, 0, "the current star clears");
    }

    #[test]
    fn an_increment_action_steps_the_slider_and_decrement_steps_it_back() {
        let (mut h, mut p) = settled_page();
        let out = run(&mut h, &mut p, 1100);
        let slider = by_name(&out, Role::Slider, "Volume").id;
        for (action, want) in [(Action::Increment, 50.0), (Action::Decrement, 40.0)] {
            h.input.access(AccessAction {
                target: slider,
                action,
                value: None,
            });
            run(&mut h, &mut p, 1200);
            assert!((p.volume - want).abs() < 1e-3, "{action:?}: {}", p.volume);
            run(&mut h, &mut p, 1300);
        }
    }

    #[test]
    fn a_tab_cycle_visits_each_composite_once() {
        let mut p = Page::new();
        let mut h = UiHarness::new(WIN);
        run(&mut h, &mut p, 0);
        let mut ms = 100;
        let mut seen: Vec<(WidgetId, Rect)> = Vec::new();
        for _ in 0..20 {
            key(&mut h, NamedKey::Tab);
            ms += 100;
            let out = run(&mut h, &mut p, ms);
            let f = out.focused.expect("Tab focuses something");
            if seen.iter().any(|&(s, _)| s == f) {
                break;
            }
            seen.push((f, node(&out, f).bounds));
        }
        let stops: Vec<WidgetId> = seen.iter().map(|&(f, _)| f).collect();
        let bounds = |f: WidgetId| {
            seen.iter()
                .find(|&&(s, _)| s == f)
                .map(|&(_, r)| r)
                .expect("a recorded stop")
        };
        let inside = |r: Rect, outer: Rect, tall: f32| {
            let (cx, cy) = centre(r);
            cx >= outer.x && cx <= outer.right() && cy >= outer.y && cy <= outer.y + tall
        };
        let composites: [(&str, Rect, f32); 6] = [
            ("nav", NAV, 34.0 * 3.0),
            ("radio", RADIO, 28.0 * 3.0),
            ("stars", Rect::new(STAR_AT.0, STAR_AT.1, 200.0, 0.0), 40.0),
            ("slider", SLIDER, 24.0),
            ("strip", STRIP, 32.0),
            ("filter", FILTER, 32.0),
        ];
        for (name, outer, tall) in composites {
            let n = stops
                .iter()
                .filter(|&&f| inside(bounds(f), outer, tall))
                .count();
            // The strip is one tab stop plus its new-tab button; every other composite is one stop.
            let want = if name == "strip" { 2 } else { 1 };
            assert_eq!(n, want, "{name}: {n} stops in {stops:?}");
        }
        // A roving composite's one stop is its selected child.
        let focused_in = |outer: Rect, tall: f32| {
            stops
                .iter()
                .copied()
                .find(|&f| inside(bounds(f), outer, tall))
                .expect("a stop")
        };
        assert_eq!(focused_in(NAV, 102.0), id("nav").with(1u64));
        assert_eq!(focused_in(RADIO, 84.0), id("radio").with(2u64));
        assert_eq!(
            focused_in(Rect::new(STAR_AT.0, STAR_AT.1, 200.0, 0.0), 40.0),
            id("stars").with(2u32)
        );
        // The selectable text never takes a stop.
        assert!(!stops.contains(&id("select")));
    }

    #[test]
    fn a_page_of_them_all_at_rest_reports_idle_once_the_loops_are_off() {
        let (mut h, mut p) = settled_page();
        for ms in [1100, 1200, 5000] {
            assert_eq!(run(&mut h, &mut p, ms).repaint, Repaint::Idle, "{ms}");
        }
        // A turning spinner is a loop: the page asks for frames, and stops asking when it stops.
        p.busy = true;
        let out = run(&mut h, &mut p, 5100);
        assert_ne!(out.repaint, Repaint::Idle, "the busy spinner moves");
        p.busy = false;
        let mut ms = 5200;
        let mut idle = false;
        for _ in 0..10 {
            ms += 100;
            idle = run(&mut h, &mut p, ms).repaint == Repaint::Idle;
            if idle {
                break;
            }
        }
        assert!(idle, "the page settles once the spinner stops");
    }

    #[test]
    fn under_the_off_tier_a_busy_spinner_reports_idle_too() {
        let (mut h, mut p) = settled_page();
        h.tier = MotionTier::Off;
        p.busy = true;
        run(&mut h, &mut p, 1100);
        assert_eq!(run(&mut h, &mut p, 1200).repaint, Repaint::Idle);
    }
}

// ---------------------------------------------------------------------------------------------
// M7: the overlay, container and navigation set together. One page holds every interactive M7
// widget; the per-widget suites own the behaviour, this section owns what only holds side by
// side: the semantic tree, accessibility actions, one tab stop per composite, the disabled walk,
// the idle report, and the overlays that rise above a dialog.
// ---------------------------------------------------------------------------------------------

mod m7 {
    use super::*;
    use std::time::Duration;

    use lune_ui_core::GlassLayer;
    use lune_ui_core::access::{AccessValue, Live};
    use lune_ui_core::icon;
    use lune_ui_core::layout::Axis;
    use lune_ui_core::shortcuts::{Binding, Chord, Scope};
    use lune_ui_core::testing::disabled_violations;
    use lune_ui_core::widgets::command_palette::{Palette, PaletteRow, PaletteSection};
    use lune_ui_core::widgets::flyout::Flyout;
    use lune_ui_core::widgets::icon_rail::RailItem;
    use lune_ui_core::widgets::menu::{MenuItem, MenuOptions, MenuPlace};
    use lune_ui_core::widgets::nav_item::NavEntry;
    use lune_ui_core::widgets::page::{PageOptions, PageSlot};
    use lune_ui_core::widgets::panel::PanelOptions;
    use lune_ui_core::widgets::popover::{PlaceAlign, Side};
    use lune_ui_core::widgets::row::SettingRow;
    use lune_ui_core::widgets::settings_page::SettingsPage;
    use lune_ui_core::widgets::side_panel::SidePanelOptions;
    use lune_ui_core::widgets::sidebar::{Sidebar, SidebarBody, SidebarFooter};
    use lune_ui_core::widgets::split_panes::{Pane, SplitOptions, SplitSide};
    use lune_ui_core::widgets::toast::{ToastAction, ToastKind, ToastSpec, Toasts};
    use lune_ui_core::widgets::tone::Tone;
    use lune_ui_core::widgets::tree_view::{ChevronStyle, GuideStyle, TreeOptions, TreeRow};

    const M7_WIN: (f32, f32) = (1240.0, 900.0);
    const RAIL: Rect = Rect::new(0.0, 0.0, 48.0, 420.0);
    const NAV: Rect = Rect::new(60.0, 10.0, 170.0, 0.0);
    const TREE: Rect = Rect::new(250.0, 10.0, 280.0, 200.0);
    const SPLIT: Rect = Rect::new(550.0, 10.0, 400.0, 200.0);
    const SIDEBAR: Rect = Rect::new(60.0, 240.0, 180.0, 180.0);
    const PAGE: Rect = Rect::new(250.0, 240.0, 400.0, 300.0);
    const SIDE_PANEL: Rect = Rect::new(670.0, 240.0, 200.0, 150.0);
    const SETTINGS: Rect = Rect::new(890.0, 240.0, 340.0, 300.0);
    const MENU_AT: (f32, f32) = (300.0, 620.0);
    const ANCHOR: Rect = Rect::new(1180.0, 860.0, 24.0, 24.0);
    const GLASS: GlassLayer = GlassLayer {
        tint: [10, 20, 30, 255],
        tint_opacity: 0.8,
        material_opacity: 0.7,
        curve: lune_ui_core::LuminosityCurve::AVALONIA,
    };
    const UNDER: [f32; 4] = [0.05, 0.06, 0.07, 1.0];
    const SPLIT_DEFAULT: f32 = 200.0;
    /// The splitter's largest size: the rect less the strip less the other pane's smallest.
    const SPLIT_MAX: f32 = 400.0 - 4.0 - 100.0;

    const NAVS: [NavEntry<'static>; 2] = [
        NavEntry {
            key: 1,
            label: "General",
            icon: None,
            enabled: true,
        },
        NavEntry {
            key: 2,
            label: "Network",
            icon: None,
            enabled: true,
        },
    ];

    const fn tree_row(key: u64, depth: u16, text: &'static str, kids: bool) -> TreeRow<'static> {
        TreeRow {
            key,
            depth,
            text,
            glyph: None,
            has_children: kids,
        }
    }

    /// `src` (open at the start) holds two files; `docs` is closed and holds one.
    const TREE_ROWS: [TreeRow<'static>; 5] = [
        tree_row(1, 0, "src", true),
        tree_row(2, 1, "main.rs", false),
        tree_row(3, 1, "lib.rs", false),
        tree_row(4, 0, "docs", true),
        tree_row(5, 1, "readme.md", false),
    ];

    /// A header, an enabled row, a disabled one and a danger one: the picks are 1 and 3.
    const MENU_ROWS: [MenuRow<'static>; 4] = [
        MenuRow::Header {
            name: "Atlas",
            status: "Serving",
            tone: Tone::Ok,
        },
        MenuRow::Entry(MenuItem {
            glyph: Some(icon::ADD),
            text: "Open",
            shortcut: "",
            disabled: false,
            danger: false,
            chevron: false,
        }),
        MenuRow::Entry(MenuItem {
            glyph: None,
            text: "Pause",
            shortcut: "",
            disabled: true,
            danger: false,
            chevron: false,
        }),
        MenuRow::Entry(MenuItem {
            glyph: None,
            text: "Quit",
            shortcut: "",
            disabled: false,
            danger: true,
            chevron: false,
        }),
    ];

    const fn palette_row(key: u64, text: &'static str) -> PaletteRow<'static> {
        PaletteRow {
            key,
            text,
            detail: None,
            icon: None,
            hint: None,
            destructive: false,
        }
    }

    const PALETTE_ROWS: [PaletteRow<'static>; 2] =
        [palette_row(10, "Reload"), palette_row(11, "Rename")];

    fn rail_item(key: u64, glyph: &'static str, name: &'static str) -> RailItem<'static> {
        RailItem {
            key,
            glyph,
            name,
            active: false,
            enabled: true,
            dot: false,
            badge: None,
            note: None,
        }
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn prompt(message: &str) -> ToastSpec {
        ToastSpec {
            actions: vec![
                ToastAction {
                    label: "Open".into(),
                    primary: true,
                    id: 2,
                },
                ToastAction {
                    label: "Later".into(),
                    primary: false,
                    id: 1,
                },
            ],
            ..ToastSpec::new(ToastKind::Info, message)
        }
    }

    /// The app side of the page: what is open, and what the frames reported.
    #[derive(Debug)]
    struct Page {
        enabled: bool,
        started: bool,
        nav: u64,
        restart: bool,
        menu_open: bool,
        flyout_open: bool,
        palette_open: bool,
        query: String,
        toasts: Toasts,
        // Reported.
        rail_picked: Vec<u64>,
        tree_toggled: Vec<u64>,
        tree_activated: Vec<u64>,
        split_size: f32,
        menu_picked: Vec<usize>,
        palette_activated: Vec<u64>,
    }

    impl Page {
        fn new() -> Self {
            Page {
                enabled: true,
                started: false,
                nav: 1,
                restart: true,
                menu_open: false,
                flyout_open: false,
                palette_open: false,
                query: String::new(),
                toasts: Toasts::default(),
                rail_picked: Vec::new(),
                tree_toggled: Vec::new(),
                tree_activated: Vec::new(),
                split_size: 0.0,
                menu_picked: Vec::new(),
                palette_activated: Vec::new(),
            }
        }
    }

    fn page_frame(h: &mut UiHarness, p: &mut Page) -> FrameOutput {
        let enabled = p.enabled;
        let (_, out) = h.frame(|ui: &mut Ui<'_>| {
            ui.painter().on(LayerId::Page);
            if !p.started {
                p.started = true;
                ui.tree_state("tree").expanded.insert(1);
            }
            ui.with_enabled(enabled, |ui| {
                // The rail: the first item is open and dirty, the second counts, the third is off.
                let top = [
                    RailItem {
                        active: true,
                        dot: true,
                        note: Some("Uncommitted changes"),
                        ..rail_item(1, icon::EDIT, "Files")
                    },
                    RailItem {
                        badge: Some(3),
                        note: Some("3 unread"),
                        ..rail_item(2, icon::SEARCH, "Search")
                    },
                    RailItem {
                        enabled: false,
                        ..rail_item(3, icon::INFO, "Hidden")
                    },
                ];
                let bottom = [rail_item(4, icon::GEAR, "Settings")];
                if let Some(k) = ui.icon_rail("rail", RAIL, &top, &bottom, "Panels") {
                    p.rail_picked.push(k);
                }
                ui.nav_list("nav", NAV, &NAVS, &mut p.nav, "Sections");
                let t = ui.tree_view(
                    "tree",
                    TREE,
                    &TreeOptions {
                        rows: &TREE_ROWS,
                        label: "Project",
                        guides: true,
                        guide_style: GuideStyle::Elbow,
                        chevron: ChevronStyle::Triangle,
                        toggle_on_row: false,
                    },
                    |_, _| {},
                );
                p.tree_toggled.extend(t.toggled);
                p.tree_activated.extend(t.activated);
                let s = ui.split_panes(
                    "split",
                    SPLIT,
                    &SplitOptions {
                        axis: Axis::Horizontal,
                        sized: SplitSide::First,
                        default: SPLIT_DEFAULT,
                        min: 120.0,
                        max: 300.0,
                        min_other: 100.0,
                        hidden: false,
                        name: "Side panel width",
                    },
                    |ui, pane, r| {
                        if pane == Pane::First {
                            ui.panel(
                                "panel",
                                r,
                                &PanelOptions {
                                    title: Some("Changes"),
                                    meta: Some("3 files"),
                                    ..PanelOptions::default()
                                },
                                |_, _| 0.0,
                            );
                        }
                    },
                );
                p.split_size = s.size;
                ui.sidebar(
                    "sb",
                    SIDEBAR,
                    Sidebar {
                        body: SidebarBody::Steps {
                            labels: &["Connect", "Review"],
                            current: 0,
                        },
                        footer: Some(SidebarFooter {
                            tone: Tone::Ok,
                            text: "Connected",
                            mono: false,
                        }),
                        name: "Setup",
                    },
                );
                ui.page(
                    "page",
                    PAGE,
                    &PageOptions {
                        title: "Overview",
                        subtitle: Some("The project at a glance"),
                        glass: GLASS,
                        under: UNDER,
                        actions_w: 0.0,
                        footer: false,
                        swap_key: 1,
                        name: "",
                    },
                    |_, slot| match slot {
                        PageSlot::Body(..) => 40.0,
                        _ => 0.0,
                    },
                );
                ui.side_panel(
                    "sp",
                    SIDE_PANEL,
                    &SidePanelOptions {
                        panel: 1,
                        keys: &[1, 2],
                        glass: GLASS,
                        under: UNDER,
                        name: "History",
                    },
                    |_, _| {},
                );
                ui.settings_page(
                    "settings",
                    SETTINGS,
                    &SettingsPage {
                        glass: GLASS,
                        under: UNDER,
                        restart: p.restart,
                        name: "General",
                    },
                    |ui, r| {
                        let row = SettingRow {
                            title: "Updates",
                            description: None,
                            off: false,
                            control_w: 0.0,
                        };
                        ui.setting_row("row", Rect::new(r.x, r.y, r.w, 0.0), &row, |_, _| {})
                    },
                );
                if p.menu_open {
                    let r = ui.menu_with(
                        "menu",
                        &MENU_ROWS,
                        &MenuOptions {
                            place: MenuPlace::At(MENU_AT),
                            anchor_id: None,
                            name: "Actions",
                        },
                    );
                    p.menu_picked.extend(r.picked);
                }
                if p.flyout_open {
                    ui.flyout(
                        "flyout",
                        &Flyout {
                            anchor: ANCHOR,
                            anchor_id: None,
                            side: Side::Top,
                            align: PlaceAlign::End,
                            mark: None,
                            wordmark: "Atlas",
                            state: "Serving",
                            tone: Tone::Ok,
                            stats: &[],
                            routes: &[("Opus", "route-one")],
                            rows: &MENU_ROWS[1..],
                            name: "Tray",
                        },
                    );
                }
                if p.palette_open {
                    let secs = [PaletteSection {
                        title: "COMMANDS",
                        rows: &PALETTE_ROWS,
                        more: false,
                    }];
                    let r = ui.command_palette(
                        "palette",
                        &mut p.query,
                        &Palette {
                            sections: &secs,
                            placeholder: "Search",
                            footer_left: "Up/Down",
                            error: None,
                            name: "Commands",
                        },
                        &glass(),
                    );
                    p.palette_activated.extend(r.activated);
                }
                ui.toasts("toasts", &mut p.toasts);
            });
        });
        out
    }

    fn run(h: &mut UiHarness, p: &mut Page, at: u64) -> FrameOutput {
        h.at_ms(at);
        page_frame(h, p)
    }

    /// Two frames to open, one more to settle.
    fn settled_with(mut p: Page) -> (UiHarness, Page) {
        let mut h = UiHarness::new(M7_WIN);
        for at in [0, 500, 1000] {
            run(&mut h, &mut p, at);
        }
        (h, p)
    }

    fn by_name<'a>(out: &'a FrameOutput, role: Role, name: &str) -> &'a Node {
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == role && n.name == name)
            .unwrap_or_else(|| panic!("no {role:?} named {name:?}"))
    }

    fn inject(h: &mut UiHarness, target: WidgetId, action: Action, value: Option<AccessValue>) {
        h.input.access(AccessAction {
            target,
            action,
            value,
        });
    }

    #[test]
    fn a_frame_of_every_m7_widget_yields_the_expected_roles_names_and_states() {
        let mut p = Page::new();
        p.menu_open = true;
        p.flyout_open = true;
        p.toasts
            .push(ToastSpec::new(ToastKind::Error, "Build failed"), ms(0));
        p.toasts
            .push(ToastSpec::new(ToastKind::Info, "Saved"), ms(0));
        let (mut h, mut p) = settled_with(p);
        let out = run(&mut h, &mut p, 1100);

        // The rail: a toolbar of buttons; the open one reads pressed, the notes describe the
        // dot and the badge, the off one is disabled and has no stop.
        assert_eq!(by_name(&out, Role::Toolbar, "Panels").id, id("rail"));
        let files = by_name(&out, Role::Button, "Files");
        assert!(files.states.contains(State::Pressed));
        assert_eq!(files.description.as_deref(), Some("Uncommitted changes"));
        let search = by_name(&out, Role::Button, "Search");
        assert!(!search.states.contains(State::Pressed));
        assert_eq!(search.description.as_deref(), Some("3 unread"));
        let off = by_name(&out, Role::Button, "Hidden");
        assert!(off.states.contains(State::Disabled) && !off.states.contains(State::Focusable));
        // The nav list.
        assert_eq!(by_name(&out, Role::List, "Sections").id, id("nav"));
        // The tree: the rows in view with their level and open state, the hidden row absent.
        let tree = by_name(&out, Role::Tree, "Project");
        assert_eq!(tree.id, id("tree"));
        assert!(tree.states.contains(State::Focusable));
        let rows: Vec<&str> = out
            .tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::TreeItem)
            .map(|n| n.name.as_str())
            .collect();
        assert_eq!(rows, ["src", "main.rs", "lib.rs", "docs"]);
        let src = by_name(&out, Role::TreeItem, "src");
        assert_eq!(src.level, Some(1));
        assert!(src.states.contains(State::Expanded));
        assert!(src.actions.contains(Action::Collapse));
        assert_eq!(by_name(&out, Role::TreeItem, "main.rs").level, Some(2));
        let docs = by_name(&out, Role::TreeItem, "docs");
        assert!(!docs.states.contains(State::Expanded));
        assert!(docs.actions.contains(Action::Expand));
        // The splitter: a separator whose value is the sized pane's size.
        let split = by_name(&out, Role::Separator, "Side panel width");
        assert!(split.states.contains(State::Focusable));
        // The largest size is the room: 400 less the 4 px strip less the other pane's 100.
        assert!(
            matches!(split.value, Some(Value::Number { value, min, max, .. })
            if value == f64::from(SPLIT_DEFAULT) && min == 120.0 && max == f64::from(SPLIT_MAX))
        );
        for a in [Action::Increment, Action::Decrement, Action::SetValue] {
            assert!(split.actions.contains(a), "{a:?}");
        }
        // The containers: a group each, the panel described by its meta, the page by its subtitle.
        assert_eq!(
            by_name(&out, Role::Group, "Changes").description.as_deref(),
            Some("3 files")
        );
        assert_eq!(
            by_name(&out, Role::Group, "Overview")
                .description
                .as_deref(),
            Some("The project at a glance")
        );
        by_name(&out, Role::Group, "History");
        by_name(&out, Role::Group, "General");
        // The sidebar's steps are a navigation named by it, its footer a polite status.
        assert_eq!(
            by_name(&out, Role::Navigation, "Setup").id,
            id("sb").with("steps")
        );
        by_name(&out, Role::Status, "Connected");
        // The settings page's restart banner: a status over an action button.
        assert!(
            out.tree
                .nodes
                .iter()
                .any(|n| n.role == Role::Button && SETTINGS.contains(centre(n.bounds)))
        );
        // The menu: rows by name, the disabled one off, the header text only.
        by_name(&out, Role::Menu, "Actions");
        by_name(&out, Role::MenuItem, "Open");
        let pause = by_name(&out, Role::MenuItem, "Pause");
        assert!(pause.states.contains(State::Disabled) && !pause.states.contains(State::Focusable));
        by_name(&out, Role::StaticText, "Atlas");
        // The flyout's rows are a menu of their own.
        by_name(&out, Role::Menu, "Tray");
        // The toasts: an error is assertive, an info is polite.
        let err = by_name(&out, Role::Alert, "Build failed");
        assert_eq!(err.live, Some(Live::Assertive));
        let info = by_name(&out, Role::Status, "Saved");
        assert_eq!(info.live, Some(Live::Polite));
    }

    #[test]
    fn a_palette_over_the_page_is_a_dialog_of_rows_with_a_search_box() {
        let mut p = Page::new();
        p.palette_open = true;
        p.query = "r".into();
        let (mut h, mut p) = settled_with(p);
        let out = run(&mut h, &mut p, 1100);
        assert_eq!(by_name(&out, Role::Dialog, "Commands").id, id("palette"));
        by_name(&out, Role::TextInput, "Search");
        for name in ["Reload", "Rename"] {
            let n = by_name(&out, Role::ListItem, name);
            assert!(n.actions.contains(Action::Default), "{name}");
        }
    }

    #[test]
    fn injected_actions_work_on_a_menu_item_tree_rows_the_splitter_a_rail_button_and_a_palette_row()
    {
        let mut p = Page::new();
        p.menu_open = true;
        let (mut h, mut p) = settled_with(p);
        let out = run(&mut h, &mut p, 1100);
        assert_eq!(p.split_size, SPLIT_DEFAULT);

        // A rail button, a closed tree row (expand) and a menu item, in one frame.
        inject(
            &mut h,
            by_name(&out, Role::Button, "Search").id,
            Action::Default,
            None,
        );
        inject(
            &mut h,
            by_name(&out, Role::TreeItem, "docs").id,
            Action::Expand,
            None,
        );
        inject(
            &mut h,
            by_name(&out, Role::MenuItem, "Open").id,
            Action::Default,
            None,
        );
        run(&mut h, &mut p, 1200);
        assert_eq!(p.rail_picked, [2], "rail button");
        assert_eq!(p.tree_toggled, [4], "tree expand");
        assert_eq!(p.menu_picked, [1], "menu item (the header is row 0)");

        // An open row collapses, a leaf's default selects it.
        p.menu_open = false;
        let out = run(&mut h, &mut p, 1300);
        inject(
            &mut h,
            by_name(&out, Role::TreeItem, "src").id,
            Action::Collapse,
            None,
        );
        run(&mut h, &mut p, 1400);
        assert_eq!(p.tree_toggled, [4, 1], "tree collapse");
        let out = run(&mut h, &mut p, 2000);
        inject(
            &mut h,
            by_name(&out, Role::TreeItem, "docs").id,
            Action::Default,
            None,
        );
        run(&mut h, &mut p, 2100);
        let out = run(&mut h, &mut p, 2200);
        assert!(
            by_name(&out, Role::TreeItem, "docs")
                .states
                .contains(State::Selected)
        );

        // The splitter steps by 16 and takes a value, clamped.
        let split = by_name(&out, Role::Separator, "Side panel width").id;
        inject(&mut h, split, Action::Increment, None);
        run(&mut h, &mut p, 2300);
        assert_eq!(p.split_size, SPLIT_DEFAULT + 16.0, "increment");
        inject(
            &mut h,
            split,
            Action::SetValue,
            Some(AccessValue::Number(250.0)),
        );
        run(&mut h, &mut p, 2400);
        assert_eq!(p.split_size, 250.0, "set value");
        inject(
            &mut h,
            split,
            Action::SetValue,
            Some(AccessValue::Number(9e9)),
        );
        run(&mut h, &mut p, 2500);
        assert_eq!(p.split_size, SPLIT_MAX, "clamped to the room");
        inject(&mut h, split, Action::Decrement, None);
        run(&mut h, &mut p, 2600);
        assert_eq!(p.split_size, SPLIT_MAX - 16.0, "decrement");

        // A palette row.
        let mut p = Page::new();
        p.palette_open = true;
        p.query = "r".into();
        let (mut h, mut p) = settled_with(p);
        let out = run(&mut h, &mut p, 1100);
        inject(
            &mut h,
            by_name(&out, Role::ListItem, "Rename").id,
            Action::Default,
            None,
        );
        run(&mut h, &mut p, 1200);
        assert_eq!(p.palette_activated, [11], "palette row");
    }

    #[test]
    fn a_tab_cycle_over_the_rail_nav_tree_and_splitter_visits_each_once() {
        let (mut h, mut p) = settled_with(Page::new());
        let mut ms = 1000;
        let mut seen: Vec<(WidgetId, Rect)> = Vec::new();
        for _ in 0..30 {
            key(&mut h, NamedKey::Tab);
            ms += 100;
            let out = run(&mut h, &mut p, ms);
            let f = out.focused.expect("Tab focuses something");
            if seen.iter().any(|&(s, _)| s == f) {
                break;
            }
            seen.push((f, node(&out, f).bounds));
        }
        let inside = |outer: Rect| {
            seen.iter()
                .filter(|&&(_, b)| {
                    let (cx, cy) = centre(b);
                    outer.contains((cx, cy))
                })
                .map(|&(f, _)| f)
                .collect::<Vec<_>>()
        };
        // The rail's one stop is its open item; the nav's is its current one; the tree and the
        // splitter are one each.
        assert_eq!(inside(RAIL), [id("rail").with(1_u64)], "rail");
        assert_eq!(
            inside(Rect::new(NAV.x, NAV.y, NAV.w, 80.0)),
            [id("nav").with(1_u64)],
            "nav"
        );
        assert_eq!(inside(TREE), [id("tree")], "tree");
        assert_eq!(inside(SPLIT), [id("split")], "splitter");
    }

    /// The keys a focused composite uses.
    const KEYS: [NamedKey; 10] = [
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::Home,
        NamedKey::End,
        NamedKey::PageUp,
        NamedKey::PageDown,
        NamedKey::Enter,
        NamedKey::Space,
    ];

    #[test]
    fn every_interactive_m7_widget_in_a_disabled_scope_passes_the_walk() {
        let mut p = Page::new();
        p.menu_open = true;
        p.flyout_open = true;
        p.toasts
            .push(ToastSpec::new(ToastKind::Error, "Build failed"), ms(0));
        p.toasts.push(prompt("Update?"), ms(0));
        p.enabled = false;
        let (mut h, mut p) = settled_with(p);
        let out = run(&mut h, &mut p, 1100);
        let toast = |n: u64| id("toasts").with(n);
        // A flyout has no node of its own: its rows, state and routes are the roots.
        let fly = |role: Role, name: &str| by_name(&out, role, name).id;
        for (name, root) in [
            ("rail", id("rail")),
            ("nav", id("nav")),
            ("tree", id("tree")),
            ("splitter", id("split")),
            ("sidebar", id("sb").with("steps")),
            ("page", id("page")),
            ("side panel", id("sp")),
            ("settings page", id("settings")),
            ("menu", id("menu")),
            ("flyout rows", fly(Role::Menu, "Tray")),
            ("flyout state", fly(Role::Status, "Serving")),
            ("flyout routes", fly(Role::Group, "Opus")),
            ("error toast", toast(1)),
            ("prompt toast", toast(2)),
        ] {
            assert!(out.tree.get(root).is_some(), "{name}: no node");
            assert_eq!(disabled_violations(&out.tree, root), [], "{name}");
        }
        // And the palette, which is modal.
        let mut p = Page::new();
        p.palette_open = true;
        p.query = "r".into();
        p.enabled = false;
        let (mut h, mut p) = settled_with(p);
        let out = run(&mut h, &mut p, 1100);
        assert_eq!(disabled_violations(&out.tree, id("palette")), [], "palette");
    }

    /// A page with a Global binding on every key of `KEYS`, the `stop`th composite focused by Tab
    /// (the rail, the nav, the tree, the splitter), and the time it is at.
    fn focused_page(stop: usize) -> (UiHarness, Page, u64) {
        let mut h = UiHarness::new(M7_WIN);
        for k in KEYS {
            h.shortcuts.add(Binding::new(
                Scope::Global,
                Chord::key(Key::Named(k)),
                format!("g-{k:?}"),
            ));
        }
        let mut p = Page::new();
        for at in [0, 500, 1000] {
            run(&mut h, &mut p, at);
        }
        let mut ms = 1000;
        for _ in 0..=stop {
            key(&mut h, NamedKey::Tab);
            ms += 100;
            run(&mut h, &mut p, ms);
        }
        (h, p, ms)
    }

    #[test]
    fn a_disabled_page_takes_every_key_a_focused_composite_takes_and_fires_no_global_binding() {
        let names = ["rail", "nav", "tree", "splitter"];
        for (stop, name) in names.into_iter().enumerate() {
            // The instrument: the keys the composite uses are the ones that fire nothing while it
            // is enabled; the rest stay shortcuts.
            let mut used = Vec::new();
            for k in KEYS {
                let (mut h, mut p, ms) = focused_page(stop);
                key(&mut h, k);
                if run(&mut h, &mut p, ms + 100).fired.is_empty() {
                    used.push(k);
                }
            }
            assert!(
                used.len() >= 4,
                "{name} uses at least four of the keys: {used:?}"
            );
            for k in used {
                let (mut h, mut p, mut ms) = focused_page(stop);
                p.enabled = false;
                let before = (p.nav, p.split_size);
                key(&mut h, k);
                ms += 100;
                let out = run(&mut h, &mut p, ms);
                assert!(
                    out.fired.is_empty(),
                    "{name}: {k:?} reached {:?}",
                    out.fired
                );
                assert!(p.rail_picked.is_empty(), "{name}: {k:?}");
                assert!(
                    p.tree_toggled.is_empty() && p.tree_activated.is_empty(),
                    "{name}: {k:?}"
                );
                assert_eq!((p.nav, p.split_size), before, "{name}: {k:?}");
            }
        }
    }

    #[test]
    fn a_page_of_them_all_at_rest_reports_idle_once_the_loops_are_off() {
        let mut p = Page::new();
        // A toast with no timeout and no loop: an error.
        p.toasts
            .push(ToastSpec::new(ToastKind::Error, "Build failed"), ms(0));
        let (mut h, mut p) = settled_with(p);
        for at in [1100, 1200, 5000] {
            assert_eq!(run(&mut h, &mut p, at).repaint, Repaint::Idle, "{at}");
        }
        // With a menu, a flyout and the palette open too.
        let mut q = Page::new();
        q.menu_open = true;
        q.flyout_open = true;
        let (mut h, mut q) = settled_with(q);
        assert_eq!(run(&mut h, &mut q, 1100).repaint, Repaint::Idle, "overlays");
        let mut r = Page::new();
        r.palette_open = true;
        let (mut h, mut r) = settled_with(r);
        assert_eq!(run(&mut h, &mut r, 1100).repaint, Repaint::Idle, "palette");
    }

    // A dialog holding a combo, with a toast up and a tooltip above: the overlays that open over
    // the dialog rise above it (C4's Raised layer), the toast takes only its own presses, and Tab
    // and Escape go to the top.

    const DIALOG: Dialog<'static> = Dialog {
        title: "Fonts",
        message: "Pick a font.",
        extra_h: 80.0,
        confirm: "Apply",
        cancel: Cancel::Default,
    };

    #[derive(Debug)]
    struct Bed {
        combo_open: bool,
        dialog: bool,
        cancelled: u32,
        dismissed: u32,
        toast_actions: Vec<u64>,
        toasts: Toasts,
        /// The dialog body's origin, where the combo and the info button sit.
        origin: (f32, f32),
    }

    const COMBO_AT: (f32, f32, f32, f32) = (0.0, 0.0, 180.0, 32.0);
    const INFO_AT: (f32, f32, f32, f32) = (200.0, 0.0, 90.0, 32.0);

    impl Bed {
        fn rect(&self, (dx, dy, w, h): (f32, f32, f32, f32)) -> Rect {
            Rect::new(self.origin.0 + dx, self.origin.1 + dy, w, h)
        }
    }

    fn bed_frame(h: &mut UiHarness, b: &mut Bed, at: u64) -> Frame {
        h.at_ms(at);
        h.frame(|ui: &mut Ui<'_>| {
            ui.painter().on(LayerId::Page);
            ui.button("page-button", PAGE_BTN, "Page", ButtonKind::Secondary);
            if b.dialog {
                let r = ui.dialog("dialog", &DIALOG, &glass(), |ui: &mut Ui<'_>, x, y, _| {
                    b.origin = (x, y);
                    let combo = ui.combo(
                        "combo",
                        Rect::new(x, y, 180.0, 32.0),
                        "RznSans",
                        &mut b.combo_open,
                        "Font",
                    );
                    let info = ui.button(
                        "info",
                        Rect::new(x + 200.0, y, 90.0, 32.0),
                        "Info",
                        ButtonKind::Secondary,
                    );
                    ui.tooltip(&info, "More about fonts");
                    if b.combo_open {
                        let d = ui.dropdown("dropdown", combo.rect, combo.id, &OPTIONS, 0);
                        if d.picked.is_some() || d.dismissed {
                            b.combo_open = false;
                        }
                        b.dismissed += u32::from(d.dismissed);
                    }
                });
                if r.cancelled {
                    b.cancelled += 1;
                    b.dialog = false;
                }
            }
            let t = ui.toasts("toasts", &mut b.toasts);
            b.toast_actions.extend(t.action.map(|(_, a)| a));
        })
    }

    /// The dialog open and settled with a prompt toast up and the combo's list open over it.
    fn open_bed() -> (UiHarness, Bed) {
        let mut h = UiHarness::new(WIN);
        let mut b = Bed {
            combo_open: false,
            dialog: true,
            cancelled: 0,
            dismissed: 0,
            toast_actions: Vec::new(),
            toasts: Toasts::default(),
            origin: (0.0, 0.0),
        };
        b.toasts.push(prompt("Update?"), ms(0));
        for at in [0, 300, 600, 900, 1200] {
            bed_frame(&mut h, &mut b, at);
        }
        let (x, y) = centre(b.rect(COMBO_AT));
        h.input.pointer_to(x, y);
        bed_frame(&mut h, &mut b, 1300);
        h.input.click(P);
        bed_frame(&mut h, &mut b, 1310);
        let mut last = bed_frame(&mut h, &mut b, 1400);
        for at in [1700, 2000] {
            last = bed_frame(&mut h, &mut b, at);
        }
        assert!(b.combo_open, "the click opened the list");
        let raised = &last.0.merged(LayerId::Raised);
        assert!(
            !raised.geometry.under.is_empty() || !raised.texts.is_empty(),
            "the list is drawn on the raised layer, over the dialog"
        );
        (h, b)
    }

    fn name_of(out: &FrameOutput) -> Option<String> {
        out.focused
            .and_then(|f| out.tree.get(f))
            .map(|n| n.name.clone())
    }

    #[test]
    fn a_press_on_the_toast_works_over_the_open_list_and_a_press_outside_closes_only_the_list() {
        let (mut h, mut b) = open_bed();
        let (_, out) = bed_frame(&mut h, &mut b, 2100);
        let open = by_name(&out, Role::Button, "Open").bounds;
        let (x, y) = centre(open);
        h.input.pointer_to(x, y);
        bed_frame(&mut h, &mut b, 2200);
        h.input.click(P);
        bed_frame(&mut h, &mut b, 2210);
        bed_frame(&mut h, &mut b, 2300);
        assert_eq!(b.toast_actions, [2], "the toast's action");
        assert!(
            b.combo_open,
            "a press on a toast is no outside press for the list"
        );
        assert!(b.dialog && b.cancelled == 0 && b.dismissed == 0);

        // A press on the page, outside the card and the list: the list closes, the dialog stays.
        h.input.pointer_to(5.0, 5.0);
        bed_frame(&mut h, &mut b, 2400);
        h.input.click(P);
        bed_frame(&mut h, &mut b, 2410);
        bed_frame(&mut h, &mut b, 2500);
        assert_eq!(b.dismissed, 1, "the list closed");
        assert!(!b.combo_open);
        assert!(b.dialog && b.cancelled == 0, "the dialog stays");
    }

    #[test]
    fn tab_over_the_open_list_never_reaches_the_toast_or_the_page() {
        let (mut h, mut b) = open_bed();
        let mut at = 2100;
        for i in 0..12 {
            at += 100;
            if i % 4 == 3 {
                shift_tab(&mut h);
            } else {
                key(&mut h, NamedKey::Tab);
            }
            let (_, out) = bed_frame(&mut h, &mut b, at);
            let name = name_of(&out);
            assert!(
                !matches!(name.as_deref(), Some("Page" | "Open" | "Later" | "Dismiss")),
                "press {i}: {name:?}"
            );
        }
        assert!(b.dialog && b.cancelled == 0);
    }

    #[test]
    fn escape_closes_the_tooltip_then_the_list_then_the_dialog() {
        let (mut h, mut b) = open_bed();
        // Rest on the info button until its tooltip shows (above the list, on the raised layer).
        let (x, y) = centre(b.rect(INFO_AT));
        h.input.pointer_to(x, y);
        let mut shown = false;
        for at in [2100, 2500, 3000, 3200] {
            let (_, out) = bed_frame(&mut h, &mut b, at);
            shown = out.tree.nodes.iter().any(|n| n.role == Role::Tooltip);
        }
        assert!(shown, "the tooltip shows after its delay");

        let mut at = 3300;
        let mut esc = |h: &mut UiHarness, b: &mut Bed| {
            key(h, NamedKey::Escape);
            at += 100;
            let (_, out) = bed_frame(h, b, at);
            at += 100;
            bed_frame(h, b, at);
            out
        };
        esc(&mut h, &mut b);
        let (_, out) = bed_frame(&mut h, &mut b, 3600);
        assert!(
            out.tree.nodes.iter().all(|n| n.role != Role::Tooltip),
            "the tooltip first"
        );
        assert!(b.combo_open && b.cancelled == 0 && b.dismissed == 0);

        esc(&mut h, &mut b);
        assert_eq!((b.dismissed, b.cancelled), (1, 0), "then the list");
        assert!(!b.combo_open);

        esc(&mut h, &mut b);
        assert_eq!(b.cancelled, 1, "then the dialog");
    }
}

/// M8's data views together: the table, the bar and pie charts, a lane graph row and the kanban
/// board in one frame, with the table and the board scripted through their composites.
mod m8 {
    use super::*;

    use lune_ui_core::GlassLayer;
    use lune_ui_core::access::Sort;
    use lune_ui_core::shortcuts::{Binding, Chord, Scope};
    use lune_ui_core::testing::disabled_violations;
    use lune_ui_core::widgets::bar_chart::{BarChartOptions, BarColumn, BarSeries};
    use lune_ui_core::widgets::kanban_board::{
        BoardOptions, CardMove, KanbanCard, KanbanColumn, landed_index,
    };
    use lune_ui_core::widgets::lane_graph::{LaneLine, LaneRow};
    use lune_ui_core::widgets::pie_chart::{PieOptions, PieSlice};
    use lune_ui_core::widgets::table::{Cell, ColumnWidth, SortKey, TableColumn, TableOptions};
    use lune_ui_core::widgets::virtual_list::{RowKey, SelectionMode};

    const M8_WIN: (f32, f32) = (1240.0, 900.0);
    const TABLE: Rect = Rect::new(10.0, 10.0, 500.0, 200.0);
    const BAR: Rect = Rect::new(10.0, 230.0, 500.0, 0.0);
    const PIE: Rect = Rect::new(10.0, 470.0, 160.0, 160.0);
    const LANE: Rect = Rect::new(240.0, 480.0, 65.0, 45.0);
    const BOARD: Rect = Rect::new(530.0, 10.0, 700.0, 500.0);
    const GLASS: GlassLayer = GlassLayer {
        tint: [10, 20, 30, 255],
        tint_opacity: 0.8,
        material_opacity: 0.7,
        curve: lune_ui_core::LuminosityCurve::AVALONIA,
    };

    const ROUTE: u64 = 1;
    const STATUS: u64 = 2;
    const REQUESTS: u64 = 3;
    const COLUMNS: [TableColumn<'static>; 3] = [
        TableColumn {
            key: ROUTE,
            title: "Route",
            width: ColumnWidth::Fill(1.0),
            num: false,
            sortable: true,
        },
        TableColumn {
            key: STATUS,
            title: "Status",
            width: ColumnWidth::Fixed(90.0),
            num: false,
            sortable: false,
        },
        TableColumn {
            key: REQUESTS,
            title: "Requests",
            width: ColumnWidth::Fixed(110.0),
            num: true,
            sortable: true,
        },
    ];

    /// The keys a focused composite uses.
    const KEYS: [NamedKey; 10] = [
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::Home,
        NamedKey::End,
        NamedKey::PageUp,
        NamedKey::PageDown,
        NamedKey::Enter,
        NamedKey::Space,
    ];

    /// The app side of the page: the board's model, and what the frames reported.
    #[derive(Debug, Clone)]
    struct Page {
        enabled: bool,
        cols: Vec<(u64, Vec<u64>)>,
        // Reported.
        sorted: Vec<SortKey>,
        selected: Vec<u64>,
        moved: Vec<CardMove>,
        opened: Vec<u64>,
        context: Vec<u64>,
        added: u32,
    }

    impl Page {
        fn new() -> Self {
            Page {
                enabled: true,
                cols: vec![(1, vec![11, 12, 13]), (2, vec![21, 22]), (3, vec![])],
                sorted: Vec::new(),
                selected: Vec::new(),
                moved: Vec::new(),
                opened: Vec::new(),
                context: Vec::new(),
                added: 0,
            }
        }

        /// Everything the keys of a composite could change.
        fn state(&self) -> impl PartialEq + std::fmt::Debug + use<> {
            (
                self.sorted.clone(),
                self.selected.clone(),
                self.moved.clone(),
                self.opened.clone(),
                self.context.clone(),
                self.added,
                self.cols.clone(),
            )
        }
    }

    fn apply(cols: &mut [(u64, Vec<u64>)], mv: CardMove) {
        let (from, at) = cols
            .iter()
            .enumerate()
            .find_map(|(c, col)| Some((c, col.1.iter().position(|k| *k == mv.card)?)))
            .expect("the moved card");
        let to = cols
            .iter()
            .position(|c| c.0 == mv.column)
            .expect("the target column");
        let index = landed_index((from == to).then_some(at), mv.index);
        let card = cols[from].1.remove(at);
        cols[to].1.insert(index, card);
    }

    fn page_frame(h: &mut UiHarness, p: &mut Page) -> FrameOutput {
        let enabled = p.enabled;
        let mut moved = None;
        let (_, out) = h.frame(|ui: &mut Ui<'_>| {
            ui.painter().on(LayerId::Page);
            ui.with_enabled(enabled, |ui| {
                // The table: thirty rows, sortable on Route and Requests.
                let opts = TableOptions {
                    columns: &COLUMNS,
                    count: 30,
                    selection: SelectionMode::Multi,
                    wrap: false,
                    index_of: None,
                    label: "Routes",
                };
                let t = ui.table(
                    "table",
                    TABLE,
                    &opts,
                    |i| RowKey(i as u64),
                    |_, c| match c.column {
                        0 => Cell::text(format!("/v1/route/{}", c.key.0)),
                        1 => Cell::text("ok"),
                        _ => Cell::text(format!("{}", c.key.0 * 10)),
                    },
                );
                p.sorted.extend(t.sort);
                p.selected = ui
                    .table_selection("table")
                    .selection
                    .iter()
                    .map(|k| k.0)
                    .collect();

                // The bar chart: two series over four columns.
                let series = [
                    BarSeries {
                        name: "Cache read",
                        color: [90, 160, 250, 255],
                    },
                    BarSeries {
                        name: "Uncached",
                        color: [240, 170, 80, 255],
                    },
                ];
                let v = [[4.0, 2.0], [6.0, 1.0], [0.0, 0.0], [8.0, 2.0]];
                let columns: Vec<BarColumn<'_>> =
                    v.iter().map(|v| BarColumn { values: v, tip: "" }).collect();
                ui.bar_chart(
                    "bars",
                    BAR,
                    &BarChartOptions {
                        series: &series,
                        columns: &columns,
                        max: 10.0,
                        y_labels: ["10", "5", "0"],
                        x_labels: &[(0, "00"), (3, "03")],
                        label: "Hourly tokens",
                        description: "Four hours, two series",
                        live: false,
                    },
                );

                // The pie chart and one lane graph row.
                let slices = [
                    PieSlice {
                        key: 1,
                        value: 3.0,
                        color: [90, 160, 250, 255],
                    },
                    PieSlice {
                        key: 2,
                        value: 1.0,
                        color: [240, 170, 80, 255],
                    },
                ];
                ui.pie_chart(
                    "pie",
                    PIE,
                    &PieOptions {
                        slices: &slices,
                        label: "Storage",
                        description: "Cache 75 percent, other 25 percent",
                        live: false,
                    },
                );
                let line = [LaneLine {
                    lane: 0,
                    color: [90, 160, 250, 255],
                    above: true,
                    below: true,
                }];
                ui.lane_graph_row(
                    LANE,
                    &LaneRow {
                        dot_lane: 1,
                        dot_color: [240, 170, 80, 255],
                        lanes: &line,
                        edges: &[],
                    },
                );

                // The board over the model.
                let titles: Vec<(u64, String)> = p
                    .cols
                    .iter()
                    .flat_map(|c| c.1.iter().map(|k| (*k, format!("card {k}"))))
                    .collect();
                let cards: Vec<Vec<KanbanCard<'_>>> = p
                    .cols
                    .iter()
                    .map(|c| {
                        c.1.iter()
                            .map(|k| KanbanCard {
                                key: *k,
                                title: &titles.iter().find(|t| t.0 == *k).expect("a title").1,
                                tags: &[],
                                assignee: None,
                                note: false,
                            })
                            .collect()
                    })
                    .collect();
                let names: Vec<String> = p.cols.iter().map(|c| format!("col {}", c.0)).collect();
                let columns: Vec<KanbanColumn<'_>> = p
                    .cols
                    .iter()
                    .zip(&cards)
                    .zip(&names)
                    .map(|((c, cards), title)| KanbanColumn {
                        key: c.0,
                        title,
                        cards,
                    })
                    .collect();
                let b = ui.kanban_board(
                    "board",
                    BOARD,
                    &BoardOptions {
                        title: "Roadmap",
                        columns: &columns,
                        error: None,
                        glass: GLASS,
                        under: [0.05, 0.06, 0.07, 1.0],
                        name: "",
                    },
                );
                moved = b.moved;
                p.opened.extend(b.opened);
                p.context.extend(b.context.map(|c| c.0));
                p.added += u32::from(b.add_column) + u32::from(b.add_card.is_some());
            });
        });
        // The app applies a move after the frame that reported it.
        if let Some(mv) = moved {
            p.moved.push(mv);
            apply(&mut p.cols, mv);
        }
        out
    }

    fn run(h: &mut UiHarness, p: &mut Page, at: u64) -> FrameOutput {
        h.at_ms(at);
        page_frame(h, p)
    }

    fn settled_with(mut p: Page) -> (UiHarness, Page) {
        let mut h = UiHarness::new(M8_WIN);
        for at in [0, 500, 1000] {
            run(&mut h, &mut p, at);
        }
        (h, p)
    }

    fn by_name<'a>(out: &'a FrameOutput, role: Role, name: &str) -> &'a Node {
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == role && n.name == name)
            .unwrap_or_else(|| panic!("no {role:?} named {name:?}"))
    }

    fn inject(h: &mut UiHarness, target: WidgetId, action: Action) {
        h.input.access(AccessAction {
            target,
            action,
            value: None,
        });
    }

    fn table_id() -> WidgetId {
        id("table")
    }

    fn header(col: u64) -> WidgetId {
        table_id().with("head").with(col)
    }

    fn body() -> WidgetId {
        table_id().with("body")
    }

    fn card(k: u64) -> WidgetId {
        id("board").with(("card", k))
    }

    #[test]
    fn a_frame_of_every_m8_widget_yields_the_expected_roles_names_and_states() {
        let (mut h, mut p) = settled_with(Page::new());
        let out = run(&mut h, &mut p, 1100);

        // The table: a table node over a header row of column headers and the body's group of
        // rows. A sortable header reads Sort::None until it is the sort.
        let table = by_name(&out, Role::Table, "Routes");
        assert_eq!(table.id, table_id());
        let head = out.tree.get(table_id().with("head")).expect("header row");
        assert_eq!(head.role, Role::Row);
        let sorts: Vec<Option<Sort>> = head
            .children
            .iter()
            .map(|c| out.tree.get(*c).expect("a header").sort)
            .collect();
        assert_eq!(sorts, [Some(Sort::None), None, Some(Sort::None)]);
        let titles: Vec<&str> = head
            .children
            .iter()
            .map(|c| out.tree.get(*c).expect("a header").name.as_str())
            .collect();
        assert_eq!(titles, ["Route", "Status", "Requests"]);
        let group = out.tree.get(body()).expect("the body");
        assert_eq!(group.role, Role::Group);
        assert_eq!(group.name, "Routes");
        assert!(group.states.contains(State::Focusable));
        let rows = out
            .tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::Row && n.id != head.id)
            .count();
        assert!((6..30).contains(&rows), "only the window is built: {rows}");

        // The bar chart: an image with the app's name, its legend as static text.
        let bars = by_name(&out, Role::Image, "Hourly tokens");
        assert_eq!(bars.description.as_deref(), Some("Four hours, two series"));
        for name in ["Cache read", "Uncached"] {
            by_name(&out, Role::StaticText, name);
        }
        // The pie chart: an image.
        let pie = by_name(&out, Role::Image, "Storage");
        assert_eq!(
            pie.description.as_deref(),
            Some("Cache 75 percent, other 25 percent")
        );
        // The lane graph row is decorative: nothing in its rect.
        assert!(
            !out.tree
                .nodes
                .iter()
                .any(|n| LANE.contains(centre(n.bounds))),
            "a lane graph row has no node"
        );

        // The board: a list named by its title, a group per column with its card count, a
        // list item per card; the buttons named.
        let board = by_name(&out, Role::List, "Roadmap");
        assert_eq!(board.id, id("board"));
        let col = by_name(&out, Role::Group, "col 1");
        assert_eq!(col.description.as_deref(), Some("3"));
        assert_eq!(
            by_name(&out, Role::Group, "col 3").description.as_deref(),
            Some("0")
        );
        let c = by_name(&out, Role::ListItem, "card 21");
        assert_eq!(c.id, card(21));
        assert!(c.states.contains(State::Focusable));
        for a in [Action::Default, Action::Focus, Action::ShowMenu] {
            assert!(c.actions.contains(a), "{a:?}");
        }
        by_name(&out, Role::Button, "Add column");
    }

    #[test]
    fn injected_actions_work_on_a_table_header_and_row_and_on_a_card() {
        let (mut h, mut p) = settled_with(Page::new());
        run(&mut h, &mut p, 1100);
        // A header sorts and a row selects, in one frame.
        inject(&mut h, header(REQUESTS), Action::Default);
        inject(
            &mut h,
            table_id().with("body").with(RowKey(4)),
            Action::Default,
        );
        let out = run(&mut h, &mut p, 1200);
        assert_eq!(
            p.sorted,
            [SortKey {
                column: REQUESTS,
                descending: false
            }]
        );
        assert_eq!(p.selected, [4]);
        assert_eq!(
            out.tree.get(header(REQUESTS)).expect("header").sort,
            Some(Sort::Ascending)
        );
        assert!(
            out.tree
                .get(table_id().with("body").with(RowKey(4)))
                .expect("row")
                .states
                .contains(State::Selected)
        );
        // A card opens by its default action and asks for its menu by ShowMenu.
        inject(&mut h, card(21), Action::Default);
        run(&mut h, &mut p, 1300);
        assert_eq!(p.opened, [21]);
        inject(&mut h, card(13), Action::ShowMenu);
        run(&mut h, &mut p, 1400);
        assert_eq!(p.context, [13]);
        // Neither touched the other composite.
        assert_eq!(p.selected, [4]);
        assert!(p.moved.is_empty());
    }

    #[test]
    fn the_table_has_two_stops_and_the_board_one_for_all_its_cards() {
        let (mut h, mut p) = settled_with(Page::new());
        let mut ms = 1000;
        let mut seen: Vec<WidgetId> = Vec::new();
        for _ in 0..40 {
            key(&mut h, NamedKey::Tab);
            ms += 100;
            let out = run(&mut h, &mut p, ms);
            let f = out.focused.expect("Tab focuses something");
            if seen.contains(&f) {
                break;
            }
            seen.push(f);
        }
        let out = run(&mut h, &mut p, ms + 100);
        let of = |role: Role| {
            seen.iter()
                .filter(|f| out.tree.get(**f).is_some_and(|n| n.role == role))
                .count()
        };
        assert_eq!(of(Role::ColumnHeader), 1, "the header is one roving stop");
        assert!(seen.contains(&body()), "the body is a stop");
        assert_eq!(of(Role::ListItem), 1, "all the cards are one stop");
        // The table's two stops are in its rect; the cards' stop is in the board's.
        let inside = |outer: Rect| {
            seen.iter()
                .filter(|f| {
                    out.tree
                        .get(**f)
                        .is_some_and(|n| outer.contains(centre(n.bounds)))
                })
                .count()
        };
        assert_eq!(inside(TABLE), 2, "a table has two stops");
        // The board's plain buttons are stops of their own: the header's add, and an add and a
        // menu button for each of the three columns. (Their places are not checked: focusing a
        // button scrolls the board sideways.)
        assert_eq!(of(Role::Button), 1 + 2 * 3, "the board's buttons");
        assert_eq!(seen.len(), 2 + 1 + 7, "table, cards, buttons: nothing else");
    }

    #[test]
    fn every_m8_widget_in_a_disabled_scope_passes_the_walk() {
        let p = Page {
            enabled: false,
            ..Page::new()
        };
        let (mut h, mut p) = settled_with(p);
        let out = run(&mut h, &mut p, 1100);
        for (name, root) in [
            ("table", table_id()),
            ("bar chart", by_name(&out, Role::Image, "Hourly tokens").id),
            ("pie chart", by_name(&out, Role::Image, "Storage").id),
            ("board", id("board")),
        ] {
            assert!(out.tree.get(root).is_some(), "{name}: no node");
            assert_eq!(disabled_violations(&out.tree, root), [], "{name}");
        }
    }

    /// The first Tab stop with this role, focused, on a page with a Global binding on every key
    /// of `KEYS`; and the time.
    fn focused_page(role: Role) -> (UiHarness, Page, u64) {
        focused_page_where(|n| n.role == role)
    }

    fn focused_page_where(want: impl Fn(&Node) -> bool) -> (UiHarness, Page, u64) {
        let mut h = UiHarness::new(M8_WIN);
        for k in KEYS {
            h.shortcuts.add(Binding::new(
                Scope::Global,
                Chord::key(Key::Named(k)),
                format!("g-{k:?}"),
            ));
        }
        let mut p = Page::new();
        for at in [0, 500, 1000] {
            run(&mut h, &mut p, at);
        }
        let mut ms = 1000;
        for _ in 0..12 {
            key(&mut h, NamedKey::Tab);
            ms += 100;
            let out = run(&mut h, &mut p, ms);
            if out.focused.and_then(|f| out.tree.get(f)).is_some_and(&want) {
                return (h, p, ms);
            }
        }
        panic!("no Tab stop of that kind");
    }

    #[test]
    fn a_disabled_page_takes_every_key_a_focused_composite_takes_and_fires_no_global_binding() {
        // The table's header, its body (a group) and the board's cards.
        for (role, name) in [
            (Role::ColumnHeader, "table header"),
            (Role::Group, "table body"),
            (Role::ListItem, "board cards"),
        ] {
            // The instrument: the keys the composite uses are the ones that fire nothing while it
            // is enabled; the rest stay shortcuts.
            let mut used = Vec::new();
            for k in KEYS {
                let (mut h, mut p, ms) = focused_page(role);
                key(&mut h, k);
                if run(&mut h, &mut p, ms + 100).fired.is_empty() {
                    used.push(k);
                }
            }
            assert!(
                used.len() >= 4,
                "{name} uses at least four of the keys: {used:?}"
            );
            for k in used {
                let (mut h, mut p, mut ms) = focused_page(role);
                p.enabled = false;
                let before = p.state();
                key(&mut h, k);
                ms += 100;
                let out = run(&mut h, &mut p, ms);
                assert!(
                    out.fired.is_empty(),
                    "{name}: {k:?} reached {:?}",
                    out.fired
                );
                assert_eq!(p.state(), before, "{name}: {k:?} changed the page");
            }
        }
    }

    #[test]
    fn a_page_of_them_all_at_rest_reports_idle() {
        let (mut h, mut p) = settled_with(Page::new());
        for at in [1100, 1200, 5000] {
            assert_eq!(run(&mut h, &mut p, at).repaint, Repaint::Idle, "{at}");
        }
        // After a sort, a selection and a card opened too.
        inject(&mut h, header(ROUTE), Action::Default);
        inject(
            &mut h,
            table_id().with("body").with(RowKey(2)),
            Action::Default,
        );
        run(&mut h, &mut p, 5100);
        for at in [5600, 6100, 9000] {
            assert_eq!(
                run(&mut h, &mut p, at).repaint,
                Repaint::Idle,
                "after: {at}"
            );
        }
    }

    #[test]
    fn a_shift_arrow_moves_a_card_and_the_model_lands_it_where_the_report_said() {
        let (mut h, mut p, mut ms) = focused_page(Role::ListItem);
        ms += 50;
        assert_eq!(
            run(&mut h, &mut p, ms).focused,
            Some(card(11)),
            "the cards' stop is the first card"
        );
        h.input.set_modifiers(Modifiers {
            shift: true,
            ..Modifiers::default()
        });
        key(&mut h, NamedKey::ArrowDown);
        h.input.set_modifiers(Modifiers::default());
        ms += 100;
        run(&mut h, &mut p, ms);
        assert_eq!(
            p.moved,
            [CardMove {
                card: 11,
                column: 1,
                index: 2
            }]
        );
        assert_eq!(p.cols[0].1, [12, 11, 13]);
        // The table's own state is untouched.
        assert!(p.sorted.is_empty() && p.selected.is_empty());
    }

    #[test]
    fn the_boards_buttons_focused_then_disabled_keep_enter_and_space_from_the_shortcuts() {
        for name in ["Add column", "Add card", "Column actions"] {
            for k in [NamedKey::Enter, NamedKey::Space] {
                // The instrument: enabled, the key presses the button and fires nothing.
                let (mut h, mut p, ms) = focused_page_where(|n| n.name == name);
                key(&mut h, k);
                let out = run(&mut h, &mut p, ms + 100);
                assert!(out.fired.is_empty(), "{name}: {k:?} enabled");
                if name != "Column actions" {
                    assert_eq!(p.added, 1, "{name}: {k:?} presses it enabled");
                }
                // Disabled, the same key is taken and presses nothing.
                let (mut h, mut p, ms) = focused_page_where(|n| n.name == name);
                p.enabled = false;
                key(&mut h, k);
                let out = run(&mut h, &mut p, ms + 100);
                assert!(
                    out.fired.is_empty(),
                    "{name}: {k:?} reached {:?}",
                    out.fired
                );
                assert_eq!(p.added, 0, "{name}: {k:?} pressed it disabled");
            }
        }
    }
}

mod m9 {
    use super::*;

    use lune_ui_core::access::AccessValue;
    use lune_ui_core::code_doc::CodeDoc;
    use lune_ui_core::icon;
    use lune_ui_core::shortcuts::{Binding, Chord, Scope};
    use lune_ui_core::testing::disabled_violations;
    use lune_ui_core::widgets::code_block::CodeBlockOptions;
    use lune_ui_core::widgets::code_editor::CodeOptions;
    use lune_ui_core::widgets::diff_view::{
        DiffLine, DiffMode, DiffOptions, HunkAction, LineKind, pair_rows,
    };
    use lune_ui_core::widgets::markdown::{MdBlock, MdInline, MdItem, MdOptions, MdTask};

    const M9_WIN: (f32, f32) = (1240.0, 900.0);
    const BLOCK: Rect = Rect::new(10.0, 10.0, 480.0, 62.0);
    const EDITOR: Rect = Rect::new(510.0, 10.0, 500.0, 200.0);
    const DOC: Rect = Rect::new(10.0, 230.0, 480.0, 300.0);
    const DIFF: Rect = Rect::new(510.0, 230.0, 700.0, 300.0);

    const SNIPPET: &str = "let a = 1;\nlet b = 2;\nlet c = a + b;";
    const HUNK: [HunkAction<'static>; 1] = [HunkAction {
        glyph: icon::ADD,
        label: "Stage hunk",
    }];

    /// A key a focused composite uses: a named key, or Command with a letter.
    #[derive(Debug, Clone, Copy)]
    enum Probe {
        Named(NamedKey),
        Cmd(&'static str),
    }

    const KEYS: [Probe; 12] = [
        Probe::Named(NamedKey::ArrowUp),
        Probe::Named(NamedKey::ArrowDown),
        Probe::Named(NamedKey::ArrowLeft),
        Probe::Named(NamedKey::ArrowRight),
        Probe::Named(NamedKey::Home),
        Probe::Named(NamedKey::End),
        Probe::Named(NamedKey::PageUp),
        Probe::Named(NamedKey::PageDown),
        Probe::Named(NamedKey::Enter),
        Probe::Named(NamedKey::Space),
        Probe::Cmd("a"),
        Probe::Cmd("c"),
    ];

    fn press(h: &mut UiHarness, p: Probe) {
        match p {
            Probe::Named(k) => key(h, k),
            Probe::Cmd(c) => {
                h.input.set_modifiers(Modifiers {
                    logo: true,
                    ..Modifiers::default()
                });
                h.input.key_down(Key::Character(c.to_owned()), None);
                h.input.set_modifiers(Modifiers::default());
            }
        }
    }

    fn binding(p: Probe) -> Binding {
        match p {
            Probe::Named(k) => {
                Binding::new(Scope::Global, Chord::key(Key::Named(k)), format!("g-{k:?}"))
            }
            Probe::Cmd(c) => Binding::new(
                Scope::Global,
                Chord::key(Key::Character(c.to_owned())).primary(),
                format!("g-cmd-{c}"),
            ),
        }
    }

    /// The app side of the page: the editor's document, and what the frames reported.
    #[derive(Debug, Clone)]
    struct Page {
        enabled: bool,
        /// The diff side by side.
        split: bool,
        doc: CodeDoc,
        // Reported.
        copied: u32,
        changed: u32,
        links: Vec<String>,
        tasks: Vec<u64>,
        hunks: Vec<(u32, usize)>,
    }

    impl Page {
        fn new() -> Self {
            Page {
                enabled: true,
                split: false,
                doc: CodeDoc::new("one\ntwo\nthree"),
                copied: 0,
                changed: 0,
                links: Vec::new(),
                tasks: Vec::new(),
                hunks: Vec::new(),
            }
        }

        /// Everything the keys of a composite could change.
        fn state(&self) -> impl PartialEq + std::fmt::Debug + use<> {
            (
                self.doc.text().to_owned(),
                self.copied,
                self.changed,
                self.links.clone(),
                self.tasks.clone(),
                self.hunks.clone(),
            )
        }
    }

    fn text(s: &str) -> MdInline<'_> {
        MdInline::Text {
            text: s,
            bold: false,
            italic: false,
            strike: false,
        }
    }

    fn page_frame(h: &mut UiHarness, p: &mut Page) -> FrameOutput {
        let (enabled, split) = (p.enabled, p.split);
        let (_, out) = h.frame(|ui: &mut Ui<'_>| {
            ui.painter().on(LayerId::Page);
            ui.with_enabled(enabled, |ui| {
                let b = ui.code_block(
                    "block",
                    BLOCK,
                    SNIPPET,
                    &CodeBlockOptions {
                        label: "Snippet",
                        ..CodeBlockOptions::default()
                    },
                );
                p.copied += u32::from(b.copied);

                let e = ui.code_editor(
                    "editor",
                    EDITOR,
                    &mut p.doc,
                    &CodeOptions {
                        label: "Notes",
                        ..CodeOptions::default()
                    },
                );
                p.changed += u32::from(e.changed);

                let title = [text("Guide")];
                let para = [
                    text("See the "),
                    MdInline::Link {
                        label: "docs",
                        url: "docs/a.md",
                    },
                ];
                let step = [text("ship it")];
                let step_blocks = [MdBlock::Paragraph(&step)];
                let items = [MdItem {
                    marker: "-",
                    blocks: &step_blocks,
                    task: Some(MdTask {
                        checked: false,
                        id: 7,
                    }),
                }];
                let blocks = [
                    MdBlock::Heading {
                        level: 2,
                        inlines: &title,
                    },
                    MdBlock::Paragraph(&para),
                    MdBlock::List { items: &items },
                    MdBlock::Code {
                        text: "let x = 1;",
                        spans: &[],
                    },
                ];
                let m = ui.markdown(
                    "md",
                    DOC,
                    &blocks,
                    &MdOptions {
                        label: "Guide",
                        tasks_live: true,
                        identity: None,
                    },
                );
                p.links.extend(m.link);
                p.tasks.extend(m.task);

                let lines = [
                    DiffLine {
                        hunk: Some(0),
                        ..DiffLine::new(LineKind::Hunk, "@@ -1,3 +1,3 @@")
                    },
                    DiffLine {
                        old: Some(1),
                        new: Some(1),
                        ..DiffLine::new(LineKind::Context, "alpha")
                    },
                    DiffLine {
                        old: Some(2),
                        ..DiffLine::new(LineKind::Removed, "beta old")
                    },
                    DiffLine {
                        new: Some(2),
                        ..DiffLine::new(LineKind::Added, "beta new")
                    },
                ];
                let rows = pair_rows(&lines);
                let d = ui.diff_view(
                    "diff",
                    DIFF,
                    &lines,
                    &DiffOptions {
                        mode: if split {
                            DiffMode::Split(&rows)
                        } else {
                            DiffMode::Unified
                        },
                        wrap: false,
                        label: "Changes",
                        actions: &HUNK,
                        identity: 1,
                        widest_chars: 0,
                    },
                );
                p.hunks.extend(d.hunk);
            });
        });
        out
    }

    fn run(h: &mut UiHarness, p: &mut Page, at: u64) -> FrameOutput {
        h.at_ms(at);
        page_frame(h, p)
    }

    fn settled_with(mut p: Page) -> (UiHarness, Page) {
        let mut h = UiHarness::new(M9_WIN);
        for at in [0, 500, 1000] {
            run(&mut h, &mut p, at);
        }
        (h, p)
    }

    fn by_name<'a>(out: &'a FrameOutput, role: Role, name: &str) -> &'a Node {
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == role && n.name == name)
            .unwrap_or_else(|| panic!("no {role:?} named {name:?}"))
    }

    fn inject(h: &mut UiHarness, target: WidgetId, action: Action, value: Option<AccessValue>) {
        h.input.access(AccessAction {
            target,
            action,
            value,
        });
    }

    /// The unified pane's node (a split diff has `l` and `r`).
    fn diff_pane() -> WidgetId {
        id("diff").with("u")
    }

    fn block_copy() -> WidgetId {
        id("block").with("copy")
    }

    #[test]
    fn a_frame_of_every_m9_widget_yields_the_expected_roles_names_and_states() {
        let (mut h, mut p) = settled_with(Page::new());
        let out = run(&mut h, &mut p, 1100);

        // The code block: read-only multiline text under its label, and a named copy button.
        let body = by_name(&out, Role::MultilineText, "Snippet");
        assert_eq!(body.id, id("block").with("body"));
        assert!(body.states.contains(State::ReadOnly));
        assert!(!body.actions.contains(Action::SetValue));
        let copy = out.tree.get(block_copy()).expect("the copy button");
        assert_eq!((copy.role, copy.name.as_str()), (Role::Button, "Copy code"));
        assert!(copy.actions.contains(Action::Default));

        // The editor: editable multiline text carrying its lines, with a set-value action.
        let ed = by_name(&out, Role::MultilineText, "Notes");
        assert_eq!(ed.id, id("editor"));
        assert!(!ed.states.contains(State::ReadOnly));
        assert!(ed.states.contains(State::Focusable));
        assert!(ed.actions.contains(Action::SetValue));
        assert_eq!(ed.value, Some(Value::Text("one\ntwo\nthree".to_owned())));

        // The document: a group under its label; the heading is static text with its level, the
        // link carries its URL, the task box is a checkbox named by its text, the fenced code is a
        // block of its own.
        let doc = by_name(&out, Role::Group, "Guide");
        assert_eq!(doc.id, id("md"));
        let heading = by_name(&out, Role::StaticText, "Guide");
        assert_eq!(heading.level, Some(2));
        let link = by_name(&out, Role::Link, "docs");
        assert_eq!(link.description.as_deref(), Some("docs/a.md"));
        assert!(link.states.contains(State::Focusable));
        assert!(link.actions.contains(Action::Default));
        let task = by_name(&out, Role::Checkbox, "ship it");
        assert!(!task.states.contains(State::Checked));
        assert!(task.actions.contains(Action::Default));
        assert_eq!(
            out.tree
                .nodes
                .iter()
                .filter(|n| n.role == Role::MultilineText && n.name.is_empty())
                .count(),
            1,
            "the fenced code is one more block"
        );

        // The diff: a table of rows with cells, and the hunk's action a named button.
        let diff = by_name(&out, Role::Table, "Changes");
        assert_eq!(diff.id, diff_pane());
        let rows: Vec<&Node> = out
            .tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::Row && diff.children.contains(&n.id))
            .collect();
        assert_eq!(rows.len(), 4, "a row per line");
        assert!(rows.iter().all(|r| !r.children.is_empty()), "with cells");
        let stage = by_name(&out, Role::Button, "Stage hunk");
        assert!(stage.actions.contains(Action::Default));
    }

    #[test]
    fn injected_actions_work_on_a_link_a_task_box_a_hunk_action_the_copy_button_and_a_set_value() {
        let (mut h, mut p) = settled_with(Page::new());
        let out = run(&mut h, &mut p, 1100);
        let link = by_name(&out, Role::Link, "docs").id;
        let task = by_name(&out, Role::Checkbox, "ship it").id;
        let stage = by_name(&out, Role::Button, "Stage hunk").id;

        inject(&mut h, link, Action::Default, None);
        run(&mut h, &mut p, 1200);
        assert_eq!(p.links, ["docs/a.md"]);
        inject(&mut h, task, Action::Default, None);
        run(&mut h, &mut p, 1300);
        assert_eq!(p.tasks, [7]);
        inject(&mut h, stage, Action::Default, None);
        run(&mut h, &mut p, 1400);
        assert_eq!(p.hunks, [(0, 0)]);
        inject(&mut h, block_copy(), Action::Default, None);
        let out = run(&mut h, &mut p, 1500);
        assert_eq!(p.copied, 1);
        assert!(
            out.tree
                .nodes
                .iter()
                .any(|n| n.role == Role::Status && n.live.is_some()),
            "the copy says so politely"
        );
        let new = Some(AccessValue::Text("a\r\nb".to_owned()));
        inject(&mut h, id("editor"), Action::SetValue, new);
        run(&mut h, &mut p, 1600);
        assert_eq!(p.doc.text(), "a\nb");
        assert_eq!(p.changed, 1);
        // Each acted once: none touched another's report.
        assert_eq!((p.links.len(), p.tasks.len(), p.hunks.len()), (1, 1, 1));
    }

    #[test]
    fn a_tab_cycle_over_all_four_visits_the_stops_in_order_once() {
        let (mut h, mut p) = settled_with(Page::new());
        let mut ms = 1000;
        let mut seen: Vec<WidgetId> = Vec::new();
        for _ in 0..40 {
            key(&mut h, NamedKey::Tab);
            ms += 100;
            let out = run(&mut h, &mut p, ms);
            let f = out.focused.expect("Tab focuses something");
            if seen.contains(&f) {
                break;
            }
            seen.push(f);
        }
        let out = run(&mut h, &mut p, ms + 100);
        let roles: Vec<Role> = seen
            .iter()
            .map(|f| out.tree.get(*f).expect("a stop").role)
            .collect();
        // The block's text and copy button, the editor, the link, the task box, the fenced code's
        // copy button (its text is off the Tab order), the diff's body and its hunk action.
        assert_eq!(
            roles,
            [
                Role::MultilineText,
                Role::Button,
                Role::MultilineText,
                Role::Link,
                Role::Checkbox,
                Role::Button,
                Role::Table,
                Role::Button,
            ],
            "{seen:?}"
        );
        assert_eq!(seen[0], id("block").with("body"));
        assert_eq!(seen[1], block_copy());
        assert_eq!(seen[2], id("editor"));
        assert_eq!(seen[6], diff_pane());
    }

    #[test]
    fn every_m9_widget_in_a_disabled_scope_passes_the_walk() {
        let p = Page {
            enabled: false,
            ..Page::new()
        };
        let (mut h, mut p) = settled_with(p);
        let out = run(&mut h, &mut p, 1100);
        for (name, root) in [
            ("code block", id("block").with("body")),
            ("code block copy", block_copy()),
            ("editor", id("editor")),
            ("markdown", id("md")),
            ("diff", diff_pane()),
        ] {
            assert!(out.tree.get(root).is_some(), "{name}: no node");
            assert_eq!(disabled_violations(&out.tree, root), [], "{name}");
        }
        // The diff side by side: both panes and their hunk actions.
        let p = Page {
            enabled: false,
            split: true,
            ..Page::new()
        };
        let (mut h, mut p) = settled_with(p);
        let out = run(&mut h, &mut p, 1100);
        for side in ["l", "r"] {
            let root = id("diff").with(side);
            assert!(out.tree.get(root).is_some(), "pane {side}: no node");
            assert_eq!(disabled_violations(&out.tree, root), [], "pane {side}");
        }
    }

    /// The first Tab stop `want` accepts, focused, on a page with a Global binding on every key
    /// of `KEYS`; and the time.
    fn focused_page(split: bool, want: impl Fn(&Node) -> bool) -> (UiHarness, Page, u64) {
        let mut h = UiHarness::new(M9_WIN);
        for k in KEYS {
            h.shortcuts.add(binding(k));
        }
        let mut p = Page {
            split,
            ..Page::new()
        };
        for at in [0, 500, 1000] {
            run(&mut h, &mut p, at);
        }
        let mut ms = 1000;
        for _ in 0..12 {
            key(&mut h, NamedKey::Tab);
            ms += 100;
            let out = run(&mut h, &mut p, ms);
            if out.focused.and_then(|f| out.tree.get(f)).is_some_and(&want) {
                return (h, p, ms);
            }
        }
        panic!("no Tab stop of that kind");
    }

    #[test]
    fn a_disabled_page_takes_every_key_a_focused_composite_takes_and_fires_no_global_binding() {
        type Want = fn(&Node) -> bool;
        // What each takes at least: a read-only text only Select all (the area leaves the rest of
        // its keys to it while there is nothing to scroll), a button Enter and Space.
        let stops: [(&str, Want, usize, bool); 9] = [
            (
                "code block text",
                |n| n.id == id("block").with("body"),
                1,
                false,
            ),
            ("code block copy", |n| n.id == block_copy(), 2, false),
            ("editor", |n| n.id == id("editor"), 8, false),
            ("markdown link", |n| n.role == Role::Link, 2, false),
            ("markdown task box", |n| n.role == Role::Checkbox, 1, false),
            ("diff body", |n| n.id == diff_pane(), 4, false),
            ("diff left pane", |n| n.id == id("diff").with("l"), 4, true),
            ("diff right pane", |n| n.id == id("diff").with("r"), 4, true),
            ("diff hunk action", |n| n.name == "Stage hunk", 2, false),
        ];
        // Every leak is listed, so a mutant of one reader names the stops it reaches.
        let mut leaks: Vec<String> = Vec::new();
        for (name, want, least, split) in stops {
            // The instrument: the keys the composite uses are the ones that fire nothing while it
            // is enabled; the rest stay shortcuts.
            let mut used = Vec::new();
            for k in KEYS {
                let (mut h, mut p, ms) = focused_page(split, want);
                press(&mut h, k);
                if run(&mut h, &mut p, ms + 100).fired.is_empty() {
                    used.push(k);
                }
            }
            assert!(
                used.len() >= least,
                "{name} uses at least {least} of the keys: {used:?}"
            );
            for k in used {
                let (mut h, mut p, mut ms) = focused_page(split, want);
                p.enabled = false;
                let before = p.state();
                press(&mut h, k);
                ms += 100;
                let out = run(&mut h, &mut p, ms);
                if !out.fired.is_empty() {
                    leaks.push(format!("{name}: {k:?} reached {:?}", out.fired));
                }
                if p.state() != before {
                    leaks.push(format!("{name}: {k:?} changed the page"));
                }
            }
        }
        assert_eq!(leaks, Vec::<String>::new());
    }

    #[test]
    fn a_page_of_them_all_at_rest_reports_idle() {
        let (mut h, mut p) = settled_with(Page::new());
        for at in [1100, 1200, 5000] {
            assert_eq!(run(&mut h, &mut p, at).repaint, Repaint::Idle, "{at}");
        }
        // After a copy too, once its check mark has gone (1.5 s of app time).
        inject(&mut h, block_copy(), Action::Default, None);
        run(&mut h, &mut p, 5100);
        // The frame that ends the feedback fades the check mark out, then rests.
        run(&mut h, &mut p, 7000);
        for at in [7500, 8000, 9000] {
            assert_eq!(
                run(&mut h, &mut p, at).repaint,
                Repaint::Idle,
                "after: {at}"
            );
        }
    }
}

/// D176: a composite that nests an area and has nothing to scroll leaves the wheel to the area
/// around it; one that overflows keeps it, under the single-topmost rule. The board's own cases are
/// in `widget_kanban_board`.
mod nested_wheel {
    use std::cell::RefCell;
    use std::time::Duration;

    use super::*;

    use lune_ui_core::code_doc::CodeDoc;
    use lune_ui_core::widgets::code_block::CodeBlockOptions;
    use lune_ui_core::widgets::code_editor::CodeOptions;
    use lune_ui_core::widgets::diff_view::{DiffLine, DiffMode, DiffOptions, LineKind, pair_rows};
    use lune_ui_core::widgets::markdown::{MdBlock, MdInline, MdOptions};
    use lune_ui_core::widgets::scroll_area::{Bars, Extent, ScrollOptions};
    use lune_ui_core::widgets::tab_strip::{Tab, TabKey, TabStripOptions};
    use lune_ui_core::widgets::table::{Cell, ColumnWidth, TableColumn, TableOptions};
    use lune_ui_core::widgets::text_area::AreaOptions;
    use lune_ui_core::widgets::toast::{ToastKind, ToastSpec, Toasts};
    use lune_ui_core::widgets::tone::Tone;
    use lune_ui_core::widgets::tree_view::{ChevronStyle, GuideStyle, TreeOptions, TreeRow};
    use lune_ui_core::widgets::virtual_list::{Heights, ListOptions, RowKey, SelectionMode};

    /// F11: two non-modal overlays on one layer. The wheel over the part where the later one
    /// (which has nothing to scroll) covers the earlier one (which does) goes to neither: the
    /// topmost region is the later overlay's, and a region in an overlay opened before it is
    /// under it. Per axis; the earlier overlay still scrolls where nothing covers it.
    #[test]
    fn a_wheel_over_a_fit_popover_does_not_reach_a_list_in_the_popover_under_it() {
        use lune_ui_core::widgets::popover::PopoverOptions;
        let under_at = Rect::new(100.0, 100.0, 200.0, 200.0);
        let top_at = Rect::new(160.0, 160.0, 200.0, 200.0);
        let at = |r: Rect| PopoverOptions {
            gap: 0.0,
            margin: 0.0,
            ..PopoverOptions::new(Rect::new(r.x, r.y, 10.0, 0.0), (r.w, r.h))
        };
        // The offset of the earlier popover's area after a 60 px step at `point`.
        let run = |point: (f32, f32), dx: f32, dy: f32, top_open: bool| -> (f64, f64) {
            let mut offset = (0.0, 0.0);
            let mut h = UiHarness::new(WIN);
            let mut frame = |h: &mut UiHarness, ms: u64, top: bool| {
                h.at_ms(ms).frame(|ui: &mut Ui<'_>| {
                    ui.popover("under", &at(under_at), |ui, rect| {
                        let o = ScrollOptions {
                            extent: Some((900.0, 900.0)),
                            ..ScrollOptions::default()
                        };
                        let a = ui
                            .scroll_area("list", rect, &o, |_, _| ((), Extent::new(900.0, 900.0)));
                        offset = a.offset;
                    });
                    if top {
                        ui.popover("top", &at(top_at), |ui, rect| {
                            // A composite with nothing to scroll: no live axis, so no scroll sense.
                            let o = ScrollOptions {
                                bars: (Bars::Disabled, Bars::Disabled),
                                extent: Some((rect.w as f64, rect.h as f64)),
                                ..ScrollOptions::default()
                            };
                            ui.scroll_area("fit", rect, &o, |_, _| {
                                ((), Extent::new(rect.w as f64, rect.h as f64))
                            });
                        });
                    }
                });
            };
            frame(&mut h, 0, false);
            frame(&mut h, 400, false);
            if top_open {
                frame(&mut h, 500, true);
                frame(&mut h, 900, true);
            }
            h.input.pointer_to(point.0, point.1);
            frame(&mut h, 1000, top_open);
            h.input.scroll_pixels(dx, dy);
            frame(&mut h, 1010, top_open);
            frame(&mut h, 1400, top_open);
            offset
        };
        let over_both = (250.0, 250.0);
        let over_under_only = (120.0, 120.0);
        // Instrument: with the later popover closed, the earlier one scrolls on each axis.
        assert!(run(over_both, 0.0, -60.0, false).1 > 0.0, "y, alone");
        assert!(run(over_both, -60.0, 0.0, false).0 > 0.0, "x, alone");
        // Where the later popover covers it, neither axis reaches the earlier one.
        assert_eq!(run(over_both, 0.0, -60.0, true), (0.0, 0.0), "y, covered");
        assert_eq!(run(over_both, -60.0, 0.0, true), (0.0, 0.0), "x, covered");
        // And off the covered part it still scrolls, with the later popover open.
        assert!(
            run(over_under_only, 0.0, -60.0, true).1 > 0.0,
            "y, uncovered"
        );
        assert!(
            run(over_under_only, -60.0, 0.0, true).0 > 0.0,
            "x, uncovered"
        );
    }

    const INNER: Rect = Rect::new(40.0, 40.0, 400.0, 200.0);
    const FEW: usize = 2;
    const MANY: usize = 100;

    /// The page's offset after a 60 px wheel step over the middle of `at`, with `draw` in the page.
    fn page_after_a_wheel_step(at: Rect, draw: &mut dyn FnMut(&mut Ui<'_>)) -> f64 {
        let draw = RefCell::new(draw);
        let frame = |h: &mut UiHarness, ms: u64| {
            let mut off = 0.0;
            h.at_ms(ms).frame(|ui: &mut Ui<'_>| {
                let o = ScrollOptions {
                    extent: Some((900.0, 3000.0)),
                    ..ScrollOptions::default()
                };
                let rect = Rect::new(0.0, 0.0, 900.0, 700.0);
                let page = ui.scroll_area("page", rect, &o, |ui, _| {
                    (draw.borrow_mut())(ui);
                    ((), Extent::new(900.0, 3000.0))
                });
                off = page.offset.1;
            });
            off
        };
        let mut h = UiHarness::new(WIN);
        frame(&mut h, 0);
        h.input.pointer_to(at.x + at.w / 2.0, at.y + at.h / 2.0);
        frame(&mut h, 10);
        h.input.scroll_pixels(0.0, -60.0);
        frame(&mut h, 20);
        frame(&mut h, 400)
    }

    /// `draw(n)` is the composite with `n` things: few fit, many overflow.
    fn check(name: &str, at: Rect, draw: &dyn Fn(&mut Ui<'_>, usize)) {
        let fits = page_after_a_wheel_step(at, &mut |ui| draw(ui, FEW));
        assert_eq!(fits, 60.0, "{name}: nothing to scroll, the page moves");
        let overflows = page_after_a_wheel_step(at, &mut |ui| draw(ui, MANY));
        assert_eq!(overflows, 0.0, "{name}: it keeps the delta");
    }

    #[test]
    fn a_virtual_list() {
        check("list", INNER, &|ui, count| {
            let o = ListOptions {
                count,
                heights: Heights::Fixed(30.0),
                selection: SelectionMode::Single,
                index_of: None,
                label: "rows",
            };
            ui.virtual_list("l", INNER, &o, |i| RowKey(i as u64), |_, _| 30.0);
        });
    }

    #[test]
    fn a_table() {
        const COLUMNS: [TableColumn<'static>; 1] = [TableColumn {
            key: 1,
            title: "Route",
            width: ColumnWidth::Fill(1.0),
            num: false,
            sortable: false,
        }];
        check("table", INNER, &|ui, count| {
            let o = TableOptions {
                columns: &COLUMNS,
                count,
                selection: SelectionMode::Single,
                wrap: false,
                index_of: None,
                label: "Routes",
            };
            ui.table(
                "t",
                INNER,
                &o,
                |i| RowKey(i as u64),
                |_, _| Cell::text("ok"),
            );
        });
    }

    #[test]
    fn a_tree_view() {
        check("tree", INNER, &|ui, count| {
            let rows: Vec<TreeRow<'_>> = (0..count as u64)
                .map(|key| TreeRow {
                    key,
                    depth: 0,
                    text: "row",
                    glyph: None,
                    has_children: false,
                })
                .collect();
            let o = TreeOptions {
                rows: &rows,
                label: "Files",
                guides: false,
                guide_style: GuideStyle::Elbow,
                chevron: ChevronStyle::Triangle,
                toggle_on_row: false,
            };
            ui.tree_view("t", INNER, &o, |_, _| {});
        });
    }

    #[test]
    fn a_tab_strip() {
        let strip = Rect::new(40.0, 40.0, 400.0, 32.0);
        let card = Rect::new(40.0, 40.0, 400.0, 200.0);
        check("tab strip", strip, &|ui, count| {
            let tabs: Vec<Tab<'_>> = (0..count as u64)
                .map(|k| Tab {
                    key: TabKey(k),
                    title: "A tab",
                    closable: false,
                    progress: None,
                })
                .collect();
            let o = TabStripOptions {
                active: Some(TabKey(0)),
                panel_focused: false,
                busy: None,
                busy_tone: Tone::Neutral,
                card,
                menu_extra: &[],
                name: "Sessions",
                header_tint: None,
            };
            ui.tab_strip("s", strip, &tabs, &o);
        });
    }

    #[test]
    fn a_text_area_wrapped_or_not() {
        for wrap in [false, true] {
            check("text area", INNER, &|ui, lines| {
                let mut text = "a line\n".repeat(lines);
                let o = AreaOptions {
                    wrap,
                    ..AreaOptions::default()
                };
                ui.text_area("a", INNER, &mut text, &o);
            });
        }
    }

    #[test]
    fn a_toast_history() {
        let now = Duration::ZERO;
        let filled = |n: usize| {
            let mut toasts = Toasts::default();
            for _ in 0..n {
                let id = toasts.push(ToastSpec::new(ToastKind::Info, "Saved"), now);
                toasts.dismiss(id, now);
            }
            RefCell::new(toasts)
        };
        let (few, many) = (filled(FEW), filled(MANY));
        check("toast history", INNER, &|ui, n| {
            let toasts = if n == FEW { &few } else { &many };
            ui.toast_history("hist", INNER, &mut toasts.borrow_mut());
        });
    }

    /// D178: a sideways swipe over a composite that scrolls only vertically goes to the area around
    /// it, whether the composite overflows or not; the vertical wheel keeps going to it when it does.
    #[test]
    fn a_sideways_swipe_over_a_vertical_composite_moves_the_page() {
        let swipe = |draw: &mut dyn FnMut(&mut Ui<'_>), delta: (f32, f32)| -> (f64, f64) {
            let draw = RefCell::new(draw);
            let frame = |h: &mut UiHarness, ms: u64| {
                let mut off = (0.0, 0.0);
                h.at_ms(ms).frame(|ui: &mut Ui<'_>| {
                    let o = ScrollOptions {
                        extent: Some((2000.0, 3000.0)),
                        ..ScrollOptions::default()
                    };
                    let rect = Rect::new(0.0, 0.0, 900.0, 700.0);
                    let page = ui.scroll_area("page", rect, &o, |ui, _| {
                        (draw.borrow_mut())(ui);
                        ((), Extent::new(2000.0, 3000.0))
                    });
                    off = page.offset;
                });
                off
            };
            let mut h = UiHarness::new(WIN);
            frame(&mut h, 0);
            h.input
                .pointer_to(INNER.x + INNER.w / 2.0, INNER.y + INNER.h / 2.0);
            frame(&mut h, 10);
            h.input.scroll_pixels(delta.0, delta.1);
            frame(&mut h, 20);
            frame(&mut h, 400)
        };
        for count in [FEW, MANY] {
            let list = &mut |ui: &mut Ui<'_>| {
                let o = ListOptions {
                    count,
                    heights: Heights::Fixed(30.0),
                    selection: SelectionMode::Single,
                    index_of: None,
                    label: "rows",
                };
                ui.virtual_list("l", INNER, &o, |i| RowKey(i as u64), |_, _| 30.0);
            };
            assert_eq!(swipe(list, (-60.0, 0.0)), (60.0, 0.0), "list of {count}");
        }
        // A diagonal gesture over an overflowing list: the list keeps its vertical part, the page
        // takes the sideways one.
        let list = &mut |ui: &mut Ui<'_>| {
            let o = ListOptions {
                count: MANY,
                heights: Heights::Fixed(30.0),
                selection: SelectionMode::Single,
                index_of: None,
                label: "rows",
            };
            ui.virtual_list("l", INNER, &o, |i| RowKey(i as u64), |_, _| 30.0);
        };
        assert_eq!(swipe(list, (-60.0, -60.0)), (60.0, 0.0));
    }
    /// D178 and the M9 composites: the editor scrolls vertically (and sideways unwrapped), the
    /// document and the diff vertically, so each keeps the wheel when it overflows and gives it to the
    /// page when it fits. The block's own cases are `a_code_block_scrolls_only_sideways`.
    #[test]
    fn a_code_editor_wrapped_or_not() {
        for wrap in [false, true] {
            check("code editor", INNER, &|ui, lines| {
                let mut doc = CodeDoc::new("a line\n".repeat(lines));
                let o = CodeOptions {
                    wrap,
                    ..CodeOptions::default()
                };
                ui.code_editor("e", INNER, &mut doc, &o);
            });
        }
    }

    #[test]
    fn a_markdown_document() {
        let words = [MdInline::Text {
            text: "a paragraph",
            bold: false,
            italic: false,
            strike: false,
        }];
        check("markdown", INNER, &|ui, count| {
            let blocks: Vec<MdBlock<'_>> = (0..count).map(|_| MdBlock::Paragraph(&words)).collect();
            let o = MdOptions {
                label: "Doc",
                tasks_live: false,
                identity: None,
            };
            ui.markdown("m", INNER, &blocks, &o);
        });
    }

    #[test]
    fn a_diff_unified_or_split() {
        for split in [false, true] {
            check("diff", INNER, &|ui, count| {
                let texts: Vec<String> = (0..count).map(|i| format!("line {i}")).collect();
                let lines: Vec<DiffLine<'_>> = texts
                    .iter()
                    .map(|t| DiffLine::new(LineKind::Context, t))
                    .collect();
                let rows = pair_rows(&lines);
                let o = DiffOptions {
                    mode: if split {
                        DiffMode::Split(&rows)
                    } else {
                        DiffMode::Unified
                    },
                    wrap: false,
                    label: "Changes",
                    actions: &[],
                    identity: 1,
                    widest_chars: 0,
                };
                ui.diff_view("d", INNER, &lines, &o);
            });
        }
    }

    /// The page's offset after `delta` over the middle of `INNER`, with `draw` in a page that
    /// scrolls both ways.
    fn page_after(delta: (f32, f32), draw: &mut dyn FnMut(&mut Ui<'_>)) -> (f64, f64) {
        let draw = RefCell::new(draw);
        let frame = |h: &mut UiHarness, ms: u64| {
            let mut off = (0.0, 0.0);
            h.at_ms(ms).frame(|ui: &mut Ui<'_>| {
                let o = ScrollOptions {
                    extent: Some((2000.0, 3000.0)),
                    ..ScrollOptions::default()
                };
                let rect = Rect::new(0.0, 0.0, 900.0, 700.0);
                let page = ui.scroll_area("page", rect, &o, |ui, _| {
                    (draw.borrow_mut())(ui);
                    ((), Extent::new(2000.0, 3000.0))
                });
                off = page.offset;
            });
            off
        };
        let mut h = UiHarness::new(WIN);
        frame(&mut h, 0);
        h.input
            .pointer_to(INNER.x + INNER.w / 2.0, INNER.y + INNER.h / 2.0);
        frame(&mut h, 10);
        h.input.scroll_pixels(delta.0, delta.1);
        frame(&mut h, 20);
        frame(&mut h, 400)
    }

    /// D178: the block has no vertical extent, so the vertical wheel is the page's whatever the line
    /// count; the sideways one is the block's only when a line is wider than it.
    #[test]
    fn a_code_block_scrolls_only_sideways() {
        for lines in [FEW, MANY] {
            let text = "let a = 1;\n".repeat(lines);
            let at = page_after((0.0, -60.0), &mut |ui| {
                ui.code_block("b", INNER, &text, &CodeBlockOptions::default());
            });
            assert_eq!(
                at,
                (0.0, 60.0),
                "{lines} lines: the page takes the vertical wheel"
            );
        }
        for (width, page_x) in [(5, 60.0), (150, 0.0)] {
            let text = "x".repeat(width);
            let at = page_after((-60.0, 0.0), &mut |ui| {
                ui.code_block("b", INNER, &text, &CodeBlockOptions::default());
            });
            assert_eq!(at, (page_x, 0.0), "a line of {width}: the sideways wheel");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// M10: the theme reach audit. A loaded theme must reach every widget: draw the controls and a
// spread of the M6 and M7 widgets under a theme whose every colour field is a value no preset
// field has, and no fill, stroke or text may hold a preset colour. A hit is a colour a widget
// hard-codes (or takes from the preset instead of `ui.theme()`).
// ---------------------------------------------------------------------------------------------

mod m10 {
    use super::*;
    use lune_ui_core::color::Rgba;
    use lune_ui_core::icon;
    use lune_ui_core::strings::DisplayFormat;
    use lune_ui_core::testing::FakeText;
    use lune_ui_core::text::{CaretPos, LineSpan, Spec, TextKey, TextSystem, Unit};
    use lune_ui_core::theme::{DARK, Theme};
    use lune_ui_core::ui::FrameConfig;
    use lune_ui_core::widgets::button::{ButtonOptions, IconButton};
    use lune_ui_core::widgets::filter_bar::FilterOptions;
    use lune_ui_core::widgets::progress::ProgressOptions;
    use lune_ui_core::widgets::radio_group::RadioOption;
    use lune_ui_core::widgets::slider::SliderOptions;
    use lune_ui_core::widgets::spinner::SpinnerOptions;
    use lune_ui_core::widgets::tab_strip::{BusyCue, Tab, TabKey, TabStripOptions};
    use lune_ui_core::widgets::toast::{ToastKind, ToastSpec, Toasts};
    use lune_ui_core::widgets::tone::Tone;
    use lune_ui_core::widgets::tree_view::{ChevronStyle, GuideStyle, TreeOptions, TreeRow};
    use std::time::Duration;

    type Rgb = [u8; 3];

    // What `extras` draws beyond the controls and `more`.
    use lune_ui_core::widgets::callout::CalloutOptions;
    use lune_ui_core::widgets::caption::CaptionOptions;
    use lune_ui_core::widgets::command_palette::{Palette, PaletteRow, PaletteSection};
    use lune_ui_core::widgets::diff_view::{
        DiffLine, DiffMark, DiffMode, DiffOptions, HunkAction, LineKind, pair_rows,
    };
    use lune_ui_core::widgets::flyout::Flyout;
    use lune_ui_core::widgets::icon_rail::RailItem;
    use lune_ui_core::widgets::kanban_board::{BoardOptions, KanbanCard, KanbanColumn};
    use lune_ui_core::widgets::markdown::{MdBlock, MdInline, MdOptions};
    use lune_ui_core::widgets::menu::{MenuItem, MenuRow};
    use lune_ui_core::widgets::page::PageOptions;
    use lune_ui_core::widgets::popover::{PlaceAlign, PopoverOptions, Side, Surface};
    use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};
    use lune_ui_core::widgets::settings_page::SettingsPage;
    use lune_ui_core::widgets::stat::StatOptions;
    use lune_ui_core::widgets::table::{Cell, ColumnWidth, TableColumn, TableOptions};
    use lune_ui_core::widgets::toolbar::ToolbarOptions;
    use lune_ui_core::widgets::virtual_list::{RowKey, SelectionMode};

    /// Every colour field of a theme by name, in one list, the glass tints last. The destructuring
    /// below does not compile when `Theme` gains a field this list does not name.
    macro_rules! colour_fields {
        ($($f:ident),+ $(,)?) => {
            fn fields(t: &Theme) -> Vec<(&'static str, Rgba)> {
                let mut v = vec![$((stringify!($f), t.$f)),+];
                v.push(("glass.window.tint", t.glass.window.tint));
                v.push(("glass.panel.tint", t.glass.panel.tint));
                v.push(("glass.card.tint", t.glass.card.tint));
                v
            }

            fn set_field(t: &mut Theme, i: usize, c: Rgba) {
                let mut n = 0;
                $( if i == n { t.$f = c; } n += 1; )+
                if i == n { t.glass.window.tint = c; }
                if i == n + 1 { t.glass.panel.tint = c; }
                if i == n + 2 { t.glass.card.tint = c; }
            }

            #[allow(dead_code)]
            fn every_field_is_named(t: Theme) {
                let Theme {
                    $($f: _,)+
                    glass: _,
                    font_s: _,
                    font_base: _,
                    font_m: _,
                    font_title: _,
                    line_title: _,
                    item_radius: _,
                    motion: _,
                } = t;
            }
        };
    }

    colour_fields!(
        fg_primary,
        fg_secondary,
        fg_tertiary,
        surface_input,
        surface_hover,
        surface_selected,
        border,
        border_subtle,
        tree_guide,
        accent,
        scroll_thumb,
        scroll_thumb_hover,
        text_selection,
        on_accent,
        dialog_backdrop,
        dialog_footer,
        menu_text_primary,
        menu_text_tertiary,
        menu_text_quaternary,
        menu_popup_border,
        menu_item_hover,
        menu_separator,
        menu_glass,
        menu_item_pressed,
        menu_text_disabled,
        toast_info,
        toast_success,
        toast_warning,
        toast_error,
        toast_log_bg,
        toast_log_fg,
        dirty_indicator,
        count_badge,
        caption_close_hover,
        caption_close_fg,
        fg_muted,
        fg_subtle,
        fg_bright,
        fg_quaternary,
        fg_dim,
        fg_faint,
        surface_overlay,
        surface_active,
        surface_separator,
        surface_divider,
        ok,
        warn,
        err,
        status_warning,
        status_warning_light,
        status_error_light,
        status_error,
        status_warning_dark,
        data,
        data_hi,
        gold,
        paused,
        link,
        code_inline,
        diff_added,
        diff_removed,
        diff_added_mark,
        diff_removed_mark,
    );

    fn rgb(c: Rgba) -> Rgb {
        [c[0], c[1], c[2]]
    }

    fn far(a: Rgb, b: Rgb) -> bool {
        a.iter().zip(&b).any(|(x, y)| x.abs_diff(*y) > 3)
    }

    fn same(a: Rgb, b: Rgb) -> bool {
        !far(a, b)
    }

    /// A theme whose every colour field has a colour no preset field has and no other field of
    /// itself has (more than 3 apart on some channel), at alpha 0xF0 so the colour comes back out
    /// of a premultiplied shape.
    fn distinct() -> Theme {
        let n = fields(&DARK).len();
        let mut used: Vec<Rgb> = fields(&DARK).iter().map(|f| rgb(f.1)).collect();
        let mut t = DARK;
        let mut k = 1_u32;
        for i in 0..n {
            let c = loop {
                let c = [
                    ((k * 37) % 253) as u8 + 1,
                    ((k * 91 + 40) % 253) as u8 + 1,
                    ((k * 53 + 90) % 253) as u8 + 1,
                ];
                k += 1;
                if used.iter().all(|u| far(*u, c)) {
                    break c;
                }
            };
            used.push(c);
            set_field(&mut t, i, [c[0], c[1], c[2], 0xF0]);
        }
        t
    }

    /// The fake's shaping, recording the colour of every run drawn (`prepare`); `measure` is not
    /// recorded, since a measured spec is not drawn.
    #[derive(Default)]
    struct Rec {
        inner: FakeText,
        colours: Vec<Rgb>,
    }

    impl TextSystem for Rec {
        fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
            for (text, style) in &spec.runs {
                if !text.is_empty() {
                    self.colours.push(rgb(style.color));
                }
            }
            self.inner.prepare(spec, scale)
        }
        fn measure(&mut self, spec: &Spec, scale: f32) -> (f32, f32) {
            self.inner.measure(spec, scale)
        }
        fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos {
            self.inner.caret(spec, scale, offset)
        }
        fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize {
            self.inner.hit(spec, scale, pos)
        }
        fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan> {
            self.inner.lines(spec, scale)
        }
        fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
            self.inner.segments(text, unit)
        }
    }

    /// `UiHarness::frame` over the recording text system.
    fn run(h: &mut UiHarness, rec: &mut Rec, f: impl FnOnce(&mut Ui<'_>)) -> Scene {
        let raw = h.input.end_frame();
        let config = FrameConfig {
            theme: h.theme,
            tier: h.tier,
            strings: &h.strings,
            arg_format: &DisplayFormat,
            keymap: h.keymap,
            shortcuts: &h.shortcuts,
        };
        let mut ui = Ui::begin(&mut h.store, &raw, config, rec);
        f(&mut ui);
        ui.finish().0
    }

    /// The colours a scene draws and the colours its texts were shaped with.
    #[derive(Default)]
    struct Seen {
        fills: Vec<Rgb>,
        texts: Vec<Rgb>,
        /// Every acrylic and frost colour (premultiplied), which the glass recipe makes from a tint.
        acrylics: Vec<[f32; 4]>,
    }

    fn straight(c: [f32; 4]) -> Option<Rgb> {
        if c[3] < 0.02 {
            return None;
        }
        let k = |v: f32| (v / c[3] * 255.0).round().clamp(0.0, 255.0) as u8;
        Some([k(c[0]), k(c[1]), k(c[2])])
    }

    impl Seen {
        fn take(&mut self, scene: &Scene, rec: &Rec) {
            let mut colours = Vec::new();
            for layer in scene.in_order() {
                let g = &layer.geometry;
                colours.extend(g.under.iter().map(|s| s.color));
                colours.extend(g.over.iter().map(|s| s.color));
                colours.extend(g.acrylic.iter().map(|a| a.shape.color));
                colours.extend(g.strokes.iter().map(|v| v.color));
                colours.extend(g.fills.iter().map(|v| v.color));
            }
            for layer in scene.in_order() {
                let g = &layer.geometry;
                self.acrylics
                    .extend(g.acrylic.iter().map(|a| a.shape.color));
                let frosts = g.under.iter().filter(|s| s.is_frost());
                self.acrylics.extend(frosts.map(|s| s.color));
            }
            self.fills.extend(colours.into_iter().filter_map(straight));
            self.texts.extend(rec.colours.iter().copied());
        }
    }

    // The widgets beside the controls: the same calls the M6 and M7 sections make.
    const RADIOS: [RadioOption<'static>; 2] = [
        RadioOption {
            key: 1,
            label: "Small",
            enabled: true,
        },
        RadioOption {
            key: 2,
            label: "Medium",
            enabled: true,
        },
    ];
    const SLIDER: SliderOptions<'static> = SliderOptions {
        min: 0.0,
        max: 100.0,
        step: 10.0,
        name: "Volume",
    };

    #[derive(Default)]
    struct Extra {
        radio: u64,
        stars: u32,
        volume: f32,
        filter: String,
    }

    /// The right half of the window: buttons, a radio group, stars, a slider, a tab strip, a
    /// filter bar, progress bars and a spinner, a badge and a status dot in each tone, a banner
    /// and the four toasts.
    fn more(ui: &mut Ui<'_>, ex: &mut Extra, toasts: &mut Toasts) {
        ui.painter().on(LayerId::Page);
        ui.button_with(
            "ghost",
            Rect::new(300.0, 10.0, 90.0, 32.0),
            "Cancel",
            &ButtonOptions {
                kind: ButtonKind::Ghost,
                small: true,
                icon: None,
            },
        );
        ui.button(
            "primary",
            Rect::new(400.0, 10.0, 90.0, 32.0),
            "Save",
            ButtonKind::Primary,
        );
        ui.icon_button(
            "icon-button",
            Rect::new(500.0, 10.0, 32.0, 32.0),
            &IconButton {
                glyph: icon::SEARCH,
                glyph_size: 16.0,
                color: None,
                radius: 4.0,
                name: "Search",
            },
        );
        ui.radio_group(
            "radio",
            Rect::new(300.0, 60.0, 180.0, 0.0),
            &RADIOS,
            &mut ex.radio,
            "Size",
        );
        ui.star_rating("stars", (500.0, 60.0), &mut ex.stars, 5, "Rating");
        ui.slider(
            "slider",
            Rect::new(300.0, 130.0, 180.0, 24.0),
            &mut ex.volume,
            &SLIDER,
        );
        let tabs = [
            Tab {
                key: TabKey(1),
                title: "alpha",
                closable: true,
                progress: Some(Tone::Ok),
            },
            Tab {
                key: TabKey(2),
                title: "beta",
                closable: true,
                progress: None,
            },
        ];
        ui.tab_strip(
            "strip",
            Rect::new(300.0, 170.0, 400.0, 32.0),
            &tabs,
            &TabStripOptions {
                active: Some(TabKey(1)),
                panel_focused: true,
                busy: None,
                busy_tone: Tone::Neutral,
                card: Rect::new(300.0, 170.0, 400.0, 120.0),
                menu_extra: &[],
                name: "Documents",
                header_tint: None,
            },
        );
        ui.filter_bar(
            "filter",
            Rect::new(300.0, 300.0, 200.0, 32.0),
            &mut ex.filter,
            &FilterOptions {
                placeholder: "Filter",
                label: None,
                clear_unfocusable: false,
            },
        );
        for (i, tone) in [
            Tone::Neutral,
            Tone::Ok,
            Tone::Warn,
            Tone::Err,
            Tone::Data,
            Tone::Paused,
        ]
        .into_iter()
        .enumerate()
        {
            let y = 350.0 + 22.0 * i as f32;
            ui.progress(
                ("bar", i),
                Rect::new(300.0, y, 120.0, 4.0),
                &ProgressOptions {
                    value: Some(0.5),
                    tone,
                    name: "Upload",
                    ..ProgressOptions::default()
                },
            );
            ui.badge(("badge", i), (440.0, y - 6.0), "12", tone, false);
            ui.badge(("dot-badge", i), (490.0, y - 6.0), "ok", tone, true);
            ui.status_dot(("dot", i), (550.0, y - 6.0), tone, "word");
        }
        ui.spinner(
            "spinner",
            (680.0, 360.0),
            &SpinnerOptions {
                busy: false,
                size: 16.0,
                color: None,
                name: "Loading",
            },
        );
        ui.banner(
            "banner",
            Rect::new(300.0, 500.0, 400.0, 0.0),
            "Indexing the repository",
            false,
        );
        ui.toasts("toasts", toasts);
    }

    const UNDER: [f32; 4] = [0.05, 0.06, 0.07, 1.0];

    const MARK: [DiffMark; 1] = [DiffMark { start: 5, end: 8 }];

    fn text(s: &str) -> MdInline<'_> {
        MdInline::Text {
            text: s,
            bold: false,
            italic: false,
            strike: false,
        }
    }

    /// The widgets the controls and `more` leave out, in three windows: 1, the chrome (caption
    /// buttons, toolbars, the rail, a scroll area, a menu, a stat); 2, content (markdown, a diff,
    /// a table, the board); 3, the page shells and the overlays (page, settings page, callout,
    /// flyout, a glass popover, the palette, a loading toast with its log, a busy tab).
    fn extras(ui: &mut Ui<'_>, which: u8, toasts: &mut Toasts) {
        let glass = ui.theme().glass;
        ui.painter().on(LayerId::Page);
        match which {
            1 => {
                let win = Rect::new(0.0, 0.0, 900.0, 700.0);
                ui.caption_buttons("caption", win, CaptionOptions::default());
                let bar = |g| ToolbarOptions {
                    glass: g,
                    under: UNDER,
                    name: "Bar",
                };
                ui.toolbar(
                    "bar-off",
                    Rect::new(0.0, 60.0, 900.0, 52.0),
                    &bar(glass.window),
                    |ui, _| {
                        ui.toolbar_label(
                            "label",
                            Rect::new(8.0, 68.0, 200.0, 36.0),
                            icon::SYNC,
                            "Label",
                            "Label",
                        );
                    },
                );
                ui.toolbar(
                    "bar-on",
                    Rect::new(0.0, 120.0, 900.0, 52.0),
                    &bar(glass.panel),
                    |ui, _| {
                        ui.toolbar_action(
                            "act",
                            Rect::new(8.0, 128.0, 36.0, 36.0),
                            icon::SYNC,
                            "Act",
                            true,
                        );
                    },
                );
                let rail = [RailItem {
                    key: 1,
                    glyph: icon::SEARCH,
                    name: "Search",
                    active: true,
                    enabled: true,
                    dot: true,
                    badge: Some(3),
                    note: Some("3 unread"),
                }];
                ui.icon_rail(
                    "rail",
                    Rect::new(840.0, 200.0, 48.0, 300.0),
                    &rail,
                    &[],
                    "Rail",
                );
                let o = ScrollOptions {
                    extent: Some((300.0, 900.0)),
                    ..ScrollOptions::default()
                };
                ui.scroll_area("sa", Rect::new(20.0, 200.0, 300.0, 200.0), &o, |_, _| {
                    ((), Extent::new(300.0, 900.0))
                });
                let rows = [
                    MenuRow::Item {
                        glyph: Some(icon::COPY),
                        text: "Copy",
                        shortcut: "Cmd+C",
                    },
                    MenuRow::Separator,
                    MenuRow::Entry(MenuItem {
                        glyph: None,
                        text: "Paste",
                        shortcut: "Cmd+V",
                        disabled: true,
                        danger: false,
                        chevron: false,
                    }),
                    MenuRow::Item {
                        glyph: None,
                        text: "Delete",
                        shortcut: "",
                    },
                ];
                ui.menu("menu", &rows, (400.0, 220.0));
                ui.stat(
                    "stat",
                    Rect::new(20.0, 430.0, 220.0, 90.0),
                    &StatOptions {
                        label: "Saved",
                        value: "1,204",
                        number: None,
                        live: false,
                        sub: Some("this week"),
                        sub_tone: Tone::Ok,
                        gold: true,
                        compact: false,
                        text: false,
                        meter: Some(0.4),
                        no_data: false,
                    },
                );
                // A tree with a nested level: the guides (`tree_guide`) and a mark.
                let rows = [
                    TreeRow {
                        key: 1,
                        depth: 0,
                        text: "src",
                        glyph: None,
                        has_children: true,
                    },
                    TreeRow {
                        key: 2,
                        depth: 1,
                        text: "main.rs",
                        glyph: None,
                        has_children: false,
                    },
                    TreeRow {
                        key: 3,
                        depth: 1,
                        text: "lib.rs",
                        glyph: None,
                        has_children: false,
                    },
                ];
                ui.tree_state("audit-tree").expanded.insert(1);
                ui.tree_view(
                    "audit-tree",
                    Rect::new(560.0, 400.0, 260.0, 120.0),
                    &TreeOptions {
                        rows: &rows,
                        label: "Audit",
                        guides: true,
                        guide_style: GuideStyle::Elbow,
                        chevron: ChevronStyle::Triangle,
                        toggle_on_row: false,
                    },
                    |_, _| {},
                );
            }
            2 => {
                let title = [text("Guide")];
                let para = [
                    text("See the "),
                    MdInline::Link {
                        label: "docs",
                        url: "docs/a.md",
                    },
                    text(" and "),
                    MdInline::Code("cargo test"),
                ];
                let blocks = [
                    MdBlock::Heading {
                        level: 2,
                        inlines: &title,
                    },
                    MdBlock::Paragraph(&para),
                ];
                ui.markdown(
                    "md",
                    Rect::new(10.0, 10.0, 400.0, 160.0),
                    &blocks,
                    &MdOptions {
                        label: "Guide",
                        tasks_live: true,
                        identity: None,
                    },
                );
                let lines = [
                    DiffLine {
                        hunk: Some(0),
                        ..DiffLine::new(LineKind::Hunk, "@@ -1,3 +1,3 @@")
                    },
                    DiffLine {
                        old: Some(1),
                        new: Some(1),
                        ..DiffLine::new(LineKind::Context, "alpha")
                    },
                    DiffLine {
                        old: Some(2),
                        marks: &MARK,
                        ..DiffLine::new(LineKind::Removed, "beta old")
                    },
                    DiffLine {
                        new: Some(2),
                        marks: &MARK,
                        ..DiffLine::new(LineKind::Added, "beta new")
                    },
                ];
                let rows = pair_rows(&lines);
                let hunk = [HunkAction {
                    glyph: icon::ADD,
                    label: "Stage hunk",
                }];
                ui.diff_view(
                    "diff",
                    Rect::new(10.0, 190.0, 700.0, 160.0),
                    &lines,
                    &DiffOptions {
                        mode: DiffMode::Split(&rows),
                        wrap: false,
                        label: "Changes",
                        actions: &hunk,
                        identity: 1,
                        widest_chars: 0,
                    },
                );
                let cols = [
                    TableColumn {
                        key: 1,
                        title: "Name",
                        width: ColumnWidth::Fill(1.0),
                        num: false,
                        sortable: true,
                    },
                    TableColumn {
                        key: 2,
                        title: "Amount",
                        width: ColumnWidth::Fixed(100.0),
                        num: true,
                        sortable: true,
                    },
                ];
                ui.table(
                    "table",
                    Rect::new(10.0, 370.0, 400.0, 160.0),
                    &TableOptions {
                        columns: &cols,
                        count: 4,
                        selection: SelectionMode::Single,
                        wrap: false,
                        index_of: None,
                        label: "Requests",
                    },
                    |i| RowKey(i as u64),
                    |_, c| Cell::text(format!("{} {}", c.key.0, c.column)),
                );
                let names = ["Todo", "Doing"];
                let cards = [
                    [KanbanCard {
                        key: 1,
                        title: "One",
                        tags: &["x"],
                        assignee: Some("A"),
                        note: true,
                    }],
                    [KanbanCard {
                        key: 2,
                        title: "Two",
                        tags: &[],
                        assignee: None,
                        note: false,
                    }],
                ];
                let columns: Vec<KanbanColumn<'_>> = names
                    .iter()
                    .zip(&cards)
                    .enumerate()
                    .map(|(i, (title, cards))| KanbanColumn {
                        key: i as u64,
                        title,
                        cards,
                    })
                    .collect();
                ui.kanban_board(
                    "board",
                    Rect::new(430.0, 370.0, 460.0, 300.0),
                    &BoardOptions {
                        title: "Roadmap",
                        columns: &columns,
                        error: None,
                        glass: glass.card,
                        under: UNDER,
                        name: "",
                    },
                );
            }
            _ => {
                let page = PageOptions {
                    title: "Page",
                    subtitle: Some("Sub"),
                    glass: glass.panel,
                    under: UNDER,
                    actions_w: 0.0,
                    footer: false,
                    swap_key: 0,
                    name: "",
                };
                ui.page(
                    "page",
                    Rect::new(10.0, 10.0, 420.0, 220.0),
                    &page,
                    |_, _| 10.0,
                );
                let settings = SettingsPage {
                    glass: glass.window,
                    under: UNDER,
                    restart: true,
                    name: "General",
                };
                ui.settings_page(
                    "settings",
                    Rect::new(450.0, 10.0, 430.0, 260.0),
                    &settings,
                    |_, _| 10.0,
                );
                ui.callout(
                    "callout",
                    Rect::new(10.0, 250.0, 420.0, 0.0),
                    &CalloutOptions {
                        tone: Tone::Err,
                        icon: icon::INFO,
                        text: "The proxy is offline",
                        figure: Some("3"),
                        action: Some("Retry"),
                        announce: false,
                    },
                );
                let fly = Flyout {
                    anchor: Rect::new(700.0, 300.0, 24.0, 24.0),
                    anchor_id: None,
                    side: Side::Bottom,
                    align: PlaceAlign::Start,
                    mark: Some(icon::SYNC),
                    wordmark: "App",
                    state: "Serving",
                    tone: Tone::Ok,
                    stats: &[],
                    routes: &[("model", "opus")],
                    rows: &[MenuRow::Item {
                        glyph: None,
                        text: "Open",
                        shortcut: "",
                    }],
                    name: "",
                };
                ui.flyout("fly", &fly);
                let items = ["one"];
                let _ = items;
                let mut o =
                    PopoverOptions::new(Rect::new(20.0, 400.0, 100.0, 30.0), (200.0, 100.0));
                o.surface = Surface::Glass;
                ui.popover("pop", &o, |_, _| ());
                let rows = [PaletteRow {
                    key: 1,
                    text: "Reload",
                    detail: None,
                    icon: None,
                    hint: Some("Cmd+R"),
                    destructive: true,
                }];
                let secs = [PaletteSection {
                    title: "COMMANDS",
                    rows: &rows,
                    more: false,
                }];
                let mut q = String::new();
                ui.command_palette(
                    "palette",
                    &mut q,
                    &Palette {
                        sections: &secs,
                        placeholder: "Search",
                        footer_left: "Up/Down",
                        error: Some("Index failed"),
                        name: "",
                    },
                    &DialogGlass {
                        fill: [0.2, 0.2, 0.2, 0.62],
                        noise: true,
                    },
                );
                ui.toasts("toasts", toasts);
                let tabs = [Tab {
                    key: TabKey(1),
                    title: "alpha",
                    closable: true,
                    progress: Some(Tone::Err),
                }];
                ui.tab_strip(
                    "strip",
                    Rect::new(450.0, 300.0, 400.0, 32.0),
                    &tabs,
                    &TabStripOptions {
                        active: Some(TabKey(1)),
                        panel_focused: true,
                        busy: Some(BusyCue::Sweep),
                        busy_tone: Tone::Err,
                        card: Rect::new(450.0, 300.0, 400.0, 100.0),
                        menu_extra: &[],
                        name: "Documents",
                        header_tint: None,
                    },
                );
            }
        }
    }

    /// What is open and where the pointer is, for one run of frames.
    #[derive(Clone, Copy)]
    struct Setup {
        combo: bool,
        menu: bool,
        dialog: bool,
        pointer: Option<(f32, f32)>,
        tabs: u32,
        /// Press the pointer where it is, then select all with the command chord.
        select_all: bool,
        /// Draw `extras(n)` in place of the controls and the extras beside them (0: those).
        extras: u8,
        /// Hold the primary button down where the pointer is.
        press: bool,
    }

    const REST: Setup = Setup {
        combo: false,
        menu: false,
        dialog: false,
        pointer: None,
        tabs: 0,
        select_all: false,
        extras: 0,
        press: false,
    };

    /// Three frames of the controls and the extras under `theme`; what the third, settled, one
    /// draws goes into `seen`.
    fn observe(theme: Theme, setup: Setup, seen: &mut Seen) {
        let mut h = UiHarness::new((900.0, 700.0));
        h.theme = theme;
        let mut rec = Rec::default();
        let mut app = App::new();
        app.combo_open = setup.combo;
        app.menu_open = setup.menu;
        app.dialog_open = setup.dialog;
        let mut ex = Extra {
            radio: 2,
            stars: 2,
            volume: 40.0,
            ..Extra::default()
        };
        let mut toasts = Toasts::default();
        for (kind, text) in [
            (ToastKind::Info, "Info"),
            (ToastKind::Success, "Done"),
            (ToastKind::Warning, "Careful"),
            (ToastKind::Error, "Failed"),
        ] {
            toasts.push(ToastSpec::new(kind, text), Duration::ZERO);
        }
        // The extras' toasts: a loading one with its log open.
        let mut loud = Toasts::default();
        let id = loud.push(
            ToastSpec {
                log: vec!["step one".to_owned(), "step two".to_owned()],
                ..ToastSpec::new(
                    ToastKind::Loading {
                        progress: Some(0.5),
                        cancellable: true,
                    },
                    "Fetching",
                )
            },
            Duration::ZERO,
        );
        loud.set_expanded(id, true);
        for (ms, last) in [(0_u64, false), (300, false), (700, false), (1500, true)] {
            h.at_ms(ms);
            if ms == 300 {
                if let Some((x, y)) = setup.pointer {
                    h.input.pointer_to(x, y);
                }
                for _ in 0..setup.tabs {
                    key(&mut h, NamedKey::Tab);
                }
                if setup.select_all {
                    h.input.click(P);
                }
            }
            if ms == 700 && setup.select_all {
                let command = Modifiers {
                    logo: true,
                    ..Modifiers::default()
                };
                h.input
                    .set_modifiers(command)
                    .key_down(Key::Character("a".to_owned()), None)
                    .set_modifiers(Modifiers::default());
            }
            if ms == 700 && setup.press {
                h.input.press(P);
            }
            rec.colours.clear();
            let which = setup.extras;
            let scene = run(&mut h, &mut rec, |ui| {
                if which == 0 {
                    page(ui, &mut app);
                    more(ui, &mut ex, &mut toasts);
                } else {
                    extras(ui, which, &mut loud);
                }
            });
            if last {
                seen.take(&scene, &rec);
            }
        }
    }

    fn setups() -> Vec<Setup> {
        let mut v = vec![
            REST,
            Setup {
                combo: true,
                ..REST
            },
            Setup { menu: true, ..REST },
            Setup {
                dialog: true,
                ..REST
            },
        ];
        for rect in [PAGE_BTN, TOGGLE, CHECK, STEPPER, FIELD, COMBO] {
            v.push(Setup {
                pointer: Some(centre(rect)),
                ..REST
            });
        }
        for at in [(310.0, 20.0), (410.0, 20.0), (510.0, 20.0), (320.0, 70.0)] {
            v.push(Setup {
                pointer: Some(at),
                ..REST
            });
        }
        for tabs in 1..=4 {
            v.push(Setup { tabs, ..REST });
        }
        // A menu with the pointer on its first rows, and a field whose text is selected.
        for y in [48.0, 70.0] {
            v.push(Setup {
                menu: true,
                pointer: Some((100.0, y)),
                ..REST
            });
        }
        v.push(Setup {
            pointer: Some(centre(FIELD)),
            select_all: true,
            ..REST
        });
        // The widgets the controls page does not draw: chrome and menus, content, then overlays.
        for extras in 1..=3 {
            v.push(Setup { extras, ..REST });
        }
        // The caption's close button, the scroll area and its thumb, the stat.
        for at in [
            (877.0, 16.0),
            (200.0, 300.0),
            (315.0, 215.0),
            (100.0, 560.0),
        ] {
            v.push(Setup {
                extras: 1,
                pointer: Some(at),
                ..REST
            });
        }
        // The diff's hunk action under the pointer.
        v.push(Setup {
            extras: 2,
            pointer: Some((234.0, 200.0)),
            ..REST
        });
        // A ghost button held down.
        v.push(Setup {
            pointer: Some((345.0, 26.0)),
            press: true,
            ..REST
        });
        // A menu row held down.
        v.push(Setup {
            extras: 1,
            pointer: Some((420.0, 232.0)),
            press: true,
            ..REST
        });
        v
    }

    fn seen_under(theme: Theme) -> Seen {
        let mut seen = Seen::default();
        for setup in setups() {
            observe(theme, setup, &mut seen);
        }
        seen
    }

    #[test]
    fn the_audit_theme_has_a_colour_no_preset_field_has_in_every_field() {
        let (t, p) = (fields(&distinct()), fields(&DARK));
        assert_eq!(t.len(), p.len());
        assert_eq!(t.len(), 66, "the colour fields of `Theme`");
        for (i, (name, c)) in t.iter().enumerate() {
            assert_ne!(*c, p[i].1, "{name}");
            for (_, q) in &p {
                assert!(far(rgb(*c), rgb(*q)), "{name} is a preset colour");
            }
            for (j, (_, d)) in t.iter().enumerate() {
                assert!(i == j || far(rgb(*c), rgb(*d)), "{name} repeats a field");
            }
        }
    }

    /// The instrument, on a known answer: under the preset itself the same frames draw preset
    /// colours (every control reads one), so the scan finds them and does not report "none" by
    /// looking at nothing.
    #[test]
    fn the_scan_finds_preset_colours_under_the_preset() {
        let seen = seen_under(DARK);
        let preset = fields(&DARK);
        let found = |colours: &[Rgb]| {
            preset
                .iter()
                .filter(|(_, p)| colours.iter().any(|c| same(rgb(*p), *c)))
                .count()
        };
        assert!(
            found(&seen.fills) >= 30,
            "{} fill fields",
            found(&seen.fills)
        );
        assert!(
            found(&seen.texts) >= 4,
            "{} text fields",
            found(&seen.texts)
        );
    }

    /// A colour the toolkit draws on purpose in a fixed value, whatever the theme: none among
    /// the widgets this frame draws.
    const FIXED: &[Rgb] = &[];

    #[test]
    fn no_fill_or_text_holds_a_preset_colour_under_a_theme_of_distinct_colours() {
        let seen = seen_under(distinct());
        assert!(
            seen.fills.len() > 500 && seen.texts.len() > 100,
            "the frames draw something: {} fills, {} texts",
            seen.fills.len(),
            seen.texts.len()
        );
        let preset = fields(&DARK);
        let mut hits: Vec<String> = Vec::new();
        for (kind, colours) in [("fill", &seen.fills), ("text", &seen.texts)] {
            for c in colours {
                if FIXED.iter().any(|f| same(*f, *c)) {
                    continue;
                }
                let names: Vec<&str> = preset
                    .iter()
                    .filter(|(_, p)| same(rgb(*p), *c))
                    .map(|(n, _)| *n)
                    .collect();
                if !names.is_empty() {
                    let line = format!("{kind} {c:02X?} is the preset's {names:?}");
                    if !hits.contains(&line) {
                        hits.push(line);
                    }
                }
            }
        }
        assert!(hits.is_empty(), "hard-coded colours:\n{}", hits.join("\n"));
    }

    /// The colour fields the frames do not reach, each with the reason. A field in this list that
    /// the frames do reach fails the test, so the list cannot go stale; a field missing from it
    /// that they do not reach fails it too.
    const UNREACHED: &[(&str, &str)] = &[];

    /// Whether the frames draw `name`'s colour. A glass tint is not drawn as it is: the recipe
    /// (`GlassLayer::premultiplied`) blends it with the layer's luminosity, so it is reached when
    /// an acrylic layer is drawn in the colour its layer makes from the audit theme's tint.
    fn reached(seen: &Seen, theme: &Theme, name: &str, c: Rgba) -> bool {
        let layer = match name {
            "glass.window.tint" => Some(theme.glass.window),
            "glass.panel.tint" => Some(theme.glass.panel),
            "glass.card.tint" => Some(theme.glass.card),
            _ => None,
        };
        if let Some(layer) = layer {
            let want = layer.premultiplied();
            return seen
                .acrylics
                .iter()
                .any(|a| a.iter().zip(&want).all(|(x, y)| (x - y).abs() < 1e-3));
        }
        seen.fills
            .iter()
            .chain(&seen.texts)
            .any(|s| same(*s, rgb(c)))
    }

    /// Every colour field of `Theme` is drawn by the frames, or is named above with its reason.
    /// The frames are the controls, the widgets beside them and `extras`: scroll thumbs (held and
    /// hovered), the glass menu, the rail with its dot and count, the caption buttons under the
    /// pointer, both toolbars, markdown, the diff with its marks, the table, the board, the page
    /// and settings page, the callout, the flyout, the palette, a loading toast with its log, a
    /// busy tab and a stat.
    #[test]
    fn the_audit_reaches_every_field_or_names_why_not() {
        let t = distinct();
        let seen = seen_under(t);
        let mut missed: Vec<&str> = Vec::new();
        for (name, c) in fields(&t) {
            if !reached(&seen, &t, name, c) {
                missed.push(name);
            }
        }
        let listed: Vec<&str> = UNREACHED.iter().map(|(n, _)| *n).collect();
        println!(
            "unreached ({} of {}): {missed:?}",
            missed.len(),
            fields(&t).len()
        );
        assert!(UNREACHED.len() <= 3, "at most three fields may be listed");
        assert!(
            UNREACHED.iter().all(|(_, why)| !why.is_empty()),
            "every listed field says why"
        );
        assert_eq!(missed, listed, "unreached fields and the list disagree");
        // The instrument: the check is not satisfied by looking at nothing.
        assert!(!reached(&seen, &t, "fg_primary", [1, 2, 3, 255]));
        assert!(reached(&seen, &t, "fg_primary", t.fg_primary));
    }
}

/// M11, the key readers that took a chord or leaked when disabled (D140, D157), and the hosts of
/// `Ui::button` and `icon_button` whose rows were probes. The table is `m9`'s, typed: each stop is
/// one reader on a page of its own with a `Global` binding on each of 12 keys (a dialog's on
/// `Scope::Overlay`, which a `Global` one cannot fail in, F19); it is focused with Tab, the keys it
/// takes enabled are the ones that fire nothing, and a focused-then-disabled stop must take every
/// one of them and fire none. The five readers fixed in M11 also run a chord on each key.
mod m11 {
    use super::*;

    use std::time::Duration;

    use lune_ui_core::GlassLayer;
    use lune_ui_core::icon;
    use lune_ui_core::shortcuts::{Binding, Chord, Scope};
    use lune_ui_core::widgets::banner::EmptyState;
    use lune_ui_core::widgets::callout::CalloutOptions;
    use lune_ui_core::widgets::filter_bar::FilterOptions;
    use lune_ui_core::widgets::search_picker::{PickerOptions, Trigger};
    use lune_ui_core::widgets::settings_page::SettingsPage;
    use lune_ui_core::widgets::slider::SliderOptions;
    use lune_ui_core::widgets::tab_strip::{Tab, TabEvent, TabKey, TabStripOptions};
    use lune_ui_core::widgets::toast::{ToastAction, ToastKind, ToastSpec, Toasts};
    use lune_ui_core::widgets::tone::Tone;
    use lune_ui_core::widgets::toolbar::{ACTION_SIZE, ToolbarOptions};

    const M11_WIN: (f32, f32) = (900.0, 700.0);
    const STEP_RANGE: StepRange = StepRange {
        min: 0.0,
        max: 99.0,
        step: 1.0,
    };
    const GLASS: GlassLayer = GlassLayer {
        tint: [10, 20, 30, 255],
        tint_opacity: 0.8,
        material_opacity: 0.7,
        curve: lune_ui_core::LuminosityCurve::AVALONIA,
    };
    const UNDER: [f32; 4] = [0.05, 0.06, 0.07, 1.0];

    /// A key a focused reader uses: a named key, or Command with a letter.
    #[derive(Debug, Clone, Copy)]
    enum Probe {
        Named(NamedKey),
        Cmd(&'static str),
    }

    const KEYS: [Probe; 12] = [
        Probe::Named(NamedKey::ArrowUp),
        Probe::Named(NamedKey::ArrowDown),
        Probe::Named(NamedKey::ArrowLeft),
        Probe::Named(NamedKey::ArrowRight),
        Probe::Named(NamedKey::Home),
        Probe::Named(NamedKey::End),
        Probe::Named(NamedKey::PageUp),
        Probe::Named(NamedKey::PageDown),
        Probe::Named(NamedKey::Enter),
        Probe::Named(NamedKey::Space),
        Probe::Cmd("a"),
        Probe::Cmd("c"),
    ];

    fn press(h: &mut UiHarness, p: Probe) {
        match p {
            Probe::Named(k) => key(h, k),
            Probe::Cmd(c) => {
                h.input.set_modifiers(Modifiers {
                    logo: true,
                    ..Modifiers::default()
                });
                h.input.key_down(Key::Character(c.to_owned()), None);
                h.input.set_modifiers(Modifiers::default());
            }
        }
    }

    /// Command held for one named key.
    fn press_cmd(h: &mut UiHarness, k: NamedKey) {
        h.input.set_modifiers(Modifiers {
            logo: true,
            ..Modifiers::default()
        });
        h.input.key_down(Key::Named(k), None);
        h.input.set_modifiers(Modifiers::default());
    }

    fn plain_binding(p: Probe, scope: Scope) -> Binding {
        match p {
            Probe::Named(k) => Binding::new(scope, Chord::key(Key::Named(k)), format!("g-{k:?}")),
            Probe::Cmd(c) => Binding::new(
                scope,
                Chord::key(Key::Character(c.to_owned())).primary(),
                format!("g-cmd-{c}"),
            ),
        }
    }

    /// The app side: every value a stop's widget can change, and what its frames reported.
    #[derive(Clone)]
    struct Pg {
        enabled: bool,
        step: u32,
        slide: f32,
        picked: Option<usize>,
        rating: u32,
        active: u64,
        text: String,
        clicks: u32,
        events: Vec<String>,
        toasts: Toasts,
    }

    impl Pg {
        fn new() -> Self {
            Pg {
                enabled: true,
                step: 12,
                slide: 50.0,
                picked: None,
                rating: 2,
                active: 0,
                text: "abc".to_owned(),
                clicks: 0,
                events: Vec::new(),
                toasts: Toasts::default(),
            }
        }

        /// Everything the keys of a reader could change.
        fn state(&self) -> String {
            format!(
                "{} {} {:?} {} {} {} {} {:?}",
                self.step,
                self.slide.to_bits(),
                self.picked,
                self.rating,
                self.active,
                self.text,
                self.clicks,
                self.events
            )
        }
    }

    type Draw = fn(&mut Ui<'_>, &mut Pg);
    type Setup = fn(&mut UiHarness, &mut Pg);

    struct Stop {
        name: &'static str,
        draw: Draw,
        want: fn(&Node) -> bool,
        /// At least this many of `KEYS` fire nothing enabled (the instrument check).
        least: usize,
        /// The key of the overlay the bindings are scoped to; `None` is `Global`.
        overlay: Option<&'static str>,
        setup: Option<Setup>,
    }

    fn page_frame(h: &mut UiHarness, pg: &mut Pg, stop: &Stop, at: u64) -> FrameOutput {
        h.at_ms(at);
        let enabled = pg.enabled;
        let (_, out) = h.frame(|ui: &mut Ui<'_>| {
            ui.painter().on(LayerId::Page);
            ui.with_enabled(enabled, |ui| (stop.draw)(ui, pg));
        });
        out
    }

    /// The stop focused by Tab on a page with a binding on every key of `KEYS` (and, with
    /// `chords`, a Command chord on each named key); and the time.
    fn focused(stop: &Stop, chords: bool) -> (UiHarness, Pg, u64) {
        let mut h = UiHarness::new(M11_WIN);
        let scope = stop
            .overlay
            .map_or(Scope::Global, |k| Scope::Overlay(id(k)));
        for k in KEYS {
            h.shortcuts.add(plain_binding(k, scope));
            if chords && let Probe::Named(n) = k {
                h.shortcuts.add(Binding::new(
                    scope,
                    Chord::key(Key::Named(n)).primary(),
                    format!("c-{n:?}"),
                ));
            }
        }
        let mut pg = Pg::new();
        if let Some(setup) = stop.setup {
            setup(&mut h, &mut pg);
        }
        for at in [0, 500, 1000] {
            page_frame(&mut h, &mut pg, stop, at);
        }
        let mut ms = 1000;
        for _ in 0..12 {
            key(&mut h, NamedKey::Tab);
            ms += 100;
            let out = page_frame(&mut h, &mut pg, stop, ms);
            if out
                .focused
                .and_then(|f| out.tree.get(f))
                .is_some_and(stop.want)
            {
                return (h, pg, ms);
            }
        }
        panic!("{}: no Tab stop of that kind", stop.name);
    }

    /// The keys the stop takes while enabled: those that fire no binding.
    fn used_keys(stop: &Stop) -> Vec<Probe> {
        let mut used = Vec::new();
        for k in KEYS {
            let (mut h, mut pg, ms) = focused(stop, false);
            press(&mut h, k);
            if page_frame(&mut h, &mut pg, stop, ms + 100).fired.is_empty() {
                used.push(k);
            }
        }
        used
    }

    fn row(stop: &Stop) -> Vec<String> {
        let used = used_keys(stop);
        assert!(
            used.len() >= stop.least,
            "{} takes at least {} of the keys: {used:?}",
            stop.name,
            stop.least
        );
        let mut leaks = Vec::new();
        for k in used {
            let (mut h, mut pg, mut ms) = focused(stop, false);
            pg.enabled = false;
            let before = pg.state();
            press(&mut h, k);
            ms += 100;
            let out = page_frame(&mut h, &mut pg, stop, ms);
            if !out.fired.is_empty() {
                leaks.push(format!("{}: {k:?} reached {:?}", stop.name, out.fired));
            }
            if pg.state() != before {
                leaks.push(format!("{}: {k:?} changed the page", stop.name));
            }
        }
        leaks
    }

    /// A Command chord on each named key the stop takes is a shortcut, enabled and disabled, and
    /// changes nothing.
    fn chord_row(stop: &Stop) -> Vec<String> {
        let mut bad = Vec::new();
        for k in used_keys(stop) {
            let Probe::Named(n) = k else { continue };
            for enabled in [true, false] {
                let (mut h, mut pg, ms) = focused(stop, true);
                pg.enabled = enabled;
                let before = pg.state();
                press_cmd(&mut h, n);
                let out = page_frame(&mut h, &mut pg, stop, ms + 100);
                let want = [format!("c-{n:?}")];
                let fired: Vec<String> = out.fired.iter().map(|c| format!("{c:?}")).collect();
                if out.fired.len() != 1 || !fired[0].contains(&want[0]) {
                    bad.push(format!(
                        "{}: Command+{n:?} enabled={enabled} fired {fired:?}",
                        stop.name
                    ));
                }
                if pg.state() != before {
                    bad.push(format!(
                        "{}: Command+{n:?} enabled={enabled} changed the page",
                        stop.name
                    ));
                }
            }
        }
        bad
    }

    // ---- the stops --------------------------------------------------------------------------

    fn stepper(ui: &mut Ui<'_>, pg: &mut Pg) {
        let r = ui.stepper(
            "s",
            Rect::new(20.0, 20.0, 140.0, 32.0),
            pg.step,
            STEP_RANGE,
            "Count",
        );
        if let Some(d) = r.step {
            pg.step = (pg.step as i32 + d).max(0) as u32;
        }
    }

    fn slider(ui: &mut Ui<'_>, pg: &mut Pg) {
        let opts = SliderOptions {
            min: 0.0,
            max: 100.0,
            step: 10.0,
            name: "Volume",
        };
        ui.slider(
            "s",
            Rect::new(100.0, 100.0, 400.0, 24.0),
            &mut pg.slide,
            &opts,
        );
    }

    fn picker(ui: &mut Ui<'_>, pg: &mut Pg) {
        let items = ["Apple", "Banana", "Cherry"];
        let opts = PickerOptions {
            trigger: Trigger::Field,
            icon: None,
            placeholder: "Find",
            name: "Fruit",
        };
        ui.search_picker(
            "p",
            Rect::new(20.0, 20.0, 240.0, 32.0),
            &items,
            &mut pg.picked,
            &opts,
        );
    }

    fn tabs(ui: &mut Ui<'_>, pg: &mut Pg) {
        let titles = ["one", "two", "three"];
        let list: Vec<Tab<'_>> = titles
            .iter()
            .enumerate()
            .map(|(i, t)| Tab {
                key: TabKey(i as u64),
                title: t,
                closable: false,
                progress: None,
            })
            .collect();
        let opts = TabStripOptions {
            active: Some(TabKey(pg.active)),
            panel_focused: true,
            busy: None,
            busy_tone: Tone::Neutral,
            card: Rect::new(20.0, 10.0, 600.0, 200.0),
            menu_extra: &[],
            name: "Sessions",
            header_tint: None,
        };
        let events = ui
            .tab_strip("s", Rect::new(20.0, 10.0, 600.0, 32.0), &list, &opts)
            .events;
        for e in events {
            match e {
                TabEvent::Activate(k) => pg.active = k.0,
                other => pg.events.push(format!("{other:?}")),
            }
        }
    }

    fn stars(ui: &mut Ui<'_>, pg: &mut Pg) {
        ui.star_rating("stars", (20.0, 20.0), &mut pg.rating, 5, "Rating");
    }

    fn toolbar(ui: &mut Ui<'_>, pg: &mut Pg) {
        let opts = ToolbarOptions {
            glass: GLASS,
            under: UNDER,
            name: "Main toolbar",
        };
        ui.toolbar(
            "bar",
            Rect::new(0.0, 0.0, 900.0, 52.0),
            &opts,
            |ui, _cols| {
                let r = ui.toolbar_action(
                    "fetch",
                    Rect::new(8.0, 6.0, ACTION_SIZE, ACTION_SIZE),
                    icon::SYNC,
                    "Fetch",
                    true,
                );
                pg.clicks += u32::from(r.clicked);
            },
        );
    }

    fn filter(ui: &mut Ui<'_>, pg: &mut Pg) {
        let opts = FilterOptions {
            placeholder: "Filter",
            label: None,
            clear_unfocusable: false,
        };
        ui.filter_bar(
            "f",
            Rect::new(100.0, 100.0, 300.0, 30.0),
            &mut pg.text,
            &opts,
        );
    }

    fn prompt(message: &str) -> ToastSpec {
        ToastSpec {
            actions: vec![ToastAction {
                label: "Open".into(),
                primary: true,
                id: 2,
            }],
            ..ToastSpec::new(ToastKind::Info, message)
        }
    }

    fn toast_setup(_: &mut UiHarness, pg: &mut Pg) {
        pg.toasts.push(prompt("Update ready"), Duration::ZERO);
    }

    fn toasts(ui: &mut Ui<'_>, pg: &mut Pg) {
        let r = ui.toasts("t", &mut pg.toasts);
        pg.clicks += u32::from(r.action.is_some()) + u32::from(r.cancelled.is_some());
    }

    fn history_setup(_: &mut UiHarness, pg: &mut Pg) {
        let id = pg.toasts.push(prompt("Archived"), Duration::ZERO);
        pg.toasts.dismiss(id, Duration::ZERO);
    }

    fn history(ui: &mut Ui<'_>, pg: &mut Pg) {
        let before = pg.toasts.history().count();
        ui.toast_history("hist", Rect::new(0.0, 0.0, 320.0, 400.0), &mut pg.toasts);
        pg.clicks += u32::try_from(before - pg.toasts.history().count()).unwrap_or(0);
    }

    fn settings(ui: &mut Ui<'_>, pg: &mut Pg) {
        let page = SettingsPage {
            glass: GLASS,
            under: UNDER,
            restart: true,
            name: "General",
        };
        let r = ui.settings_page("page", Rect::new(0.0, 0.0, 800.0, 600.0), &page, |_, _| {
            10.0
        });
        pg.clicks += u32::from(r.restart);
    }

    fn dialog(ui: &mut Ui<'_>, pg: &mut Pg) {
        let d = Dialog {
            title: "Discard changes",
            message: "The document has unsaved changes.",
            extra_h: 0.0,
            confirm: "Discard",
            cancel: Cancel::Default,
        };
        let r = ui.dialog("d", &d, &glass(), |_: &mut Ui<'_>, _, _, _| {});
        pg.clicks += u32::from(r.confirmed) + u32::from(r.cancelled);
    }

    fn empty(ui: &mut Ui<'_>, pg: &mut Pg) {
        let s = EmptyState {
            image: None,
            title: "Nothing open",
            subtitle: "Open a folder.",
            actions: &["Open"],
        };
        pg.clicks += u32::from(
            ui.empty_state("e", Rect::new(0.0, 100.0, 900.0, 500.0), &s)
                .is_some(),
        );
    }

    fn callout(ui: &mut Ui<'_>, pg: &mut Pg) {
        let o = CalloutOptions {
            tone: Tone::Neutral,
            icon: icon::INFO,
            text: "The proxy is offline",
            figure: None,
            action: Some("Retry"),
            announce: false,
        };
        let r = ui.callout("c", Rect::new(20.0, 20.0, 500.0, 0.0), &o);
        pg.clicks += u32::from(r.action.is_some_and(|a| a.clicked));
    }

    fn named(n: &Node, role: Role, name: &str) -> bool {
        n.role == role && n.name == name
    }

    /// The five readers M11 fixed.
    fn readers() -> Vec<Stop> {
        vec![
            Stop {
                name: "stepper",
                draw: stepper,
                want: |n| n.role == Role::SpinButton,
                least: 2,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "slider",
                draw: slider,
                want: |n| n.role == Role::Slider,
                least: 4,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "search picker trigger",
                draw: picker,
                want: |n| n.role == Role::ComboBox,
                least: 2,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "tab strip",
                draw: tabs,
                want: |n| n.role == Role::Tab,
                least: 2,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "star rating",
                draw: stars,
                want: |n| n.name.ends_with(" of 5"),
                least: 2,
                overlay: None,
                setup: None,
            },
        ]
    }

    /// The hosts of `Ui::button` and `icon_button` (Probe rows) and the "not checked" ones.
    fn hosts() -> Vec<Stop> {
        vec![
            Stop {
                name: "dialog confirm",
                draw: dialog,
                want: |n| named(n, Role::Button, "Discard"),
                least: 2,
                overlay: Some("d"),
                setup: None,
            },
            Stop {
                name: "dialog cancel",
                draw: dialog,
                want: |n| named(n, Role::Button, "Cancel"),
                least: 2,
                overlay: Some("d"),
                setup: None,
            },
            Stop {
                name: "toolbar action",
                draw: toolbar,
                want: |n| named(n, Role::Button, "Fetch"),
                least: 2,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "filter bar clear",
                draw: filter,
                want: |n| n.id == id("f").with("clear"),
                least: 2,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "tab strip new tab",
                draw: tabs,
                want: |n| named(n, Role::Button, "New tab"),
                least: 2,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "toast action",
                draw: toasts,
                want: |n| named(n, Role::Button, "Open"),
                least: 2,
                overlay: None,
                setup: Some(toast_setup),
            },
            Stop {
                name: "toast dismiss",
                draw: toasts,
                want: |n| named(n, Role::Button, "Dismiss"),
                least: 2,
                overlay: None,
                setup: Some(toast_setup),
            },
            Stop {
                name: "settings restart",
                draw: settings,
                want: |n| named(n, Role::Button, "Restart now"),
                least: 2,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "empty state action",
                draw: empty,
                want: |n| named(n, Role::Button, "Open"),
                least: 2,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "callout action",
                draw: callout,
                want: |n| named(n, Role::Button, "Retry"),
                least: 2,
                overlay: None,
                setup: None,
            },
            Stop {
                name: "toast history clear",
                draw: history,
                want: |n| named(n, Role::Button, "Clear all"),
                least: 2,
                overlay: None,
                setup: Some(history_setup),
            },
        ]
    }

    #[test]
    fn a_disabled_reader_takes_every_key_it_takes_enabled_and_fires_no_global_binding() {
        let mut leaks = Vec::new();
        for stop in readers() {
            leaks.extend(row(&stop));
        }
        assert_eq!(leaks, Vec::<String>::new());
    }

    #[test]
    fn a_disabled_host_of_a_button_takes_enter_and_space_and_fires_no_binding() {
        let mut leaks = Vec::new();
        for stop in hosts() {
            leaks.extend(row(&stop));
        }
        assert_eq!(leaks, Vec::<String>::new());
    }

    #[test]
    fn a_chord_on_the_keys_of_the_five_readers_is_a_shortcut_enabled_and_disabled() {
        let mut bad = Vec::new();
        for stop in readers() {
            bad.extend(chord_row(&stop));
        }
        assert_eq!(bad, Vec::<String>::new());
    }
}
