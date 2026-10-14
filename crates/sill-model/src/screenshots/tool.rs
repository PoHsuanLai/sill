//! What `sill screenshot KIND` runs: `screenshot.tool`, parsed, and the command line per kind.
//!
//! The desktop portal's screenshot cannot be told area or window: it opens the portal's picker
//! (whose own buttons choose a rectangle, a window or an output, and where to save) when asked
//! to be interactive, and otherwise saves the whole screen where the portal keeps screenshots at
//! once. So `screen` is exact and `area` and `window` both open the picker.

use std::path::{Path, PathBuf};

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

/// What takes a screenshot. Stored as `screenshot.tool`'s text (`auto`, `off` or the command),
/// read through [`parse_tool`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum ShotTool {
    /// Nothing: `off`, and the passive start.
    #[default]
    Off,
    /// The desktop portal's screenshot (`auto`).
    Portal,
    /// A command line for `/bin/sh -c`, given the kind as `$1` and the folder as `$2`.
    Command(String),
}

impl From<String> for ShotTool {
    fn from(text: String) -> Self {
        parse_tool(&text)
    }
}

impl From<ShotTool> for String {
    fn from(tool: ShotTool) -> Self {
        match tool {
            ShotTool::Off => "off".to_owned(),
            ShotTool::Portal => "auto".to_owned(),
            ShotTool::Command(command) => command,
        }
    }
}

/// A program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandLine {
    pub program: String,
    pub args: Vec<String>,
}

/// `screenshot.tool`: `auto` is [`ShotTool::Portal`], `off` (and empty) is `Off`, any case,
/// trimmed; anything else is a command.
pub fn parse_tool(text: &str) -> ShotTool {
    let trimmed = text.trim();
    match trimmed.to_lowercase().as_str() {
        "auto" => ShotTool::Portal,
        "off" | "" => ShotTool::Off,
        _ => ShotTool::Command(trimmed.to_owned()),
    }
}

/// The command `tool` runs for `kind`, saving into `folder` where the tool can be told (the
/// first watched folder); `None` for `Off`. `Portal`: `gdbus call` of the portal's `Screenshot`,
/// interactive for `area` and `window` and not for `screen`. `Command(c)`: `/bin/sh -c c
/// sill-screenshot KIND FOLDER`.
pub fn tool_command(tool: &ShotTool, kind: ShotKind, folder: &Path) -> Option<CommandLine> {
    let folder = folder.to_string_lossy();
    match (tool, kind) {
        (ShotTool::Off, _) => None,
        (ShotTool::Portal, ShotKind::Screen) => Some(portal("false")),
        (ShotTool::Portal, ShotKind::Area | ShotKind::Window) => Some(portal("true")),
        (ShotTool::Command(command), _) => Some(line(
            "/bin/sh",
            ["-c", command, "sill-screenshot", kind.word(), &folder],
        )),
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

/// Where a tool is told to save: the first folder `folders` resolves to.
pub fn save_folder(folders: &[PathBuf]) -> Option<&Path> {
    folders.first().map(PathBuf::as_path)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{CommandLine, ShotKind, ShotTool, parse_tool, save_folder, tool_command};

    fn line(program: &str, args: &[&str]) -> Option<CommandLine> {
        Some(CommandLine {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
        })
    }

    #[test]
    fn the_first_folder_is_where_a_tool_saves() {
        let folders = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        assert_eq!(save_folder(&folders), Some(Path::new("/a")));
        assert_eq!(save_folder(&[]), None);
    }

    #[test]
    fn the_tool_setting_parses() {
        assert_eq!(parse_tool("auto"), ShotTool::Portal);
        assert_eq!(parse_tool(" OFF "), ShotTool::Off);
        assert_eq!(parse_tool(""), ShotTool::Off);
        assert_eq!(
            parse_tool("grim -g \"$(slurp)\" \"$2/Screenshot_$(date +%s).png\""),
            ShotTool::Command("grim -g \"$(slurp)\" \"$2/Screenshot_$(date +%s).png\"".to_owned())
        );
    }

    #[test]
    fn a_tool_is_stored_as_the_settings_text_and_reads_back() {
        const CASES: &[(&str, ShotTool, &str)] = &[
            ("off", ShotTool::Off, "\"off\""),
            ("auto", ShotTool::Portal, "\"auto\""),
        ];
        for (name, tool, text) in CASES {
            assert_eq!(
                serde_json::to_string(tool).as_deref().ok(),
                Some(*text),
                "{name}"
            );
            assert_eq!(
                serde_json::from_str::<ShotTool>(text).ok().as_ref(),
                Some(tool),
                "{name}"
            );
        }
        let command = ShotTool::Command("shoot \"$1\"".to_owned());
        let text = serde_json::to_string(&command).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(serde_json::from_str::<ShotTool>(&text).ok(), Some(command));
        assert_eq!(
            serde_json::from_str::<ShotTool>("\"  \"").ok(),
            Some(ShotTool::Off),
            "an empty text is off, as the setting always read it"
        );
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
        let dir = Path::new("/home/ada/Pictures");
        let cases = [
            (ShotTool::Off, ShotKind::Area, None),
            (
                ShotTool::Portal,
                ShotKind::Screen,
                line("gdbus", &PORTAL_QUIET),
            ),
            (
                ShotTool::Portal,
                ShotKind::Area,
                line("gdbus", &PORTAL_PICKER),
            ),
            (
                ShotTool::Portal,
                ShotKind::Window,
                line("gdbus", &PORTAL_PICKER),
            ),
            (
                ShotTool::Command("shoot \"$1\"".to_owned()),
                ShotKind::Window,
                line(
                    "/bin/sh",
                    &[
                        "-c",
                        "shoot \"$1\"",
                        "sill-screenshot",
                        "window",
                        "/home/ada/Pictures",
                    ],
                ),
            ),
        ];
        for (tool, kind, want) in cases {
            assert_eq!(tool_command(&tool, kind, dir), want, "{tool:?} {kind:?}");
        }
    }
}
