//! `sill debug ...`: the development probes' command line, and their pure mappings to requests.
//! Each made-up input is built as the `sill-model` value the service would have received.

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};
use sill_ipc::debug::{
    CornerName, DebugRequest, LauncherKey, OsdTopic, Point, PointerButton, SurfaceName,
};
use sill_ipc::request::Request;

use sill_launcher::AppId;
use sill_model::auth::FakeVerdict;
use sill_model::units::Percent;
use sill_model::volume::Mute;

use super::made_up::{NotifyArgs, PolkitArgs, polkit_request, posting};

/// `sill debug ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DebugAction {
    /// Frame statistics for every surface.
    Stats,
    /// Move the pointer to X Y on SURFACE (`sill-dock-DP-1`).
    Pointer { surface: String, x: i32, y: i32 },
    /// The pointer leaves SURFACE.
    Leave { surface: String },
    /// Render SURFACE to a PNG at PATH.
    Capture { surface: String, path: PathBuf },
    /// Press and release a pointer button at X Y on SURFACE (`sill-bar-DP-1`).
    Click {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_enum, default_value_t = ClickButton::Primary)]
        button: ClickButton,
    },
    /// Show the OSD for a made-up change, moving no real level.
    Osd {
        /// What changed.
        #[arg(value_enum)]
        topic: TopicArg,
        /// The level to show, 0-100.
        percent: u8,
        /// Whether the made-up volume is muted.
        #[arg(long, value_enum, default_value_t = MuteArg::Audible)]
        mute: MuteArg,
    },
    /// Post a made-up notification without D-Bus: a banner, and a row in the center.
    Notify(NotifyArgs),
    /// Show the screenshot thumbnail for PATH, as if it had just been saved.
    Screenshot { path: PathBuf },
    /// Carry out CORNER's hot corner action, as if the pointer had rested there.
    Corner {
        #[arg(value_enum)]
        corner: CornerArg,
    },
    /// `button` presses at X Y on SURFACE and stays down: the first half of a real drag
    /// (shell-host's `Inject::PointerDown`). `sill debug pointer` moves while it
    /// is held, and `pointer-release` lets go.
    PointerPress {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_enum, default_value_t = ClickButton::Primary)]
        button: ClickButton,
    },
    /// `button` releases at X Y on SURFACE: the second half of a real drag
    /// (`Inject::PointerUp`).
    PointerRelease {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_enum, default_value_t = ClickButton::Primary)]
        button: ClickButton,
    },
    /// Feed the lock screen's password check a made-up verdict (a debug daemon only).
    Auth {
        #[arg(value_enum)]
        verdict: VerdictArg,
    },
    /// Show a made-up polkit request (its stand-in helper accepts `sill-fake`).
    Polkit(PolkitArgs),
    /// Open the app switcher on made-up apps (desktop ids, the current one first).
    Switcher {
        #[arg(required = true)]
        apps: Vec<String>,
    },
    /// Press a key on the open launcher (a debug daemon only).
    LauncherKey {
        #[arg(value_enum)]
        key: LauncherKeyArg,
    },
    /// Print the open launcher's sections, rows and preview (a debug daemon only).
    LauncherRows,
    /// Wake idle as real input would (a debug daemon only; inside the lock grace it opens the
    /// lock).
    IdleWake,
}

/// `sill debug osd TOPIC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TopicArg {
    Volume,
    Brightness,
}

/// `sill debug osd --mute ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MuteArg {
    Audible,
    Muted,
}

/// `sill debug click --button ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ClickButton {
    Primary,
    Secondary,
    Middle,
}

/// `sill debug corner CORNER`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CornerArg {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// `sill debug auth VERDICT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum VerdictArg {
    Accept,
    Refuse,
}

/// `sill debug launcher-key KEY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LauncherKeyArg {
    Up,
    Down,
    Enter,
    Escape,
    Tab,
    ShiftTab,
    Actions,
    Space,
    QuickLook,
    Right,
    Left,
    Reveal,
}

