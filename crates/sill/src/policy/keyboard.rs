//! The keycap service on the settings' side: `keyboard.overrides` as the typed
//! [`UserOverrides`] ([`sill_settings::overrides`] parses the rows once), sent now and whenever
//! the answer changes so the service re-chooses the focused window's profile.

use sill_model::keyboard::UserOverrides;
use sill_model::keycap::KeycapCommand;
use sill_settings::{ShellFile, overrides};

use super::Policy;

/// The keycap service's overrides from the settings.
#[derive(Debug, Clone, Copy)]
pub struct KeyboardBridge;

impl Policy for KeyboardBridge {
    type Command = KeycapCommand;
    type Out = UserOverrides;

    fn of(&self, file: &ShellFile) -> UserOverrides {
        overrides(&file.keyboard).0
    }

    fn command(out: UserOverrides) -> KeycapCommand {
        KeycapCommand::Overrides(out)
    }
}

#[cfg(test)]
mod tests {
    use sill_model::keyboard::UserOverrides;
    use sill_model::keycap::KeycapCommand;
    use sill_settings::{KeyboardOverride, ShellFile};

    use super::KeyboardBridge;
    use crate::policy::Policy;

    fn file(rows: &[(&str, &str)]) -> ShellFile {
        let mut file = ShellFile::default();
        file.keyboard.overrides = rows
            .iter()
            .map(|&(app, profile)| KeyboardOverride {
                app: app.to_owned(),
                profile: profile.to_owned(),
            })
            .collect();
        file
    }

    /// The pairs `overrides` holds, as text, in app order.
    fn pairs(overrides: &UserOverrides) -> Vec<(&str, &str)> {
        overrides
            .0
            .iter()
            .map(|(app, profile)| (app.0.as_str(), profile.as_str()))
            .collect()
    }

    #[test]
    fn the_answer_is_the_rows_that_parse() {
        type Pairs = &'static [(&'static str, &'static str)];
        const CASES: &[(&str, Pairs, Pairs)] = &[
            ("no rows", &[], &[]),
            (
                "two rows",
                &[("org.gnome.Nautilus", "files"), ("foot", "term")],
                &[("foot", "term"), ("org.gnome.Nautilus", "files")],
            ),
            (
                "a bad profile costs its row",
                &[("a", "Bad Name"), ("b", "ok")],
                &[("b", "ok")],
            ),
        ];
        for (case, rows, want) in CASES {
            let answer = KeyboardBridge.of(&file(rows));
            assert_eq!(&pairs(&answer), want, "{case}");
        }
    }

    #[test]
    fn the_command_carries_the_overrides() {
        let answer = KeyboardBridge.of(&file(&[("foot", "term")]));
        assert_eq!(
            KeyboardBridge::command(answer.clone()),
            KeycapCommand::Overrides(answer)
        );
    }
}
