//! Quitting the apps before the session ends, the way macOS does for Log Out, Restart and Shut
//! Down: ask every window to close, wait for the apps to go, and only then let logind end the
//! session. An app that keeps a window open (it is asking whether to save) stops the whole thing.

mod live;
mod sequence;
#[cfg(test)]
mod tests;

pub(crate) use live::LiveApps;
pub(crate) use sequence::{AppsHost, OpenWindow, QuitOutcome, quit_apps};
