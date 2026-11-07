//! The code block (`text.code-block`, D10): its height, the copy button and its feedback, spans,
//! selection, the Tab order, the pixel probes at scale 2, the transitions under the three tiers,
//! the nested wheel and the nodes.

use std::time::Duration;

use lune_ui_core::access::{Action, Live, Role, State};
use lune_ui_core::color::{Rgba, premul};
use lune_ui_core::icon;
use lune_ui_core::input::{
    ClipboardRequest, FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint,
};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, ScriptedInput, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style, TextKey, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button;
use lune_ui_core::widgets::code_block::{COPY_FADE, CodeBlockOptions, CodeBlockResponse, height};
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};
use lune_ui_core::widgets::text_lines::HighlightSpan;

const WIN: (f32, f32) = (500.0, 400.0);
const RECT: Rect = Rect::new(20.0, 20.0, 300.0, 100.0);
const P: PointerButton = PointerButton::Primary;
const META: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: true,
};
const TEXT: &str = "alpha\nbeta\ngamma";

fn root() -> WidgetId {
    WidgetId::ROOT.with("b")
}

fn body() -> WidgetId {
    root().with("body")
}

fn copy() -> WidgetId {
    root().with("copy")
}

fn status() -> WidgetId {
    root().with("copied")
}

/// The copy button with the defaults: 21 px, 10 from the top and right of the box.
fn button_at() -> (f32, f32) {
    (RECT.right() - 10.0 - 10.5, RECT.y + 10.0 + 10.5)
}

fn opts() -> CodeBlockOptions<'static> {
    CodeBlockOptions {
        label: "Snippet",
        ..CodeBlockOptions::default()
    }
}

struct Run {
    scene: Scene,
    r: CodeBlockResponse,
    out: FrameOutput,
}

fn run(h: &mut UiHarness, text: &str, opts: &CodeBlockOptions<'_>) -> Run {
    let mut r = None;
    let (scene, out) = h.frame(|ui| r = Some(ui.code_block("b", RECT, text, opts)));
    Run {
        scene,
        r: r.expect("ran"),
        out,
    }
}

fn at(h: &mut UiHarness, ms: u64, text: &str, opts: &CodeBlockOptions<'_>) -> Run {
    h.at_ms(ms);
    run(h, text, opts)
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// A harness whose first frame is drawn, then a click on the copy button at `click_ms`.
fn clicked(text: &str, click_ms: u64) -> (UiHarness, Run) {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, text, &o);
    let (x, y) = button_at();
    h.input.pointer_to(x, y);
    at(&mut h, click_ms - 1, text, &o);
    h.input.click(P);
    let r = at(&mut h, click_ms, text, &o);
    (h, r)
}

fn key_of(spec: &Spec, scale: f32) -> TextKey {
    FakeText.prepare(spec, scale).0
}

fn glyph_key(glyph: &str, color: Rgba) -> TextKey {
    key_of(&icon::glyph(glyph, 13.0, color), 1.0)
}

fn drawn(scene: &Scene, key: TextKey) -> bool {
    scene.layers[0].texts.iter().any(|t| t.key == key)
}

/// Whether the check mark shows in full.
fn check_shows(scene: &Scene) -> bool {
    drawn(scene, glyph_key(icon::CHECK, DARK.fg_primary))
}

fn copy_icon_shows(scene: &Scene) -> bool {
    drawn(scene, glyph_key(icon::COPY, DARK.fg_tertiary))
}

fn chord(h: &mut UiHarness, c: &str) {
    h.input.set_modifiers(META);
    h.input.key_down(Key::Character(c.to_owned()), None);
    h.input.set_modifiers(Modifiers::default());
}

fn press(h: &mut UiHarness, key: NamedKey) {
    h.input.key_down(Key::Named(key), None);
}

// Height.

