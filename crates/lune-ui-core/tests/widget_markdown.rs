//! The markdown preview (`text.markdown-preview`, D12): the model drawn in a window, inline marks
//! and their copy, links, task boxes, images, tables, code, the nodes, the nested wheel and the
//! cost of a frame at scale.

use std::time::Duration;

use lune_ui_core::access::{AccessAction, Action, Node, Role, State, Value};
use lune_ui_core::color::{Rgba, premul};
use lune_ui_core::image::ImageId;
use lune_ui_core::input::{
    ClipboardRequest, CursorIcon, FrameOutput, Key, Keymap, Modifiers, NamedKey, PointerButton,
    Repaint,
};
use lune_ui_core::motion::MotionTier;
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::shortcuts::Shortcuts;
use lune_ui_core::strings::{DisplayFormat, StringTable};
use lune_ui_core::testing::{FakeText, ScriptedInput, UiHarness, disabled_violations};
use lune_ui_core::text::{LineSpan, Spec, Style, TextKey, TextSystem, Unit};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{FrameConfig, Ui, UiStore, WidgetId};
use lune_ui_core::widgets::markdown::{
    GAP, MARGIN, MdAlign, MdBlock, MdImage, MdInline, MdItem, MdOptions, MdResponse, MdRow, MdTask,
    is_actionable,
};
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};

const WIN: (f32, f32) = (500.0, 400.0);
const RECT: Rect = Rect::new(10.0, 10.0, 380.0, 300.0);
/// The width blocks are drawn in.
const W: f32 = 380.0 - 2.0 * MARGIN;
/// Where a block's text starts, with the margin.
const X: f32 = 10.0 + MARGIN;
const P: PointerButton = PointerButton::Primary;
const META: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: true,
};

fn t(text: &str) -> MdInline<'_> {
    MdInline::Text {
        text,
        bold: false,
        italic: false,
        strike: false,
    }
}

fn link<'a>(label: &'a str, url: &'a str) -> MdInline<'a> {
    MdInline::Link { label, url }
}

struct Run {
    scene: Scene,
    r: MdResponse,
    out: FrameOutput,
}

