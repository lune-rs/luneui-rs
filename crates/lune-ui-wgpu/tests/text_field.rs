//! The text field on the real text system: `SpecLayout` and the engine over `TextCache`, rest
//! identity with the Painter API, and the composition's pixel probes at scale 2, drawn through the
//! real renderer into an offscreen target and read back.
//!
//! GPU test: with no adapter the probes print `skipped: no GPU adapter` and pass, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use std::time::Duration;

use lune_ui_core::input::{FrameOutput, ImeEvent, Key, Keymap, NamedKey, PointerButton};
use lune_ui_core::motion::{Cadence, MotionTier, Time};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::shortcuts::Shortcuts;
use lune_ui_core::strings::{DisplayFormat, StringTable};
use lune_ui_core::testing::ScriptedInput;
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::text_edit::{EditCommand, EditFlags, Motion, SpecLayout, TextEdit};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{FrameConfig, Ui, UiStore, WidgetId};
use lune_ui_core::widgets::field::{FieldOptions, field};
use lune_ui_core::widgets::text_area::AreaOptions;
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const SCALE: f32 = 2.0;
/// Device px.
const SIZE: (u32, u32) = (400, 120);
const WIN: (f32, f32) = (200.0, 60.0);
const RECT: Rect = Rect::new(10.0, 10.0, 180.0, 32.0);
const TX: f32 = 20.0;

fn text_cache() -> TextCache {
    let fonts: [&[u8]; 3] = [
        include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Medium.ttf"),
    ];
    TextCache::new(&fonts, "RznSans", "RobotoMono Nerd Font Mono")
}

fn style() -> Style {
    Style::ui(DARK.font_base, DARK.fg_primary)
}

fn headless_gpu() -> Option<Gpu> {
    match pollster::block_on(Gpu::new(gpu::instance(None), None)) {
        Ok(g) => Some(g),
        Err(e) => {
            let required = std::env::var("LUNEUI_REQUIRE_GPU").is_ok_and(|v| v == "1");
            assert!(!required, "no GPU adapter: {e}");
            println!("skipped: no GPU adapter");
            None
        }
    }
}

fn render(gpu: &mut Gpu, text: &mut TextCache, scene: &Scene) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = FrameRenderer::new(gpu, format);
    let texture = dump::target(&gpu.device, format, SIZE);
    let view = texture.create_view(&Default::default());
    let params = FrameParams {
        size: SIZE,
        scale: SCALE,
        blur_sigma: 4.0,
        saturation: 1.0,
        ..FrameParams::default()
    };
    renderer
        .draw(gpu, text, scene, &view, params)
        .expect("draw");
    dump::read_rgba(&gpu.device, &gpu.queue, &texture).expect("readback")
}

fn px(img: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * SIZE.0 as usize + x) * 4;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

/// One window on the real text system.
struct Window {
    store: UiStore,
    input: ScriptedInput,
    strings: StringTable,
    shortcuts: Shortcuts,
    text: TextCache,
    ms: u64,
}

impl Window {
    fn new() -> Self {
        Self::sized(WIN)
    }

    fn sized(size: (f32, f32)) -> Self {
        let mut input = ScriptedInput::new(size, SCALE);
        input.focus(true);
        Self {
            store: UiStore::default(),
            input,
            strings: StringTable::default(),
            shortcuts: Shortcuts::default(),
            text: text_cache(),
            ms: 0,
        }
    }

    fn frame(&mut self, field_text: &mut String) -> Scene {
        self.run(|ui| {
            ui.text_field_with("f", RECT, field_text, &FieldOptions::default());
        })
        .0
    }