#[test]
fn the_height_is_the_lines_the_padding_and_the_border() {
    let mut h = UiHarness::new(WIN);
    h.frame(|ui| {
        let o = opts();
        assert_eq!(height(ui, "a", &o), 14.0 + 20.0);
        assert_eq!(height(ui, "a\nb\nc", &o), 3.0 * 14.0 + 20.0);
        // A final terminator opens a line, and `\r\n` and a lone `\r` end one each.
        assert_eq!(height(ui, "a\n", &o), 2.0 * 14.0 + 20.0);
        assert_eq!(height(ui, "a\r\nb\rc", &o), 3.0 * 14.0 + 20.0);
        // An empty block is one empty line between its paddings.
        assert_eq!(height(ui, "", &o), 14.0 + 20.0);
        let roomy = CodeBlockOptions {
            size: 13.0,
            line: 20.0,
            pad: (12.0, 10.0),
            border: true,
            ..o
        };
        assert_eq!(height(ui, "a\nb", &roomy), 2.0 * 20.0 + 2.0 * 11.0);
    });
}

#[test]
fn the_defaults_are_the_compact_look() {
    let o = CodeBlockOptions::default();
    assert_eq!((o.size, o.pad, o.border), (11.0, (10.0, 10.0), false));
    assert_eq!((o.feedback_ms, o.tab_stop), (1_500, true));
}

// Copy and its feedback.

#[test]
fn a_click_on_copy_writes_the_whole_text_and_the_check_shows_until_feedback_ms_exactly() {
    let (mut h, r) = clicked(TEXT, 100);
    assert!(r.r.copied);
    assert_eq!(
        r.out.clipboard,
        vec![ClipboardRequest::Write(TEXT.to_owned())]
    );
    let o = opts();
    // The crossfade settles; the frame then asks for the end of the feedback.
    let r = at(&mut h, 300, TEXT, &o);
    assert!(!r.r.copied);
    assert!(r.out.clipboard.is_empty());
    assert!(check_shows(&r.scene));
    assert!(!copy_icon_shows(&r.scene));
    assert_eq!(r.out.repaint, Repaint::At(ms(1_600)));

    let r = at(&mut h, 1_599, TEXT, &o);
    assert!(check_shows(&r.scene));
    assert!(r.out.tree.get(status()).is_some());
    let r = at(&mut h, 1_600, TEXT, &o);
    assert!(r.out.tree.get(status()).is_none(), "ended at 1,600 exactly");
    assert_eq!(r.out.repaint, Repaint::Now, "the icon crossfades back");
    let r = at(&mut h, 1_800, TEXT, &o);
    assert!(copy_icon_shows(&r.scene));
    assert!(!check_shows(&r.scene));
    assert_eq!(r.out.repaint, Repaint::Idle);
}

#[test]
fn the_feedback_length_is_the_options() {
    let o = CodeBlockOptions {
        feedback_ms: 400,
        ..opts()
    };
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, TEXT, &o);
    let (x, y) = button_at();
    h.input.pointer_to(x, y);
    at(&mut h, 99, TEXT, &o);
    h.input.click(P);
    at(&mut h, 100, TEXT, &o);
    let r = at(&mut h, 499, TEXT, &o);
    assert!(r.out.tree.get(status()).is_some());
    let r = at(&mut h, 500, TEXT, &o);
    assert!(r.out.tree.get(status()).is_none());
}

#[test]
fn a_second_copy_during_the_feedback_restarts_it() {
    let (mut h, _) = clicked(TEXT, 100);
    let o = opts();
    at(&mut h, 1_000, TEXT, &o);
    h.input.click(P);
    let r = at(&mut h, 1_001, TEXT, &o);
    assert!(r.r.copied);
    assert_eq!(
        r.out.clipboard,
        vec![ClipboardRequest::Write(TEXT.to_owned())]
    );
    let r = at(&mut h, 2_500, TEXT, &o);
    assert!(r.out.tree.get(status()).is_some(), "ends at 1,001 + 1,500");
    assert_eq!(r.out.repaint, Repaint::At(ms(2_501)));
    let r = at(&mut h, 2_501, TEXT, &o);
    assert!(r.out.tree.get(status()).is_none());
}

