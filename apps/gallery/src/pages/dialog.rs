//! The `dialog` page: a row of buttons over the gallery background, one of which opens a modal
//! dialog through `Ui::dialog`. The scenes freeze it closed, half way through the open, and half
//! way through the close (the app stops calling the dialog, the Ui keeps drawing it); then the
//! variants (`Ui::dialog_with`: the form width, the blurred scrim, a disabled confirm) and the
//! overlays a dialog's body opens, which draw and take input above it: a combo's list, a popover,
//! a tooltip, a glass menu and a nested dialog.

use std::time::Duration;

use lune_ui::input::NamedKey;
use lune_ui::lunegit_dark::{self, THEME};
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;
use lune_ui::widgets::button::{ButtonKind, button_width};
use lune_ui::widgets::dialog::{Cancel, Dialog, DialogGlass, DialogOptions, DialogWidth};
use lune_ui::widgets::menu::MenuRow;
use lune_ui::widgets::popover::PopoverOptions;

use super::{caption, card, heading};
use crate::page::{Ev, Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "dialog",
    title: "Dialog",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: BENCH,
    }),
};

/// When the closing scene's app stops calling the dialog (after the 250 ms open).
const STOP_AT: Duration = Duration::from_millis(300);

/// What the open dialog is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    /// The 448 px confirm dialog.
    Confirm,
    /// The 600 px form width, a body in two columns.
    Form,
    /// The confirm dialog over a blurred scrim.
    Blur,
    /// The confirm dialog with its confirm button disabled.
    Disabled,
    /// A combo in the body, its list open.
    Combo,
    /// A button in the body, its popover open.
    Popover,
    /// A button in the body that shows a tooltip.
    Tooltip,
    /// A button in the body, its glass menu open.
    Menu,
    /// A second dialog opened by the body.
    Nested,
}

impl Variant {
    /// Whether the body opens an overlay of its own.
    fn has_child(self) -> bool {
        matches!(
            self,
            Variant::Combo | Variant::Popover | Variant::Menu | Variant::Nested
        )
    }
}

/// The page with a dialog that is open, or opens from its button.
struct Dialogs {
    variant: Variant,
    open: bool,
    /// The app stops calling the dialog from this time on.
    stop_at: Option<Duration>,
    /// The overlay the body opens is open.
    child: bool,
}

fn variant(variant: Variant) -> Box<dyn PageState> {
    Box::new(Dialogs {
        variant,
        open: true,
        stop_at: None,
        child: variant.has_child(),
    })
}

fn live() -> Box<dyn PageState> {
    Box::new(Dialogs {
        variant: Variant::Confirm,
        open: false,
        stop_at: None,
        child: false,
    })
}

fn opening() -> Box<dyn PageState> {
    variant(Variant::Confirm)
}

fn closing() -> Box<dyn PageState> {
    Box::new(Dialogs {
        stop_at: Some(STOP_AT),
        ..Dialogs {
            variant: Variant::Confirm,
            open: true,
            stop_at: None,
            child: false,
        }
    })
}

fn form() -> Box<dyn PageState> {
    variant(Variant::Form)
}

fn blur() -> Box<dyn PageState> {
    variant(Variant::Blur)
}

fn disabled() -> Box<dyn PageState> {
    variant(Variant::Disabled)
}

fn combo() -> Box<dyn PageState> {
    variant(Variant::Combo)
}

fn popover() -> Box<dyn PageState> {
    variant(Variant::Popover)
}

fn tooltip() -> Box<dyn PageState> {
    variant(Variant::Tooltip)
}

fn menu() -> Box<dyn PageState> {
    variant(Variant::Menu)
}

fn nested() -> Box<dyn PageState> {
    variant(Variant::Nested)
}

fn glass() -> DialogGlass {
    DialogGlass {
        panel: lunegit_dark::PANEL.premultiplied(),
        fill: lunegit_dark::WINDOW.premultiplied(),
        noise: true,
    }
}

impl PageState for Dialogs {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Dialog");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "A DIALOG OVER THE PAGE", x, y);
        let by = y + 20.0;
        let mut bx = x;
        for (key, text, kind) in [
            ("open", "Open dialog", ButtonKind::Primary),
            ("other", "Page action", ButtonKind::Secondary),
            ("more", "Another", ButtonKind::Secondary),
        ] {
            let w = button_width(ui.painter(), text);
            let r = ui.button(key, Rect::new(bx, by, w, 32.0), text, kind);
            if key == "open" && r.clicked {
                self.open = true;
            }
            bx += w + 8.0;
        }
        caption(
            ui,
            "Enter confirms, Escape cancels, Tab stays inside.",
            x,
            by + 48.0,
        );

