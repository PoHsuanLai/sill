//! Where the docks' icons rest, from the docks to the daemon: each dock publishes its output's
//! [`AppRects`] when they change, and the daemon's seat reads the whole [`OutputRects`].

use std::sync::Arc;

use sill_model::desktop::target::{AppRects, OutputRects};
use tokio::sync::watch;

/// The docks' side and the daemon's side of one watch.
#[derive(Debug, Clone)]
pub struct DockRects(Arc<watch::Sender<OutputRects>>);

impl Default for DockRects {
    fn default() -> DockRects {
        DockRects(Arc::new(watch::channel(OutputRects::default()).0))
    }
}

impl DockRects {
    /// The dock on `output` rests its icons at `rects`; nothing is sent when they are the same.
    pub fn publish(&self, output: &str, rects: AppRects) {
        self.0.send_if_modified(|all| {
            let same = all.0.get(output) == Some(&rects);
            if !same {
                all.0.insert(output.to_owned(), rects);
            }
            !same
        });
    }

    /// The dock on `output` is gone.
    pub fn withdraw(&self, output: &str) {
        self.0
            .send_if_modified(|all| all.0.remove(output).is_some());
    }

    /// A reader of every dock's rectangles.
    pub fn subscribe(&self) -> watch::Receiver<OutputRects> {
        self.0.subscribe()
    }
}