    fn run(&mut self, draw: impl FnOnce(&mut Ui<'_>)) -> (Scene, FrameOutput) {
        // Far enough apart that every transition has settled.
        self.ms += 1000;
        self.input.set_time(Duration::from_millis(self.ms));
        let raw = self.input.end_frame();
        let config = FrameConfig {
            theme: DARK,
            tier: MotionTier::Full,
            busy_cadence: Cadence::BUSY,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: Keymap::MAC,
            shortcuts: &self.shortcuts,
        };
        let mut ui = Ui::begin(&mut self.store, &raw, config, &mut self.text);
        draw(&mut ui);
        ui.finish()
    }

    /// A field focused and settled over `text`, the caret `back` graphemes from its end.
    fn focused(&mut self, text: &mut String, back: usize) -> Scene {
        self.frame(text);
        self.input.key_down(Key::Named(NamedKey::Tab), None);
        self.frame(text);
        for _ in 0..back {
            self.input.key_down(Key::Named(NamedKey::ArrowLeft), None);
        }
        self.frame(text);
        self.frame(text)
    }
}

fn apply(
    edit: &mut TextEdit,
    system: &mut TextCache,
    text: &mut String,
    wrap: Option<f32>,
    cmd: EditCommand,
) {
    let mut spec = Spec::plain(text.as_str(), style());
    spec.wrap = wrap;
    let mut layout = SpecLayout::new(system, &spec, SCALE);
    edit.apply(cmd, text, &mut layout, Keymap::MAC, Time::default());
}

fn moved(motion: Motion) -> EditCommand {
    EditCommand::Move {
        motion,
        extend: false,
    }
}

#[test]
fn spec_layout_drives_the_engine_over_real_shaping() {
    let mut system = text_cache();
    let mut text = String::from("h\u{e9}llo w\u{f6}rld");
    let mut edit = TextEdit::new();
    let spec = Spec::plain(text.as_str(), style());
    edit.sync(&text, &mut SpecLayout::new(&mut system, &spec, SCALE));
    // Word motion on real UAX #29 segments, in chars across two-byte letters.
    apply(
        &mut edit,
        &mut system,
        &mut text,
        None,
        moved(Motion::WordRight),
    );
    assert_eq!(edit.caret(), 5);
    apply(
        &mut edit,
        &mut system,
        &mut text,
        None,
        moved(Motion::WordRight),
    );
    assert_eq!(edit.caret(), 11);
    apply(
        &mut edit,
        &mut system,
        &mut text,
        None,
        moved(Motion::WordLeft),
    );
    assert_eq!(edit.caret(), 6);
    apply(
        &mut edit,
        &mut system,
        &mut text,
        None,
        moved(Motion::LineEnd),
    );
    assert_eq!(edit.caret(), 11);
    // A real point in the middle of a real glyph run lands on the char offset the caret reports.
    let spec = Spec::plain(text.as_str(), style());
    let at = text.char_indices().nth(3).expect("char").0;
    let x = system.caret(&spec, SCALE, at).x;
    apply(
        &mut edit,
        &mut system,
        &mut text,
        None,
        EditCommand::PlaceCaret((x, 5.0)),
    );
    assert_eq!(edit.caret(), 3);
    apply(
        &mut edit,
        &mut system,
        &mut text,
        None,
        EditCommand::SelectWordAt(1),
    );
    assert_eq!(edit.selection(), 0..5);
}

#[test]
fn real_clusters_go_in_one_backspace() {
    let mut system = text_cache();
    for cluster in [
        "e\u{301}",
        "\u{1F44B}\u{1F3FD}",
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    ] {
        let mut text = format!("a{cluster}");
        let mut edit = TextEdit::new();
        edit.set_selection(usize::MAX, usize::MAX);
        let spec = Spec::plain(text.as_str(), style());
        edit.sync(&text, &mut SpecLayout::new(&mut system, &spec, SCALE));
        let cmd = EditCommand::Backspace { word: false };
        apply(&mut edit, &mut system, &mut text, None, cmd);
        assert_eq!(text, "a", "{cluster:?}");
    }
}

#[test]
fn vertical_motion_crosses_real_soft_wraps() {
    let mut system = text_cache();
    let mut text = "word ".repeat(40);
    let wrap = Some(120.0);
    let mut edit = TextEdit::new();
    edit.set_flags(EditFlags {
        multiline: true,
        ..EditFlags::default()
    });
    let mut spec = Spec::plain(text.as_str(), style());
    spec.wrap = wrap;
    let rows = system.lines(&spec, SCALE);
    assert!(rows.len() > 3, "it wraps");
    edit.sync(&text, &mut SpecLayout::new(&mut system, &spec, SCALE));
    apply(
        &mut edit,
        &mut system,
        &mut text,
        wrap,
        moved(Motion::GraphemeRight),
    );
    apply(
        &mut edit,
        &mut system,
        &mut text,
        wrap,
        moved(Motion::LineDown),
    );
    let at = edit.caret();
    assert!(
        at > rows[1].start && at < rows[1].end,
        "row 1: {at} in {:?}",
        rows[1]
    );
    apply(
        &mut edit,
        &mut system,
        &mut text,
        wrap,
        moved(Motion::LineEnd),
    );
    let end = edit.caret();
    assert!(end <= rows[1].end && end > at, "{end} in {:?}", rows[1]);
    apply(
        &mut edit,
        &mut system,
        &mut text,
        wrap,
        moved(Motion::LineUp),
    );
    assert!(edit.caret() <= rows[0].end);
}

#[test]
fn at_rest_a_focused_field_equals_the_painter_field_on_real_fonts() {
    for s in ["", "lune", "Wide WWWW iiii", "fi ffi \u{e9}"] {
        let mut w = Window::new();
        let mut text = s.to_owned();
        let scene = w.focused(&mut text, 0);
        let mut system = text_cache();
        let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
        let mut p: Painter<'_, u8> = Painter::new(SCALE, win, &mut system, None, DARK);
        field(&mut p, RECT, s, "", true, 1);
        assert_eq!(scene, p.scene, "{s:?}");
    }
}

/// The bounds of the pixels where `a` and `b` differ: `x0, y0, x1, y1`, ends exclusive.
fn diff_bounds(a: &[u8], b: &[u8]) -> Option<(usize, usize, usize, usize)> {
    let mut out: Option<(usize, usize, usize, usize)> = None;
    for y in 0..SIZE.1 as usize {
        for x in 0..SIZE.0 as usize {
            if px(a, x, y) != px(b, x, y) {
                let o = out.get_or_insert((x, y, x + 1, y + 1));
                *o = (o.0.min(x), o.1.min(y), o.2.max(x + 1), o.3.max(y + 1));
            }
        }
    }
    out
}

/// A field showing "lu" + `preedit` + "ne", composed, and the same string typed in, with the same
/// caret: what differs between the two is the composition's marking.
#[test]
fn the_underline_covers_exactly_the_preedit_with_real_fonts() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut composing = Window::new();
    let mut text = String::from("lune");
    composing.focused(&mut text, 2);
    composing.input.ime(ImeEvent::Preedit {
        text: "mm".to_owned(),
        cursor: Some((2, 2)),
    });
    composing.frame(&mut text);
    let a_scene = composing.frame(&mut text);
    assert_eq!(text, "lune");

