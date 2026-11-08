//! The panel (`layout.panel` acceptance): the outline-only box with the radius and ring probes at
//! scale 2, the header and footer paddings, the five kinds' bodies, `fit` against filling with a
//! pinned footer, the unavailable line, hidden slots, the text limit near the rounded corners, the
//! rounded clip of a flush body, the semantic nodes, tab order and the disabled walk.

use lune_ui_core::access::{Live, Role, SemanticTree, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, NamedKey, Repaint};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, TextItem};
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style, TextKey, TextSystem, elide};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::panel::{
    BUTTON_ROW, CARD_PAD, PanelKind, PanelOptions, PanelSlot, RADIUS, TEXT_INSET,
};
use lune_ui_core::widgets::stat::StatOptions;

const WIN: (f32, f32) = (900.0, 600.0);
const RECT: Rect = Rect::new(20.0, 30.0, 400.0, 300.0);
const BODY_H: f32 = 100.0;

fn panel_id() -> WidgetId {
    WidgetId::ROOT.with("panel")
}

fn id(key: &str) -> WidgetId {
    panel_id().with(key)
}

#[derive(Clone, Copy)]
struct Cfg {
    rect: Rect,
    kind: PanelKind,
    title: Option<&'static str>,
    meta: Option<&'static str>,
    unavailable: Option<&'static str>,
    note: Option<&'static str>,
    footer_w: f32,
    fit: bool,
    enabled: bool,
}

impl Cfg {
    fn new(kind: PanelKind) -> Cfg {
        Cfg {
            rect: RECT,
            kind,
            title: Some("Upstreams"),
            meta: Some("3 online"),
            unavailable: None,
            note: None,
            footer_w: 0.0,
            fit: false,
            enabled: true,
        }
    }
}

struct Out {
    scene: Scene,
    out: FrameOutput,
    slots: Vec<&'static str>,
    body: Option<Rect>,
    footer: Option<Rect>,
    height: f32,
}

fn row_style() -> Style {
    Style::ui(13.0, [255, 255, 255, 255])
}

fn frame(h: &mut UiHarness, c: &Cfg) -> Out {
    let mut slots = Vec::new();
    let (mut body, mut footer) = (None, None);
    let mut height = 0.0;
    let (scene, out) = h.frame(|ui| {
        let opts = PanelOptions {
            title: c.title,
            meta: c.meta,
            kind: c.kind,
            unavailable: c.unavailable,
            note: c.note,
            footer_w: c.footer_w,
            fit: c.fit,
            name: "",
        };
        height = ui.with_enabled(c.enabled, |ui| {
            ui.panel("panel", c.rect, &opts, |ui, slot| match slot {
                PanelSlot::Body(rect) => {
                    slots.push("body");
                    body = Some(rect);
                    // The caller keeps a flush body's text 8 px in, and a row pads itself at
                    // least 4 more inside the 4 px sides.
                    let inset = match c.kind {
                        PanelKind::Flush => TEXT_INSET,
                        PanelKind::Rows => 4.0,
                        _ => 0.0,
                    };
                    if c.kind == PanelKind::Strip {
                        let cells = [
                            StatOptions {
                                label: "Tokens",
                                value: "1,284",
                                ..StatOptions::default()
                            },
                            StatOptions {
                                label: "Requests",
                                value: "8,412",
                                ..StatOptions::default()
                            },
                            StatOptions {
                                label: "Saved",
                                value: "38%",
                                ..StatOptions::default()
                            },
                        ];
                        return ui.stat_strip("strip", rect, &cells);
                    }
                    ui.painter().text(
                        &Spec::plain("Row text", row_style()),
                        rect.x + inset,
                        rect.y,
                    );
                    ui.button(
                        "body-btn",
                        Rect::new(rect.x + inset, rect.y + 30.0, 80.0, 32.0),
                        "Go",
                        ButtonKind::Secondary,
                    );
                    BODY_H
                }
                PanelSlot::Footer(at) => {
                    slots.push("footer");
                    footer = Some(at);
                    ui.button("foot", at, "Save", ButtonKind::Primary);
                    0.0
                }
            })
        });
    });
    Out {
        scene,
        out,
        slots,
        body,
        footer,
        height,
    }
}

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers[0].texts.clone()
}

fn key_of(spec: &Spec) -> TextKey {
    TextKey(spec.cache_key(1.0))
}