#[test]
fn a_change_of_text_ends_the_feedback() {
    let (mut h, _) = clicked(TEXT, 100);
    let o = opts();
    let r = at(&mut h, 300, "other", &o);
    assert!(r.out.tree.get(status()).is_none());
    assert!(r.out.clipboard.is_empty());
}

#[test]
fn the_status_node_is_polite_and_there_only_while_the_check_shows() {
    let (mut h, r) = clicked(TEXT, 100);
    let n = r.out.tree.get(status()).expect("status");
    assert_eq!(n.role, Role::Status);
    assert_eq!(n.live, Some(Live::Polite));
    assert_eq!(n.name, "Copied");
    let r = at(&mut h, 2_000, TEXT, &opts());
    assert!(r.out.tree.get(status()).is_none());
}

#[test]
fn enter_and_space_on_the_focused_button_copy() {
    for key in [NamedKey::Enter, NamedKey::Space] {
        let o = opts();
        let mut h = UiHarness::new(WIN);
        at(&mut h, 0, TEXT, &o);
        press(&mut h, NamedKey::Tab);
        at(&mut h, 10, TEXT, &o);
        press(&mut h, NamedKey::Tab);
        let r = at(&mut h, 20, TEXT, &o);
        assert_eq!(r.out.focused, Some(copy()), "{key:?}");
        press(&mut h, key);
        let r = at(&mut h, 30, TEXT, &o);
        assert!(r.r.copied, "{key:?}");
        assert_eq!(
            r.out.clipboard,
            vec![ClipboardRequest::Write(TEXT.to_owned())]
        );
    }
}

#[test]
fn an_empty_block_disables_copy_and_emits_nothing() {
    let (mut h, r) = clicked("", 100);
    assert!(!r.r.copied);
    assert!(r.out.clipboard.is_empty());
    let n = r.out.tree.get(copy()).expect("button");
    assert!(n.states.contains(State::Disabled));
    assert!(!n.actions.contains(Action::Default));
    assert!(r.out.tree.get(status()).is_none());
    // Enter on a focused button copies nothing either.
    press(&mut h, NamedKey::Enter);
    let r = at(&mut h, 200, "", &opts());
    assert!(r.out.clipboard.is_empty());
    assert!(!check_shows(&r.scene));
}

#[test]
fn the_copy_button_is_a_named_button_whose_default_action_copies() {
    let mut h = UiHarness::new(WIN);
    let r = at(&mut h, 0, TEXT, &opts());
    let n = r.out.tree.get(copy()).expect("button");
    assert_eq!(n.role, Role::Button);
    assert_eq!(n.name, "Copy code");
    assert!(n.actions.contains(Action::Default));
    assert!(n.states.contains(State::Focusable));
}

#[test]
fn the_pointer_over_the_copy_button_fills_it_with_surface_hover() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, TEXT, &o);
    assert!(shapes_of(&at(&mut h, 10, TEXT, &o).scene, DARK.surface_hover).is_empty());
    let (x, y) = button_at();
    h.input.pointer_to(x, y);
    at(&mut h, 20, TEXT, &o);
    let r = at(&mut h, 400, TEXT, &o);
    let fills = shapes_of(&r.scene, DARK.surface_hover);
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].rect, [289.0, 30.0, 21.0, 21.0]);
}

// The text: selection, Tab, spans.

#[test]
fn a_drag_selects_and_the_shortcut_copies_the_selection_not_the_whole() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, TEXT, &o);
    h.input.pointer_to(30.5, 37.0).press(P);
    at(&mut h, 10, TEXT, &o);
    h.input.pointer_to(30.5, 51.0);
    at(&mut h, 20, TEXT, &o);
    h.input.release(P);
    let r = at(&mut h, 30, TEXT, &o);
    assert_eq!(r.out.focused, Some(body()));
    chord(&mut h, "c");
    let r = at(&mut h, 40, TEXT, &o);
    assert_eq!(
        r.out.clipboard,
        vec![ClipboardRequest::Write("alpha\n".to_owned())]
    );
}

