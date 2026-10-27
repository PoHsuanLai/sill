//! What `sill screenshot KIND` runs: the portal's screenshot, and the command line per kind.
//!
//! The desktop portal's screenshot cannot be told area or window: it opens the portal's picker
//! (whose own buttons choose a rectangle, a window or an output, and where to save) when asked
//! to be interactive, and otherwise saves the whole screen where the portal keeps screenshots at
//! once. So `screen` is exact and `area` and `window` both open the picker.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// `sill screenshot`'s argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum ShotKind {
    Area,
    Window,
    Screen,
}

impl ShotKind {
    /// The word a command gets as `$1`.
    pub fn word(self) -> &'static str {
        match self {
            ShotKind::Area => "area",
            ShotKind::Window => "window",
            ShotKind::Screen => "screen",
        }
    }
}

/// A program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandLine {
    pub program: String,
    pub args: Vec<String>,
}

/// The command `sill screenshot KIND` runs: `gdbus call` of the portal's `Screenshot`,
/// interactive (the picker) for `area` and `window` and not for `screen`.
pub fn portal_command(kind: ShotKind) -> CommandLine {
    match kind {
        ShotKind::Screen => portal("false"),
        ShotKind::Area | ShotKind::Window => portal("true"),
    }
}

/// The portal's `Screenshot` call, with the picker (`interactive` `true`) or without.
fn portal(interactive: &str) -> CommandLine {
    let options = format!("{{'interactive': <{interactive}>}}");
    line(
        "gdbus",
        [
            "call",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
            "--method",
            "org.freedesktop.portal.Screenshot.Screenshot",
            "",
            &options,
        ],
    )
}

fn line<'a>(program: &str, args: impl IntoIterator<Item = &'a str>) -> CommandLine {
    CommandLine {
        program: program.to_owned(),
        args: args.into_iter().map(str::to_owned).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandLine, ShotKind, portal_command};

    fn line(program: &str, args: &[&str]) -> Option<CommandLine> {
        Some(CommandLine {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
        })
    }

    const PORTAL_QUIET: [&str; 10] = [
        "call",
        "--session",
        "--dest",
        "org.freedesktop.portal.Desktop",
        "--object-path",
        "/org/freedesktop/portal/desktop",
        "--method",
        "org.freedesktop.portal.Screenshot.Screenshot",
        "",
        "{'interactive': <false>}",
    ];
    const PORTAL_PICKER: [&str; 10] = [
        "call",
        "--session",
        "--dest",
        "org.freedesktop.portal.Desktop",
        "--object-path",
        "/org/freedesktop/portal/desktop",
        "--method",
        "org.freedesktop.portal.Screenshot.Screenshot",
        "",
        "{'interactive': <true>}",
    ];

    #[test]
    fn each_kind_runs_its_command() {
        let cases = [
            (ShotKind::Screen, line("gdbus", &PORTAL_QUIET)),
            (ShotKind::Area, line("gdbus", &PORTAL_PICKER)),
            (ShotKind::Window, line("gdbus", &PORTAL_PICKER)),
        ];
        for (kind, want) in cases {
            assert_eq!(Some(portal_command(kind)), want, "{kind:?}");
        }
    }
}