fn title_spec(s: &str) -> Spec {
    Spec::plain(
        s,
        Style {
            line: 20.0,
            ..Style::ui(DARK.font_m, DARK.fg_primary).medium()
        },
    )
}

fn meta_spec(s: &str) -> Spec {
    Spec::plain(
        s,
        Style {
            line: 16.0,
            ..Style::ui(DARK.font_s, DARK.fg_tertiary)
        },
    )
}

fn note_spec(s: &str) -> Spec {
    meta_spec(s)
}

fn at(scene: &Scene, spec: &Spec) -> (f32, f32) {
    let t = texts(scene)
        .into_iter()
        .find(|t| t.key == key_of(spec))
        .unwrap_or_else(|| panic!("text {:?} is drawn", spec.runs[0].0));
    (t.x, t.y)
}

fn width_of(spec: &Spec) -> f32 {
    FakeText.prepare(spec, 1.0).1.0
}

fn focused(tree: &SemanticTree) -> Option<String> {
    tree.nodes
        .iter()
        .find(|n| n.states.contains(State::Focused))
        .map(|n| n.name.clone())
}

fn tab(h: &mut UiHarness, c: &Cfg) -> Out {
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(20);
    frame(h, c)
}

fn rings(scene: &Scene) -> Vec<[f32; 4]> {
    scene.layers[0]
        .geometry
        .over
        .iter()
        .filter(|s| s.params[0] == 1.0)
        .map(|s| s.rect)
        .collect()
}

#[test]
fn it_draws_an_outline_only_at_radius_8_and_last() {
    let mut h = UiHarness::new(WIN);
    let out = frame(&mut h, &Cfg::new(PanelKind::Plain));
    let g = &out.scene.layers[0].geometry;
    let whole = [20.0, 30.0, 400.0, 300.0];
    assert!(g.acrylic.is_empty(), "no glass");
    assert!(
        g.under.iter().all(|s| s.rect != whole),
        "no fill, no shadow"
    );
    assert_eq!(g.over.len(), 1);
    let ring = g.over[0];
    assert_eq!(ring.rect, [20.0, 30.0, 400.0, 300.0]);
    assert_eq!(ring.radii, [RADIUS; 4]);
    assert_eq!(ring.params[1], 1.0);
    assert_eq!(ring.color, premul(DARK.border_subtle));
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let c = Cfg {
        note: Some("Saved to disk"),
        footer_w: 100.0,
        ..Cfg::new(PanelKind::Plain)
    };
    let out = frame(&mut h, &c);
    let g = &out.scene.layers[0].geometry;
    assert_eq!(g.over[0].radii, [16.0; 4], "radius 8 is 16 px");
    assert_eq!(g.over[0].params[1], 2.0, "a 1 px ring is 2 px");
    // Header padding 12 16: the title is 24 px down and 32 px in at this scale (logical 12, 16).
    let t = texts(&out.scene)
        .into_iter()
        .find(|t| t.key == TextKey(title_spec("Upstreams").cache_key(2.0)))
        .expect("title");
    assert_eq!((t.x - RECT.x, t.y - RECT.y), (16.0, 12.0));
    // Body bottom padding 16 (32 px): the body box ends 16 above the 56 high footer.
    let body = out.body.expect("body");
    assert_eq!(RECT.bottom() - body.bottom(), 16.0 + 56.0);
}

#[test]
fn a_plain_panel_fits_header_padding_body_padding_and_returns_the_height() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        fit: true,
        ..Cfg::new(PanelKind::Plain)
    };
    let out = frame(&mut h, &c);
    // Header 12 + 20 + 12, body 0 above, the content, 16 below.
    assert_eq!(out.height, 44.0 + BODY_H + 16.0);
    let body = out.body.expect("body");
    assert_eq!(body.x, RECT.x + 16.0);
    assert_eq!(body.y, RECT.y + 44.0);
    assert_eq!(body.w, RECT.w - 32.0);
    // The ring is the fitted height.
    assert_eq!(rings(&out.scene), [[20.0, 30.0, 400.0, 160.0]]);
    // The title at (16, 12) and the meta at the right, 3 px lower to share a baseline.
    assert_eq!(
        at(&out.scene, &title_spec("Upstreams")),
        (RECT.x + 16.0, RECT.y + 12.0)
    );
    let mw = width_of(&meta_spec("3 online"));
    assert_eq!(
        at(&out.scene, &meta_spec("3 online")),
        (RECT.right() - 16.0 - mw, RECT.y + 15.0)
    );
}