fn run(h: &mut UiHarness, blocks: &[MdBlock<'_>], opts: &MdOptions<'_>) -> Run {
    let mut r = None;
    let (scene, out) = h.frame(|ui| r = Some(ui.markdown("m", RECT, blocks, opts)));
    Run {
        scene,
        r: r.expect("ran"),
        out,
    }
}

fn at(h: &mut UiHarness, ms: u64, blocks: &[MdBlock<'_>], opts: &MdOptions<'_>) -> Run {
    h.at_ms(ms);
    run(h, blocks, opts)
}

fn live() -> MdOptions<'static> {
    MdOptions {
        label: "Doc",
        tasks_live: true,
        identity: None,
    }
}

/// A harness that has drawn the document twice, so regions and heights are known.
fn drawn(blocks: &[MdBlock<'_>], opts: &MdOptions<'_>) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    run(&mut h, blocks, opts);
    run(&mut h, blocks, opts);
    h
}

fn nodes(out: &FrameOutput, role: Role) -> Vec<&Node> {
    out.tree.nodes.iter().filter(|n| n.role == role).collect()
}

fn key_of(spec: &Spec) -> TextKey {
    FakeText.prepare(spec, 1.0).0
}

/// A key as a window at scale 2 shapes it.
fn key2(spec: &Spec) -> TextKey {
    FakeText.prepare(spec, 2.0).0
}

fn body2(text: &str) -> TextKey {
    key2(&Spec::plain(text, ui_style(13.0, DARK.fg_primary)).wrapped(W))
}

fn ui_style(size: f32, color: Rgba) -> Style {
    Style::ui(size, color)
}

/// The key of a one-run body paragraph.
fn body(text: &str) -> TextKey {
    key_of(&Spec::plain(text, ui_style(13.0, DARK.fg_primary)).wrapped(W))
}

fn text_at(scene: &Scene, key: TextKey) -> Option<(f32, f32)> {
    let texts = &scene.layers[0].texts;
    texts.iter().find(|t| t.key == key).map(|t| (t.x, t.y))
}

fn fills(scene: &Scene, color: Rgba) -> Vec<Shape> {
    let want = premul(color);
    let under = &scene.layers[0].geometry.under;
    under.iter().filter(|s| s.color == want).copied().collect()
}

fn overs(scene: &Scene, color: Rgba) -> Vec<Shape> {
    let want = premul(color);
    let over = &scene.layers[0].geometry.over;
    over.iter().filter(|s| s.color == want).copied().collect()
}

fn chord(h: &mut UiHarness, c: &str) {
    h.input.set_modifiers(META);
    h.input.key_down(Key::Character(c.to_owned()), None);
    h.input.set_modifiers(Modifiers::default());
}

fn press(h: &mut UiHarness, key: NamedKey) {
    h.input.key_down(Key::Named(key), None);
}

// ---- links: the pure rule ------------------------------------------------------------

#[test]
fn is_actionable_gives_the_entrys_known_answers() {
    for (url, want) in [
        ("https://x.dev", true),
        ("http://x.dev/a?b=c#d", true),
        ("HTTPS://X.DEV", true),
        ("docs/a.md", true),
        ("../up.md", true),
        ("a:b.md/c", false),
        ("docs/a:b.md", true),
        ("#section", false),
        ("mailto:a@b.c", false),
        ("file:///etc/passwd", false),
        ("C:\\x", false),
        ("", false),
    ] {
        assert_eq!(is_actionable(url), want, "{url:?}");
    }
}

// ---- headings and layout ----------------------------------------------------------

#[test]
fn headings_take_their_size_and_colour_by_level_and_clamp() {
    let words = ["one", "two", "three", "four", "five", "six", "zero", "nine"];
    let inl: Vec<[MdInline<'_>; 1]> = words.iter().map(|w| [t(w)]).collect();
    let levels = [1u8, 2, 3, 4, 5, 6, 0, 9];
    let blocks: Vec<MdBlock<'_>> = levels
        .iter()
        .zip(&inl)
        .map(|(l, i)| MdBlock::Heading {
            level: *l,
            inlines: i,
        })
        .collect();
    let h = &mut UiHarness::new(WIN);
    let r = run(h, &blocks[..4], &live());
    let sizes = [22.0, 18.0, 14.0, 13.0, 13.0, 13.0, 22.0, 13.0];
    let colors = [
        DARK.fg_primary,
        DARK.fg_primary,
        DARK.fg_primary,
        DARK.fg_primary,
        DARK.fg_secondary,
        DARK.fg_tertiary,
        DARK.fg_primary,
        DARK.fg_tertiary,
    ];
    let mut drawn = r.scene.layers[0].texts.len();
    for i in 0..4 {
        let style = ui_style(sizes[i], colors[i]).bold();
        let key = key_of(&Spec::plain(words[i], style).wrapped(W));
        assert!(text_at(&r.scene, key).is_some(), "level {}", levels[i]);
    }
    let r = run(h, &blocks[4..], &live());
    drawn += r.scene.layers[0].texts.len();
    assert!(drawn >= 8);
    for i in 4..8 {
        let style = ui_style(sizes[i], colors[i]).bold();
        let key = key_of(&Spec::plain(words[i], style).wrapped(W));
        assert!(text_at(&r.scene, key).is_some(), "level {}", levels[i]);
    }
}

#[test]
fn a_heading_is_a_static_text_node_with_its_level() {
    let i = [t("Title")];
    let blocks = [MdBlock::Heading {
        level: 3,
        inlines: &i,
    }];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    let n = nodes(&r.out, Role::StaticText);
    assert_eq!(n.len(), 1);
    assert_eq!((n[0].name.as_str(), n[0].level), ("Title", Some(3)));
}

#[test]
fn the_column_has_a_12_margin_and_8_between_blocks_at_scale_2() {
    let (a, b) = ([t("first")], [t("second")]);
    let blocks = [MdBlock::Paragraph(&a), MdBlock::Paragraph(&b)];
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let r = run(&mut h, &blocks, &live());
    let (x, y) = text_at(&r.scene, body2("first")).expect("first");
    // Logical positions: 12 in from the view's edges, which is 24 device px.
    assert_eq!((x, y), (RECT.x + MARGIN, RECT.y + MARGIN));
    let second = text_at(&r.scene, body2("second")).expect("second");
    assert_eq!(
        second.1 - y,
        17.0 + GAP,
        "a line, then the gap: 16 device px"
    );
}

#[test]
fn a_quote_has_a_bar_3_wide_a_fill_and_nests() {
    let (a, b) = ([t("inner")], [t("deeper")]);
    let deep = [MdBlock::Paragraph(&b)];
    let inner = [MdBlock::Paragraph(&a), MdBlock::Quote(&deep)];
    let blocks = [MdBlock::Quote(&inner)];
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let r = run(&mut h, &blocks, &live());
    let bars = fills(&r.scene, DARK.link);
    assert_eq!(bars.len(), 2, "one bar per quote");
    assert_eq!(bars[0].rect[2], 6.0, "3 px at scale 2");
    assert_eq!(
        nodes(&r.out, Role::Group).len(),
        3,
        "the document and a group per quote"
    );
    let panels = fills(&r.scene, DARK.surface_input);
    assert_eq!(panels.len(), 2);
    assert_eq!(panels[0].radii, [0.0, 8.0, 8.0, 0.0], "radius 0, 4, 4, 0");
    // The text is the bar, the padding in.
    let w = W - 3.0 - 20.0;
    let key = key2(&Spec::plain("inner", ui_style(13.0, DARK.fg_primary)).wrapped(w));
    let (x, y) = text_at(&r.scene, key).expect("text");
    assert_eq!((x, y), (X + 3.0 + 10.0, RECT.y + MARGIN + 6.0));
}

#[test]
fn a_rule_is_1_high_in_surface_active_with_6_above_and_below() {
    let (a, b) = ([t("above")], [t("below")]);
    let blocks = [
        MdBlock::Paragraph(&a),
        MdBlock::Rule,
        MdBlock::Paragraph(&b),
    ];
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let r = run(&mut h, &blocks, &live());
    let rule = fills(&r.scene, DARK.surface_active);
    assert_eq!(rule.len(), 1);
    assert_eq!(rule[0].rect[3], 2.0, "1 px at scale 2");
    let above = text_at(&r.scene, body2("above")).expect("a").1;
    let below = text_at(&r.scene, body2("below")).expect("b").1;
    assert_eq!(below - above, 17.0 + GAP + 13.0 + GAP);
    assert_eq!(rule[0].rect[1] / 2.0 - above, 17.0 + GAP + 6.0);
}

#[test]
fn an_empty_model_draws_nothing_and_rests() {
    let mut h = UiHarness::new(WIN);
    run(&mut h, &[], &live());
    let r = at(&mut h, 500, &[], &live());
    assert!(r.scene.layers[0].texts.is_empty());
    assert_eq!(r.out.repaint, Repaint::Idle);
}

// ---- inline marks and their copy -----------------------------------------------------

#[test]
fn inline_runs_carry_bold_italic_link_colour_and_code_in_mono() {
    let items = [
        MdInline::Text {
            text: "bold",
            bold: true,
            italic: false,
            strike: false,
        },
        MdInline::Text {
            text: "ital",
            bold: false,
            italic: true,
            strike: false,
        },
        link("site", "https://x.dev"),
        MdInline::Code("code"),
    ];
    let blocks = [MdBlock::Paragraph(&items)];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    let base = ui_style(13.0, DARK.fg_primary);
    let pad = base;
    let spec = Spec {
        runs: vec![
            ("bold".into(), base.bold()),
            ("ital".into(), base.italic()),
            (
                "site".into(),
                Style {
                    color: DARK.link,
                    ..base
                },
            ),
            ("\u{2009}\u{2009}".into(), pad),
            ("code".into(), Style::mono(13.0, base.line, base.color)),
            ("\u{2009}\u{2009}".into(), pad),
        ],
        wrap: Some(W),
    };
    assert!(text_at(&r.scene, key_of(&spec)).is_some());
}

#[test]
fn a_code_run_draws_a_pill_a_strike_a_bar_and_an_actionable_link_an_underline() {
    let items = [
        t("a "),
        MdInline::Code("x"),
        MdInline::Text {
            text: " gone ",
            bold: false,
            italic: false,
            strike: true,
        },
        link("go", "docs/a.md"),
        t(" "),
        link("anchor", "#top"),
    ];
    let blocks = [MdBlock::Paragraph(&items)];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    assert_eq!(fills(&r.scene, DARK.code_inline).len(), 1, "one pill");
    assert_eq!(overs(&r.scene, DARK.fg_primary).len(), 1, "one strike");
    let underlines = overs(&r.scene, DARK.link);
    assert_eq!(underlines.len(), 1, "the actionable link only");
    assert_eq!(underlines[0].rect[3], 1.0);
    // Whole pixels, so the 1 px bars are two clean rows at 2x; the pill ends on the row box, 2 px (at the body size)
    // below the underline's row.
    let strike = overs(&r.scene, DARK.fg_primary)[0].rect;
    assert_eq!(strike[1].fract(), 0.0, "the strike sits on a whole pixel");
    assert_eq!(underlines[0].rect[1].fract(), 0.0, "so does the underline");
    let pill = fills(&r.scene, DARK.code_inline)[0].rect;
    assert_eq!(pill[1] + pill[3] - underlines[0].rect[1], 2.0);
}

#[test]
fn a_selection_across_a_pill_copies_without_its_padding_and_a_source_thin_space_survives() {
    let items = [t("ab"), MdInline::Code("c\u{2009}d"), t("ef")];
    let blocks = [MdBlock::Paragraph(&items)];
    let mut h = UiHarness::new(WIN);
    run(&mut h, &blocks, &live());
    // Press at the start of the line and drag to its end: 2 + 2 + 3 + 2 + 2 chars of 6.5.
    h.input.pointer_to(X + 0.5, RECT.y + MARGIN + 8.0).press(P);
    run(&mut h, &blocks, &live());
    h.input
        .pointer_to(X + 11.0 * 6.5 + 20.0, RECT.y + MARGIN + 8.0);
    run(&mut h, &blocks, &live());
    h.input.release(P);
    run(&mut h, &blocks, &live());
    chord(&mut h, "c");
    let r = run(&mut h, &blocks, &live());
    assert_eq!(
        r.out.clipboard,
        vec![ClipboardRequest::Write("abc\u{2009}def".to_owned())]
    );
}

#[test]
fn the_node_holds_the_text_without_the_pads_and_its_selection_follows() {
    let items = [t("ab"), MdInline::Code("c"), t("ef")];
    let blocks = [MdBlock::Paragraph(&items)];
    let mut h = UiHarness::new(WIN);
    run(&mut h, &blocks, &live());
    h.input.pointer_to(X + 0.5, RECT.y + MARGIN + 8.0).press(P);
    run(&mut h, &blocks, &live());
    h.input.pointer_to(X + 200.0, RECT.y + MARGIN + 8.0);
    run(&mut h, &blocks, &live());
    h.input.release(P);
    let r = run(&mut h, &blocks, &live());
    let n = nodes(&r.out, Role::StaticText);
    assert_eq!(n[0].name, "abcef");
    assert_eq!(n[0].value, Some(Value::Text("abcef".to_owned())));
    let sel = n[0].text.as_ref().expect("text").selection;
    assert_eq!(sel, (0, 5), "the whole run, pads left out");
}

#[test]
fn a_press_starts_a_selection_inside_a_block_and_never_across_blocks() {
    let (a, b) = ([t("first line")], [t("second")]);
    let blocks = [MdBlock::Paragraph(&a), MdBlock::Paragraph(&b)];
    let mut h = UiHarness::new(WIN);
    run(&mut h, &blocks, &live());
    h.input.pointer_to(X + 0.5, RECT.y + MARGIN + 8.0).press(P);
    run(&mut h, &blocks, &live());
    h.input
        .pointer_to(X + 20.0, RECT.y + MARGIN + 17.0 + GAP + 8.0);
    run(&mut h, &blocks, &live());
    h.input.release(P);
    run(&mut h, &blocks, &live());
    chord(&mut h, "c");
    let r = run(&mut h, &blocks, &live());
    let Some(ClipboardRequest::Write(text)) = r.out.clipboard.first() else {
        panic!("a copy");
    };
    assert!(!text.contains("second"), "{text:?}");
}

// ---- links ---------------------------------------------------------------------------

fn link_doc() -> [MdInline<'static>; 3] {
    [t("see "), link("docs", "docs/a.md"), t(" now")]
}

/// The centre of the link label "docs" in a paragraph that starts "see ".
fn on_link() -> (f32, f32) {
    (X + 4.0 * 6.5 + 12.0, RECT.y + MARGIN + 8.0)
}

#[test]
fn a_press_on_an_actionable_link_raises_it_once_and_starts_no_selection() {
    let items = link_doc();
    let blocks = [MdBlock::Paragraph(&items)];
    let mut h = drawn(&blocks, &live());
    let (x, y) = on_link();
    h.input.pointer_to(x, y).press(P);
    let r = run(&mut h, &blocks, &live());
    assert_eq!(r.r.link.as_deref(), Some("docs/a.md"));
    h.input.release(P);
    let r = run(&mut h, &blocks, &live());
    assert_eq!(r.r.link, None, "the release raises nothing more");
    let n = nodes(&r.out, Role::StaticText);
    let sel = n[0].text.as_ref().expect("text").selection;
    assert_eq!(sel.0, sel.1, "no range selected");
    assert_eq!(r.out.cursor, CursorIcon::Pointer, "the hand over a link");
}

#[test]
fn a_press_beside_the_link_selects_text_and_raises_nothing() {
    let items = link_doc();
    let blocks = [MdBlock::Paragraph(&items)];
    let mut h = drawn(&blocks, &live());
    h.input.pointer_to(X + 1.0, RECT.y + MARGIN + 8.0).press(P);
    let r = run(&mut h, &blocks, &live());
    assert_eq!(r.r.link, None);
    assert_eq!(r.out.cursor, CursorIcon::Text);
}

#[test]
fn an_inert_link_is_plain_coloured_text_with_no_region_node_or_cursor() {
    for url in ["#section", "mailto:a@b.c", ""] {
        let items = [t("see "), link("docs", url), t(" now")];
        let blocks = [MdBlock::Paragraph(&items)];
        let mut h = drawn(&blocks, &live());
        let (x, y) = on_link();
        h.input.pointer_to(x, y).press(P);
        let r = run(&mut h, &blocks, &live());
        assert_eq!(r.r.link, None, "{url:?}");
        assert!(nodes(&r.out, Role::Link).is_empty(), "{url:?}");
        assert_ne!(r.out.cursor, CursorIcon::Pointer, "{url:?}");
        assert!(overs(&r.scene, DARK.link).is_empty(), "no underline");
    }
}

#[test]
fn enter_and_space_on_the_focused_link_raise_it() {
    for key in [NamedKey::Enter, NamedKey::Space] {
        let items = link_doc();
        let blocks = [MdBlock::Paragraph(&items)];
        let mut h = drawn(&blocks, &live());
        press(&mut h, NamedKey::Tab);
        let r = run(&mut h, &blocks, &live());
        let n = nodes(&r.out, Role::Link);
        assert_eq!(r.out.focused, Some(n[0].id), "the link is the first stop");
        press(&mut h, key);
        let r = run(&mut h, &blocks, &live());
        assert_eq!(r.r.link.as_deref(), Some("docs/a.md"), "{key:?}");
    }
}

#[test]
fn the_link_node_has_the_label_for_name_the_url_for_description_and_a_default_action() {
    let items = link_doc();
    let blocks = [MdBlock::Paragraph(&items)];
    let mut h = drawn(&blocks, &live());
    let r = run(&mut h, &blocks, &live());
    let n = nodes(&r.out, Role::Link)[0].clone();
    assert_eq!(n.name, "docs");
    assert_eq!(n.description.as_deref(), Some("docs/a.md"));
    assert!(n.states.contains(State::Focusable));
    h.input.access(AccessAction {
        target: n.id,
        action: Action::Default,
        value: None,
    });
    let r = run(&mut h, &blocks, &live());
    assert_eq!(r.r.link.as_deref(), Some("docs/a.md"));
}

#[test]
fn a_link_across_two_rows_hits_on_both_and_is_one_tab_stop() {
    let long = "word ".repeat(40);
    let (first, second) = ([link(&long, "https://x.dev")], [link("next", "docs/b.md")]);
    let blocks = [MdBlock::Paragraph(&first), MdBlock::Paragraph(&second)];
    let mut h = drawn(&blocks, &live());
    for row in [0.0, 17.0] {
        h.input
            .pointer_to(X + 30.0, RECT.y + MARGIN + 8.0 + row)
            .press(P);
        let r = run(&mut h, &blocks, &live());
        assert_eq!(r.r.link.as_deref(), Some("https://x.dev"), "row at {row}");
        h.input.release(P);
        run(&mut h, &blocks, &live());
    }
    let r = run(&mut h, &blocks, &live());
    let links: Vec<WidgetId> = nodes(&r.out, Role::Link).iter().map(|n| n.id).collect();
    assert_eq!(links.len(), 2, "one node per link");
    // The pointer press focused the first link: Tab goes to the next paragraph's link.
    press(&mut h, NamedKey::Tab);
    let r = run(&mut h, &blocks, &live());
    assert_eq!(
        r.out.focused,
        Some(links[1]),
        "not the second row of the first"
    );
}

#[test]
fn a_disabled_document_raises_nothing_and_takes_its_keys() {
    let items = link_doc();
    let blocks = [MdBlock::Paragraph(&items)];
    let mut h = drawn(&blocks, &live());
    press(&mut h, NamedKey::Tab);
    run(&mut h, &blocks, &live());
    press(&mut h, NamedKey::Enter);
    let mut resp = None;
    let (_, out) = h.frame(|ui| {
        ui.with_enabled(false, |ui| {
            resp = Some(ui.markdown("m", RECT, &blocks, &live()));
        });
    });
    assert_eq!(resp.expect("ran").link, None);
    assert!(disabled_violations(&out.tree, WidgetId::ROOT.with("m")).is_empty());
}

// ---- tasks ---------------------------------------------------------------------------

#[test]
fn a_task_box_shows_the_model_and_a_click_reports_its_id_when_live() {
    let text = [t("ship it")];
    let para = [MdBlock::Paragraph(&text)];
    let items = [
        MdItem {
            marker: "-",
            blocks: &para,
            task: Some(MdTask {
                checked: false,
                id: 7,
            }),
        },
        MdItem {
            marker: "-",
            blocks: &para,
            task: Some(MdTask {
                checked: true,
                id: 8,
            }),
        },
    ];
    let blocks = [MdBlock::List { items: &items }];
    let mut h = drawn(&blocks, &live());
    let boxes = |r: &Run| {
        nodes(&r.out, Role::Checkbox)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    };
    let r = run(&mut h, &blocks, &live());
    let b = boxes(&r);
    assert_eq!(b.len(), 2);
    assert!(!b[0].states.contains(State::Checked) && b[1].states.contains(State::Checked));
    assert_eq!(b[0].name, "ship it");
    assert_eq!(
        fills(&r.scene, DARK.accent).len(),
        1,
        "the checked one is filled"
    );
    assert_eq!(fills(&r.scene, DARK.accent)[0].rect[2], 15.0, "15 px");
    // A click on the first reports 7 and changes nothing in the model's look.
    let c = b[0].bounds;
    h.input.pointer_to(c.x + 7.0, c.y + 7.0).click(P);
    let r = run(&mut h, &blocks, &live());
    assert_eq!(r.r.task, Some(7));
    let r = run(&mut h, &blocks, &live());
    assert!(
        !boxes(&r)[0].states.contains(State::Checked),
        "the model's value"
    );
}

#[test]
fn task_boxes_on_a_read_only_host_report_nothing_and_still_draw() {
    let text = [t("ship it")];
    let para = [MdBlock::Paragraph(&text)];
    let items = [MdItem {
        marker: "-",
        blocks: &para,
        task: Some(MdTask {
            checked: true,
            id: 1,
        }),
    }];
    let blocks = [MdBlock::List { items: &items }];
    let opts = MdOptions {
        tasks_live: false,
        ..live()
    };
    let mut h = drawn(&blocks, &opts);
    let r = run(&mut h, &blocks, &opts);
    let c = nodes(&r.out, Role::Checkbox)[0].bounds;
    assert_eq!(fills(&r.scene, DARK.accent).len(), 1);
    h.input.pointer_to(c.x + 7.0, c.y + 7.0).click(P);
    assert_eq!(run(&mut h, &blocks, &opts).r.task, None);
    press(&mut h, NamedKey::Space);
    assert_eq!(run(&mut h, &blocks, &opts).r.task, None);
}

#[test]
fn space_on_the_focused_task_box_reports_it() {
    let text = [t("ship it")];
    let para = [MdBlock::Paragraph(&text)];
    let items = [MdItem {
        marker: "-",
        blocks: &para,
        task: Some(MdTask {
            checked: false,
            id: 3,
        }),
    }];
    let blocks = [MdBlock::List { items: &items }];
    let mut h = drawn(&blocks, &live());
    press(&mut h, NamedKey::Tab);
    let r = run(&mut h, &blocks, &live());
    assert_eq!(r.out.focused, Some(nodes(&r.out, Role::Checkbox)[0].id));
    press(&mut h, NamedKey::Space);
    assert_eq!(run(&mut h, &blocks, &live()).r.task, Some(3));
}

#[test]
fn lists_draw_markers_in_fg_secondary_and_nest_with_list_and_item_nodes() {
    let (a, b) = ([t("one")], [t("inner")]);
    let pa = [MdBlock::Paragraph(&a)];
    let pb = [MdBlock::Paragraph(&b)];
    let inner_items = [MdItem {
        marker: "\u{2022}",
        blocks: &pb,
        task: None,
    }];
    let outer_blocks = [
        MdBlock::Paragraph(&a),
        MdBlock::List {
            items: &inner_items,
        },
    ];
    let items = [
        MdItem {
            marker: "1.",
            blocks: &pa,
            task: None,
        },
        MdItem {
            marker: "2.",
            blocks: &outer_blocks,
            task: None,
        },
    ];
    let blocks = [MdBlock::List { items: &items }];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    let marker = |s: &str| key_of(&Spec::plain(s, ui_style(13.0, DARK.fg_secondary)));
    assert!(text_at(&r.scene, marker("1.")).is_some());
    assert!(text_at(&r.scene, marker("2.")).is_some());
    assert!(text_at(&r.scene, marker("\u{2022}")).is_some());
    assert_eq!(nodes(&r.out, Role::List).len(), 2);
    assert_eq!(nodes(&r.out, Role::ListItem).len(), 3);
    // The items' text starts after the widest marker and 6.
    let w = (2.0 * 6.5) + 6.0;
    let key = key_of(&Spec::plain("one", ui_style(13.0, DARK.fg_primary)).wrapped(W - w));
    assert_eq!(text_at(&r.scene, key).expect("item").0, X + w);
}

// ---- images --------------------------------------------------------------------------

fn images(scene: &Scene) -> Vec<[f32; 4]> {
    scene.layers[0]
        .images
        .iter()
        .map(|i| i.dst)
        .map(|r| [r.x, r.y, r.w, r.h])
        .collect()
}

const PIC: MdImage = MdImage {
    id: ImageId(7),
    width: 1_000,
    height: 500,
};

#[test]
fn a_resolved_image_is_at_most_640_wide_and_keeps_its_aspect() {
    let wide = Rect::new(0.0, 0.0, 1_000.0, 400.0);
    let mut h = UiHarness::new((1_000.0, 400.0));
    let blocks = [MdBlock::Image {
        target: "a.png",
        alt: "alt",
        image: Some(PIC),
    }];
    let mut r = None;
    let (scene, out) = h.frame(|ui| {
        r = Some(ui.markdown("m", wide, &blocks, &live()));
    });
    let dst = images(&scene);
    assert_eq!(dst.len(), 1);
    assert_eq!((dst[0][2], dst[0][3]), (640.0, 320.0));
    assert_eq!(nodes(&out, Role::Image)[0].name, "alt");
    // Narrower than the picture's limit: the view's width.
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    assert_eq!(images(&r.scene)[0][2], W);
}

#[test]
fn an_unresolved_or_failed_image_shows_its_alt_or_its_target() {
    for (alt, want) in [("the alt", "the alt"), ("", "docs/p.png")] {
        let blocks = [MdBlock::Image {
            target: "docs/p.png",
            alt,
            image: None,
        }];
        let r = run(&mut UiHarness::new(WIN), &blocks, &live());
        assert!(images(&r.scene).is_empty());
        let spec = Spec::plain(want, ui_style(11.0, DARK.fg_tertiary)).wrapped(W);
        assert!(text_at(&r.scene, key_of(&spec)).is_some(), "{want}");
    }
    // A picture with no size is a bad value: the placeholder.
    let bad = MdImage { width: 0, ..PIC };
    let blocks = [MdBlock::Image {
        target: "a.png",
        alt: "none",
        image: Some(bad),
    }];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    assert!(images(&r.scene).is_empty());
}

#[test]
fn a_remote_target_is_never_drawn_even_with_a_picture_and_says_so() {
    for target in ["https://x.dev/a.png", "http://x.dev/a.png"] {
        let blocks = [MdBlock::Image {
            target,
            alt: "alt",
            image: Some(PIC),
        }];
        let r = run(&mut UiHarness::new(WIN), &blocks, &live());
        assert!(images(&r.scene).is_empty(), "{target}");
        let text = format!("Remote image not loaded: {target}");
        let spec = Spec::plain(text, ui_style(11.0, DARK.fg_tertiary)).wrapped(W);
        assert!(text_at(&r.scene, key_of(&spec)).is_some(), "{target}");
    }
}

// ---- tables --------------------------------------------------------------------------

fn table_doc<'a>(cells: &'a [[&'a [MdInline<'a>]; 3]; 2]) -> [MdRow<'a>; 2] {
    [
        MdRow {
            header: true,
            cells: &cells[0],
        },
        MdRow {
            header: false,
            cells: &cells[1],
        },
    ]
}

