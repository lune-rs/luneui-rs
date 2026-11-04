//! A `Ui` test bench: one window's retained store, a fake text system, the default strings and a
//! scripted input, run one frame at a time.

use std::time::Duration;

use crate::input::{FrameOutput, Keymap};
use crate::motion::MotionTier;
use crate::scene::Scene;
use crate::strings::{DisplayFormat, StringTable};
use crate::theme::{DARK, Theme};
use crate::ui::{FrameConfig, Ui, UiStore};

use super::{FakeText, ScriptedInput};

/// One scripted window for `Ui` tests. Script input on [`UiHarness::input`], then run
/// [`UiHarness::frame`]; the frame's input is everything scripted since the previous frame.
#[derive(Debug)]
pub struct UiHarness {
    pub store: UiStore,
    pub text: FakeText,
    pub strings: StringTable,
    pub input: ScriptedInput,
    pub theme: Theme,
    pub tier: MotionTier,
    /// The keyboard conventions the frames run with; macOS by default.
    pub keymap: Keymap,
}

impl UiHarness {
    /// A focused window of logical `size` at scale 1, the dark preset, the Full tier and an empty
    /// string table.
    pub fn new(size: (f32, f32)) -> Self {
        let mut input = ScriptedInput::new(size, 1.0);
        input.focus(true);
        Self {
            store: UiStore::default(),
            text: FakeText,
            strings: StringTable::default(),
            input,
            theme: DARK,
            tier: MotionTier::Full,
            keymap: Keymap::MAC,
        }
    }

    /// Sets the time of the following frames, in milliseconds of app time.
    pub fn at_ms(&mut self, ms: u64) -> &mut Self {
        self.input.set_time(Duration::from_millis(ms));
        self
    }

    /// Runs one frame: `f` builds it, the result is what `Ui::finish` returns.
    pub fn frame(&mut self, f: impl FnOnce(&mut Ui<'_>)) -> (Scene, FrameOutput) {
        let raw = self.input.end_frame();
        let config = FrameConfig {
            theme: self.theme,
            tier: self.tier,
            strings: &self.strings,
            arg_format: &DisplayFormat,
            keymap: self.keymap,
        };
        let mut ui = Ui::begin(&mut self.store, &raw, config, &mut self.text);
        f(&mut ui);
        ui.finish()
    }
}