#[test]
fn an_empty_header_and_an_empty_footer_take_no_height() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        title: None,
        meta: None,
        fit: true,
        ..Cfg::new(PanelKind::Flush)
    };
    let out = frame(&mut h, &c);
    assert_eq!(
        out.height, BODY_H,
        "a flush body with no header: nothing else"
    );
    assert_eq!(
        out.body.expect("body"),
        Rect::new(RECT.x, RECT.y, 400.0, 300.0)
    );
    // An empty title string is no title either.
    let c = Cfg {
        title: Some(""),
        meta: Some(""),
        note: Some(""),
        ..c
    };
    let out = frame(&mut h, &c);
    assert_eq!(out.height, BODY_H);
    assert_eq!(out.slots, ["body"], "no footer slot");
    // A plain panel with no header pads its top by 16 so the body does not touch the edge.
    let c = Cfg {
        title: None,
        meta: None,
        ..Cfg::new(PanelKind::Plain)
    };
    let c = Cfg { fit: true, ..c };
    assert_eq!(frame(&mut h, &c).height, 16.0 + BODY_H + 16.0);
}

#[test]
fn rows_pad_0_4_8() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        fit: true,
        ..Cfg::new(PanelKind::Rows)
    };
    let out = frame(&mut h, &c);
    let body = out.body.expect("body");
    assert_eq!((body.x, body.w), (RECT.x + 4.0, RECT.w - 8.0));
    assert_eq!(out.height, 44.0 + BODY_H + 8.0);
}

#[test]
fn a_flush_body_is_edge_to_edge_under_a_hairline() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        fit: true,
        ..Cfg::new(PanelKind::Flush)
    };
    let out = frame(&mut h, &c);
    let body = out.body.expect("body");
    assert_eq!((body.x, body.w), (RECT.x, RECT.w));
    assert_eq!(body.y, RECT.y + 44.0 + 1.0, "under the 1 px hairline");
    assert_eq!(out.height, 44.0 + 1.0 + BODY_H);
    let hair = out.scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|s| s.rect[3] == 1.0)
        .expect("hairline");
    assert_eq!(hair.rect, [RECT.x + 1.0, RECT.y + 44.0, RECT.w - 2.0, 1.0]);
    assert_eq!(hair.color, premul(DARK.border_subtle));
}

#[test]
fn a_strip_body_is_edge_to_edge_and_stat_strip_draws_its_dividers() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        title: None,
        meta: None,
        ..Cfg::new(PanelKind::Strip)
    };
    let out = frame(&mut h, &c);
    let body = out.body.expect("body");
    assert_eq!((body.x, body.w, body.y), (RECT.x, RECT.w, RECT.y));
    // Two dividers between three cells.
    let dividers = out.scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.rect[2] == 1.0 && s.color == premul(DARK.border_subtle))
        .count();
    assert_eq!(dividers, 2);
}

#[test]
fn a_card_pads_12_all_round_and_pins_its_footer_button_with_no_hairline() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        footer_w: 90.0,
        ..Cfg::new(PanelKind::Card)
    };
    let out = frame(&mut h, &c);
    let body = out.body.expect("body");
    assert_eq!(body.x, RECT.x + CARD_PAD);
    assert_eq!(body.w, RECT.w - 2.0 * CARD_PAD);
    let foot = out.footer.expect("footer");
    // 12 from the right and the bottom, BUTTON_ROW high, no hairline above it.
    assert_eq!(
        foot,
        Rect::new(
            RECT.right() - 12.0 - 90.0,
            RECT.bottom() - 12.0 - BUTTON_ROW,
            90.0,
            BUTTON_ROW
        )
    );
    assert!(
        out.scene.layers[0]
            .geometry
            .under
            .iter()
            .all(|s| s.rect[3] != 1.0),
        "no hairline"
    );
    // The title pads 12 as well.
    assert_eq!(
        at(&out.scene, &title_spec("Upstreams")),
        (RECT.x + 12.0, RECT.y + 12.0)
    );
    // Fit: header 44, content, 12, then the 32 + 12 footer band.
    let fit = frame(&mut h, &Cfg { fit: true, ..c });
    assert_eq!(fit.height, 44.0 + BODY_H + 12.0 + BUTTON_ROW + 12.0);
}

