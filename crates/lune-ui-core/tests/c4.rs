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
