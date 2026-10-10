//! Where a profile can come from: one trait, implemented by each source.

use keycap_protocol::ProfileName;

use super::model::{CategoryTable, DeclaredProfile, FocusedWindow, OwnProfiles, UserOverrides};

/// One opinion about which profile a window wants. A source that has none says `None` and the
/// next one is asked; a new source is a new impl, never a new branch in [`super::choose`].
pub trait ProfileSource {
    fn profile_for(&self, window: &FocusedWindow<'_>) -> Option<ProfileName>;
}

impl ProfileSource for UserOverrides {
    fn profile_for(&self, window: &FocusedWindow<'_>) -> Option<ProfileName> {
        window.app.and_then(|app| self.0.get(&app.id)).cloned()
    }
}

impl ProfileSource for DeclaredProfile {
    fn profile_for(&self, window: &FocusedWindow<'_>) -> Option<ProfileName> {
        window.app.and_then(|app| app.keyboard_profile.clone())
    }
}

impl CategoryTable {
    /// What every install starts with: terminal emulators get the terminal profile.
    pub fn standard() -> Self {
        Self(vec![("TerminalEmulator".to_owned(), terminal_profile())])
    }
}

/// The name keycap's built-in config gives its terminal profile.
fn terminal_profile() -> ProfileName {
    ProfileName::parse("terminal").unwrap_or_else(|_| ProfileName::default_profile())
}

/// The profile of an app that resolves Command itself through the shared keymap: keycap leaves
/// Command as Super for it (the desktop's own chords stay rewritten).
pub fn keymap_aware_profile() -> ProfileName {
    ProfileName::parse("keymap-aware").unwrap_or_else(|_| ProfileName::default_profile())
}

impl OwnProfiles {
    /// The apps keycap's built-in config has a profile for: Ghostty (`com.mitchellh.ghostty`),
    /// whose split and clear chords differ from the other terminals', and temor, which resolves
    /// Command itself and so must receive Super (`keymap-aware`).
    pub fn standard() -> Self {
        let ghostty =
            ProfileName::parse("ghostty").unwrap_or_else(|_| ProfileName::default_profile());
        Self(vec![
            ("ghostty".to_owned(), ghostty),
            ("org.quire.temor".to_owned(), keymap_aware_profile()),
        ])
    }
}

impl ProfileSource for OwnProfiles {
    fn profile_for(&self, window: &FocusedWindow<'_>) -> Option<ProfileName> {
        let app_id = window.toplevel.app_id.to_lowercase();
        self.0
            .iter()
            .find(|(part, _)| app_id.contains(part.as_str()))
            .map(|(_, profile)| profile.clone())
    }
}

impl ProfileSource for CategoryTable {
    fn profile_for(&self, window: &FocusedWindow<'_>) -> Option<ProfileName> {
        let app = window.app?;
        self.0
            .iter()
            .find(|(category, _)| app.categories.contains(category))
            .map(|(_, profile)| profile.clone())
    }
}