#[test]
fn a_filling_panel_pins_its_footer_so_a_row_of_panels_shares_a_bottom_edge() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        note: Some("Saved"),
        footer_w: 100.0,
        ..Cfg::new(PanelKind::Plain)
    };
    let out = frame(&mut h, &c);
    assert_eq!(out.height, RECT.h);
    // The footer row: 12 padding, 32 high, 16 from the right; its hairline 56 above the bottom.
    let foot = out.footer.expect("footer");
    assert_eq!(
        foot,
        Rect::new(
            RECT.right() - 16.0 - 100.0,
            RECT.bottom() - 56.0 + 12.0,
            100.0,
            32.0
        )
    );
    let hair = out.scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|s| s.rect[3] == 1.0)
        .expect("hairline");
    assert_eq!(hair.rect[1], RECT.bottom() - 56.0);
    // A shorter body does not move the footer or the ring.
    let short = frame(
        &mut h,
        &Cfg {
            title: None,
            meta: None,
            ..c
        },
    );
    assert_eq!(short.footer, out.footer);
    assert_eq!(rings(&short.scene), rings(&out.scene));
    // The body room is what is left between header and footer.
    let body = out.body.expect("body");
    assert_eq!(body.bottom(), RECT.bottom() - 56.0 - 16.0);
}

#[test]
fn the_note_sits_left_of_the_buttons_and_is_elided_to_the_room() {
    let mut h = UiHarness::new(WIN);
    let long = "A long footer note that cannot fit beside the buttons at all, whatever it says";
    let c = Cfg {
        note: Some(long),
        footer_w: 200.0,
        ..Cfg::new(PanelKind::Plain)
    };
    let out = frame(&mut h, &c);
    let room = RECT.w - 32.0 - 200.0 - 12.0;
    let want = elide(&mut FakeText, &note_spec(long), 1.0, room);
    assert_ne!(want.runs[0].0, long);
    let (x, y) = at(&out.scene, &want);
    assert_eq!(x, RECT.x + 16.0);
    // Centred on the 32 px button row.
    assert_eq!(y, RECT.bottom() - 56.0 + 12.0 + 8.0);
    // A note alone makes a 40 px footer and has no button slot.
    let c = Cfg {
        note: Some("Saved"),
        footer_w: 0.0,
        ..Cfg::new(PanelKind::Plain)
    };
    let out = frame(&mut h, &c);
    assert_eq!(out.slots, ["body"]);
    assert_eq!(
        out.body.expect("body").bottom(),
        RECT.bottom() - 40.0 - 16.0
    );
    let (_, y) = at(&out.scene, &note_spec("Saved"));
    assert_eq!(y, RECT.bottom() - 40.0 + 12.0);
}

#[test]
fn unavailable_replaces_the_body_with_one_tertiary_status_line() {
    let mut h = UiHarness::new(WIN);
    let msg = "Unavailable while the proxy is stopped.";
    let c = Cfg {
        unavailable: Some(msg),
        footer_w: 100.0,
        ..Cfg::new(PanelKind::Plain)
    };
    let out = frame(&mut h, &c);
    assert_eq!(out.slots, ["footer"], "the body slot is not called");
    let line = Spec::plain(
        msg,
        Style {
            line: 20.0,
            ..Style::ui(DARK.font_base, DARK.fg_tertiary)
        },
    );
    assert_eq!(at(&out.scene, &line), (RECT.x + 16.0, RECT.y + 44.0));
    let n = out.out.tree.get(id("unavailable")).expect("node");
    assert_eq!(n.role, Role::Status);
    assert_eq!(n.name, msg);
    assert_eq!(n.live, Some(Live::Polite));
    assert_eq!(n.value, Some(Value::Text(msg.to_owned())));
    assert_eq!(n.parent, Some(panel_id()));
    // Fitted, it is one 20 px line.
    let fit = frame(
        &mut h,
        &Cfg {
            fit: true,
            footer_w: 0.0,
            ..c
        },
    );
    assert_eq!(fit.height, 44.0 + 20.0 + 16.0);
}

#[test]
fn a_flush_unavailable_line_still_keeps_16_from_the_edge() {
    let mut h = UiHarness::new(WIN);
    let msg = "Nothing here";
    let c = Cfg {
        unavailable: Some(msg),
        ..Cfg::new(PanelKind::Flush)
    };
    let out = frame(&mut h, &c);
    let line = Spec::plain(
        msg,
        Style {
            line: 20.0,
            ..Style::ui(DARK.font_base, DARK.fg_tertiary)
        },
    );
    let (x, _) = at(&out.scene, &line);
    assert!(x - RECT.x >= RADIUS);
}