#[test]
fn select_all_and_copy_take_the_whole_text() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, TEXT, &o);
    press(&mut h, NamedKey::Tab);
    let r = at(&mut h, 10, TEXT, &o);
    assert_eq!(r.out.focused, Some(body()));
    chord(&mut h, "a");
    at(&mut h, 20, TEXT, &o);
    chord(&mut h, "c");
    let r = at(&mut h, 30, TEXT, &o);
    assert_eq!(
        r.out.clipboard,
        vec![ClipboardRequest::Write(TEXT.to_owned())]
    );
}

#[test]
fn the_text_is_the_first_tab_stop_and_the_button_the_next() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, TEXT, &o);
    press(&mut h, NamedKey::Tab);
    assert_eq!(at(&mut h, 10, TEXT, &o).out.focused, Some(body()));
    press(&mut h, NamedKey::Tab);
    assert_eq!(at(&mut h, 20, TEXT, &o).out.focused, Some(copy()));
}

#[test]
fn tab_stop_false_keeps_the_text_off_the_tab_order_and_a_press_still_focuses_it() {
    let o = CodeBlockOptions {
        tab_stop: false,
        ..opts()
    };
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, TEXT, &o);
    press(&mut h, NamedKey::Tab);
    assert_eq!(at(&mut h, 10, TEXT, &o).out.focused, Some(copy()));

    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, TEXT, &o);
    h.input.pointer_to(40.0, 37.0).click(P);
    assert_eq!(at(&mut h, 10, TEXT, &o).out.focused, Some(body()));
}

const RED: Rgba = [255, 0, 0, 255];

fn mono() -> Style {
    Style::mono(11.0, 14.0, DARK.fg_primary)
}

fn colored(runs: &[(&str, bool)]) -> TextKey {
    let spec = Spec {
        runs: runs
            .iter()
            .map(|(t, red)| {
                let style = Style {
                    color: if *red { RED } else { DARK.fg_primary },
                    ..mono()
                };
                ((*t).to_owned(), style)
            })
            .collect(),
        wrap: None,
    };
    key_of(&spec, 1.0)
}

fn span(start: usize, end: usize) -> HighlightSpan {
    HighlightSpan {
        start,
        end,
        color: RED,
    }
}

#[test]
fn spans_clamp_empty_and_inverted_ones_draw_nothing_and_a_missing_line_is_ignored() {
    let cases: [(Vec<(usize, HighlightSpan)>, TextKey); 5] = [
        (
            vec![(0, span(3, 99))],
            colored(&[("012", false), ("3456789", true)]),
        ),
        (vec![(0, span(5, 5))], colored(&[("0123456789", false)])),
        (vec![(0, span(7, 2))], colored(&[("0123456789", false)])),
        // Line 50 of a block of five lines.
        (vec![(50, span(0, 4))], colored(&[("0123456789", false)])),
        (
            vec![(0, span(0, 3))],
            colored(&[("012", true), ("3456789", false)]),
        ),
    ];
    for (spans, want) in cases {
        let o = CodeBlockOptions {
            spans: &spans,
            ..opts()
        };
        let mut h = UiHarness::new(WIN);
        let r = run(&mut h, "0123456789\nb\nc\nd\ne", &o);
        assert!(drawn(&r.scene, want), "{spans:?}");
    }
}

#[test]
fn spans_for_a_line_colour_that_line_only() {
    let spans = [(1, span(0, 1))];
    let o = CodeBlockOptions {
        spans: &spans,
        ..opts()
    };
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, "abc\nxyz", &o);
    assert!(drawn(&r.scene, colored(&[("abc", false)])));
    assert!(drawn(&r.scene, colored(&[("x", true), ("yz", false)])));
}

// Pixel probes at scale 2.