/// The request `sill debug ACTION` sends.
pub fn debug_request(action: &DebugAction) -> Request {
    Request::Debug(match action {
        DebugAction::Stats => DebugRequest::Stats,
        DebugAction::Pointer { surface, x, y } => DebugRequest::Pointer {
            surface: surface_name(surface),
            at: point(*x, *y),
        },
        DebugAction::Leave { surface } => DebugRequest::Leave {
            surface: surface_name(surface),
        },
        DebugAction::Capture { surface, path } => DebugRequest::Capture {
            surface: surface_name(surface),
            path: path.clone(),
        },
        DebugAction::Click {
            surface,
            x,
            y,
            button,
        } => DebugRequest::Click {
            surface: surface_name(surface),
            at: point(*x, *y),
            button: pointer_button(*button),
        },
        DebugAction::PointerPress {
            surface,
            x,
            y,
            button,
        } => DebugRequest::PointerPress {
            surface: surface_name(surface),
            at: point(*x, *y),
            button: pointer_button(*button),
        },
        DebugAction::PointerRelease {
            surface,
            x,
            y,
            button,
        } => DebugRequest::PointerRelease {
            surface: surface_name(surface),
            at: point(*x, *y),
            button: pointer_button(*button),
        },
        DebugAction::Osd {
            topic,
            percent,
            mute,
        } => DebugRequest::Osd {
            topic: osd_topic(*topic),
            percent: Percent(u16::from(*percent)),
            mute: mute_of(*mute),
        },
        DebugAction::Notify(args) => DebugRequest::Notify(Box::new(posting(args))),
        DebugAction::Screenshot { path } => DebugRequest::Screenshot { path: path.clone() },
        DebugAction::Corner { corner } => DebugRequest::Corner(corner_name(*corner)),
        DebugAction::Auth { verdict } => DebugRequest::Auth(verdict_of(*verdict)),
        DebugAction::Polkit(args) => DebugRequest::Polkit(polkit_request(args)),
        DebugAction::Switcher { apps } => DebugRequest::Switcher {
            apps: apps.iter().cloned().map(AppId).collect(),
        },
        DebugAction::LauncherKey { key } => DebugRequest::LauncherKey(launcher_key(*key)),
        DebugAction::LauncherRows => DebugRequest::LauncherRows,
        DebugAction::IdleWake => DebugRequest::IdleWake,
    })
}

fn surface_name(surface: &str) -> SurfaceName {
    SurfaceName(surface.to_owned())
}

fn point(x: i32, y: i32) -> Point {
    Point { x, y }
}

fn pointer_button(button: ClickButton) -> PointerButton {
    match button {
        ClickButton::Primary => PointerButton::Primary,
        ClickButton::Secondary => PointerButton::Secondary,
        ClickButton::Middle => PointerButton::Middle,
    }
}

fn osd_topic(topic: TopicArg) -> OsdTopic {
    match topic {
        TopicArg::Volume => OsdTopic::Volume,
        TopicArg::Brightness => OsdTopic::Brightness,
    }
}

fn mute_of(mute: MuteArg) -> Mute {
    match mute {
        MuteArg::Audible => Mute::Audible,
        MuteArg::Muted => Mute::Muted,
    }
}

fn corner_name(corner: CornerArg) -> CornerName {
    match corner {
        CornerArg::TopLeft => CornerName::TopLeft,
        CornerArg::TopRight => CornerName::TopRight,
        CornerArg::BottomLeft => CornerName::BottomLeft,
        CornerArg::BottomRight => CornerName::BottomRight,
    }
}

fn verdict_of(verdict: VerdictArg) -> FakeVerdict {
    match verdict {
        VerdictArg::Accept => FakeVerdict::Accept,
        VerdictArg::Refuse => FakeVerdict::Refuse,
    }
}

fn launcher_key(key: LauncherKeyArg) -> LauncherKey {
    match key {
        LauncherKeyArg::Up => LauncherKey::Up,
        LauncherKeyArg::Down => LauncherKey::Down,
        LauncherKeyArg::Enter => LauncherKey::Enter,
        LauncherKeyArg::Escape => LauncherKey::Escape,
        LauncherKeyArg::Tab => LauncherKey::Tab,
        LauncherKeyArg::ShiftTab => LauncherKey::ShiftTab,
        LauncherKeyArg::Actions => LauncherKey::Actions,
        LauncherKeyArg::Space => LauncherKey::Space,
        LauncherKeyArg::QuickLook => LauncherKey::QuickLook,
        LauncherKeyArg::Right => LauncherKey::Right,
        LauncherKeyArg::Left => LauncherKey::Left,
        LauncherKeyArg::Reveal => LauncherKey::Reveal,
    }
}
