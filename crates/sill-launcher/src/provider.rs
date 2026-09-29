//! The provider seam: apps, windows, calculator, settings and system are five implementations
//! swapped behind it (CONVENTIONS §5).

use crate::activation::Activation;
use crate::ids::ProviderKind;
use crate::item::{Choice, Item};
use crate::query::Query;
use crate::sink::ResultSink;

/// One source of launcher results.
///
/// `query` runs off the UI thread and may be called again before it returns (a newer
/// keystroke); it should check [`Query::is_cancelled`] between expensive steps and stop early.
/// `activate` is pure: it says what should happen and the daemon does it.
pub trait Provider: Send + Sync {
    /// Which provider this is.
    fn kind(&self) -> ProviderKind;

    /// Push zero or more batches of results for `query` into `sink`.
    fn query(&self, query: &Query, sink: &mut dyn ResultSink);

    /// What choosing `choice` on `item` (one of this provider's own rows) should do.
    fn activate(&self, item: &Item, choice: &Choice) -> Activation;
}