    let mut typed = Window::new();
    let mut plain = String::from("lummne");
    let b_scene = typed.focused(&mut plain, 2);

    let a = render(&mut gpu, &mut composing.text, &a_scene);
    let b = render(&mut gpu, &mut typed.text, &b_scene);

    let spec = Spec::plain("lummne", style());
    let from = composing.text.caret(&spec, SCALE, 2).x;
    let to = composing.text.caret(&spec, SCALE, 4).x;
    let (x0, x1) = ((TX + from) * SCALE, (TX + to) * SCALE);
    let line = style().line;
    let top = RECT.y + (RECT.h - line) / 2.0;
    let row = ((top + line - 1.0) * SCALE) as usize;

    let d = diff_bounds(&a, &b).expect("the composition draws something");
    assert_eq!(
        (d.1, d.3),
        (row, row + 2),
        "two device rows, at the line's bottom"
    );
    assert!(
        d.0 as f32 >= x0.floor() && d.2 as f32 <= x1.ceil() + 1.0,
        "{d:?} in {x0}..{x1}"
    );
    assert!(
        d.0 as f32 <= x0 + 2.0 && d.2 as f32 >= x1 - 2.0,
        "{d:?} spans {x0}..{x1}"
    );
    // Solid across the range, in the text's colour over the box: the target is premultiplied.
    let [_, _, _, alpha] = DARK.fg_primary;
    for x in x0.ceil() as usize + 1..x1.floor() as usize - 1 {
        for y in row..row + 2 {
            let p = px(&a, x, y);
            assert!(p[3].abs_diff(alpha) <= 4, "{p:?} at {x},{y}");
        }
    }
}