        if self.open && self.stop_at.is_none_or(|t| ui.time() < t) {
            let kind = self.variant;
            let child = &mut self.child;
            let d = dialog_of(kind);
            let options = DialogOptions {
                width: if kind == Variant::Form {
                    DialogWidth::Form
                } else {
                    DialogWidth::Confirm
                },
                scrim_blur: kind == Variant::Blur,
                confirm_enabled: kind != Variant::Disabled,
            };
            let r = ui.dialog_with("dialog", &d, &options, &glass(), |ui, bx, by, bw| {
                body(ui, kind, child, bx, by, bw);
            });
            if r.confirmed || r.cancelled {
                self.open = false;
            }
        }
    }
}

/// What each variant says.
fn dialog_of(kind: Variant) -> Dialog<'static> {
    match kind {
        Variant::Confirm | Variant::Blur | Variant::Disabled => Dialog {
            title: "Discard changes",
            message: "The document has unsaved changes. Closing it will discard them.",
            extra_h: 28.0,
            confirm: "Discard",
            cancel: Cancel::Default,
        },
        Variant::Form => Dialog {
            title: "Add upstream",
            message: "Name the upstream and say where it listens.",
            extra_h: 116.0,
            confirm: "Add",
            cancel: Cancel::Default,
        },
        Variant::Combo | Variant::Popover | Variant::Tooltip | Variant::Menu | Variant::Nested => {
            Dialog {
                title: "Discard changes",
                message: "The document has unsaved changes.",
                extra_h: 124.0,
                confirm: "Discard",
                cancel: Cancel::Default,
            }
        }
    }
}

/// The app's part of the dialog.
fn body(ui: &mut Ui<'_>, kind: Variant, child: &mut bool, bx: f32, by: f32, bw: f32) {
    match kind {
        Variant::Confirm | Variant::Blur | Variant::Disabled => {
            caption(ui, "3 files changed since the last save.", bx, by + 12.0);
        }
        Variant::Form => form_body(ui, bx, by, bw),
        _ => anchored(ui, kind, child, bx, by),
    }
}

/// Two columns of labelled fields, the form's extra slot.
fn form_body(ui: &mut Ui<'_>, bx: f32, by: f32, bw: f32) {
    let col = (bw - 16.0) / 2.0;
    let fields = [
        ("Name", "staging", 0.0, 0.0),
        ("Listens on", "10.0.0.4:8443", 1.0, 0.0),
        ("Token", "wane-staging", 0.0, 1.0),
        ("Weight", "100", 1.0, 1.0),
    ];
    for (label, value, c, r) in fields {
        let x = bx + c * (col + 16.0);
        let y = by + 12.0 + r * 56.0;
        caption(ui, label, x, y);
        let field = Rect::new(x, y + 18.0, col, 28.0);
        let p = ui.painter();
        p.fill(field, 6.0, THEME.surface_input);
        p.ring(field, 6.0, 1.0, THEME.border, false);
        let spec = Spec::plain(value, Style::ui(THEME.font_base, THEME.fg_primary));
        p.text(&spec, field.x + 8.0, field.y + 6.0);
    }
}

const OPTIONS: [(&str, bool); 4] = [
    ("main", true),
    ("develop", true),
    ("release/1.2", true),
    ("hotfix", false),
];

const MENU_ROWS: [MenuRow<'static>; 3] = [
    MenuRow::Item {
        glyph: None,
        text: "Save all",
        shortcut: "",
    },
    MenuRow::Item {
        glyph: None,
        text: "Discard all",
        shortcut: "",
    },
    MenuRow::Item {
        glyph: None,
        text: "Review changes",
        shortcut: "",
    },
];

