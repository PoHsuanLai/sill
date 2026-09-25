//! Which desktop entries quire's own app icons stand for (`dock.shipped_icons`). Pure.
//!
//! quire ships five icons (`assets/icons/apps/<app>/`); an entry takes one only when its desktop
//! id is in [`DEFAULT_TABLE`], compared without regard to case. The table is short on purpose:
//! a wrong match puts our terminal on someone else's app, while a missing one only leaves the
//! theme's icon in place. No category rule: `App` does not keep `Categories=`, and even
//! `TerminalEmulator` would sweep in things that are not terminals to their users.

use sill_launcher::AppId;
use sill_settings::ShippedIcons;

/// One of quire's shipped app icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShippedApp {
    Mail,
    Files,
    Terminal,
    Notes,
    Photos,
}

impl ShippedApp {
    /// Every shipped icon.
    pub const ALL: [ShippedApp; 5] = [
        ShippedApp::Mail,
        ShippedApp::Files,
        ShippedApp::Terminal,
        ShippedApp::Notes,
        ShippedApp::Photos,
    ];

    /// Its directory under `assets/icons/apps/`.
    pub fn slug(self) -> &'static str {
        match self {
            ShippedApp::Mail => "mail",
            ShippedApp::Files => "files",
            ShippedApp::Terminal => "terminal",
            ShippedApp::Notes => "notes",
            ShippedApp::Photos => "photos",
        }
    }
}

/// Desktop ids (lower case, without `.desktop`) and the icon each takes.
pub const DEFAULT_TABLE: &[(&str, ShippedApp)] = &[
    // COSMIC's own.
    ("com.system76.cosmicterm", ShippedApp::Terminal),
    ("com.system76.cosmicfiles", ShippedApp::Files),
    ("com.system76.cosmicedit", ShippedApp::Notes),
    // Terminals.
    ("com.mitchellh.ghostty", ShippedApp::Terminal),
    ("foot", ShippedApp::Terminal),
    ("org.codeberg.dnkl.foot", ShippedApp::Terminal),
    ("kitty", ShippedApp::Terminal),
    ("alacritty", ShippedApp::Terminal),
    ("org.alacritty.alacritty", ShippedApp::Terminal),
    ("org.gnome.terminal", ShippedApp::Terminal),
    ("org.gnome.console", ShippedApp::Terminal),
    ("org.kde.konsole", ShippedApp::Terminal),
    // File managers.
    ("org.gnome.nautilus", ShippedApp::Files),
    ("org.kde.dolphin", ShippedApp::Files),
    ("thunar", ShippedApp::Files),
    // Mail.
    ("mailo", ShippedApp::Mail),
    ("thunderbird", ShippedApp::Mail),
    ("org.mozilla.thunderbird", ShippedApp::Mail),
    ("org.gnome.geary", ShippedApp::Mail),
    ("org.gnome.evolution", ShippedApp::Mail),
    // Text editors, as notes.
    ("org.gnome.texteditor", ShippedApp::Notes),
    ("org.kde.kate", ShippedApp::Notes),
    // Image viewers, as photos.
    ("org.gnome.loupe", ShippedApp::Photos),
    ("org.gnome.eog", ShippedApp::Photos),
    ("org.kde.gwenview", ShippedApp::Photos),
    ("org.gnome.shotwell", ShippedApp::Photos),
];

/// The shipped icon the entry `id` takes under `setting`, looked up in `table`.
pub fn shipped_app(
    id: &AppId,
    setting: ShippedIcons,
    table: &[(&str, ShippedApp)],
) -> Option<ShippedApp> {
    match setting {
        ShippedIcons::Off => None,
        ShippedIcons::On => table
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(&id.0))
            .map(|(_, app)| *app),
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_TABLE, ShippedApp, shipped_app};
    use sill_launcher::AppId;
    use sill_settings::ShippedIcons;

    #[test]
    fn known_entries_take_their_icon_and_nothing_else_does() {
        const CASES: &[(&str, Option<ShippedApp>)] = &[
            ("com.system76.CosmicTerm", Some(ShippedApp::Terminal)),
            ("com.system76.CosmicFiles", Some(ShippedApp::Files)),
            ("com.system76.CosmicEdit", Some(ShippedApp::Notes)),
            ("com.mitchellh.ghostty", Some(ShippedApp::Terminal)),
            ("foot", Some(ShippedApp::Terminal)),
            ("org.kde.konsole", Some(ShippedApp::Terminal)),
            ("org.gnome.Nautilus", Some(ShippedApp::Files)),
            ("org.mozilla.Thunderbird", Some(ShippedApp::Mail)),
            ("mailo", Some(ShippedApp::Mail)),
            ("org.gnome.TextEditor", Some(ShippedApp::Notes)),
            ("org.gnome.Loupe", Some(ShippedApp::Photos)),
            // Near misses stay with the theme: no prefix, suffix or substring matching.
            ("com.system76.CosmicTermExtra", None),
            ("footclient", None),
            ("org.gnome.Nautilus.Extension", None),
            ("files", None),
            ("firefox", None),
            ("", None),
        ];
        for &(id, want) in CASES {
            let got = shipped_app(&AppId(id.to_owned()), ShippedIcons::On, DEFAULT_TABLE);
            assert_eq!(got, want, "{id}");
        }
    }

    #[test]
    fn off_keeps_every_theme_icon() {
        for &(id, _) in DEFAULT_TABLE {
            assert_eq!(
                shipped_app(&AppId(id.to_owned()), ShippedIcons::Off, DEFAULT_TABLE),
                None,
                "{id}"
            );
        }
    }

    #[test]
    fn the_table_is_lower_case_without_duplicates_and_uses_every_icon() {
        for (i, (id, _)) in DEFAULT_TABLE.iter().enumerate() {
            assert_eq!(*id, id.to_ascii_lowercase(), "{id}");
            assert!(!id.ends_with(".desktop"), "{id}");
            assert!(
                DEFAULT_TABLE[i + 1..].iter().all(|(other, _)| other != id),
                "{id} twice"
            );
        }
        for app in ShippedApp::ALL {
            assert!(DEFAULT_TABLE.iter().any(|(_, a)| *a == app), "{app:?}");
        }
    }
}