fn scaled(o: &CodeBlockOptions<'_>, text: &str) -> Run {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    run(&mut h, text, o)
}

fn shapes_of(scene: &Scene, color: Rgba) -> Vec<lune_ui_core::scene::Shape> {
    let want = premul(color);
    let under = &scene.layers[0].geometry.under;
    under.iter().filter(|s| s.color == want).copied().collect()
}

#[test]
fn at_scale_2_the_box_has_radius_12_padding_20_the_input_fill_and_a_26_px_copy_icon() {
    let r = scaled(&opts(), TEXT);
    let fills = shapes_of(&r.scene, DARK.surface_input);
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].rect, [40.0, 40.0, 600.0, 200.0]);
    assert_eq!(fills[0].radii, [12.0; 4]);
    // The first line starts 10 logical px, 20 device px, in from the box's left and top edges.
    let text = Spec::plain("alpha", Style::mono(11.0, 14.0, DARK.fg_primary));
    let first = r.scene.layers[0]
        .texts
        .iter()
        .find(|t| t.key == key_of(&text, 2.0))
        .expect("first line");
    assert_eq!((first.x, first.y), (30.0, 30.0));
    let icon = key_of(&icon::glyph(icon::COPY, 13.0, DARK.fg_tertiary), 2.0);
    assert!(drawn(&r.scene, icon), "13 px at scale 2 is 26");
}

#[test]
fn a_ring_of_1_px_shows_only_when_the_border_is_on() {
    let ring = |scene: &Scene| {
        let want = premul(DARK.border_subtle);
        let under = &scene.layers[0].geometry.under;
        under
            .iter()
            .filter(|s| s.color == want && s.params[0] == 1.0)
            .map(|s| (s.rect, s.params[1]))
            .collect::<Vec<_>>()
    };
    assert!(ring(&scaled(&opts(), TEXT).scene).is_empty());
    let o = CodeBlockOptions {
        border: true,
        ..opts()
    };
    let r = scaled(&o, TEXT);
    assert_eq!(ring(&r.scene), vec![([40.0, 40.0, 600.0, 200.0], 2.0)]);
}

#[test]
fn the_roomy_look_is_13_over_20_with_the_padding_inside_the_border_and_the_button_at_6() {
    let o = CodeBlockOptions {
        size: 13.0,
        line: 20.0,
        pad: (12.0, 10.0),
        border: true,
        ..opts()
    };
    let r = scaled(&o, TEXT);
    let text = Spec::plain("alpha", Style::mono(13.0, 20.0, DARK.fg_primary));
    let first = r.scene.layers[0]
        .texts
        .iter()
        .find(|t| t.key == key_of(&text, 2.0))
        .expect("first line");
    // Border 1 + padding 12 from the left, 1 + 10 from the top.
    assert_eq!((first.x, first.y), (20.0 + 13.0, 20.0 + 11.0));
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, TEXT, &o);
    let n = r.out.tree.get(copy()).expect("button");
    assert_eq!(
        n.bounds,
        Rect::new(RECT.right() - 7.0 - 21.0, RECT.y + 7.0, 21.0, 21.0)
    );
}

// Transitions.

#[test]
fn the_copy_hover_and_the_crossfade_conform_under_the_three_tiers() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&COPY_FADE, &DARK.motion, tier, 0.0f32, 1.0f32).expect("crossfade");
        conformance(
            &button::HOVER,
            &DARK.motion,
            tier,
            [0u8, 0, 0, 0].map(f32::from),
            DARK.surface_hover.map(f32::from),
        )
        .expect("hover");
    }
}

