//! What the work-areas service is told.

use super::model::WorkArea;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkAreaCommand {
    /// Claim the bus name and answer; sent once by the daemon.
    Serve,
    /// These are now the outputs' work areas (the daemon sends them when they change).
    Publish(Vec<WorkArea>),
}
