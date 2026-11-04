//! Static text (C4 retrofit, `widget.label` acceptance): the roles' styles, rest identity with the
//! Painter API for the `controls` lines, and the static-text node.

use lune_ui_core::access::{Role, Value};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Font, Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::label::LabelRole;

const WIN: (f32, f32) = (400.0, 300.0);

/// The three `controls` lines the legacy scene draws (base, title, small) at fixed points.
fn old_scene() -> Scene {
    let t = DARK;
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    p.text(
        &Spec::plain("Primary text", Style::ui(t.font_base, t.fg_primary)),
        20.0,
        20.0,
    );
    p.text(
        &Spec::plain("Title text", Style::ui(t.font_m, t.fg_primary).medium()),
        20.0,
        44.0,
    );
    p.text(
        &Spec::plain("Small caption", Style::ui(t.font_s, t.fg_tertiary)),
        20.0,
        72.0,
    );
    p.scene
}

#[test]
fn the_controls_label_lines_draw_exactly_what_the_painter_text_does() {
    let mut h = UiHarness::new(WIN);
    let (scene, _) = h.frame(|ui| {
        ui.label("base", (20.0, 20.0), "Primary text", LabelRole::Base);
        ui.label("title", (20.0, 44.0), "Title text", LabelRole::Section);
        ui.label("small", (20.0, 72.0), "Small caption", LabelRole::Small);
    });
    assert_eq!(scene, old_scene());
}

#[test]
fn each_role_draws_with_its_label_md_size_weight_and_colour() {
    let t = DARK;
    let base = Style::ui(t.font_base, t.fg_primary);
    assert_eq!(LabelRole::Base.style(&t), base);
    let small = LabelRole::Small.style(&t);
    assert_eq!((small.size, small.color), (11.0, t.fg_tertiary));
    let section = LabelRole::Section.style(&t);
    assert_eq!((section.size, section.weight), (14.0, 500));
    let title = LabelRole::Title.style(&t);
    assert_eq!(
        (title.size, title.line, title.weight, title.color),
        (18.0, 24.0, 500, t.fg_bright)
    );
    let mono = LabelRole::Mono.style(&t);
    assert_eq!((mono.font, mono.size), (Font::Mono, 13.0));
    let strong = LabelRole::Strong.style(&t);
    assert_eq!(
        (strong.size, strong.weight, strong.color),
        (13.0, 500, t.fg_bright)
    );
}

#[test]
fn the_node_is_static_text_named_and_valued_by_the_text() {
    let mut h = UiHarness::new(WIN);
    let (_, out) = h.frame(|ui| {
        let (w, height) = ui.label("note", (20.0, 20.0), "Hello", LabelRole::Base);
        assert!(w > 0.0 && height > 0.0);
    });
    let node = out
        .tree
        .get(WidgetId::ROOT.with("note"))
        .expect("the label's node");
    assert_eq!(node.role, Role::StaticText);
    assert_eq!(node.name, "Hello");
    assert_eq!(node.value, Some(Value::Text("Hello".to_owned())));
}
