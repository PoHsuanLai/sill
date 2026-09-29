//! `sill display {brightness,volume} ...`: the brightness and volume keys' command line, and
//! its pure mapping to a [`DisplayRequest`].

use clap::Subcommand;
use sill_ipc::{DisplayRequest, LevelRequest, VolumeRequest};

/// `sill display ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DisplayAction {
    /// The display under the pointer (or every display, per `display.brightness_keys_target`).
    Brightness {
        #[command(subcommand)]
        change: LevelAction,
    },
    /// The output volume (and a monitor's own speakers, per `display.hardware_volume`).
    Volume {
        #[command(subcommand)]
        change: VolumeAction,
    },
}

/// `... up|down|set <percent>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum LevelAction {
    /// One key press brighter or louder.
    Up,
    /// One key press dimmer or quieter.
    Down,
    /// Straight to PERCENT (0-100).
    Set {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
        percent: u8,
    },
}

/// `sill display volume ...`: a level change or the mute key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum VolumeAction {
    Up,
    Down,
    Set {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
        percent: u8,
    },
    Mute,
    Unmute,
    ToggleMute,
}

fn level(action: LevelAction) -> LevelRequest {
    match action {
        LevelAction::Up => LevelRequest::Up,
        LevelAction::Down => LevelRequest::Down,
        LevelAction::Set { percent } => LevelRequest::Set { percent },
    }
}

/// The request `action` sends.
pub fn display_request(action: &DisplayAction) -> DisplayRequest {
    match action {
        DisplayAction::Brightness { change } => DisplayRequest::Brightness(level(*change)),
        DisplayAction::Volume { change } => DisplayRequest::Volume(match change {
            VolumeAction::Up => VolumeRequest::Level(LevelRequest::Up),
            VolumeAction::Down => VolumeRequest::Level(LevelRequest::Down),
            VolumeAction::Set { percent } => {
                VolumeRequest::Level(LevelRequest::Set { percent: *percent })
            }
            VolumeAction::Mute => VolumeRequest::Mute,
            VolumeAction::Unmute => VolumeRequest::Unmute,
            VolumeAction::ToggleMute => VolumeRequest::ToggleMute,
        }),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use sill_ipc::{DisplayRequest, LevelRequest, Request, VolumeRequest};

    use crate::cli::{Cli, request};

    #[test]
    fn every_display_command_line_sends_its_request() {
        let brightness = |l| Some(Request::Display(DisplayRequest::Brightness(l)));
        let volume = |v| Some(Request::Display(DisplayRequest::Volume(v)));
        let cases: Vec<(&[&str], Option<Request>)> = vec![
            (
                &["display", "brightness", "up"],
                brightness(LevelRequest::Up),
            ),
            (
                &["display", "brightness", "down"],
                brightness(LevelRequest::Down),
            ),
            (
                &["display", "brightness", "set", "40"],
                brightness(LevelRequest::Set { percent: 40 }),
            ),
            (
                &["display", "volume", "up"],
                volume(VolumeRequest::Level(LevelRequest::Up)),
            ),
            (
                &["display", "volume", "set", "0"],
                volume(VolumeRequest::Level(LevelRequest::Set { percent: 0 })),
            ),
            (&["display", "volume", "mute"], volume(VolumeRequest::Mute)),
            (
                &["display", "volume", "unmute"],
                volume(VolumeRequest::Unmute),
            ),
            (
                &["display", "volume", "toggle-mute"],
                volume(VolumeRequest::ToggleMute),
            ),
        ];
        for (args, want) in cases {
            let cli = Cli::try_parse_from(std::iter::once("sill").chain(args.iter().copied()))
                .unwrap_or_else(|e| panic!("{args:?}: {e}"));
            let command = cli
                .command
                .unwrap_or_else(|| panic!("{args:?}: no command"));
            assert_eq!(request(&command), want, "{args:?}");
        }
    }

    #[test]
    fn a_malformed_display_command_is_refused() {
        for args in [
            &["display"][..],
            &["display", "brightness"],
            &["display", "brightness", "set", "101"],
            &["display", "brightness", "set", "-1"],
            &["display", "volume", "louder"],
        ] {
            assert!(
                Cli::try_parse_from(std::iter::once("sill").chain(args.iter().copied())).is_err(),
                "{args:?}"
            );
        }
    }
}