#[test]
fn the_crossfade_runs_instant_in_full_and_snaps_off() {
    let o = opts();
    let (x, y) = button_at();
    for (tier, settled_by) in [(MotionTier::Full, 140), (MotionTier::Off, 101)] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        at(&mut h, 0, TEXT, &o);
        h.input.pointer_to(x, y);
        at(&mut h, 99, TEXT, &o);
        h.input.click(P);
        at(&mut h, 100, TEXT, &o);
        let mid = at(&mut h, 160, TEXT, &o);
        let done = at(&mut h, 100 + settled_by, TEXT, &o);
        assert!(check_shows(&done.scene), "{tier:?}");
        if tier == MotionTier::Full {
            assert!(!check_shows(&mid.scene), "{tier:?}: half way at 160");
            assert!(!mid.scene.layers[0].texts.is_empty());
            assert_eq!(mid.out.repaint, Repaint::Now);
        } else {
            assert!(check_shows(&mid.scene), "{tier:?}");
        }
    }
}

// The wheel.

fn text_x(scene: &Scene, line: &str) -> Option<f32> {
    let key = key_of(&Spec::plain(line, mono()), 1.0);
    scene.layers[0]
        .texts
        .iter()
        .find(|t| t.key == key)
        .map(|t| t.x)
}

/// The block in a page that scrolls both ways: the page's offset, and where the text is drawn.
fn in_page(h: &mut UiHarness, text: &str, line: &str) -> ((f64, f64), Option<f32>) {
    let scroll = ScrollOptions::default();
    let outer = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut at = (0.0, 0.0);
    let (scene, _) = h.frame(|ui| {
        let r = ui.scroll_area("page", outer, &scroll, |ui, _| {
            ui.code_block("b", RECT, text, &opts());
            ((), Extent::new(2_000.0, 2_000.0))
        });
        at = r.offset;
    });
    (at, text_x(&scene, line))
}

#[test]
fn a_block_that_overflows_takes_the_horizontal_wheel_and_leaves_the_vertical_to_the_page() {
    let wide = "z".repeat(300);
    let mut h = UiHarness::new(WIN);
    let (_, x0) = in_page(&mut h, &wide, &wide);
    in_page(&mut h, &wide, &wide);
    h.input.pointer_to(100.0, 60.0).scroll_pixels(-40.0, 0.0);
    let (page, x) = in_page(&mut h, &wide, &wide);
    assert_eq!(
        page,
        (0.0, 0.0),
        "the page did not take the horizontal delta"
    );
    assert_eq!(x.expect("drawn"), x0.expect("drawn") - 40.0);
    // The vertical one passes the block, which only scrolls sideways.
    h.input.pointer_to(100.0, 60.0).scroll_pixels(0.0, -50.0);
    let (page, x2) = in_page(&mut h, &wide, &wide);
    assert_eq!(page, (0.0, 50.0), "the page took the vertical delta");
    assert_eq!(x2, x, "the block stayed");
}

#[test]
fn a_block_that_fits_gives_both_axes_to_the_page() {
    let mut h = UiHarness::new(WIN);
    in_page(&mut h, "short", "short");
    in_page(&mut h, "short", "short");
    h.input.pointer_to(100.0, 60.0).scroll_pixels(-30.0, -50.0);
    let (page, _) = in_page(&mut h, "short", "short");
    assert_eq!(page, (30.0, 50.0));
}

