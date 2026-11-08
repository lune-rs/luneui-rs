//! The banner and the empty view (`widget.banner-empty-state` acceptance): the banner's look, its
//! wrapped height and spinner, the empty column's layout, the actions' result, press and hover
//! motion per tier, pixel probes at scale 2, and the semantic nodes.

use lune_ui_core::access::{Action, Live, Role, SemanticTree, State};
use lune_ui_core::color::premul;
use lune_ui_core::image::{Fit, ImageAlign, ImageId};
use lune_ui_core::input::{PointerButton, Repaint};
use lune_ui_core::motion::MotionTier;
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{ImageSource, Scene};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Spec, Style, TextKey, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::banner::{
    BANNER_BORDER, BANNER_GAP, BANNER_ICON, BANNER_PAD_X, BANNER_PAD_Y, BANNER_RADIUS,
    EMPTY_ACTION_GAP, EMPTY_ACTION_HEIGHT, EMPTY_IMAGE, EMPTY_MAX_WIDTH, EMPTY_SPACING, EmptyState,
};

const WIN: (f32, f32) = (900.0, 700.0);
const BANNER: Rect = Rect::new(20.0, 20.0, 400.0, 0.0);
const VIEW: Rect = Rect::new(0.0, 100.0, 900.0, 500.0);
const PIC: ImageId = ImageId(7);

fn id(key: &str) -> WidgetId {
    WidgetId::ROOT.with(key)
}

fn text_size(spec: &Spec) -> (f32, f32) {
    FakeText.measure(spec, 1.0)
}

fn banner_frame(h: &mut UiHarness, text: &str, busy: bool) -> (Scene, f32, SemanticTree, Repaint) {
    let mut height = 0.0;
    let (scene, out) = h.frame(|ui| height = ui.banner("b", BANNER, text, busy));
    (scene, height, out.tree, out.repaint)
}

// ---- Banner ----

#[test]
fn the_banner_is_the_warning_fill_with_a_one_px_light_border() {
    let mut h = UiHarness::new(WIN);
    let (scene, height, _, _) = banner_frame(&mut h, "Indexing", true);
    let g = &scene.layers[0].geometry;
    assert_eq!(g.under.len(), 2, "fill and border");
    let (fill, border) = (g.under[0], g.under[1]);
    assert_eq!(fill.color, premul(DARK.status_warning_dark));
    assert_eq!(border.color, premul(DARK.status_warning_light));
    for s in [fill, border] {
        assert_eq!(s.rect, [20.0, 20.0, 400.0, height]);
        assert_eq!(s.radii, [BANNER_RADIUS; 4]);
    }
    assert_eq!(border.params[0], 1.0, "a ring");
    assert_eq!(border.params[1], BANNER_BORDER);
}

#[test]
fn a_one_line_banner_is_the_icon_and_padding_high_and_a_long_one_wraps() {
    let mut h = UiHarness::new(WIN);
    let (_, one, _, _) = banner_frame(&mut h, "Indexing", true);
    let line = Style::ui(DARK.font_base, DARK.status_warning_light).line;
    assert_eq!(
        one,
        line.max(BANNER_ICON) + 2.0 * (BANNER_PAD_Y + BANNER_BORDER)
    );
    let long = "word ".repeat(60);
    let (scene, many, _, _) = banner_frame(&mut h, &long, true);
    assert!(many > one + line, "{many} vs {one}");
    // The text is as tall as the wrapped spec and centred in the bar.
    let tw = 400.0 - 2.0 * (BANNER_BORDER + BANNER_PAD_X) - BANNER_ICON - BANNER_GAP;
    let spec = Spec::plain(
        long.as_str(),
        Style::ui(DARK.font_base, DARK.status_warning_light),
    )
    .wrapped(tw);
    let th = text_size(&spec).1;
    assert_eq!(many, th + 2.0 * (BANNER_PAD_Y + BANNER_BORDER));
    let items = &scene.layers[0].texts;
    let text = items
        .iter()
        .find(|t| t.key == TextKey(spec.cache_key(1.0)))
        .expect("the wrapped text");
    assert_eq!(
        text.x,
        20.0 + BANNER_BORDER + BANNER_PAD_X + BANNER_ICON + BANNER_GAP
    );
    assert_eq!(text.y, 20.0 + BANNER_PAD_Y + BANNER_BORDER);
}

