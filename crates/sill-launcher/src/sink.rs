//! Where a provider puts its results: batches, each stamped with the query generation that
//! produced it, so the merger can drop stale ones.

use serde::{Deserialize, Serialize};

use crate::ids::ProviderKind;
use crate::item::Item;
use crate::query::Generation;

/// Some of one provider's results for one query generation. A provider may send several
/// (fast rows first, slow ones later).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Batch {
    pub provider: ProviderKind,
    pub generation: Generation,
    pub items: Vec<Item>,
}

/// A receiver of batches. The live one forwards to the results merger on another thread; the
/// test one collects.
pub trait ResultSink {
    /// Hand over one batch.
    fn push(&mut self, batch: Batch);
}

/// A sink that keeps every batch, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VecSink(pub Vec<Batch>);

impl ResultSink for VecSink {
    fn push(&mut self, batch: Batch) {
        self.0.push(batch);
    }
}
