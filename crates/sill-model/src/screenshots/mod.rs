//! Screenshots: the tool that takes them, the folders they land in, and the shots the thumbnail
//! follows.

mod command;
mod model;
mod names;
mod step;
#[cfg(test)]
mod tests;
mod tool;

pub use command::{ShotCommand, ShotEffect, ShotEvent, ShotFolders, ShotLeave, ShotPolicy};
pub use model::{Shot, ShotId, ShotPreview, Shots, Taking, Watching};
pub use names::{ShotName, folders_of, is_screenshot_name, parse_folders, preview_name};
pub use step::step;
pub use tool::{CommandLine, ShotKind, portal_command};
