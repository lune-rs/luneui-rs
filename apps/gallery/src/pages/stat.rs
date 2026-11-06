//! The `stat` page: a strip of three cells, the gold, no-data, meter and compact forms, and a
//! value whose number changes (D126).

use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::ui::Ui;
use lune_ui::widgets::stat::StatOptions;
use lune_ui::widgets::tone::Tone;

use super::{card, heading};
use crate::page::{Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "stat",
    title: "Stat",
    kind: PageKind::Ui(UiPage {
        state: all,
        scenes: SCENES,
        bench: &[],
    }),
};

/// The flyout's inner width.
const FLYOUT_W: f32 = 330.0;

/// A page state that draws its sections with one function.
struct Sections(&'static [fn(&mut Ui<'_>, f32, f32) -> f32]);

impl PageState for Sections {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Stat");
        let (x, mut y) = (card.x + 24.0, card.y + 24.0);
        for section in self.0 {
            y += section(ui, x, y) + 28.0;
        }
    }
}

fn whole(n: f64) -> String {
    format!("{n:.0}")
}

fn default_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "STRIP", x, y);
    let cells = [
        StatOptions {
            label: "Tokens saved",
            value: "1,284,300",
            sub: Some("+12% vs last week"),
            sub_tone: Tone::Ok,
            ..StatOptions::default()
        },
        StatOptions {
            label: "Requests",
            value: "8,412",
            sub: Some("3 failed"),
            sub_tone: Tone::Warn,
            ..StatOptions::default()
        },
        StatOptions {
            label: "Compression",
            value: "38%",
            sub: Some("of input tokens"),
            ..StatOptions::default()
        },
    ];
    20.0 + ui.stat_strip("strip", Rect::new(x, y + 20.0, 720.0, 0.0), &cells)
}

fn gold_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "GOLD: THE NET-SAVED FIGURE", x, y);
    let opts = StatOptions {
        label: "Net saved",
        value: "$ 41.60",
        sub: Some("this month"),
        gold: true,
        ..StatOptions::default()
    };
    20.0 + ui.stat("gold", Rect::new(x, y + 20.0, 240.0, 0.0), &opts)
}

fn no_data_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "NO DATA", x, y);
    let opts = StatOptions {
        label: "Net saved",
        gold: true,
        no_data: true,
        ..StatOptions::default()
    };
    20.0 + ui.stat("no-data", Rect::new(x, y + 20.0, 240.0, 0.0), &opts)
}

fn meter_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "METER", x, y);
    let opts = StatOptions {
        label: "Budget used",
        value: "$ 8.20",
        sub: Some("of $ 20.00 a day"),
        meter: Some(0.41),
        ..StatOptions::default()
    };
    20.0 + ui.stat("meter", Rect::new(x, y + 20.0, 240.0, 0.0), &opts)
}

fn compact_section(ui: &mut Ui<'_>, x: f32, y: f32) -> f32 {
    heading(ui, "COMPACT, IN THE FLYOUT'S WIDTH", x, y);
    let cells = [
        StatOptions {
            label: "Saved",
            value: "1.2M",
            compact: true,
            ..StatOptions::default()
        },
        StatOptions {
            label: "Requests",
            value: "8,412",
            sub: Some("live"),
            sub_tone: Tone::Ok,
            compact: true,
            ..StatOptions::default()
        },
        StatOptions {
            label: "Spend",
            value: "$ 8.20",
            compact: true,
            ..StatOptions::default()
        },
    ];
    20.0 + ui.stat_strip("compact", Rect::new(x, y + 20.0, FLYOUT_W, 0.0), &cells)
}

fn all() -> Box<dyn PageState> {
    Box::new(Sections(&[
        default_section,
        gold_section,
        no_data_section,
        meter_section,
        compact_section,
    ]))
}

fn strip() -> Box<dyn PageState> {
    Box::new(Sections(&[default_section]))
}

fn gold() -> Box<dyn PageState> {
    Box::new(Sections(&[gold_section]))
}

fn no_data() -> Box<dyn PageState> {
    Box::new(Sections(&[no_data_section]))
}

fn meter() -> Box<dyn PageState> {
    Box::new(Sections(&[meter_section]))
}

fn compact() -> Box<dyn PageState> {
    Box::new(Sections(&[compact_section]))
}

/// One cell whose number changes on its second frame: 1000 first, then 2000.
struct ValueChange {
    frames: u32,
}

impl PageState for ValueChange {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Stat");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "VALUE CHANGE", x, y);
        let (n, text) = if self.frames == 0 {
            (1000.0, "1,000")
        } else {
            (2000.0, "2,000")
        };
        self.frames += 1;
        let opts = StatOptions {
            label: "Requests",
            value: text,
            number: Some((n, &whole)),
            ..StatOptions::default()
        };
        ui.stat("change", Rect::new(x, y + 20.0, 240.0, 0.0), &opts);
    }
}

fn value_change() -> Box<dyn PageState> {
    Box::new(ValueChange { frames: 0 })
}

/// The second frame at time 0 carries the new number, so the capture at 90 ms is the middle of
/// the 180 ms tween.
const CHANGE: Step = Step {
    at_ms: 0,
    events: &[],
};

const SCENES: &[UiScene] = &[
    UiScene {
        id: "stat-default",
        scale: 2.0,
        tier: MotionTier::Full,
        state: strip,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "stat-gold",
        scale: 2.0,
        tier: MotionTier::Full,
        state: gold,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "stat-no-data",
        scale: 2.0,
        tier: MotionTier::Full,
        state: no_data,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "stat-meter",
        scale: 2.0,
        tier: MotionTier::Full,
        state: meter,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "stat-compact",
        scale: 2.0,
        tier: MotionTier::Full,
        state: compact,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "stat-value-mid",
        scale: 2.0,
        tier: MotionTier::Full,
        state: value_change,
        script: &[CHANGE],
        capture_ms: 90,
        expect: Expect::Moving,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::Value;
    use lune_ui::testing::FakeText;

    #[test]
    fn the_value_scene_is_in_the_middle_of_the_tween_and_the_node_keeps_the_new_text() {
        let scene = SCENES
            .iter()
            .find(|s| s.id == "stat-value-mid")
            .expect("scene");
        let (_, out) = run_scene(PAGE.id, scene, &mut FakeText).expect("runs");
        let node = out
            .tree
            .nodes
            .iter()
            .find(|n| n.name == "2,000")
            .expect("the value node is valued by the app's text, not a frame of the tween");
        assert_eq!(node.value, Some(Value::Text("2,000".to_owned())));
    }
}
