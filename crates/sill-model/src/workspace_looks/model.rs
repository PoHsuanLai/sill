//! The vocabulary: one published workspace, what the service holds, and what it is told.

use crate::work_areas::{WorkAreaLink, WorkTrouble};

/// One workspace as an app sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PublishedWorkspace {
    /// The compositor's stable id for the workspace when it gives one, else the shell's key for
    /// it, which lasts only while the shell runs.
    pub id: String,
    /// The workspace's name.
    pub name: String,
    /// The connector name of the first output its group shows; empty when the group has none.
    pub output: String,
    /// The workspace's `SpaceLook`, as the JSON quire reads (`SpaceLook::from_json`).
    pub look: String,
    /// Whether it is the active workspace of its group.
    pub active: bool,
}

/// Why the bus name is not served.
pub type WorkspaceLooksTrouble = WorkTrouble;

/// Whether the workspaces are on the bus.
pub type WorkspaceLooksLink = WorkAreaLink;

/// What the service publishes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceLooks {
    /// Every workspace of every group, in the compositor's order.
    pub workspaces: Vec<PublishedWorkspace>,
    pub link: WorkspaceLooksLink,
}

/// What the service is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceLooksCommand {
    /// Claim the bus name and answer; sent once by the daemon.
    Serve,
    /// These are now the workspaces (the daemon sends them when they change).
    Publish(Vec<PublishedWorkspace>),
}