#[test]
fn a_table_has_a_border_a_header_fill_a_rule_per_row_and_right_aligned_cells() {
    let (a, b, c) = ([t("a")], [t("b")], [t("c")]);
    let (d, e, f) = ([t("d")], [t("e")], [t("1234")]);
    let cells = [[&a[..], &b[..], &c[..]], [&d[..], &e[..], &f[..]]];
    let rows = table_doc(&cells);
    let align = [MdAlign::Left, MdAlign::Center, MdAlign::Right];
    let blocks = [MdBlock::Table {
        rows: &rows,
        align: &align,
    }];
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let r = run(&mut h, &blocks, &live());
    let ring = r.scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|s| s.color == premul(DARK.surface_active) && s.params[0] == 1.0)
        .expect("border");
    assert_eq!(ring.params[1], 2.0, "1 px at scale 2");
    let header = fills(&r.scene, DARK.surface_hover);
    assert_eq!(header.len(), 1);
    // Both rows' rules, and the header fill reaches the table's width: three columns of 80.
    let rules: Vec<_> = fills(&r.scene, DARK.surface_active)
        .into_iter()
        .filter(|s| s.rect[3] == 2.0)
        .collect();
    assert_eq!(rules.len(), 2);
    assert_eq!(header[0].rect[2], 2.0 * W - 4.0, "to the right edge");
    // The third column's cell is right-aligned: its right edge is the column's, less the padding.
    let key = key2(&Spec::plain("1234", ui_style(13.0, DARK.fg_primary)));
    let (x, _) = text_at(&r.scene, key).expect("cell");
    let first = X + 1.0;
    assert_eq!(x, first + 3.0 * 80.0 - 8.0 - 4.0 * 6.5);
}

