//! `org.quire.Workspaces1`, as the shell exports it: one read-only property over the published
//! state.

use sill_model::workspace_looks::WorkspaceLooks;
use tokio::sync::watch;
use zbus::interface;

use super::wire::{WireWorkspace, wire};

pub(super) struct WorkspacesObject {
    pub state: watch::Receiver<WorkspaceLooks>,
}

#[interface(name = "org.quire.Workspaces1")]
impl WorkspacesObject {
    #[zbus(property)]
    fn workspaces(&self) -> Vec<WireWorkspace> {
        self.state.borrow().workspaces.iter().map(wire).collect()
    }
}
