use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A widget's stable identity across frames: a hash of its parent's id and its own key, so the
/// same path of keys gives the same id every frame. The hasher has fixed keys, so the value is
/// the same in every process.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct WidgetId(u64);

impl WidgetId {
    /// The id of the window's root; every top-level widget is a child of it.
    pub const ROOT: WidgetId = WidgetId(0);

    /// The id of a child of `self` with `key`.
    pub fn with(self, key: impl Hash) -> WidgetId {
        let mut h = DefaultHasher::new();
        (self.0, key).hash(&mut h);
        WidgetId(h.finish())
    }

    /// The raw hash, for logs and tests.
    pub fn value(self) -> u64 {
        self.0
    }

    /// The id whose [`value`](WidgetId::value) is `raw`, for an adapter mapping an id back from an
    /// outside system. Nothing checks that a widget has it: a target the frame cannot find is
    /// ignored.
    pub fn from_value(raw: u64) -> WidgetId {
        WidgetId(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::WidgetId;

    #[test]
    fn the_same_path_gives_the_same_id() {
        let a = WidgetId::ROOT.with("panel").with(3_u32);
        let b = WidgetId::ROOT.with("panel").with(3_u32);
        assert_eq!(a, b);
        assert_eq!(a.value(), b.value());
    }

    #[test]
    fn the_order_of_keys_matters() {
        assert_ne!(
            WidgetId::ROOT.with("a").with("b"),
            WidgetId::ROOT.with("b").with("a")
        );
    }

    #[test]
    fn from_value_is_the_inverse_of_value() {
        let id = WidgetId::ROOT.with("panel").with(3_u32);
        assert_eq!(WidgetId::from_value(id.value()), id);
        assert_eq!(WidgetId::from_value(0), WidgetId::ROOT);
        assert_eq!(WidgetId::from_value(u64::MAX).value(), u64::MAX);
    }

    #[test]
    fn ten_thousand_sibling_keys_give_ten_thousand_ids() {
        let ids: std::collections::BTreeSet<WidgetId> =
            (0..10_000_u32).map(|k| WidgetId::ROOT.with(k)).collect();
        assert_eq!(ids.len(), 10_000);
    }
}
