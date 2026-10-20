//! What the work-areas service is told.

use super::model::WorkArea;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkAreaCommand {
    /// These are now the outputs' work areas (the daemon sends them when they change).
    Publish(Vec<WorkArea>),
}
