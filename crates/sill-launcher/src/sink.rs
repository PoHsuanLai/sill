//! Where a provider puts its results: rows, which the runner stamps with the provider and the
//! query generation that produced them (a [`Batch`]), so the merger can drop stale ones.

use serde::{Deserialize, Serialize};

use crate::ids::ProviderKind;
use crate::item::Item;
use crate::query::Generation;

/// Some of one provider's results for one query generation, as the runner stamps them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Batch {
    pub provider: ProviderKind,
    pub generation: Generation,
    pub items: Vec<Item>,
}

/// A receiver of rows. The live one stamps them and forwards them to the results merger on
/// another thread; the test one collects.
pub trait ResultSink {
    /// Hand over some rows. A provider may hand over several times (fast rows first, slow ones
    /// later).
    fn push(&mut self, items: Vec<Item>);
}

/// A sink that keeps every push, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VecSink(pub Vec<Vec<Item>>);

impl ResultSink for VecSink {
    fn push(&mut self, items: Vec<Item>) {
        self.0.push(items);
    }
}
