//! What a popup menu is: the entries it shows and the value picking one yields. The owning
//! surface writes an `impl MenuModel` for its menu and its own `Pick` enum; the popup, the
//! tracking and the keyboard are not its business.

use ds::prelude::*;

/// A menu's contents.
pub trait MenuModel: Clone + PartialEq + 'static {
    /// What picking an entry yields: the owning surface's own vocabulary.
    type Pick: Clone + PartialEq + 'static;

    /// The entries, top to bottom, each carrying its pick.
    fn entries(&self) -> Vec<MenuItem<Self::Pick>>;

    /// What a context menu does with an entry that cannot be picked: leave it out (the
    /// default) or keep it dimmed, for a menu whose entries are always the same set.
    fn unavailable(&self) -> ContextUnavailable {
        ContextUnavailable::Hide
    }
}