/// The first and last device columns with ink in the text's rows above the underline, from `from`.
fn ink_columns(img: &[u8], ground: &[u8], from: usize) -> Option<(usize, usize)> {
    let line = style().line;
    let top = ((RECT.y + (RECT.h - line) / 2.0) * SCALE) as usize;
    let bottom = top + ((line - 2.0) * SCALE) as usize;
    let right = ((RECT.right() - 8.0) * SCALE) as usize;
    let ink = |x: usize| (top..bottom).any(|y| px(img, x, y) != px(ground, x, y));
    let first = (from..right).find(|x| ink(*x))?;
    let last = (from..right).rev().find(|x| ink(*x))?;
    Some((first, last))
}

#[test]
fn the_text_after_the_caret_shifts_by_the_preedits_measured_width() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let unfocused = |text: &str| {
        let mut w = Window::new();
        let mut text = text.to_owned();
        let scene = w.frame(&mut text);
        (w, scene)
    };
    // An empty field is the ground the ink is measured against.
    let (mut w, empty) = unfocused("");
    let ground = render(&mut gpu, &mut w.text, &empty);
    let (mut w, rest) = unfocused("lune");
    let before = render(&mut gpu, &mut w.text, &rest);

    let mut composing = Window::new();
    let mut text = String::from("lune");
    composing.focused(&mut text, 2);
    composing.input.ime(ImeEvent::Preedit {
        text: "mm".to_owned(),
        cursor: None,
    });
    composing.frame(&mut text);
    let scene = composing.frame(&mut text);
    let after = render(&mut gpu, &mut composing.text, &scene);

    let spec = Spec::plain("lune", style());
    let split = composing.text.caret(&spec, SCALE, 2).x;
    let joined = Spec::plain("lumm", style());
    let shift = composing.text.caret(&joined, SCALE, 4).x - split;
    assert!(shift > 0.0);

    // "ne" before and after, each from its own start. The focus ring only touches the box's edge.
    let start = ((TX + split) * SCALE) as usize;
    let moved = ((TX + split + shift) * SCALE) as usize;
    let (b0, b1) = ink_columns(&before, &ground, start + 1).expect("ink");
    let (a0, a1) = ink_columns(&after, &ground, moved + 1).expect("ink");
    let device = shift * SCALE;
    assert!(
        (a0 as f32 - b0 as f32 - device).abs() <= 1.5,
        "{a0} {b0} {device}"
    );
    assert!(
        (a1 as f32 - b1 as f32 - device).abs() <= 1.5,
        "{a1} {b1} {device}"
    );
}

/// The field's selection in chars, after a frame of `text`.
fn selection(w: &mut Window, text: &mut String) -> (usize, usize) {
    let (_, out) = w.run(|ui| {
        ui.text_field_with("f", RECT, text, &FieldOptions::default());
    });
    let node = out.tree.get(WidgetId::ROOT.with("f")).expect("node");
    node.text.as_ref().expect("text").selection
}