#[test]
fn the_banner_spins_while_busy_and_holds_still_when_not() {
    let mut h = UiHarness::new(WIN);
    banner_frame(&mut h, "Indexing", true);
    h.at_ms(300);
    let (scene, _, _, repaint) = banner_frame(&mut h, "Indexing", true);
    assert_eq!(repaint, Repaint::Now, "a running spinner is moving");
    let turned = &scene.layers[0].images;
    assert_eq!(turned.len(), 1, "the glyph is a rotated text image");
    assert!(turned[0].rotation > 0.0);

    let mut h = UiHarness::new(WIN);
    banner_frame(&mut h, "Indexing", false);
    h.at_ms(300);
    let (_, _, _, repaint) = banner_frame(&mut h, "Indexing", false);
    assert_eq!(repaint, Repaint::Idle);
}

#[test]
fn the_spinner_glyph_is_the_warning_light_colour() {
    use lune_ui_core::icon;
    let mut h = UiHarness::new(WIN);
    h.tier = MotionTier::Reduced;
    let (scene, _, _, _) = banner_frame(&mut h, "Indexing", true);
    // Reduced: a static glyph at angle 0, in the banner's colour and the 14 px size.
    let want =
        TextKey(icon::glyph(icon::SYNC, BANNER_ICON, DARK.status_warning_light).cache_key(1.0));
    let img = scene.layers[0].images[0];
    assert!(matches!(img.source, ImageSource::Text(k, _) if k == want));
    assert_eq!(img.rotation, 0.0);
}

#[test]
fn the_banner_node_is_a_polite_status_named_by_the_text() {
    let mut h = UiHarness::new(WIN);
    let (_, height, tree, _) = banner_frame(&mut h, "Indexing", true);
    let node = tree.get(id("b")).expect("node");
    assert_eq!((node.role, node.name.as_str()), (Role::Status, "Indexing"));
    assert_eq!(node.live, Some(Live::Polite));
    assert_eq!(node.bounds, Rect::new(20.0, 20.0, 400.0, height));
    assert!(!node.states.contains(State::Focusable), "no input");
    assert!(!node.actions.contains(Action::Default));
    // The spinner is its child and busy.
    let child = tree.get(id("b").with("spinner")).expect("spinner node");
    assert_eq!(child.parent, Some(id("b")));
    assert!(child.states.contains(State::Busy));
}

#[test]
fn pixel_probes_at_scale_two_banner() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let (scene, height, _, _) = banner_frame(&mut h, "Indexing", false);
    let g = &scene.layers[0].geometry;
    assert_eq!(g.under[0].radii, [12.0; 4], "radius 12 px");
    assert_eq!(g.under[1].params[1], 2.0, "border 2 px");
    assert_eq!(g.under[0].rect, [40.0, 40.0, 800.0, height * 2.0]);
}

// ---- Empty view ----

const SUBTITLE: &str = "Open a folder to begin, or create a new repository to start tracking your work from the first commit.";

