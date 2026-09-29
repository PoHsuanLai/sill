//! What `sill screenshot KIND` runs: `screenshot.tool`, parsed, and the command line per kind.
//!
//! COSMIC's tool cannot be told area or window: `cosmic-screenshot` opens the portal's picker
//! (whose own buttons choose a rectangle, a window or an output, and where to save) unless
//! `--interactive=false`, which saves the whole screen into `--save-dir` at once. So `screen`
//! is exact and `area` and `window` both open the picker.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// `sill screenshot`'s argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// What takes a screenshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ShotTool {
    /// Nothing: `off`, and the passive start.
    #[default]
    Off,
    /// `cosmic-screenshot` (`auto`).
    Cosmic,
    /// A command line for `/bin/sh -c`, given the kind as `$1` and the folder as `$2`.
    Command(String),
}

/// A program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandLine {
    pub program: String,
    pub args: Vec<String>,
}

/// `screenshot.tool`: `auto` is [`ShotTool::Cosmic`], `off` (and empty) is `Off`, any case,
/// trimmed; anything else is a command.
pub fn parse_tool(text: &str) -> ShotTool {
    let trimmed = text.trim();
    match trimmed.to_lowercase().as_str() {
        "auto" => ShotTool::Cosmic,
        "off" | "" => ShotTool::Off,
        _ => ShotTool::Command(trimmed.to_owned()),
    }
}

/// The command `tool` runs for `kind`, saving into `folder` where the tool can be told (the
/// first watched folder); `None` for `Off`. `Cosmic`: `screen` is `cosmic-screenshot
/// --interactive=false --modal=false --notify=false --save-dir FOLDER`, `area` and `window`
/// `cosmic-screenshot --interactive=true --notify=false`. `Command(c)`: `/bin/sh -c c
/// sill-screenshot KIND FOLDER`.
pub fn tool_command(tool: &ShotTool, kind: ShotKind, folder: &Path) -> Option<CommandLine> {
    let folder = folder.to_string_lossy();
    match (tool, kind) {
        (ShotTool::Off, _) => None,
        (ShotTool::Cosmic, ShotKind::Screen) => Some(line(
            COSMIC,
            [
                "--interactive=false",
                "--modal=false",
                "--notify=false",
                "--save-dir",
                &folder,
            ],
        )),
        (ShotTool::Cosmic, ShotKind::Area | ShotKind::Window) => {
            Some(line(COSMIC, ["--interactive=true", "--notify=false"]))
        }
        (ShotTool::Command(command), _) => Some(line(
            "/bin/sh",
            ["-c", command, "sill-screenshot", kind.word(), &folder],
        )),
    }
}

/// COSMIC's screenshot program.
const COSMIC: &str = "cosmic-screenshot";

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
        assert_eq!(parse_tool("auto"), ShotTool::Cosmic);
        assert_eq!(parse_tool(" OFF "), ShotTool::Off);
        assert_eq!(parse_tool(""), ShotTool::Off);
        assert_eq!(
            parse_tool("grim -g \"$(slurp)\" \"$2/Screenshot_$(date +%s).png\""),
            ShotTool::Command("grim -g \"$(slurp)\" \"$2/Screenshot_$(date +%s).png\"".to_owned())
        );
    }

    #[test]
    fn each_kind_runs_its_command() {
        let dir = Path::new("/home/ada/Pictures");
        let cases = [
            (ShotTool::Off, ShotKind::Area, None),
            (
                ShotTool::Cosmic,
                ShotKind::Screen,
                line(
                    "cosmic-screenshot",
                    &[
                        "--interactive=false",
                        "--modal=false",
                        "--notify=false",
                        "--save-dir",
                        "/home/ada/Pictures",
                    ],
                ),
            ),
            (
                ShotTool::Cosmic,
                ShotKind::Area,
                line(
                    "cosmic-screenshot",
                    &["--interactive=true", "--notify=false"],
                ),
            ),
            (
                ShotTool::Cosmic,
                ShotKind::Window,
                line(
                    "cosmic-screenshot",
                    &["--interactive=true", "--notify=false"],
                ),
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