#[test]
fn a_block_with_more_lines_than_its_rect_never_scrolls_vertically() {
    let tall = (0..40)
        .map(|i| format!("l{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut h = UiHarness::new(WIN);
    in_page(&mut h, &tall, "l0");
    in_page(&mut h, &tall, "l0");
    h.input.pointer_to(100.0, 60.0).scroll_pixels(0.0, -50.0);
    let (page, _) = in_page(&mut h, &tall, "l0");
    assert_eq!(page, (0.0, 50.0));
}

// The text's right margin and the bar.

/// `wide` settled at the far right: where its last character ends.
fn scrolled_right_end(o: &CodeBlockOptions<'_>, wide: &str) -> f32 {
    let mut h = UiHarness::new(WIN);
    run(&mut h, wide, o);
    h.input
        .pointer_to(100.0, 60.0)
        .scroll_pixels(-100_000.0, 0.0);
    let mut r = at(&mut h, 20, wide, o);
    for k in 2..6 {
        r = at(&mut h, k * 20, wide, o);
    }
    let x = text_x(&r.scene, wide).expect("the line is drawn");
    let mut t = FakeText;
    x + t.measure(&Spec::plain(wide, mono()), 1.0).0
}

#[test]
fn scrolled_fully_right_the_last_character_ends_clear_of_the_copy_button() {
    let wide = "z".repeat(300);
    let end = scrolled_right_end(&opts(), &wide);
    let clip = button_at().0 - 10.5 - 4.0;
    assert!(end <= clip, "text ends at {end}, the clip at {clip}");
    assert!(
        end > clip - 8.0,
        "no more scrolled than the last column needs: {end}"
    );
}

#[test]
fn a_line_that_fits_the_padding_but_reaches_under_the_button_scrolls() {
    // 10 + text + 10 would fit the 300 px box; the 25 px margin makes it overflow.
    let line = "z".repeat(48);
    let mut t = FakeText;
    let w = t.measure(&Spec::plain(&line, mono()), 1.0).0;
    assert!(w > 300.0 - 20.0 - 25.0 && w < 300.0 - 20.0, "fixture: {w}");
    let at_rest = scrolled_right_end(&opts(), &line);
    assert!(
        at_rest < RECT.x + 10.0 + w,
        "it moved left to clear the button"
    );
}

#[test]
fn a_bordered_block_keeps_the_same_margin_from_the_button() {
    let wide = "z".repeat(300);
    let o = CodeBlockOptions {
        border: true,
        ..opts()
    };
    let end = scrolled_right_end(&o, &wide);
    let clip = RECT.right() - 1.0 - 6.0 - 21.0 - 4.0;
    assert!(
        end <= clip && end > clip - 8.0,
        "ends at {end}, the clip at {clip}"
    );
}

/// The thumb fills of a scene.
fn thumbs(scene: &Scene) -> Vec<lune_ui_core::scene::Shape> {
    scene.layers[0].geometry.over.clone()
}

fn thumbs_after(o: &CodeBlockOptions<'_>, pointer: (f32, f32)) -> Vec<lune_ui_core::scene::Shape> {
    let wide = "z".repeat(300);
    let mut h = UiHarness::new(WIN);
    run(&mut h, &wide, o);
    h.input.pointer_to(pointer.0, pointer.1);
    let mut r = at(&mut h, 20, &wide, o);
    for k in 2..16 {
        r = at(&mut h, k * 20, &wide, o);
    }
    thumbs(&r.scene)
}

#[test]
fn the_pointer_anywhere_over_the_block_shows_the_bar_in_the_bottom_padding() {
    // The bottom padding, the left padding, the top padding, the text.
    for p in [
        (100.0, RECT.bottom() - 3.0),
        (RECT.x + 3.0, 60.0),
        (100.0, RECT.y + 3.0),
        (100.0, 40.0),
    ] {
        let t = thumbs_after(&opts(), p);
        assert_eq!(t.len(), 1, "pointer {p:?}");
        let [_, y, _, h] = t[0].rect;
        // 6 px thumb, 1 px from the box's bottom edge, below the text's 10 px padding line.
        assert_eq!(y + h, RECT.bottom() - 1.0, "pointer {p:?}");
        assert!(y >= RECT.bottom() - 10.0, "inside the padding: {y}");
    }
    assert!(
        thumbs_after(&opts(), (RECT.right() + 20.0, 60.0)).is_empty(),
        "no bar with the pointer away"
    );
}

#[test]
fn a_ring_holds_the_bar_inside_it() {
    let o = CodeBlockOptions {
        border: true,
        pad: (12.0, 10.0),
        ..opts()
    };
    let t = thumbs_after(&o, (100.0, RECT.bottom() - 4.0));
    assert_eq!(t.len(), 1);
    let [x, y, w, h] = t[0].rect;
    assert_eq!(y + h, RECT.bottom() - 2.0, "1 px ring, 1 px gap");
    assert!(x >= RECT.x + 1.0 && x + w <= RECT.right() - 1.0);
    assert!(
        y >= RECT.bottom() - 11.0,
        "within the 10 px padding and the ring: {y}"
    );
}

#[test]
fn in_a_one_line_block_the_thumb_never_lies_under_the_copy_button() {
    // Compact, one line: the 21 px button reaches 4 px into the bar's 9 px band.
    let wide = "z".repeat(300);
    let o = opts();
    let mut ui_h = UiHarness::new(WIN);
    let mut rect = RECT;
    ui_h.frame(|ui| rect.h = height(ui, &wide, &o));
    assert_eq!(rect.h, 34.0, "fixture");
    let button = Rect::new(rect.right() - 10.0 - 21.0, rect.y + 10.0, 21.0, 21.0);
    let mut h = UiHarness::new(WIN);
    h.input.pointer_to(100.0, rect.y + 17.0);
    let mut ms = 0;
    let mut frame = |h: &mut UiHarness| {
        ms += 20;
        h.at_ms(ms);
        h.frame(|ui| {
            ui.code_block("b", rect, &wide, &o);
        })
        .0
    };
    let mut seen = 0;
    // From the left end to past the right one, then settled at the right.
    for step in 0..40 {
        if step > 0 {
            h.input.scroll_pixels(-80.0, 0.0);
        }
        let scene = frame(&mut h);
        let thumbs = thumbs(&scene);
        let Some(t) = thumbs.first() else {
            continue;
        };
        seen += 1;
        let [x, y, w, th] = t.rect;
        let thumb = Rect::new(x, y, w, th);
        assert!(
            thumb.right() <= button.x || thumb.bottom() <= button.y,
            "step {step}: thumb {thumb:?} under the button {button:?}"
        );
        if step == 39 {
            assert_eq!(thumb.right(), button.x - 4.0, "fully right, 4 px clear");
            assert!(
                thumb.y < button.bottom(),
                "the instrument: they share a band"
            );
        }
    }
    assert!(seen > 30, "the instrument: the thumb showed {seen} times");
}

// Nodes, focus and idle.

#[test]
fn the_text_node_is_read_only_multiline_text_under_the_label() {
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, TEXT, &opts());
    let n = r.out.tree.get(body()).expect("node");
    assert_eq!(n.role, Role::MultilineText);
    assert_eq!(n.name, "Snippet");
    assert!(n.states.contains(State::ReadOnly));
    assert!(!n.actions.contains(Action::SetValue));
}

#[test]
fn a_disabled_block_has_no_enabled_action_left() {
    let mut h = UiHarness::new(WIN);
    let (_, out) = h.frame(|ui| {
        ui.with_enabled(false, |ui| {
            ui.code_block("b", RECT, TEXT, &opts());
        });
    });
    assert!(disabled_violations(&out.tree, root()).is_empty());
}

#[test]
fn the_ring_goes_round_the_whole_box_on_keyboard_focus() {
    let ring = |scene: &Scene| {
        let want = premul(DARK.fg_secondary);
        scene.layers[0]
            .geometry
            .under
            .iter()
            .filter(|s| s.color == want && s.params[0] == 1.0)
            .map(|s| s.rect)
            .collect::<Vec<_>>()
    };
    let o = opts();
    let mut h = UiHarness::new(WIN);
    let r = at(&mut h, 0, TEXT, &o);
    assert!(ring(&r.scene).is_empty());
    press(&mut h, NamedKey::Tab);
    at(&mut h, 10, TEXT, &o);
    let r = at(&mut h, 20, TEXT, &o);
    assert_eq!(ring(&r.scene), vec![[20.0, 20.0, 300.0, 100.0]]);
}

#[test]
fn a_settled_block_reports_idle() {
    let o = opts();
    let mut h = UiHarness::new(WIN);
    at(&mut h, 0, TEXT, &o);
    let r = at(&mut h, 500, TEXT, &o);
    assert_eq!(r.out.repaint, Repaint::Idle);
}
