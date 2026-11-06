//! The clipboard service over a fake backend: no test in the gate touches the OS clipboard. The one
//! real round trip is `#[ignore]`d, macOS only, and run by hand.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lune_ui_core::input::{ClipboardRequest, Event, FrameOutput};
use lune_ui_winit::clipboard::{Backend, Clipboard, MAX_PASTE_BYTES};
use lune_ui_winit::input::Translator;
use winit::dpi::PhysicalSize;

/// What the fake clipboard holds and how often it was touched.
#[derive(Default)]
struct State {
    text: Option<String>,
    writes: Vec<usize>,
    reads: usize,
    broken: bool,
}

struct Fake(Rc<RefCell<State>>);

impl Backend for Fake {
    fn read(&mut self) -> Result<String, ()> {
        let mut s = self.0.borrow_mut();
        s.reads += 1;
        if s.broken {
            return Err(());
        }
        s.text.clone().ok_or(())
    }

    fn write(&mut self, text: &str) -> Result<(), ()> {
        let mut s = self.0.borrow_mut();
        if s.broken {
            return Err(());
        }
        s.writes.push(text.len());
        s.text = Some(text.to_owned());
        Ok(())
    }
}

struct Bench {
    clipboard: Clipboard,
    translator: Translator,
    state: Rc<RefCell<State>>,
}

fn bench() -> Bench {
    let state = Rc::new(RefCell::new(State::default()));
    Bench {
        clipboard: Clipboard::with_backend(Box::new(Fake(state.clone()))),
        translator: Translator::new(PhysicalSize::new(100, 100), 1.0),
        state,
    }
}

impl Bench {
    /// One frame whose output holds `requests`; returns the next frame's events.
    fn frame(&mut self, requests: Vec<ClipboardRequest>) -> Vec<Event> {
        let out = FrameOutput {
            clipboard: requests,
            ..FrameOutput::default()
        };
        self.clipboard.service(&out, &mut self.translator);
        self.translator.take(Duration::ZERO).events
    }
}

fn write(t: &str) -> ClipboardRequest {
    ClipboardRequest::Write(t.to_owned())
}

#[test]
fn write_then_read_returns_the_text_with_multi_byte_characters_intact() {
    let mut b = bench();
    let text = "caf\u{e9} \u{65e5}\u{672c}\u{8a9e} \u{1f600}";
    assert!(
        b.frame(vec![write(text)]).is_empty(),
        "a write answers nothing"
    );
    assert_eq!(
        b.frame(vec![ClipboardRequest::Read]),
        [Event::ClipboardText(text.to_owned())]
    );
}

#[test]
fn two_writes_in_a_frame_keep_the_last() {
    let mut b = bench();
    b.frame(vec![write("first"), write("second")]);
    assert_eq!(
        b.frame(vec![ClipboardRequest::Read]),
        [Event::ClipboardText("second".to_owned())]
    );
    // Order is kept: a read between two writes sees the first.
    let events = b.frame(vec![write("a"), ClipboardRequest::Read, write("b")]);
    assert_eq!(events, [Event::ClipboardText("a".to_owned())]);
    assert_eq!(b.state.borrow().text.as_deref(), Some("b"));
}

#[test]
fn a_read_reply_lands_in_the_next_take_exactly_once() {
    let mut b = bench();
    b.frame(vec![write("x")]);
    let out = FrameOutput {
        clipboard: vec![ClipboardRequest::Read],
        ..FrameOutput::default()
    };
    b.clipboard.service(&out, &mut b.translator);
    // Not in an earlier take; in the next one; then gone.
    assert_eq!(
        b.translator.take(Duration::ZERO).events,
        [Event::ClipboardText("x".to_owned())]
    );
    assert!(b.translator.take(Duration::ZERO).events.is_empty());
    // Two reads in a frame get two replies; none gets none.
    let events = b.frame(vec![ClipboardRequest::Read, ClipboardRequest::Read]);
    assert_eq!(events.len(), 2);
    assert!(b.frame(vec![]).is_empty());
    assert_eq!(
        b.state.borrow().reads,
        3,
        "the service reads only when asked"
    );
}

#[test]
fn a_failed_read_gives_the_event() {
    let mut b = bench();
    // Nothing on it.
    assert_eq!(
        b.frame(vec![ClipboardRequest::Read]),
        [Event::ClipboardUnavailable]
    );
    // A backend that fails.
    b.state.borrow_mut().broken = true;
    assert_eq!(
        b.frame(vec![ClipboardRequest::Read]),
        [Event::ClipboardUnavailable]
    );
    // A failed write is silent.
    assert!(b.frame(vec![write("lost")]).is_empty());
}

