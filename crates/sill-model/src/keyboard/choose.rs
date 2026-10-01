//! The pure choice: which window has the focus, and which profile its sources pick.

use keycap_protocol::ProfileName;

use super::model::{CategoryTable, DeclaredProfile, FocusedWindow, UserOverrides};
use super::source::ProfileSource;
use crate::apps::{AppIndex, match_app};
use crate::cosmic_wl::{Activity, Toplevels};

/// The first source that has a profile for `window`, in the order given; the default profile
/// when none does or nothing is focused.
pub fn choose(sources: &[&dyn ProfileSource], window: Option<FocusedWindow<'_>>) -> ProfileName {
    window
        .and_then(|window| {
            sources
                .iter()
                .find_map(|source| source.profile_for(&window))
        })
        .unwrap_or_else(ProfileName::default_profile)
}

/// The profile the focused window wants under sill's sources, most specific first: the user's
/// overrides, what the app declares, then the category table.
pub fn wanted(
    overrides: &UserOverrides,
    categories: &CategoryTable,
    toplevels: &Toplevels,
    apps: &AppIndex,
) -> ProfileName {
    let sources: [&dyn ProfileSource; 3] = [overrides, &DeclaredProfile, categories];
    choose(&sources, focused(toplevels, apps))
}

/// The active toplevel and its desktop entry; `None` while no toplevel is active.
///
/// Only toplevels are considered. sill's own surfaces (the bar, the dock, the launcher) are
/// layer surfaces, which the compositor never lists as toplevels, so they can never be the
/// focused window here, and no source is ever asked about them.
pub fn focused<'a>(toplevels: &'a Toplevels, apps: &'a AppIndex) -> Option<FocusedWindow<'a>> {
    let toplevel = toplevels
        .windows
        .iter()
        .find(|window| window.activity == Activity::Active)?;
    let app = match_app(&toplevel.app_id, None, apps).and_then(|id| apps.get(&id));
    Some(FocusedWindow { toplevel, app })
}