#[test]
fn a_column_is_as_wide_as_its_widest_cell_and_80_at_least() {
    let long = "a long cell of text";
    let (a, b) = ([t("x")], [t(long)]);
    let cells = [[&a[..], &a[..], &a[..]], [&b[..], &a[..], &a[..]]];
    let rows = table_doc(&cells);
    let blocks = [MdBlock::Table {
        rows: &rows,
        align: &[],
    }];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    let cell_nodes = nodes(&r.out, Role::Cell);
    let widths: Vec<f32> = cell_nodes.iter().map(|n| n.bounds.w).collect();
    assert_eq!(widths[0], long.chars().count() as f32 * 6.5 + 16.0);
    assert_eq!(widths[1], 80.0);
}

#[test]
fn the_table_nodes_are_a_table_of_rows_with_column_headers_and_cells() {
    let (a, b) = ([t("h")], [t("v")]);
    let cells = [[&a[..], &a[..], &a[..]], [&b[..], &b[..], &b[..]]];
    let rows = table_doc(&cells);
    let blocks = [MdBlock::Table {
        rows: &rows,
        align: &[],
    }];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    assert_eq!(nodes(&r.out, Role::Table).len(), 1);
    assert_eq!(nodes(&r.out, Role::Row).len(), 2);
    assert_eq!(nodes(&r.out, Role::ColumnHeader).len(), 3);
    assert_eq!(nodes(&r.out, Role::Cell).len(), 3);
}

/// The document in a page that scrolls both ways: the page's offset and where `text` is drawn.
fn in_page(h: &mut UiHarness, blocks: &[MdBlock<'_>], key: TextKey) -> ((f64, f64), Option<f32>) {
    let outer = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut page = (0.0, 0.0);
    let (scene, _) = h.frame(|ui| {
        let r = ui.scroll_area("page", outer, &ScrollOptions::default(), |ui, _| {
            ui.markdown("m", RECT, blocks, &live());
            ((), Extent::new(2_000.0, 2_000.0))
        });
        page = r.offset;
    });
    (page, text_at(&scene, key).map(|p| p.0))
}

#[test]
fn a_wide_table_takes_the_horizontal_wheel_and_leaves_the_vertical_to_the_document() {
    let wide = "w".repeat(120);
    let (a, b) = ([t("h")], [t(&wide)]);
    let cells = [[&a[..], &a[..], &a[..]], [&b[..], &a[..], &a[..]]];
    let rows = table_doc(&cells);
    let blocks = [MdBlock::Table {
        rows: &rows,
        align: &[],
    }];
    let key = key_of(&Spec::plain(&*wide, ui_style(13.0, DARK.fg_primary)));
    let mut h = UiHarness::new(WIN);
    let (_, x0) = in_page(&mut h, &blocks, key);
    in_page(&mut h, &blocks, key);
    // Over the second row of the table.
    h.input
        .pointer_to(100.0, RECT.y + MARGIN + 40.0)
        .scroll_pixels(-40.0, 0.0);
    let (page, x) = in_page(&mut h, &blocks, key);
    assert_eq!(
        page,
        (0.0, 0.0),
        "the page did not take the horizontal delta"
    );
    assert_eq!(x.expect("drawn"), x0.expect("drawn") - 40.0);
    // The document fits its window, so the vertical delta passes it to the page.
    h.input
        .pointer_to(100.0, RECT.y + MARGIN + 40.0)
        .scroll_pixels(0.0, -30.0);
    let (page, _) = in_page(&mut h, &blocks, key);
    assert_eq!(page.1, 30.0);
}

// ---- code, assets, html --------------------------------------------------------------

#[test]
fn fenced_code_is_a_code_block_that_keeps_its_body_off_the_tab_order() {
    let blocks = [MdBlock::Code {
        text: "let a = 1;",
        spans: &[],
    }];
    let mut h = drawn(&blocks, &live());
    let r = run(&mut h, &blocks, &live());
    assert_eq!(nodes(&r.out, Role::MultilineText).len(), 1);
    assert_eq!(fills(&r.scene, DARK.surface_input).len(), 1);
    press(&mut h, NamedKey::Tab);
    let r = run(&mut h, &blocks, &live());
    let button = nodes(&r.out, Role::Button);
    assert_eq!(
        r.out.focused,
        Some(button[0].id),
        "the copy button, not the body"
    );
}

#[test]
fn mermaid_and_math_show_the_source_with_a_failure_note_or_the_picture() {
    let blocks = [
        MdBlock::Mermaid {
            source: "graph TD",
            rendered: None,
        },
        MdBlock::Math {
            source: "x^2",
            rendered: None,
        },
    ];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    let note = |s: &str| key_of(&Spec::plain(s, ui_style(11.0, DARK.fg_tertiary)).wrapped(W));
    assert!(
        text_at(
            &r.scene,
            note("Diagram could not be rendered \u{2014} showing source")
        )
        .is_some()
    );
    assert!(
        text_at(
            &r.scene,
            note("Formula could not be rendered \u{2014} showing source")
        )
        .is_some()
    );
    assert_eq!(nodes(&r.out, Role::MultilineText).len(), 2);

    let blocks = [MdBlock::Mermaid {
        source: "graph TD",
        rendered: Some(PIC),
    }];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    assert_eq!(images(&r.scene).len(), 1);
    assert!(nodes(&r.out, Role::MultilineText).is_empty());
}

#[test]
fn html_is_drawn_as_inert_text_on_the_code_fill() {
    let raw = "<script>alert(1)</script>";
    let blocks = [MdBlock::Html(raw)];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    assert_eq!(fills(&r.scene, DARK.surface_input).len(), 1);
    let spec = Spec::plain(raw, Style::mono(11.0, 14.0, DARK.fg_primary)).wrapped(W - 16.0);
    assert_eq!(
        text_at(&r.scene, key_of(&spec)),
        Some((X + 8.0, RECT.y + MARGIN + 8.0))
    );
    assert!(nodes(&r.out, Role::Link).is_empty());
}

// ---- the document: nodes, virtualization, rest ---------------------------------------

#[test]
fn the_document_is_a_group_named_by_its_label() {
    let a = [t("x")];
    let blocks = [MdBlock::Paragraph(&a)];
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    let g = r.out.tree.get(WidgetId::ROOT.with("m")).expect("group");
    assert_eq!((g.role, g.name.as_str()), (Role::Group, "Doc"));
}

#[test]
fn a_new_identity_starts_at_the_top() {
    let long: Vec<[MdInline<'_>; 1]> = (0..60).map(|_| [t("line of text")]).collect();
    let blocks: Vec<MdBlock<'_>> = long.iter().map(|i| MdBlock::Paragraph(i)).collect();
    let mut h = UiHarness::new(WIN);
    let one = MdOptions {
        identity: Some(1),
        ..live()
    };
    run(&mut h, &blocks, &one);
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -400.0);
    run(&mut h, &blocks, &one);
    let r = run(&mut h, &blocks, &one);
    assert!(r.r.response.rect.h > 0.0);
    let two = MdOptions {
        identity: Some(2),
        ..live()
    };
    run(&mut h, &blocks, &two);
    let r = run(&mut h, &blocks, &two);
    assert!(text_at(&r.scene, body("line of text")).is_some());
    let first = text_at(&r.scene, body("line of text")).expect("first");
    assert_eq!(first.1, RECT.y + MARGIN);
}

/// A text system that counts what a frame shapes.
#[derive(Default)]
struct Metered {
    inner: FakeText,
    prepares: usize,
    bytes: usize,
}

impl Metered {
    fn saw(&mut self, spec: &Spec) {
        self.prepares += 1;
        self.bytes += spec.runs.iter().map(|r| r.0.len()).sum::<usize>();
    }
}

impl TextSystem for Metered {
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
        self.saw(spec);
        self.inner.prepare(spec, scale)
    }

    fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> lune_ui_core::text::CaretPos {
        self.saw(spec);
        self.inner.caret(spec, scale, offset)
    }

    fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize {
        self.saw(spec);
        self.inner.hit(spec, scale, pos)
    }

    fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan> {
        self.saw(spec);
        self.inner.lines(spec, scale)
    }

    fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
        self.inner.segments(text, unit)
    }
}

