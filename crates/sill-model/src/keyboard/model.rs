//! The values the profile choice reads and the sources are made of (data only).

use std::collections::BTreeMap;

use keycap_protocol::ProfileName;
use sill_launcher::AppId;

use crate::apps::App;
use crate::windows::Toplevel;

/// The window that has the keyboard focus, and the desktop entry it belongs to when
/// `match_app` found one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusedWindow<'a> {
    pub toplevel: &'a Toplevel,
    pub app: Option<&'a App>,
}

/// The user's own choices, app by app; the one source that beats what an app declares.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserOverrides(pub BTreeMap<AppId, ProfileName>);

/// The profile an app declares for itself with `X-Keycap-Profile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeclaredProfile;

/// A profile per desktop-entry category, in priority order: the first row whose category the
/// app lists wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryTable(pub Vec<(String, ProfileName)>);

/// Profiles keycap defines for one app of its own (`[profile.ghostty]`, inheriting `terminal`),
/// matched by a part of the window's app id, lowercased: the way keycap's Plasma feed does with
/// the resource class. Above the category table, so a terminal that has a profile of its own
/// gets it, and below the user's overrides and what the app declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnProfiles(pub Vec<(String, ProfileName)>);