#[test]
fn the_nodes_are_a_group_with_the_note_and_the_buttons_inside() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        note: Some("Saved"),
        footer_w: 100.0,
        ..Cfg::new(PanelKind::Plain)
    };
    let out = frame(&mut h, &c);
    let tree = &out.out.tree;
    let g = tree.get(panel_id()).expect("group");
    assert_eq!(g.role, Role::Group);
    assert_eq!(g.name, "Upstreams");
    assert_eq!(g.description.as_deref(), Some("3 online"));
    assert_eq!(g.bounds, RECT);
    let kids: Vec<(Role, String)> = g
        .children
        .iter()
        .map(|c| {
            let n = tree.get(*c).expect("child");
            (n.role, n.name.clone())
        })
        .collect();
    assert_eq!(
        kids,
        [
            (Role::Button, "Go".to_owned()),
            (Role::StaticText, "Saved".to_owned()),
            (Role::Button, "Save".to_owned()),
        ]
    );
    // A fitted panel's bounds are its height.
    let fit = frame(&mut h, &Cfg { fit: true, ..c });
    assert_eq!(
        fit.out.tree.get(panel_id()).expect("group").bounds,
        Rect::new(20.0, 30.0, 400.0, fit.height)
    );
    // The name falls back to nothing without a title, and an explicit one wins.
    let (_, o) = h.frame(|ui| {
        let opts = PanelOptions {
            name: "Stats",
            ..PanelOptions::default()
        };
        ui.panel("panel", RECT, &opts, |_, _| 0.0);
    });
    assert_eq!(o.tree.get(panel_id()).expect("group").name, "Stats");
}

#[test]
fn tab_order_is_body_then_footer() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        footer_w: 100.0,
        ..Cfg::new(PanelKind::Plain)
    };
    frame(&mut h, &c);
    assert_eq!(focused(&tab(&mut h, &c).out.tree).as_deref(), Some("Go"));
    assert_eq!(focused(&tab(&mut h, &c).out.tree).as_deref(), Some("Save"));
}

#[test]
fn the_text_the_panel_draws_keeps_its_edges_8_from_the_outline() {
    let long_title = "A title long enough to need eliding beside the meta, said at length";
    let cases = [
        PanelKind::Plain,
        PanelKind::Rows,
        PanelKind::Flush,
        PanelKind::Strip,
        PanelKind::Card,
    ];
    for kind in cases {
        for (title, note, unavailable) in [
            (Some("Upstreams"), Some("Saved"), None),
            (
                Some(long_title),
                Some("A note that is long enough to be elided beside the buttons, surely"),
                None,
            ),
            (
                Some("Upstreams"),
                None,
                Some("Unavailable while the proxy is stopped."),
            ),
        ] {
            let mut h = UiHarness::new(WIN);
            let c = Cfg {
                title,
                note,
                unavailable,
                footer_w: 120.0,
                ..Cfg::new(kind)
            };
            let out = frame(&mut h, &c);
            // Every text is one of ours, an elided one of ours, or the caller's body line (which
            // keeps its own inset, checked too) or a button label.
            let mut specs = vec![
                title_spec(title.unwrap_or("")),
                meta_spec("3 online"),
                Spec::plain("Row text", row_style()),
            ];
            let room_t = RECT.w
                - 2.0 * if kind == PanelKind::Card { 12.0 } else { 16.0 }
                - width_of(&meta_spec("3 online"))
                - 12.0;
            specs.push(elide(
                &mut FakeText,
                &title_spec(title.unwrap_or("")),
                1.0,
                room_t,
            ));
            if let Some(n) = note {
                let pad = if kind == PanelKind::Card { 12.0 } else { 16.0 };
                let room = RECT.w - 2.0 * pad - 120.0 - 12.0;
                specs.push(note_spec(n));
                specs.push(elide(&mut FakeText, &note_spec(n), 1.0, room));
            }
            if let Some(u) = unavailable {
                specs.push(Spec::plain(
                    u,
                    Style {
                        line: 20.0,
                        ..Style::ui(DARK.font_base, DARK.fg_tertiary)
                    },
                ));
            }
            let mut seen = 0;
            for t in texts(&out.scene) {
                let Some(spec) = specs.iter().find(|s| key_of(s) == t.key) else {
                    continue;
                };
                seen += 1;
                let w = width_of(spec);
                assert!(
                    t.x - RECT.x >= TEXT_INSET,
                    "{kind:?} left edge of {:?}: {}",
                    spec.runs[0].0,
                    t.x - RECT.x
                );
                assert!(
                    RECT.right() - (t.x + w) >= TEXT_INSET,
                    "{kind:?} right edge of {:?}",
                    spec.runs[0].0
                );
            }
            assert!(seen >= 3, "{kind:?}: only {seen} texts matched");
        }
    }
}

