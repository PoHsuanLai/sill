//! Which window an agent means.

use crate::windows::{Toplevel, Toplevels};

/// The window `ask` names: its key, or an app id (the app's most recent window); the focused
/// window (the most recently activated) when nothing is asked.
pub fn window_of<'a>(toplevels: &'a Toplevels, ask: Option<&str>) -> Option<&'a Toplevel> {
    let ask = ask.map(str::trim).filter(|ask| !ask.is_empty());
    let Some(ask) = ask else {
        return toplevels.windows.first();
    };
    let by_key = ask
        .parse::<u64>()
        .ok()
        .and_then(|key| toplevels.windows.iter().find(|w| w.key.0 == key));
    by_key.or_else(|| toplevels.windows.iter().find(|w| w.app_id == ask))
}