#[test]
fn a_press_anywhere_in_the_box_keeps_its_column() {
    // The box runs y 10 to 42 and its one line y 10 to 27 from the box's top, so the lower half of
    // the box is below the line.
    let mut text = String::from("Hello world");
    let mut w = Window::new();
    w.frame(&mut text);
    for y in [11.0, 12.0, 20.0, 26.9, 27.0, 34.0, 40.0, 41.5] {
        w.input.pointer_to(30.0, y);
        w.frame(&mut text);
        w.input.press(PointerButton::Primary);
        assert_eq!(selection(&mut w, &mut text), (1, 1), "press at y {y}");
        w.input.release(PointerButton::Primary);
        w.frame(&mut text);
    }
}

#[test]
fn a_drag_along_the_boxs_lower_edge_selects_by_column() {
    let mut text = String::from("Hello world");
    let mut w = Window::new();
    w.frame(&mut text);
    w.input.pointer_to(30.0, 40.0);
    w.frame(&mut text);
    w.input.press(PointerButton::Primary);
    w.frame(&mut text);
    let spec = Spec::plain(text.as_str(), style());
    let x = w.text.caret(&spec, SCALE, 5).x;
    w.input.pointer_to(TX + x, 40.0);
    w.frame(&mut text);
    assert_eq!(selection(&mut w, &mut text), (1, 5));
}

#[test]
fn enter_at_the_bottom_of_a_full_text_area_keeps_the_whole_caret_in_view() {
    let rect = Rect::new(10.0, 10.0, 180.0, 100.0);
    let mut text: String = (0..10).map(|i| format!("line {i}\n")).collect();
    text.pop();
    let mut w = Window::sized((200.0, 140.0));
    let mut area = None;
    let mut frame = |w: &mut Window, text: &mut String| {
        w.run(|ui| {
            area = ui
                .text_area("a", rect, text, &AreaOptions::default())
                .ime_area;
        });
        area
    };
    frame(&mut w, &mut text);
    w.input.key_down(Key::Named(NamedKey::Tab), None);
    frame(&mut w, &mut text);
    w.input.key_down(Key::Named(NamedKey::End), None);
    frame(&mut w, &mut text);
    frame(&mut w, &mut text);
    let line = style().line;
    assert_eq!(frame(&mut w, &mut text).expect("focused").h, line);
    w.input.key_down(Key::Named(NamedKey::Enter), None);
    frame(&mut w, &mut text);
    frame(&mut w, &mut text);
    assert!(text.ends_with('\n'));
    assert_eq!(frame(&mut w, &mut text).expect("focused").h, line);
}

/// The text after a click at the left of row `row` of an area holding `text`, then typing X.
fn click_row_and_type(text: &str, row: usize) -> String {
    let rect = Rect::new(10.0, 10.0, 180.0, 100.0);
    let mut text = text.to_owned();
    let mut w = Window::sized((200.0, 140.0));
    let frame = |w: &mut Window, text: &mut String| {
        w.run(|ui| {
            ui.text_area("a", rect, text, &AreaOptions::default());
        });
    };
    frame(&mut w, &mut text);
    // The frame of 1 and the padding of 9 and 6.
    let y = 17.0 + (row as f32 + 0.5) * style().line;
    w.input.pointer_to(20.5, y);
    frame(&mut w, &mut text);
    w.input.press(PointerButton::Primary);
    frame(&mut w, &mut text);
    w.input.release(PointerButton::Primary);
    frame(&mut w, &mut text);
    w.input.key_down(Key::Character("x".into()), Some("X"));
    frame(&mut w, &mut text);
    text
}

#[test]
fn a_click_on_a_row_after_a_crlf_lands_on_that_row_in_any_text() {
    assert_eq!(click_row_and_type("ab\r\ncd\r\nef", 2), "ab\r\ncd\r\nXef");
    assert_eq!(
        click_row_and_type("\u{e9}b\r\ncd\r\nef", 2),
        "\u{e9}b\r\ncd\r\nXef"
    );
}