#[test]
fn a_read_over_the_cap_is_refused_whole() {
    let mut b = bench();
    b.state.borrow_mut().text = Some("x".repeat(MAX_PASTE_BYTES + 1));
    assert_eq!(MAX_PASTE_BYTES, 33_554_432);
    assert_eq!(
        b.frame(vec![ClipboardRequest::Read]),
        [Event::ClipboardUnavailable]
    );
    // At the cap it passes whole.
    b.state.borrow_mut().text = Some("x".repeat(MAX_PASTE_BYTES));
    let events = b.frame(vec![ClipboardRequest::Read]);
    assert!(matches!(&events[..], [Event::ClipboardText(t)] if t.len() == MAX_PASTE_BYTES));
}

#[test]
fn a_write_over_the_cap_never_reaches_the_backend() {
    let mut b = bench();
    b.frame(vec![write("kept")]);
    b.frame(vec![write(&"x".repeat(MAX_PASTE_BYTES + 1))]);
    let s = b.state.borrow();
    assert_eq!(s.writes, [4], "only the small write arrived");
    assert_eq!(
        s.text.as_deref(),
        Some("kept"),
        "the old text is untouched, nothing truncated"
    );
}

#[test]
fn ten_million_bytes_pass_whole() {
    let mut b = bench();
    let big = "ab".repeat(5_000_000);
    b.frame(vec![write(&big)]);
    assert_eq!(b.state.borrow().writes, [10_000_000]);
    let events = b.frame(vec![ClipboardRequest::Read]);
    assert!(matches!(&events[..], [Event::ClipboardText(t)] if *t == big));
}

#[test]
fn the_empty_string_round_trips() {
    let mut b = bench();
    b.frame(vec![write("")]);
    assert_eq!(
        b.frame(vec![ClipboardRequest::Read]),
        [Event::ClipboardText(String::new())]
    );
}

#[test]
fn new_opens_nothing_and_an_empty_frame_touches_nothing() {
    // The real backend is lazy: constructing it and servicing a frame with no request never opens
    // the OS clipboard (this test must not touch it).
    let mut clipboard = Clipboard::new();
    let mut translator = Translator::new(PhysicalSize::new(100, 100), 1.0);
    clipboard.service(&FrameOutput::default(), &mut translator);
    assert!(translator.take(Duration::ZERO).events.is_empty());
    assert!(format!("{clipboard:?}").contains("Clipboard"));
}

/// The one test that touches the OS clipboard. Run by hand: `cargo test -p lune-ui-winit --test
/// clipboard -- --ignored real_round_trip --nocapture`. It overwrites the clipboard for a moment
/// and puts the text back; it runs only when `osascript` says every type on the clipboard is a
/// plain-text class, so nothing else is lost.
#[test]
#[ignore = "touches the real clipboard; run by hand"]
#[cfg(target_os = "macos")]
fn real_round_trip() {
    use std::process::Command;

    let info = Command::new("osascript")
        .args(["-e", "clipboard info"])
        .output();
    let Ok(info) = info else {
        println!("skip: osascript did not run");
        return;
    };
    let listing = String::from_utf8_lossy(&info.stdout).to_string();
    // `osascript` prints type names and byte sizes, comma separated, with or without braces
    // (`\u{ab}class utf8\u{bb}, 110, string, 110`): the names are the tokens that are not numbers.
    let plain = [
        "\u{ab}class utf8\u{bb}",
        "string",
        "Unicode text",
        "\u{ab}class ut16\u{bb}",
    ];
    let classes: Vec<&str> = listing
        .split(',')
        .map(|t| t.trim().trim_matches(|c| c == '{' || c == '}').trim())
        .filter(|t| !t.is_empty() && t.parse::<u64>().is_err())
        .collect();
    let only_text = !classes.is_empty() && classes.iter().all(|c| plain.contains(c));
    if !info.status.success() || !only_text {
        println!("skip: the clipboard holds something other than plain text ({classes:?})");
        return;
    }

    let mut clipboard = Clipboard::new();
    let mut translator = Translator::new(PhysicalSize::new(100, 100), 1.0);
    let mut run = |requests: Vec<ClipboardRequest>| {
        let out = FrameOutput {
            clipboard: requests,
            ..FrameOutput::default()
        };
        clipboard.service(&out, &mut translator);
        translator.take(Duration::ZERO).events
    };
    let saved = match run(vec![ClipboardRequest::Read]).as_slice() {
        [Event::ClipboardText(t)] => t.clone(),
        other => {
            println!("skip: could not read the clipboard first ({other:?})");
            return;
        }
    };
    let probe = "luneui clipboard probe: caf\u{e9} \u{65e5}\u{672c}\u{8a9e} \u{1f600}";
    run(vec![write(probe)]);
    let got = run(vec![ClipboardRequest::Read]);
    run(vec![write(&saved)]);
    assert_eq!(got, [Event::ClipboardText(probe.to_owned())]);
    let restored = run(vec![ClipboardRequest::Read]);
    assert_eq!(
        restored,
        [Event::ClipboardText(saved)],
        "the clipboard text is restored"
    );
}