/// An anchor, two captions under it, and the overlay the variant opens over them: the overlay
/// lands on the captions, so the picture shows which one is on top.
fn anchored(ui: &mut Ui<'_>, kind: Variant, child: &mut bool, bx: f32, by: f32) {
    let anchor = Rect::new(bx, by + 4.0, 160.0, 28.0);
    caption(
        ui,
        "3 files changed since the last save.",
        bx + 40.0,
        by + 46.0,
    );
    caption(ui, "Review them first.", bx + 70.0, by + 72.0);
    match kind {
        Variant::Combo => {
            let c = ui.combo("branch", anchor, "main", child, "Branch");
            if *child {
                let l = ui.dropdown("branches", anchor, c.id, &OPTIONS, 0);
                if l.picked.is_some() || l.dismissed {
                    *child = false;
                }
            }
        }
        Variant::Popover => {
            let b = ui.button("details", anchor, "Details", ButtonKind::Secondary);
            *child |= b.clicked;
            if *child {
                let options = PopoverOptions {
                    anchor_id: Some(b.id),
                    ..PopoverOptions::new(b.rect, (220.0, 100.0))
                };
                let (frame, _, done) = ui.popover("details-popover", &options, |ui, at| {
                    caption(ui, "Three files differ.", at.x + 12.0, at.y + 12.0);
                    caption(ui, "Escape closes this first.", at.x + 12.0, at.y + 32.0);
                    let r = Rect::new(at.x + 12.0, at.y + 62.0, 80.0, 28.0);
                    ui.button("done", r, "Done", ButtonKind::Secondary).clicked
                });
                if done || frame.dismissed.is_some() {
                    *child = false;
                }
            }
        }
        Variant::Tooltip => {
            let b = ui.button("save-all", anchor, "Save all", ButtonKind::Secondary);
            ui.tooltip(&b, "Writes every open file");
        }
        Variant::Menu => {
            let b = ui.button("actions", anchor, "Actions", ButtonKind::Secondary);
            *child |= b.clicked;
            if *child {
                let m = ui.menu(
                    "actions-menu",
                    &MENU_ROWS,
                    (anchor.x, anchor.bottom() + 4.0),
                );
                if m.picked.is_some() || m.dismissed {
                    *child = false;
                }
            }
        }
        Variant::Nested => {
            let b = ui.button("again", anchor, "Review", ButtonKind::Secondary);
            *child |= b.clicked;
            if *child {
                let inner = Dialog {
                    title: "Review changes",
                    message: "Three files differ.",
                    extra_h: 0.0,
                    confirm: "Open",
                    cancel: Cancel::Default,
                };
                let r = ui.dialog("review", &inner, &glass(), |_, _, _, _| {});
                if r.confirmed || r.cancelled {
                    *child = false;
                }
            }
        }
        _ => {}
    }
}

const OPEN: &[&str] = &["open"];
const SAVE_ALL: &[&str] = &["save-all"];

/// A variant open since the start and settled by `capture_ms` (the open is 250 ms).
const fn settled(id: &'static str, state: fn() -> Box<dyn PageState>) -> UiScene {
    UiScene {
        id,
        scale: 2,
        tier: MotionTier::Full,
        state,
        script: &[],
        capture_ms: 700,
        expect: Expect::Settled,
    }
}

