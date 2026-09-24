//! How a provider's batches leave its blocking task: stamped with the query's provider and
//! generation, dropped at once when a newer keystroke has overtaken them, and followed by one
//! `Finished` however the query ended (a panicking provider included).

use sill_launcher::{Batch, Generation, ProviderKind, Query, ResultSink};
use tokio::sync::mpsc;

/// What a provider's task tells the runner.
#[derive(Debug)]
pub(crate) enum Arrival {
    /// Some rows.
    Batch(Batch),
    /// One provider is done with `generation`.
    Finished(Generation),
}

/// The live [`ResultSink`]: forwards to the runner.
pub(crate) struct ChannelSink<'a> {
    pub(crate) provider: ProviderKind,
    pub(crate) query: &'a Query,
    pub(crate) tx: &'a mpsc::UnboundedSender<Arrival>,
}

impl ResultSink for ChannelSink<'_> {
    fn push(&mut self, batch: Batch) {
        if self.query.is_cancelled() {
            return;
        }
        // The runner owns generations and knows which provider this task runs, so it stamps
        // both rather than trusting the provider to copy them.
        let batch = Batch {
            provider: self.provider,
            generation: self.query.generation,
            items: batch.items,
        };
        // A closed channel means the runner is gone; nobody is left to show the rows.
        let _ = self.tx.send(Arrival::Batch(batch));
    }
}

/// Sends `Finished` when dropped, so a provider that panics still counts as done.
pub(crate) struct FinishedOnDrop {
    pub(crate) generation: Generation,
    pub(crate) tx: mpsc::UnboundedSender<Arrival>,
}

impl Drop for FinishedOnDrop {
    fn drop(&mut self) {
        let _ = self.tx.send(Arrival::Finished(self.generation));
    }
}