#[test]
fn a_flush_body_fill_is_cut_by_the_rounded_clip_at_the_panels_rect() {
    let mut h = UiHarness::new(WIN);
    let (scene, _) = h.frame(|ui| {
        let opts = PanelOptions {
            title: Some("Rows"),
            kind: PanelKind::Flush,
            ..PanelOptions::default()
        };
        ui.panel("panel", RECT, &opts, |ui, slot| {
            if let PanelSlot::Body(r) = slot {
                ui.painter().fill(
                    Rect::new(r.x, r.bottom() - 30.0, r.w, 30.0),
                    0.0,
                    [200, 0, 0, 255],
                );
            }
            0.0
        });
    });
    let fills: Vec<_> = scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.color == premul([200, 0, 0, 255]))
        .collect();
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].round, [RECT.x, RECT.y, RECT.w, RECT.h]);
    assert_eq!(fills[0].round_radius, RADIUS);
    // The ring is not clipped by it: it is drawn after the clip was popped.
    let ring = scene.layers[0].geometry.over.last().expect("ring");
    assert_eq!(ring.round_radius, 0.0);
}

#[test]
fn a_fit_panel_cuts_a_flush_body_at_its_final_height_from_the_second_frame() {
    let red = [200, 0, 0, 255];
    let mut h = UiHarness::new(WIN);
    let one = |h: &mut UiHarness| {
        let mut height = 0.0;
        let (scene, out) = h.frame(|ui| {
            let opts = PanelOptions {
                title: Some("Rows"),
                kind: PanelKind::Flush,
                fit: true,
                ..PanelOptions::default()
            };
            height = ui.panel("panel", RECT, &opts, |ui, slot| {
                if let PanelSlot::Body(r) = slot {
                    ui.painter().fill(Rect::new(r.x, r.y, r.w, 30.0), 0.0, red);
                }
                0.0
            });
        });
        let fills: Vec<_> = scene.layers[0]
            .geometry
            .under
            .iter()
            .filter(|s| s.color == premul(red))
            .map(|s| s.round)
            .collect();
        (height, fills, out.repaint)
    };
    // A title line and its hairline: shorter than the rect given.
    let (height, fills, repaint) = one(&mut h);
    assert_eq!(height, 45.0);
    assert!(height < RECT.h);
    // The first frame cuts at the rect given and asks for the frame that cuts at the height.
    assert_eq!(fills, vec![[RECT.x, RECT.y, RECT.w, RECT.h]]);
    assert_eq!(repaint, Repaint::Now);
    let (_, fills, repaint) = one(&mut h);
    assert_eq!(fills, vec![[RECT.x, RECT.y, RECT.w, 45.0]]);
    assert_eq!(repaint, Repaint::Idle, "a settled panel is idle");
}

#[test]
fn a_disabled_panel_is_disabled_all_the_way_down() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        enabled: false,
        note: Some("Saved"),
        footer_w: 100.0,
        ..Cfg::new(PanelKind::Plain)
    };
    let out = frame(&mut h, &c);
    let bad = disabled_violations(&out.out.tree, panel_id());
    assert!(bad.is_empty(), "{bad:?}");
    assert_eq!(
        focused(&tab(&mut h, &c).out.tree),
        None,
        "nothing takes focus"
    );
    // Unavailable too.
    let c = Cfg {
        unavailable: Some("Off"),
        ..c
    };
    let out = frame(&mut h, &c);
    assert!(disabled_violations(&out.out.tree, panel_id()).is_empty());
}

#[test]
fn the_panel_takes_no_keys_of_its_own() {
    for enabled in [true, false] {
        for k in [
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
            NamedKey::Home,
            NamedKey::PageDown,
        ] {
            let mut h = UiHarness::new(WIN);
            h.shortcuts
                .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
            let c = Cfg {
                enabled,
                ..Cfg::new(PanelKind::Plain)
            };
            frame(&mut h, &c);
            h.input.key_down(Key::Named(k), None);
            h.at_ms(10);
            let out = frame(&mut h, &c);
            assert_eq!(out.out.fired.len(), 1, "{k:?}, enabled {enabled}");
        }
    }
}
