//! The OS clipboard as text. [`Clipboard`] executes the frame's [`ClipboardRequest`]s: a write goes
//! to the OS, a read is answered with an event on the translator, which the next frame's input
//! carries. One per app. Text only; the core never touches the OS.
//!
//! Pasted text comes from outside the process, so it is capped at [`MAX_PASTE_BYTES`]: a longer
//! text is refused whole (a read answers `ClipboardUnavailable`, a write is dropped). Nothing is
//! ever truncated. A failed write is silent. The OS backend is opened on first use,
//! so a process that never copies or pastes opens nothing; a backend that cannot open answers
//! `ClipboardUnavailable` to a read and drops a write.

use lune_ui_core::input::{ClipboardRequest, FrameOutput};

use crate::input::Translator;

/// The longest text the adapter reads from or writes to the OS clipboard, in bytes: 32 MiB, three
/// times the 11 MB document the code view measured (D213). It bounds the adapter, not the widget: a
/// one-line editor refuses any insert over 1 MiB itself (`text_edit::MAX_ONE_LINE_INSERT`, D232).
pub const MAX_PASTE_BYTES: usize = 33_554_432;

/// What the adapter needs of an OS clipboard. The real one is `arboard`; tests give a fake. Not
/// part of the API: it is public only so a test can name it.
#[doc(hidden)]
pub trait Backend {
    /// The clipboard's text, or `Err` when it holds none or cannot be read.
    #[allow(clippy::result_unit_err)]
    fn read(&mut self) -> Result<String, ()>;
    /// Puts `text` on the clipboard, or `Err` when it cannot.
    #[allow(clippy::result_unit_err)]
    fn write(&mut self, text: &str) -> Result<(), ()>;
}

/// `arboard`, opened on first use.
#[derive(Default)]
struct Os {
    inner: Option<arboard::Clipboard>,
}

impl Os {
    fn open(&mut self) -> Result<&mut arboard::Clipboard, ()> {
        if self.inner.is_none() {
            self.inner = Some(arboard::Clipboard::new().map_err(|_| ())?);
        }
        self.inner.as_mut().ok_or(())
    }
}

impl Backend for Os {
    fn read(&mut self) -> Result<String, ()> {
        // `ContentNotAvailable` (no text on it) and every other error are the same answer.
        self.open()?.get_text().map_err(|_| ())
    }

    fn write(&mut self, text: &str) -> Result<(), ()> {
        self.open()?.set_text(text).map_err(|_| ())
    }
}

/// The app's clipboard service. See the module note.
pub struct Clipboard {
    backend: Box<dyn Backend>,
}

impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clipboard").finish_non_exhaustive()
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clipboard {
    /// The OS clipboard. Never fails and opens nothing yet.
    pub fn new() -> Self {
        Self::with_backend(Box::new(Os::default()))
    }

    /// A clipboard over another backend (tests).
    #[doc(hidden)]
    pub fn with_backend(backend: Box<dyn Backend>) -> Self {
        Self { backend }
    }

    /// Executes `out`'s clipboard requests in order. A `Write` goes to the OS (so of two writes in
    /// a frame the last stays), unless it is over the cap. A `Read` queues exactly one reply on
    /// `translator`: the text, or `ClipboardUnavailable` when there is none, it cannot be read or
    /// it is over the cap. The reply reaches the app in the translator's next `take`.
    pub fn service(&mut self, out: &FrameOutput, translator: &mut Translator) {
        for request in &out.clipboard {
            match request {
                ClipboardRequest::Write(text) => {
                    if text.len() <= MAX_PASTE_BYTES {
                        let _ = self.backend.write(text);
                    }
                }
                ClipboardRequest::Read => match self.backend.read() {
                    Ok(text) if text.len() <= MAX_PASTE_BYTES => translator.clipboard_text(&text),
                    _ => translator.clipboard_unavailable(),
                },
            }
        }
    }
}