const SCENES: &[UiScene] = &[
    UiScene {
        id: "dialog-closed",
        scale: 2,
        tier: MotionTier::Full,
        state: live,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The open is `pop`, base (250 ms): half way at 125 ms.
    UiScene {
        id: "dialog-opening",
        scale: 2,
        tier: MotionTier::Full,
        state: opening,
        script: &[],
        capture_ms: 125,
        expect: Expect::Moving,
    },
    // Open by 260 ms; the app stops calling at 300 ms; the close is `dismiss`, fast (180 ms):
    // half way at 390 ms.
    UiScene {
        id: "dialog-closing",
        scale: 2,
        tier: MotionTier::Full,
        state: closing,
        script: &[
            Step {
                at_ms: 260,
                events: &[],
            },
            Step {
                at_ms: 300,
                events: &[],
            },
        ],
        capture_ms: 390,
        expect: Expect::Moving,
    },
    settled("dialog-form", form),
    // The scrim blurs the window content at sigma 8; the page behind has a heading, buttons and
    // a caption to show it.
    settled("dialog-scrim-blur", blur),
    settled("dialog-confirm-disabled", disabled),
    // The overlays below open in the dialog's body and draw above the dialog's own text.
    settled("dialog-combo-open", combo),
    settled("dialog-popover", popover),
    // The pointer rests on the button from the start; the tooltip is due at 800 ms and has
    // settled by 1100 ms.
    UiScene {
        id: "dialog-tooltip",
        scale: 2,
        tier: MotionTier::Full,
        state: tooltip,
        script: &[
            Step {
                at_ms: 0,
                events: &[Ev::PointerOver(SAVE_ALL)],
            },
            Step {
                at_ms: 800,
                events: &[],
            },
        ],
        capture_ms: 1100,
        expect: Expect::Settled,
    },
    settled("dialog-menu-glass", menu),
    settled("dialog-nested", nested),
];

/// What the bench replays: open from the button, Tab, Enter, reopen, Escape.
const BENCH: &[Step] = &[
    Step {
        at_ms: 0,
        events: &[Ev::PointerOver(OPEN)],
    },
    Step {
        at_ms: 100,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 500,
        events: &[Ev::Tab],
    },
    Step {
        at_ms: 700,
        events: &[Ev::Key(NamedKey::Enter)],
    },
    Step {
        at_ms: 1200,
        events: &[Ev::Click],
    },
    Step {
        at_ms: 1700,
        events: &[Ev::Key(NamedKey::Escape)],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::run_scene;
    use lune_ui::access::Role;
    use lune_ui::scene::{LayerId, Scene};
    use lune_ui::testing::FakeText;

    fn run(id: &str) -> (Scene, lune_ui::input::FrameOutput) {
        let scene = SCENES.iter().find(|s| s.id == id).expect("a scene");
        run_scene(PAGE.id, scene, &mut FakeText).expect("runs")
    }

    /// Whether a shape of `layer` (acrylic or fill, physical px) covers the origin of a text of the
    /// dialog's own layer (logical px; the scenes run at scale 2): the overlay's surface is above
    /// the dialog's text.
    fn covers_dialog_text(scene: &Scene, layer: LayerId) -> bool {
        let over = &scene.layers[layer as usize];
        let shapes = over
            .geometry
            .under
            .iter()
            .map(|s| s.rect)
            .chain(over.geometry.acrylic.iter().map(|a| a.shape.rect));
        let texts = &scene.layers[LayerId::Modal as usize].texts;
        shapes.into_iter().any(|r| {
            texts.iter().any(|t| {
                let (x, y) = (t.x * 2.0, t.y * 2.0);
                x > r[0] && x < r[0] + r[2] && y > r[1] && y < r[1] + r[3]
            })
        })
    }

    fn has(out: &lune_ui::input::FrameOutput, role: Role) -> bool {
        out.tree.nodes.iter().any(|n| n.role == role)
    }

    #[test]
    fn the_overlays_a_dialog_opens_draw_above_its_text() {
        for (id, role) in [
            ("dialog-combo-open", Some(Role::List)),
            ("dialog-popover", Some(Role::Button)),
            ("dialog-tooltip", Some(Role::Tooltip)),
            ("dialog-menu-glass", Some(Role::Menu)),
            ("dialog-nested", None),
        ] {
            let (scene, out) = run(id);
            assert!(
                covers_dialog_text(&scene, LayerId::Raised),
                "{id}: fill above the text"
            );
            assert!(
                scene.layers[LayerId::Popup as usize]
                    .geometry
                    .under
                    .is_empty(),
                "{id}: nothing on Popup"
            );
            match role {
                Some(Role::Button) => {
                    assert!(
                        out.tree.nodes.iter().any(|n| n.name == "Done"),
                        "{id}: the popover's button"
                    );
                }
                Some(role) => assert!(has(&out, role), "{id}: {role:?} in the tree"),
                None => {
                    let dialogs = out
                        .tree
                        .nodes
                        .iter()
                        .filter(|n| n.role == Role::Dialog)
                        .count();
                    assert_eq!(dialogs, 2, "{id}: two dialogs");
                }
            }
        }
    }

    #[test]
    fn the_variants_are_what_they_say() {
        let (form, out) = run("dialog-form");
        let card = out
            .tree
            .nodes
            .iter()
            .find(|n| n.role == Role::Dialog)
            .expect("dialog")
            .bounds;
        assert_eq!(card.w, 600.0);
        assert!(form.backdrop.is_none());
        let (blur, _) = run("dialog-scrim-blur");
        let b = blur.backdrop.expect("blur");
        assert_eq!((b.layer, b.sigma), (LayerId::Modal as usize, 8.0));
        let (_, out) = run("dialog-confirm-disabled");
        let confirm = out
            .tree
            .nodes
            .iter()
            .find(|n| n.name == "Discard")
            .expect("confirm");
        assert!(confirm.states.contains(lune_ui::access::State::Disabled));
    }
}