struct Bench {
    store: UiStore,
    text: Metered,
    input: ScriptedInput,
    shortcuts: Shortcuts,
    strings: StringTable,
}

impl Bench {
    fn new() -> Bench {
        let mut input = ScriptedInput::new(WIN, 1.0);
        input.focus(true);
        Bench {
            store: UiStore::default(),
            text: Metered::default(),
            input,
            shortcuts: Shortcuts::default(),
            strings: StringTable::default(),
        }
    }

    fn frame(&mut self, f: impl FnOnce(&mut Ui<'_>)) -> ((usize, usize), Repaint) {
        self.text.prepares = 0;
        self.text.bytes = 0;
        let raw = self.input.end_frame();
        let config = FrameConfig {
            theme: DARK,
            tier: MotionTier::Full,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: Keymap::MAC,
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &raw, config, &mut self.text);
        f(&mut ui);
        let (_, out) = ui.finish();
        ((self.text.prepares, self.text.bytes), out.repaint)
    }
}

fn costs(n: usize) -> Vec<(usize, usize)> {
    let items = [t("a paragraph of ordinary text"), MdInline::Code("code")];
    let blocks: Vec<MdBlock<'_>> = (0..n).map(|_| MdBlock::Paragraph(&items)).collect();
    let mut b = Bench::new();
    let mut out = Vec::new();
    let mut frame = |b: &mut Bench| {
        let (cost, repaint) = b.frame(|ui| {
            ui.markdown("m", RECT, &blocks, &live());
        });
        out.push(cost);
        repaint
    };
    frame(&mut b);
    frame(&mut b);
    frame(&mut b);
    assert_eq!(frame(&mut b), Repaint::Idle, "a settled frame, {n} blocks");
    b.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -500.0);
    frame(&mut b);
    frame(&mut b);
    b.input.pointer_leave();
    frame(&mut b);
    b.input.set_time(Duration::from_secs(5));
    frame(&mut b);
    assert_eq!(frame(&mut b), Repaint::Idle, "after the scroll, {n} blocks");
    out
}

#[test]
fn a_frame_costs_the_same_at_1_000_and_10_000_blocks_and_rests() {
    let (small, big) = (costs(1_000), costs(10_000));
    // The settled frames (the 4th and the last): the same count and bytes.
    for i in [3, small.len() - 1] {
        assert_eq!(small[i], big[i], "frame {i}");
    }
    // The first frame builds the window at both sizes.
    assert_eq!(small[0], big[0]);
    assert!(small[3].0 > 0, "the instrument sees shaping");
}

#[test]
fn only_the_blocks_in_view_are_built() {
    let items = [t("x")];
    let blocks: Vec<MdBlock<'_>> = (0..5_000).map(|_| MdBlock::Paragraph(&items)).collect();
    let r = run(&mut UiHarness::new(WIN), &blocks, &live());
    let drawn = r.scene.layers[0].texts.len();
    assert!(drawn > 0 && drawn < 40, "{drawn} blocks drawn");
    assert_eq!(nodes(&r.out, Role::StaticText).len(), drawn);
}

#[test]
fn a_block_above_the_window_changing_height_shifts_the_area_so_what_shows_stays() {
    // Every block measures 25, the estimate is 48: blocks measured above the window move the area.
    let texts: Vec<String> = (0..100).map(|i| format!("para {i}")).collect();
    let items: Vec<[MdInline<'_>; 1]> = texts.iter().map(|s| [t(s)]).collect();
    let blocks: Vec<MdBlock<'_>> = items.iter().map(|i| MdBlock::Paragraph(i)).collect();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &blocks, &live());
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -600.0);
    let first = run(&mut h, &blocks, &live());
    let ys = |r: &Run| -> Vec<(usize, f32)> {
        (0..100)
            .filter_map(|i| text_at(&r.scene, body(&texts[i])).map(|p| (i, p.1)))
            .collect()
    };
    let seen = ys(&first);
    assert!(seen.len() > 3, "blocks are on screen after the jump");
    let (tracked, y0) = seen[seen.len() / 2];
    for n in 0..5 {
        let r = at(&mut h, 10 + n, &blocks, &live());
        let now = ys(&r).into_iter().find(|(i, _)| *i == tracked).map(|p| p.1);
        assert_eq!(now, Some(y0), "frame {n}: what showed stayed");
    }
}

// ---- keys, the page and rest ---------------------------------------------------------

#[test]
fn a_disabled_document_takes_task_clicks_and_keys_and_reports_none() {
    let text = [t("ship it")];
    let para = [MdBlock::Paragraph(&text)];
    let items = [MdItem {
        marker: "-",
        blocks: &para,
        task: Some(MdTask {
            checked: false,
            id: 1,
        }),
    }];
    let blocks = [MdBlock::List { items: &items }];
    let mut h = drawn(&blocks, &live());
    let c = nodes(&run(&mut h, &blocks, &live()).out, Role::Checkbox)[0].bounds;
    h.input.pointer_to(c.x + 7.0, c.y + 7.0).click(P);
    press(&mut h, NamedKey::Space);
    let mut resp = None;
    let (_, out) = h.frame(|ui| {
        ui.with_enabled(false, |ui| {
            resp = Some(ui.markdown("m", RECT, &blocks, &live()));
        });
    });
    assert_eq!(resp.expect("ran").task, None);
    assert!(disabled_violations(&out.tree, WidgetId::ROOT.with("m")).is_empty());
}

#[test]
fn a_shortcut_chord_on_a_focused_link_stays_a_shortcut() {
    let items = link_doc();
    let blocks = [MdBlock::Paragraph(&items)];
    let mut h = drawn(&blocks, &live());
    press(&mut h, NamedKey::Tab);
    run(&mut h, &blocks, &live());
    h.input.set_modifiers(META);
    press(&mut h, NamedKey::Enter);
    h.input.set_modifiers(Modifiers::default());
    assert_eq!(run(&mut h, &blocks, &live()).r.link, None);
}