fn state<'a>(image: Option<ImageId>, actions: &'a [&'a str]) -> EmptyState<'a> {
    EmptyState {
        image,
        title: "No repository open",
        subtitle: SUBTITLE,
        actions,
    }
}

struct View {
    scene: Scene,
    tree: SemanticTree,
    repaint: Repaint,
    clicked: Option<usize>,
}

fn view_frame(h: &mut UiHarness, s: &EmptyState<'_>) -> View {
    let mut clicked = None;
    let (scene, out) = h.frame(|ui| clicked = ui.empty_state("e", VIEW, s));
    View {
        scene,
        tree: out.tree,
        repaint: out.repaint,
        clicked,
    }
}

fn title_style() -> Style {
    Style {
        line: DARK.line_title,
        ..Style::ui(DARK.font_title, DARK.fg_primary)
    }
}

fn sub_style() -> Style {
    Style::ui(DARK.font_base, DARK.fg_secondary)
}

const ACTIONS: [&str; 2] = ["Open", "Create"];

#[test]
fn the_column_is_centred_with_the_entrys_sizes_and_spacing() {
    let mut h = UiHarness::new(WIN);
    let v = view_frame(&mut h, &state(Some(PIC), &ACTIONS));
    let col_w = EMPTY_MAX_WIDTH;
    let col_x = (900.0 - col_w) / 2.0;
    let title_h = text_size(&Spec::plain("No repository open", title_style()).wrapped(col_w)).1;
    let sub_h = text_size(&Spec::plain(SUBTITLE, sub_style()).wrapped(col_w)).1;
    let total = EMPTY_IMAGE + title_h + sub_h + EMPTY_ACTION_HEIGHT + 3.0 * EMPTY_SPACING;
    let top = 100.0 + (500.0 - total) / 2.0;

    let pic = &v.scene.layers[0].images;
    assert_eq!(pic.len(), 1);
    assert_eq!(pic[0].dst, Rect::new(450.0 - 50.0, top, 100.0, 100.0));
    assert_eq!(
        (pic[0].fit, pic[0].align),
        (Fit::Contain, ImageAlign::CENTER)
    );

    let title = v.tree.get(id("e").with("title")).expect("title");
    let sub = v.tree.get(id("e").with("subtitle")).expect("subtitle");
    assert_eq!(title.bounds, Rect::new(col_x, top + 124.0, col_w, title_h));
    assert_eq!(
        sub.bounds,
        Rect::new(col_x, top + 124.0 + title_h + 24.0, col_w, sub_h)
    );
    // Actions: secondary buttons 51 high (E1: padding 16, a 17 px line, a 1 px border), 12 apart,
    // centred as a row.
    let a0 = v.tree.get(id("e").with("action-0")).expect("a0").bounds;
    let a1 = v.tree.get(id("e").with("action-1")).expect("a1").bounds;
    let ay = top + 124.0 + title_h + 24.0 + sub_h + 24.0;
    assert_eq!((a0.y, a0.h, a1.y, a1.h), (ay, 51.0, ay, 51.0));
    assert!(a0.w >= 140.0 && a1.w >= 140.0, "at least 140 wide");
    assert_eq!(a1.x - (a0.x + a0.w), EMPTY_ACTION_GAP);
    assert!(((a0.x + a1.x + a1.w) / 2.0 - 450.0).abs() < 1e-3, "centred");
}

#[test]
fn the_column_is_never_wider_than_520_or_the_rect() {
    let mut h = UiHarness::new(WIN);
    let narrow = Rect::new(100.0, 100.0, 300.0, 500.0);
    let s = state(None, &[]);
    h.frame(|ui| {
        ui.empty_state("e", narrow, &s);
    });
    let (_, out) = h.frame(|ui| {
        ui.empty_state("e", narrow, &s);
    });
    let sub = out.tree.get(id("e").with("subtitle")).expect("subtitle");
    assert_eq!((sub.bounds.x, sub.bounds.w), (100.0, 300.0));
}

#[test]
fn the_subtitle_rows_are_each_centred() {
    let mut h = UiHarness::new(WIN);
    let v = view_frame(&mut h, &state(None, &[]));
    let spec = Spec::plain(SUBTITLE, sub_style()).wrapped(EMPTY_MAX_WIDTH);
    let rows = FakeText.lines(&spec, 1.0);
    assert!(rows.len() >= 2, "it wraps");
    for row in rows {
        let text = SUBTITLE[row.start..row.end].trim();
        let r = Spec::plain(text, sub_style());
        let item = v.scene.layers[0]
            .texts
            .iter()
            .find(|t| t.key == TextKey(r.cache_key(1.0)))
            .expect("row drawn on its own");
        let w = text_size(&r).0;
        assert!(
            (item.x + w / 2.0 - 450.0).abs() < 1e-3,
            "row {text:?} centred on the column"
        );
    }
}

#[test]
fn the_picture_node_is_hidden_and_none_means_no_picture() {
    let mut h = UiHarness::new(WIN);
    let v = view_frame(&mut h, &state(Some(PIC), &ACTIONS));
    let n = v.tree.get(id("e").with("image")).expect("image node");
    assert_eq!(n.role, Role::Image);
    assert!(n.states.contains(State::Hidden));
    assert!(n.name.is_empty());

    let v = view_frame(&mut h, &state(None, &ACTIONS));
    assert!(v.scene.layers[0].images.is_empty());
    assert!(v.tree.get(id("e").with("image")).is_none());
}

#[test]
fn title_and_subtitle_are_static_text_and_actions_are_buttons() {
    let mut h = UiHarness::new(WIN);
    let v = view_frame(&mut h, &state(Some(PIC), &ACTIONS));
    let title = v.tree.get(id("e").with("title")).expect("title");
    assert_eq!(
        (title.role, title.name.as_str()),
        (Role::StaticText, "No repository open")
    );
    let sub = v.tree.get(id("e").with("subtitle")).expect("subtitle");
    assert_eq!((sub.role, sub.name.as_str()), (Role::StaticText, SUBTITLE));
    for (i, name) in ACTIONS.iter().enumerate() {
        let n = v
            .tree
            .get(id("e").with(format!("action-{i}")))
            .expect("button");
        assert_eq!((n.role, n.name.as_str()), (Role::Button, *name));
        assert!(n.actions.contains(Action::Default));
    }
}

#[test]
fn an_action_click_reports_its_index_once() {
    for want in 0..ACTIONS.len() {
        let mut h = UiHarness::new(WIN);
        let s = state(Some(PIC), &ACTIONS);
        let v = view_frame(&mut h, &s);
        let b = v
            .tree
            .get(id("e").with(format!("action-{want}")))
            .expect("button")
            .bounds;
        h.input
            .pointer_to(b.x + b.w / 2.0, b.y + b.h / 2.0)
            .click(PointerButton::Primary);
        h.at_ms(10);
        assert_eq!(view_frame(&mut h, &s).clicked, Some(want));
        h.at_ms(20);
        assert_eq!(view_frame(&mut h, &s).clicked, None);
    }
}

#[test]
fn pixel_probes_at_scale_two_empty_view() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let v = view_frame(&mut h, &state(Some(PIC), &ACTIONS));
    let dst = v.scene.layers[0].images[0].dst;
    assert_eq!((dst.w, dst.h), (200.0, 200.0), "icon 200 x 200 px");
}

