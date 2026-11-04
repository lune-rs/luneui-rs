//! The retained store: everything of a window's `Ui` that outlives one frame (D5). Typed entries
//! keyed `(WidgetId, TypeId)` in `BTreeMap`s, so iteration order never depends on the process.

use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet};

use crate::motion::{MotionTier, Transition};
use crate::paint::Rect;

use super::WidgetId;
use super::focus::FocusState;
use super::overlay::OverlayRec;
use super::pointer::{HitRecord, PointerState};
use crate::access::SemanticTree;

/// One retained value and the frame that last used it.
pub(crate) struct Entry {
    /// `UiStore::frame` of the last use.
    pub(crate) used: u64,
    /// The keep-alive scopes open at the last use, outermost first.
    pub(crate) chain: Vec<WidgetId>,
    pub(crate) value: Box<dyn Any>,
}

/// One animated value and the frame that last sampled it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Anim<T: crate::motion::Lerp> {
    pub(crate) used: u64,
    pub(crate) transition: Transition<T>,
}

/// A window's retained `Ui` state: one per window, held by the app and lent to each frame.
#[derive(Default)]
pub struct UiStore {
    /// Frames finished so far; the current frame's stamp.
    pub(crate) frame: u64,
    /// The previous frame's hit regions, in draw order.
    pub(crate) prev_hits: Vec<HitRecord>,
    /// The previous frame's registered rect per id (the last registration wins).
    pub(crate) prev_rects: BTreeMap<WidgetId, Rect>,
    pub(crate) pointer: PointerState,
    pub(crate) transient: BTreeMap<(WidgetId, TypeId), Entry>,
    pub(crate) persistent: BTreeMap<(WidgetId, TypeId), Entry>,
    /// Keep-alive scopes the app opened and has not ended (D6).
    pub(crate) live_scopes: BTreeSet<WidgetId>,
    pub(crate) anim_f32: BTreeMap<WidgetId, Anim<f32>>,
    pub(crate) anim_color: BTreeMap<WidgetId, Anim<[f32; 4]>>,
    /// The tier of the previous frame, `None` before the first.
    pub(crate) last_tier: Option<MotionTier>,
    /// Open, opening and closing overlays, in open order.
    pub(crate) overlays: Vec<OverlayRec>,
    /// The tag of the most recently opened overlay (0: none yet).
    pub(crate) next_tag: u32,
    pub(crate) focus: FocusState,
    pub(crate) prev_tree: SemanticTree,
    /// `(layer, hit tag)` each node of the previous frame was emitted under.
    pub(crate) prev_node_meta: BTreeMap<WidgetId, (usize, u32)>,
    /// The previous frame asked for a frame only because its hover was stale (D81).
    pub(crate) hover_catch_up: bool,
}

impl std::fmt::Debug for UiStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiStore")
            .field("frame", &self.frame)
            .field("hits", &self.prev_hits.len())
            .field("transient", &self.transient.len())
            .field("persistent", &self.persistent.len())
            .field("live_scopes", &self.live_scopes)
            .field("animations", &(self.anim_f32.len() + self.anim_color.len()))
            .field("overlays", &self.overlays)
            .field("focus", &self.focus)
            .finish_non_exhaustive()
    }
}

impl UiStore {
    /// The rect `id` registered in the previous frame, clipped and transformed as hits are.
    pub fn last_rect(&self, id: WidgetId) -> Option<Rect> {
        self.prev_rects.get(&id).copied()
    }

    /// The entry for `(id, T)` in `map`, created with `T::default()`, stamped as used now.
    pub(crate) fn entry<'m, T: Default + 'static>(
        map: &'m mut BTreeMap<(WidgetId, TypeId), Entry>,
        id: WidgetId,
        frame: u64,
        chain: &[WidgetId],
    ) -> &'m mut T {
        let e = map.entry((id, TypeId::of::<T>())).or_insert_with(|| Entry {
            used: frame,
            chain: Vec::new(),
            value: Box::new(T::default()),
        });
        e.used = frame;
        if e.chain.as_slice() != chain {
            e.chain = chain.to_vec();
        }
        e.value
            .downcast_mut::<T>()
            .expect("entries are keyed by their TypeId")
    }

    /// Drops what the frame `frame` did not use: transient entries and animations at once,
    /// persistent entries unless a live keep-alive scope is on their chain.
    pub(crate) fn collect(&mut self, frame: u64) {
        self.transient.retain(|_, e| e.used == frame);
        let live = &self.live_scopes;
        self.persistent
            .retain(|_, e| e.used == frame || e.chain.iter().any(|s| live.contains(s)));
        self.anim_f32.retain(|_, a| a.used == frame);
        self.anim_color.retain(|_, a| a.used == frame);
    }
}
