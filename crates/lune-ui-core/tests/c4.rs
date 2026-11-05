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
            scene.layers[LayerId::Popup as usize]
                .geometry
                .under
                .is_empty(),
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
            !scene.layers[LayerId::Popup as usize]
                .geometry
                .under
                .is_empty(),
            "{tier:?}: the pick frame still drew the list"
        );
        assert_eq!(out.repaint, Repaint::Now, "{tier:?}: pick frame");
        let (scene, _) = at(&mut h, &mut app, 2116);
        assert!(
            scene.layers[LayerId::Popup as usize]
                .geometry
                .under
                .is_empty(),
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
    use lune_ui_core::widgets::tree_view::{TreeOptions, TreeRow};

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
                        active: true,
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
                        active: true,
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
                        active: true,
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
        let raised = &last.0.layers[LayerId::Raised as usize];
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