#[test]
fn each_action_has_a_one_px_border_ring_on_the_button_rect() {
    let mut h = UiHarness::new(WIN);
    let v = view_frame(&mut h, &state(None, &ACTIONS));
    let g = &v.scene.layers[0].geometry;
    for i in 0..ACTIONS.len() {
        let b = v
            .tree
            .get(id("e").with(format!("action-{i}")))
            .expect("button")
            .bounds;
        let on_rect = g.under.iter().any(|s| {
            s.params[0] == 1.0
                && s.params[1] == 1.0
                && s.color == premul(DARK.border)
                && s.rect == [b.x, b.y, b.w, b.h]
                && s.radii == [DARK.item_radius; 4]
        });
        assert!(on_rect, "action {i}: a 1 px ring, radius item_radius");
    }
}

fn press_scale(tier: MotionTier, after_ms: u64) -> (f32, Repaint) {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let s = state(None, &ACTIONS);
    let v = view_frame(&mut h, &s);
    let b = v.tree.get(id("e").with("action-0")).expect("button").bounds;
    h.input.pointer_to(b.x + b.w / 2.0, b.y + b.h / 2.0);
    h.at_ms(10);
    view_frame(&mut h, &s);
    h.input.press(PointerButton::Primary);
    h.at_ms(20);
    view_frame(&mut h, &s);
    h.at_ms(20 + after_ms);
    let v = view_frame(&mut h, &s);
    // The pressed button's fill is the first shape: its width follows the press scale.
    let fill = v.scene.layers[0].geometry.under[0];
    (fill.rect[2] / b.w, v.repaint)
}

#[test]
fn the_action_press_scale_settles_within_180_ms_and_reduced_snaps_it() {
    let (mid, repaint) = press_scale(MotionTier::Full, 90);
    assert!(mid < 1.0 && mid > 0.98, "{mid}");
    assert_eq!(repaint, Repaint::Now);
    let (done, repaint) = press_scale(MotionTier::Full, 200);
    assert!((done - 0.98).abs() < 1e-4, "{done}");
    assert_eq!(repaint, Repaint::Idle);
    let (snap, _) = press_scale(MotionTier::Reduced, 0);
    assert!((snap - 0.98).abs() < 1e-4, "{snap}");
    let (off, repaint) = press_scale(MotionTier::Off, 0);
    assert!((off - 0.98).abs() < 1e-4, "{off}");
    assert_eq!(repaint, Repaint::Idle);
}

#[test]
fn the_action_hover_fill_tweens_over_instant_and_a_settled_view_is_idle() {
    let mut h = UiHarness::new(WIN);
    let s = state(Some(PIC), &ACTIONS);
    let v = view_frame(&mut h, &s);
    let b = v.tree.get(id("e").with("action-0")).expect("button").bounds;
    h.input.pointer_to(b.x + b.w / 2.0, b.y + b.h / 2.0);
    h.at_ms(10);
    view_frame(&mut h, &s);
    h.at_ms(70);
    let mid = view_frame(&mut h, &s);
    assert_eq!(mid.repaint, Repaint::Now);
    h.at_ms(500);
    let done = view_frame(&mut h, &s);
    assert_eq!(done.repaint, Repaint::Idle);
    let fill = done.scene.layers[0].geometry.under[0];
    assert_eq!(fill.color, premul(DARK.surface_hover));
}