#[test]
fn the_caret_stays_on_its_row_through_spaces_at_a_wrap() {
    let mut system = text_cache();
    let spec = Spec::plain("hello world again      x", style()).wrapped(120.0);
    let rows = system.lines(&spec, SCALE);
    assert_eq!(rows.len(), 2);
    let end = system.caret(&spec, SCALE, rows[0].end);
    for at in rows[0].end..rows[1].start {
        let c = system.caret(&spec, SCALE, at);
        assert_eq!((c.line, c.x), (0, end.x), "offset {at}");
    }
}

/// The engine on a line whose first space was dropped at a wrap: rows (0, 2), (3, 24), (25, 29).
#[test]
fn home_and_end_stay_on_a_row_whose_first_space_was_dropped() {
    let word = "w".repeat(20);
    let text = format!("ww\n {word}\nnext");
    let wrapped = |t: &str| Spec::plain(t, Style::ui(13.0, [255; 4])).wrapped(200.0);
    let spec = wrapped(&text);
    let mut system = text_cache();
    let after = |motion: Option<Motion>, system: &mut TextCache| {
        let mut owned = text.clone();
        let mut edit = TextEdit::new();
        edit.set_flags(EditFlags {
            multiline: true,
            ..Default::default()
        });
        let mut layout = SpecLayout::new(system, &spec, SCALE);
        edit.sync(&owned, &mut layout);
        edit.set_selection(2, 2);
        for command in [Some(Motion::GraphemeRight), motion].into_iter().flatten() {
            let cmd = moved(command);
            edit.apply(cmd, &mut owned, &mut layout, Keymap::PC, Duration::ZERO);
        }
        edit.caret()
    };
    assert_eq!(after(None, &mut system), 3);
    assert_eq!(after(Some(Motion::LineStart), &mut system), 3);
    assert_eq!(after(Some(Motion::LineEnd), &mut system), 24);
    assert_eq!(after(Some(Motion::LineUp), &mut system), 0);
    assert_eq!(after(Some(Motion::LineDown), &mut system), 25);

    let mut owned = format!(" {word}");
    let first = wrapped(&owned);
    let mut edit = TextEdit::new();
    let mut layout = SpecLayout::new(&mut system, &first, SCALE);
    edit.sync(&owned, &mut layout);
    edit.set_selection(0, 0);
    let home = moved(Motion::LineStart);
    edit.apply(home, &mut owned, &mut layout, Keymap::PC, Duration::ZERO);
    assert_eq!(edit.caret(), 0);
    let line = EditCommand::SelectLineAt(0);
    edit.apply(line, &mut owned, &mut layout, Keymap::PC, Duration::ZERO);
    assert_eq!(edit.selection(), 0..21);
}

/// Right of a row cut inside a word, a click lands where End does and the caret stays on that row.
#[test]
fn a_click_right_of_a_wrapped_row_lands_where_end_does() {
    let mut owned = "w".repeat(40);
    let spec = Spec::plain(owned.as_str(), style()).wrapped(80.0);
    let mut system = text_cache();
    let rows = system.lines(&spec, SCALE);
    assert!(rows.len() >= 2 && rows[0].end == rows[1].start, "{rows:?}");
    let press = (1000.0, rows[0].y + rows[0].height / 2.0);
    let hit = system.hit(&spec, SCALE, press);
    assert_eq!(system.caret(&spec, SCALE, hit).line, 0);
    let mut edit = TextEdit::new();
    let mut layout = SpecLayout::new(&mut system, &spec, SCALE);
    edit.sync(&owned, &mut layout);
    edit.set_selection(0, 0);
    let end = moved(Motion::LineEnd);
    edit.apply(end, &mut owned, &mut layout, Keymap::PC, Duration::ZERO);
    assert_eq!(edit.caret(), hit);
}
