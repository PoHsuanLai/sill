//! The pure choice: which window has the focus, and which profile its sources pick.

use keycap_protocol::ProfileName;

use super::model::{CategoryTable, DeclaredProfile, FocusedWindow, OwnProfiles, UserOverrides};
use super::source::ProfileSource;
use crate::apps::AppIndex;
use crate::windows::{Toplevels, app_of};

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
/// overrides, what the app declares, the profiles keycap has for one app (Ghostty), then the
/// category table.
pub fn wanted(
    overrides: &UserOverrides,
    own: &OwnProfiles,
    categories: &CategoryTable,
    toplevels: &Toplevels,
    apps: &AppIndex,
) -> ProfileName {
    let sources: [&dyn ProfileSource; 4] = [overrides, &DeclaredProfile, own, categories];
    choose(&sources, focused(toplevels, apps))
}

/// The active toplevel and its desktop entry; `None` while no toplevel is active.
///
/// Only toplevels are considered. sill's own surfaces (the bar, the dock, the launcher) are
/// layer surfaces, which the compositor never lists as toplevels, so they can never be the
/// focused window here, and no source is ever asked about them.
pub fn focused<'a>(toplevels: &'a Toplevels, apps: &'a AppIndex) -> Option<FocusedWindow<'a>> {
    let toplevel = toplevels.active()?;
    let app = app_of(toplevel, apps).entry(apps);
    Some(FocusedWindow { toplevel, app })
}