#[test]
fn the_page_keys_scroll_the_document_around_a_focused_link() {
    let texts: Vec<String> = (0..80).map(|i| format!("para {i}")).collect();
    let first = link_doc();
    let items: Vec<[MdInline<'_>; 1]> = texts.iter().map(|s| [t(s)]).collect();
    let mut blocks = vec![MdBlock::Paragraph(&first)];
    blocks.extend(items.iter().map(|i| MdBlock::Paragraph(i)));
    let mut h = drawn(&blocks, &live());
    press(&mut h, NamedKey::Tab);
    run(&mut h, &blocks, &live());
    let before = text_at(&run(&mut h, &blocks, &live()).scene, body("para 0"))
        .expect("drawn")
        .1;
    press(&mut h, NamedKey::PageDown);
    run(&mut h, &blocks, &live());
    let r = at(&mut h, 2_000, &blocks, &live());
    let after = text_at(&r.scene, body("para 0")).map(|p| p.1);
    assert!(
        after.is_none_or(|y| y < before - 100.0),
        "{before} then {after:?}"
    );
}

/// The document in a page of 2,000 px, in a window of `rect`'s height.
fn doc_in_page(h: &mut UiHarness, blocks: &[MdBlock<'_>], rect: Rect) -> (f64, f64) {
    let outer = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut page = (0.0, 0.0);
    h.frame(|ui| {
        let r = ui.scroll_area("page", outer, &ScrollOptions::default(), |ui, _| {
            ui.markdown("m", rect, blocks, &live());
            ((), Extent::new(2_000.0, 2_000.0))
        });
        page = r.offset;
    });
    page
}

#[test]
fn the_wheel_over_a_document_that_fits_goes_to_the_page_and_one_that_overflows_keeps_it() {
    let (a, b) = ([t("short")], [t("text")]);
    let fits = [MdBlock::Paragraph(&a), MdBlock::Paragraph(&b)];
    let mut h = UiHarness::new(WIN);
    doc_in_page(&mut h, &fits, RECT);
    doc_in_page(&mut h, &fits, RECT);
    h.input.pointer_to(100.0, 60.0).scroll_pixels(0.0, -50.0);
    assert_eq!(doc_in_page(&mut h, &fits, RECT), (0.0, 50.0));

    let items: Vec<[MdInline<'_>; 1]> = (0..60).map(|_| [t("line")]).collect();
    let tall: Vec<MdBlock<'_>> = items.iter().map(|i| MdBlock::Paragraph(i)).collect();
    let mut h = UiHarness::new(WIN);
    doc_in_page(&mut h, &tall, RECT);
    doc_in_page(&mut h, &tall, RECT);
    h.input.pointer_to(100.0, 60.0).scroll_pixels(0.0, -50.0);
    assert_eq!(
        doc_in_page(&mut h, &tall, RECT),
        (0.0, 0.0),
        "the document took it"
    );
}

#[test]
fn a_wide_code_block_in_a_document_in_a_page_takes_the_horizontal_wheel_only() {
    let wide = "z".repeat(300);
    let blocks = [MdBlock::Code {
        text: &wide,
        spans: &[],
    }];
    let mut h = UiHarness::new(WIN);
    doc_in_page(&mut h, &blocks, RECT);
    doc_in_page(&mut h, &blocks, RECT);
    let (x, y) = (60.0, RECT.y + MARGIN + 20.0);
    h.input.pointer_to(x, y).scroll_pixels(0.0, -40.0);
    assert_eq!(
        doc_in_page(&mut h, &blocks, RECT),
        (0.0, 40.0),
        "vertical: the page"
    );
    h.input.pointer_to(x, y - 40.0).scroll_pixels(-30.0, 0.0);
    assert_eq!(
        doc_in_page(&mut h, &blocks, RECT),
        (0.0, 40.0),
        "horizontal: not the page"
    );
}

#[test]
fn a_rich_document_at_rest_reports_idle() {
    let items = [t("a "), MdInline::Code("b"), link("c", "docs/a.md")];
    let (quote, rows) = ([MdBlock::Paragraph(&items)], [t("cell")]);
    let cells = [
        [&rows[..], &rows[..], &rows[..]],
        [&rows[..], &rows[..], &rows[..]],
    ];
    let table = table_doc(&cells);
    let blocks = [
        MdBlock::Paragraph(&items),
        MdBlock::Quote(&quote),
        MdBlock::Table {
            rows: &table,
            align: &[],
        },
        MdBlock::Code {
            text: "x",
            spans: &[],
        },
        MdBlock::Rule,
    ];
    let mut h = UiHarness::new(WIN);
    run(&mut h, &blocks, &live());
    run(&mut h, &blocks, &live());
    let r = at(&mut h, 1_000, &blocks, &live());
    assert_eq!(r.out.repaint, Repaint::Idle);
}

// ---- focus, the page keys and a width change on a long document ------------------------

/// Tab, then frames until the document rests; the focused node's box, and the node's id.
fn tab_to(h: &mut UiHarness, blocks: &[MdBlock<'_>], ms: &mut u64) -> (WidgetId, Rect) {
    press(h, NamedKey::Tab);
    let mut r = run(h, blocks, &live());
    for _ in 0..30 {
        *ms += 400;
        r = at(h, *ms, blocks, &live());
        if r.out.repaint == Repaint::Idle {
            break;
        }
    }
    let id = r.out.focused.expect("a stop");
    (
        id,
        r.out.tree.get(id).expect("the focused node drew").bounds,
    )
}

/// Tabs `n` times: every stop lands wholly in the view, and they are `stops` different ones.
fn walk(blocks: &[MdBlock<'_>], n: usize, stops: usize) {
    let mut h = drawn(blocks, &live());
    let mut ms = 0;
    let mut seen = Vec::new();
    for i in 0..n {
        let (id, b) = tab_to(&mut h, blocks, &mut ms);
        assert!(
            b.y >= RECT.y && b.bottom() <= RECT.bottom(),
            "stop {i} at {b:?}"
        );
        if !seen.contains(&id) {
            seen.push(id);
        }
    }
    assert!(seen.len() >= stops, "{} stops", seen.len());
}

#[test]
fn tab_brings_each_link_of_a_long_list_into_view() {
    let urls: Vec<String> = (0..60).map(|i| format!("https://x.dev/{i}")).collect();
    let inlines: Vec<[MdInline<'_>; 1]> = urls.iter().map(|u| [link("go", u)]).collect();
    let paras: Vec<[MdBlock<'_>; 1]> = inlines.iter().map(|i| [MdBlock::Paragraph(i)]).collect();
    let items: Vec<MdItem<'_>> = paras
        .iter()
        .map(|p| MdItem {
            marker: "-",
            blocks: p,
            task: None,
        })
        .collect();
    walk(&[MdBlock::List { items: &items }], 40, 40);
}

#[test]
fn tab_brings_each_task_box_into_view() {
    let text = [t("ship it")];
    let para = [MdBlock::Paragraph(&text)];
    let items: Vec<MdItem<'_>> = (0..40)
        .map(|id| MdItem {
            marker: "-",
            blocks: &para,
            task: Some(MdTask { checked: false, id }),
        })
        .collect();
    walk(&[MdBlock::List { items: &items }], 30, 30);
}

#[test]
fn tab_brings_a_link_in_a_tall_table_into_view_though_the_table_scrolls_only_sideways() {
    let inlines = [link("go", "https://x.dev")];
    let cell: [&[MdInline<'_>]; 1] = [&inlines];
    let rows: Vec<MdRow<'_>> = (0..40)
        .map(|_| MdRow {
            header: false,
            cells: &cell,
        })
        .collect();
    let blocks = [MdBlock::Table {
        rows: &rows,
        align: &[],
    }];
    walk(&blocks, 30, 30);
}

#[test]
fn tab_brings_a_code_blocks_copy_button_into_view() {
    // Ten paragraphs of 28 from a margin of 12: the block starts 8 px above the view's end.
    let text = [t("para")];
    let mut blocks: Vec<MdBlock<'_>> = vec![MdBlock::Paragraph(&text); 10];
    blocks.push(MdBlock::Code {
        text: "let a = 1;\nlet b = 2;\nlet c = 3;",
        spans: &[],
    });
    let mut h = drawn(&blocks, &live());
    let mut ms = 0;
    let (_, b) = tab_to(&mut h, &blocks, &mut ms);
    assert!(b.y >= RECT.y && b.bottom() <= RECT.bottom(), "{b:?}");
}

#[test]
fn a_focused_link_scrolled_out_of_the_window_keeps_the_focus_and_is_no_stop() {
    let urls: Vec<String> = (0..80).map(|i| format!("https://x.dev/{i}")).collect();
    // Two links a paragraph: the second must not be a stop while its block is out of the window.
    let items: Vec<[MdInline<'_>; 3]> = urls
        .iter()
        .map(|u| [link("go", u), t(" and "), link("on", u)])
        .collect();
    let blocks: Vec<MdBlock<'_>> = items.iter().map(|i| MdBlock::Paragraph(i)).collect();
    let mut h = drawn(&blocks, &live());
    let mut ms = 0;
    let (id, _) = tab_to(&mut h, &blocks, &mut ms);

    // Away, far past the window: the block stays built, off the view.
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -600.0);
    run(&mut h, &blocks, &live());
    let r = run(&mut h, &blocks, &live());
    assert_eq!(r.out.focused, Some(id));
    let b = r
        .out
        .tree
        .get(id)
        .expect("the focused link stays built")
        .bounds;
    assert!(b.bottom() < RECT.y, "{b:?}");

    // Nearly back: the wheel's place stands, no reveal takes it over.
    h.input.scroll_pixels(0.0, 4.0);
    run(&mut h, &blocks, &live());
    ms += 2_000;
    let moved = at(&mut h, ms, &blocks, &live());
    let c = moved.out.tree.get(id).expect("built").bounds;
    assert_eq!(c.y, b.y + 4.0);

    // The next stop is in the view, not the block that was left behind.
    let (next, n) = tab_to(&mut h, &blocks, &mut ms);
    assert!(n.y >= RECT.y && n.bottom() <= RECT.bottom(), "{n:?}");
    let r = run(&mut h, &blocks, &live());
    let node = r.out.tree.get(next).expect("drew");
    assert_ne!(node.description.as_deref(), Some(urls[0].as_str()));
}

/// The y of "para 0"'s text.
fn top_of(h: &mut UiHarness, blocks: &[MdBlock<'_>], ms: u64) -> Option<f32> {
    run(h, blocks, &live());
    text_at(&at(h, ms, blocks, &live()).scene, body("para 0")).map(|p| p.1)
}

/// A press at `on`, then the Page keys, Home and End: each moves the document.
fn page_keys_walk(blocks: &[MdBlock<'_>], on: (f32, f32)) {
    let mut h = drawn(blocks, &live());
    let home = top_of(&mut h, blocks, 100);
    assert!(home.is_some(), "the instrument sees the first paragraph");
    h.input.pointer_to(on.0, on.1).click(P);
    run(&mut h, blocks, &live());
    let focused = run(&mut h, blocks, &live()).out.focused;
    assert!(focused.is_some(), "the press focused the text");

    press(&mut h, NamedKey::PageDown);
    let paged = top_of(&mut h, blocks, 2_000);
    assert!(paged.is_none_or(|y| y < RECT.y), "PageDown moved past it");
    press(&mut h, NamedKey::Home);
    assert_eq!(top_of(&mut h, blocks, 4_000), home, "Home is the top");
    press(&mut h, NamedKey::End);
    run(&mut h, blocks, &live());
    let last = text_at(&at(&mut h, 6_000, blocks, &live()).scene, body("para 79"));
    assert!(last.is_some(), "End is the bottom");
    press(&mut h, NamedKey::PageUp);
    run(&mut h, blocks, &live());
    let moved = text_at(&at(&mut h, 8_000, blocks, &live()).scene, body("para 79"));
    assert_ne!(moved, last, "PageUp moved");
}

#[test]
fn the_page_keys_scroll_the_document_around_a_focused_paragraph_heading_and_html_block() {
    let texts: Vec<String> = (0..80).map(|i| format!("para {i}")).collect();
    let items: Vec<[MdInline<'_>; 1]> = texts.iter().map(|s| [t(s)]).collect();
    let paras = || items.iter().map(|i| MdBlock::Paragraph(i));

    // A press on the first paragraph's text.
    let on = (X + 5.0, RECT.y + MARGIN + 8.0);
    page_keys_walk(&paras().collect::<Vec<_>>(), on);

    let title = [t("Title")];
    let mut blocks = vec![MdBlock::Heading {
        level: 1,
        inlines: &title,
    }];
    blocks.extend(paras());
    page_keys_walk(&blocks, on);

    let mut blocks = vec![MdBlock::Html("<b>raw</b>")];
    blocks.extend(paras());
    page_keys_walk(&blocks, (X + 14.0, RECT.y + MARGIN + 14.0));
}

#[test]
fn the_page_keys_reach_the_document_from_a_text_cell_of_a_table_that_fits() {
    let texts: Vec<String> = (0..80).map(|i| format!("para {i}")).collect();
    let items: Vec<[MdInline<'_>; 1]> = texts.iter().map(|s| [t(s)]).collect();
    let word = [t("cell")];
    let cell: [&[MdInline<'_>]; 1] = [&word];
    let rows = [MdRow {
        header: false,
        cells: &cell,
    }];
    let mut blocks = vec![MdBlock::Table {
        rows: &rows,
        align: &[],
    }];
    blocks.extend(items.iter().map(|i| MdBlock::Paragraph(i)));
    let on = (X + 16.0, RECT.y + MARGIN + 14.0);
    page_keys_walk(&blocks, on);
}

#[test]
fn shift_with_the_page_keys_still_extends_the_selection_and_a_chord_stays_the_engines() {
    let text = [t("para zero")];
    let more = [t("next")];
    let mut blocks = vec![MdBlock::Paragraph(&text)];
    blocks.extend((0..60).map(|_| MdBlock::Paragraph(&more)));
    let mut h = drawn(&blocks, &live());
    h.input.pointer_to(X + 2.0, RECT.y + MARGIN + 8.0).click(P);
    run(&mut h, &blocks, &live());
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    h.input.set_modifiers(shift);
    h.input.key_down(Key::Named(NamedKey::End), None);
    h.input.set_modifiers(Modifiers::default());
    let r = run(&mut h, &blocks, &live());
    let id = r.out.focused.expect("focused");
    let node = r.out.tree.get(id).expect("the paragraph");
    let (a, b) = node.text.as_ref().expect("text").selection;
    assert!(b > a, "Shift+End selected: {a}..{b}");
    let h0 = text_at(&r.scene, body("para zero"));
    assert!(h0.is_some(), "and nothing scrolled");
}

/// The first text node in view of `out`: its name and top.
fn top_node(out: &FrameOutput) -> (String, f32) {
    let n = nodes(out, Role::StaticText)
        .into_iter()
        .find(|n| n.bounds.bottom() > RECT.y)
        .expect("a text in view");
    (n.name.clone(), n.bounds.y)
}

fn long_paras(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| format!("para{i} {}", "word ".repeat(120)))
        .collect()
}

#[test]
fn a_width_change_keeps_the_readers_place() {
    let texts = long_paras(200);
    let items: Vec<[MdInline<'_>; 1]> = texts.iter().map(|s| [t(s)]).collect();
    let blocks: Vec<MdBlock<'_>> = items.iter().map(|i| MdBlock::Paragraph(i)).collect();
    let shown = |h: &mut UiHarness, w: f32| {
        let mut out = None;
        let rect = Rect { w, ..RECT };
        for _ in 0..3 {
            let (_, o) = h.frame(|ui| {
                ui.markdown("m", rect, &blocks, &live());
            });
            out = Some(o);
        }
        top_node(&out.expect("ran"))
    };
    let mut h = UiHarness::new(WIN);
    shown(&mut h, RECT.w);
    h.input
        .pointer_to(100.0, 100.0)
        .scroll_pixels(0.0, -9_000.0);
    let place = shown(&mut h, RECT.w);
    assert_ne!(place.0, texts[0], "it scrolled");
    for step in 1..=4 {
        let now = shown(&mut h, RECT.w + 30.0 * step as f32);
        assert_eq!(now, place, "after {step} width changes");
    }
}

#[test]
fn a_block_measured_above_the_window_shifts_the_area_so_what_shows_stays() {
    let texts = long_paras(300);
    let items: Vec<[MdInline<'_>; 1]> = texts.iter().map(|s| [t(s)]).collect();
    let blocks: Vec<MdBlock<'_>> = items.iter().map(|i| MdBlock::Paragraph(i)).collect();
    let mut h = drawn(&blocks, &live());
    // Far down on estimates: going up, each block measures taller than its estimate.
    h.input
        .pointer_to(100.0, 100.0)
        .scroll_pixels(0.0, -6_000.0);
    run(&mut h, &blocks, &live());
    run(&mut h, &blocks, &live());
    let mut last = run(&mut h, &blocks, &live()).out;
    let mut grown = false;
    for _ in 0..60 {
        h.input.scroll_pixels(0.0, 5.0);
        run(&mut h, &blocks, &live());
        let now = run(&mut h, &blocks, &live()).out;
        for n in nodes(&now, Role::StaticText) {
            if let Some(was) = nodes(&last, Role::StaticText)
                .into_iter()
                .find(|w| w.name == n.name)
            {
                assert_eq!(n.bounds.y, was.bounds.y + 5.0, "{:.8}", n.name);
            } else {
                grown = true;
            }
        }
        last = now;
    }
    assert!(grown, "new blocks came into view");
}

// ---- a new identity: another document ------------------------------------------------

fn doc(identity: u64) -> MdOptions<'static> {
    MdOptions {
        identity: Some(identity),
        ..live()
    }
}

/// One frame with the document in a live keep-alive scope, which keeps ids nothing draws.
fn alive(h: &mut UiHarness, blocks: &[MdBlock<'_>], opts: &MdOptions<'_>) -> Run {
    let mut r = None;
    let (scene, out) = h.frame(|ui| {
        ui.keep_alive("tab", |ui| r = Some(ui.markdown("m", RECT, blocks, opts)));
    });
    Run {
        scene,
        r: r.expect("ran"),
        out,
    }
}

fn selection(out: &FrameOutput) -> (usize, usize) {
    let first = nodes(out, Role::StaticText);
    first[0].text.as_ref().expect("text").selection
}

/// The x of the leftmost text drawn.
fn leftmost(scene: &Scene) -> f32 {
    let texts = &scene.layers[0].texts;
    texts.iter().map(|t| t.x).fold(f32::MAX, f32::min)
}

#[test]
fn a_new_identity_leaves_no_selection_no_focus_and_nothing_to_copy() {
    let (a, b) = ([t("alpha alpha")], [t("bravo bravo bravo bravo")]);
    let (one, two) = ([MdBlock::Paragraph(&a)], [MdBlock::Paragraph(&b)]);
    let mut h = UiHarness::new(WIN);
    run(&mut h, &one, &doc(1));
    run(&mut h, &one, &doc(1));
    h.input.pointer_to(X + 5.0, RECT.y + MARGIN + 8.0).click(P);
    run(&mut h, &one, &doc(1));
    chord(&mut h, "a");
    run(&mut h, &one, &doc(1));
    let r = run(&mut h, &one, &doc(1));
    assert_eq!(
        selection(&r.out),
        (0, 11),
        "the instrument sees a selection"
    );
    assert!(r.out.focused.is_some());

    run(&mut h, &two, &doc(2));
    chord(&mut h, "c");
    let r = run(&mut h, &two, &doc(2));
    let (start, end) = selection(&r.out);
    assert_eq!(start, end, "doc B shows no selection");
    assert_eq!(r.out.focused, None, "the block's focus went with doc A");
    assert!(r.out.clipboard.is_empty(), "{:?}", r.out.clipboard);
}

#[test]
fn no_identity_keeps_a_blocks_selection_across_frames() {
    let a = [t("alpha alpha")];
    let one = [MdBlock::Paragraph(&a)];
    let mut h = drawn(&one, &live());
    h.input.pointer_to(X + 5.0, RECT.y + MARGIN + 8.0).click(P);
    run(&mut h, &one, &live());
    chord(&mut h, "a");
    run(&mut h, &one, &live());
    run(&mut h, &one, &live());
    let r = run(&mut h, &one, &live());
    assert_eq!(selection(&r.out), (0, 11));
    assert!(r.out.focused.is_some());
}

/// A sideways wheel at `on` moves the block; a new identity puts it back.
fn a_new_identity_resets_the_block_sideways(blocks: &[MdBlock<'_>], on: (f32, f32)) {
    let mut h = UiHarness::new(WIN);
    run(&mut h, blocks, &doc(1));
    let home = leftmost(&run(&mut h, blocks, &doc(1)).scene);
    h.input.pointer_to(on.0, on.1).scroll_pixels(-40.0, 0.0);
    run(&mut h, blocks, &doc(1));
    let moved = leftmost(&run(&mut h, blocks, &doc(1)).scene);
    assert_eq!(moved, home - 40.0, "the instrument sees the offset");
    assert_eq!(leftmost(&run(&mut h, blocks, &doc(2)).scene), home);
}

#[test]
fn a_new_identity_resets_a_tables_sideways_offset() {
    let wide = "w".repeat(120);
    let (a, b) = ([t("h")], [t(&wide)]);
    let cells = [[&a[..], &a[..], &a[..]], [&b[..], &a[..], &a[..]]];
    let rows = table_doc(&cells);
    let blocks = [MdBlock::Table {
        rows: &rows,
        align: &[],
    }];
    a_new_identity_resets_the_block_sideways(&blocks, (100.0, RECT.y + MARGIN + 40.0));
}

#[test]
fn a_new_identity_resets_a_code_blocks_sideways_offset() {
    let wide = "z".repeat(300);
    let blocks = [MdBlock::Code {
        text: &wide,
        spans: &[],
    }];
    a_new_identity_resets_the_block_sideways(&blocks, (60.0, RECT.y + MARGIN + 20.0));
}

#[test]
fn a_document_reopened_under_a_keep_alive_scope_finds_nothing_of_its_last_showing() {
    let wide = "w".repeat(120);
    let (a, b, text) = ([t("h")], [t(&wide)], [t("alpha alpha")]);
    let cells = [[&a[..], &a[..], &a[..]], [&b[..], &a[..], &a[..]]];
    let rows = table_doc(&cells);
    let blocks = [
        MdBlock::Paragraph(&text),
        MdBlock::Table {
            rows: &rows,
            align: &[],
        },
    ];
    let mut h = UiHarness::new(WIN);
    alive(&mut h, &blocks, &doc(1));
    let home = alive(&mut h, &blocks, &doc(1));
    let table = RECT.y + MARGIN + 28.0 + 40.0;
    h.input.pointer_to(100.0, table).scroll_pixels(-40.0, 0.0);
    alive(&mut h, &blocks, &doc(1));
    h.input.pointer_to(X + 5.0, RECT.y + MARGIN + 8.0).click(P);
    alive(&mut h, &blocks, &doc(1));
    chord(&mut h, "a");
    alive(&mut h, &blocks, &doc(1));
    let r = alive(&mut h, &blocks, &doc(1));
    assert_eq!(
        selection(&r.out),
        (0, 11),
        "the instrument sees a selection"
    );
    assert!(r.out.focused.is_some());
    let key = key_of(&Spec::plain(&*wide, ui_style(13.0, DARK.fg_primary)));
    let at = text_at(&home.scene, key).expect("the wide cell").0;
    assert_eq!(text_at(&r.scene, key).expect("drawn").0, at - 40.0);

    alive(&mut h, &blocks, &doc(2));
    assert_eq!(alive(&mut h, &blocks, &doc(2)).out.focused, None);
    alive(&mut h, &blocks, &doc(1));
    let r = alive(&mut h, &blocks, &doc(1));
    let (start, end) = selection(&r.out);
    assert_eq!(start, end, "the last showing's selection stayed behind");
    assert_eq!(r.out.focused, None);
    assert_eq!(text_at(&r.scene, key).expect("drawn").0, at);
}

#[test]
fn a_focused_link_in_a_table_goes_with_its_document_under_a_keep_alive_scope() {
    let (a, b) = ([link("go", "https://x.dev")], [t("h")]);
    let cells = [[&b[..], &b[..], &b[..]], [&a[..], &b[..], &b[..]]];
    let rows = table_doc(&cells);
    let blocks = [MdBlock::Table {
        rows: &rows,
        align: &[],
    }];
    let mut h = UiHarness::new(WIN);
    alive(&mut h, &blocks, &doc(1));
    alive(&mut h, &blocks, &doc(1));
    press(&mut h, NamedKey::Tab);
    alive(&mut h, &blocks, &doc(1));
    let r = alive(&mut h, &blocks, &doc(1));
    assert!(r.out.focused.is_some(), "the link took the focus");
    alive(&mut h, &blocks, &doc(2));
    assert_eq!(alive(&mut h, &blocks, &doc(2)).out.focused, None);
}

#[test]
fn a_new_identity_takes_no_shift_the_last_document_measured() {
    let texts = long_paras(300);
    let items: Vec<[MdInline<'_>; 1]> = texts.iter().map(|s| [t(s)]).collect();
    let blocks: Vec<MdBlock<'_>> = items.iter().map(|i| MdBlock::Paragraph(i)).collect();
    let mut h = drawn(&blocks, &doc(1));
    h.input
        .pointer_to(100.0, 100.0)
        .scroll_pixels(0.0, -6_000.0);
    run(&mut h, &blocks, &doc(1));
    run(&mut h, &blocks, &doc(1));
    // Up onto blocks never measured: this frame queues their growth for the area.
    h.input.scroll_pixels(0.0, 400.0);
    run(&mut h, &blocks, &doc(1));
    let r = run(&mut h, &blocks, &doc(2));
    let top = nodes(&r.out, Role::StaticText)[0].bounds.y;
    assert_eq!(top, RECT.y + MARGIN);
}

// ---- the focus outside the window, and a press that shows in part ---------------------

#[test]
fn a_focused_link_left_below_the_window_keeps_the_focus_and_its_sibling_is_no_stop() {
    let urls: Vec<String> = (0..80).map(|i| format!("https://x.dev/{i}")).collect();
    // Two links a paragraph: the second must not be a stop while its block is out of the window.
    let items: Vec<[MdInline<'_>; 3]> = urls
        .iter()
        .map(|u| [link("go", u), t(" and "), link("on", u)])
        .collect();
    let blocks: Vec<MdBlock<'_>> = items.iter().map(|i| MdBlock::Paragraph(i)).collect();
    let mut h = drawn(&blocks, &live());
    let mut ms = 0;
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -600.0);
    run(&mut h, &blocks, &live());
    run(&mut h, &blocks, &live());
    let (id, _) = tab_to(&mut h, &blocks, &mut ms);
    let r = run(&mut h, &blocks, &live());
    let url = r.out.tree.get(id).expect("drew").description.clone();

    press(&mut h, NamedKey::Home);
    run(&mut h, &blocks, &live());
    ms += 2_000;
    let r = at(&mut h, ms, &blocks, &live());
    assert_eq!(r.out.focused, Some(id), "the focus stayed through Home");
    let b = r
        .out
        .tree
        .get(id)
        .expect("the focused link stays built")
        .bounds;
    assert!(b.y > RECT.bottom(), "{b:?}");

    // The next stop is in the view, not the other link of the block left behind.
    let (next, n) = tab_to(&mut h, &blocks, &mut ms);
    assert!(n.y >= RECT.y && n.bottom() <= RECT.bottom(), "{n:?}");
    let r = run(&mut h, &blocks, &live());
    let node = r.out.tree.get(next).expect("drew");
    assert_ne!(node.description, url);
}

#[test]
fn a_press_on_a_link_that_shows_in_part_leaves_the_document_still() {
    // Eleven paragraphs of 25 from a margin of 12: the link starts 13 px above the view's end.
    let (text, go) = ([t("para")], [link("go", "https://x.dev")]);
    let mut blocks: Vec<MdBlock<'_>> = vec![MdBlock::Paragraph(&text); 11];
    blocks.push(MdBlock::Paragraph(&go));
    blocks.extend(vec![MdBlock::Paragraph(&text); 20]);
    let mut h = drawn(&blocks, &live());
    let home = text_at(&run(&mut h, &blocks, &live()).scene, body("para"));
    assert!(home.is_some(), "the instrument sees the first paragraph");

    let r = run(&mut h, &blocks, &live());
    let b = nodes(&r.out, Role::Link)[0].bounds;
    assert!(b.y < RECT.bottom() && b.bottom() > RECT.bottom(), "{b:?}");
    h.input.pointer_to(b.x + 5.0, RECT.bottom() - 3.0).press(P);
    let r = run(&mut h, &blocks, &live());
    assert_eq!(r.r.link.as_deref(), Some("https://x.dev"));
    h.input.release(P);
    run(&mut h, &blocks, &live());
    let mut ms = 0;
    for _ in 0..6 {
        ms += 400;
        let r = at(&mut h, ms, &blocks, &live());
        assert!(r.out.focused.is_some(), "the press focused the link");
        assert_eq!(text_at(&r.scene, body("para")), home);
    }
}
