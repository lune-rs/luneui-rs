//! The `a11y` page: the controls composition with every node of the previous frame's semantic
//! tree outlined and its role and name written beside it. A gallery aid; it shows nothing the OS
//! bridge adds.

use lune_ui::access::Role;
use lune_ui::motion::MotionTier;
use lune_ui::paint::Rect;
use lune_ui::text::{Spec, Style};
use lune_ui::ui::Ui;

use super::card;
use super::controls_ui::Controls;
use crate::page::{Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "a11y",
    title: "Accessibility",
    kind: PageKind::Ui(UiPage {
        state: nodes,
        scenes: SCENES,
        bench: &[],
    }),
};

struct Nodes {
    controls: Controls,
}

fn nodes() -> Box<dyn PageState> {
    Box::new(Nodes {
        controls: Controls::new(),
    })
}

impl PageState for Nodes {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let area = card(ui, "luneui gallery");
        self.controls.draw(ui, area);
        let theme = *ui.theme();
        // The previous frame's nodes: empty on the very first frame.
        let found: Vec<(Rect, Role, String)> = ui
            .previous_tree()
            .nodes
            .iter()
            .map(|n| (n.bounds, n.role, format!("{:?} {}", n.role, n.name)))
            .collect();
        let accent = theme.accent;
        let outline = [accent[0], accent[1], accent[2], 160];
        for (bounds, role, text) in found {
            let p = ui.painter();
            p.ring(bounds, 0.0, 1.0, outline, true);
            let spec = Spec::plain(&text, Style::ui(9.0, theme.fg_secondary));
            // Beside the node where the space below is taken by the next row.
            let (x, y) = match role {
                Role::StaticText | Role::SpinButton | Role::ComboBox => {
                    (bounds.right() + 6.0, bounds.y + (bounds.h - 11.0) / 2.0)
                }
                _ => (bounds.x + 2.0, bounds.bottom() + 1.0),
            };
            p.text(&spec, x, y);
        }
    }
}

const SCENES: &[UiScene] = &[UiScene {
    id: "a11y-nodes",
    scale: 2.0,
    tier: MotionTier::Full,
    state: nodes,
    // Frame 0 has no previous tree; the second frame outlines the first's nodes.
    script: &[Step {
        at_ms: 50,
        events: &[],
    }],
    capture_ms: 100,
    expect: Expect::Settled,
}];
