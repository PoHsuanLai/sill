//! `org.quire.Outputs1`, as the shell exports it: one read-only property over the published
//! state.

use sill_model::work_areas::WorkAreas;
use tokio::sync::watch;
use zbus::interface;

use super::wire::{WireArea, wire};

pub(super) struct OutputsObject {
    pub state: watch::Receiver<WorkAreas>,
}

#[interface(name = "org.quire.Outputs1")]
impl OutputsObject {
    #[zbus(property)]
    fn work_areas(&self) -> Vec<WireArea> {
        self.state.borrow().areas.iter().map(wire).collect()
    }
}
