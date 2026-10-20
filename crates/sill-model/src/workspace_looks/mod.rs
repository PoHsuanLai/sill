//! What the shell publishes about its workspaces on `org.quire.Workspaces1`: each workspace's id,
//! name, output and `SpaceLook` (as quire's JSON text), and which one is active. An app that
//! keeps its Spaces from the desktop (`SpacesSource::Desktop`) paints the active workspace's look.
//! Pure; `sill-services` serves it and the daemon works the looks out.

mod model;
mod reduce;
#[cfg(test)]
mod tests;

pub use model::{PublishedWorkspace, WorkspaceLooks, WorkspaceLooksCommand, WorkspaceLooksLink};
pub use reduce::published;
