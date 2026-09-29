//! The launcher's command line: `sill clipboard status|clear`, `sill debug launcher-key KEY` and
//! `sill debug launcher-rows`, and their pure mappings to IPC requests.

use clap::{Subcommand, ValueEnum};
use sill_ipc::{ClipboardRequest, FakeLauncherKey, Request};

/// `sill clipboard ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum ClipboardAction {
    /// Whether the clipboard history is kept, and how many entries it holds (never what).
    Status,
    /// Forget every entry now.
    Clear,
}

/// The request `sill clipboard ACTION` sends.
pub fn clipboard_request(action: ClipboardAction) -> Request {
    Request::Clipboard(match action {
        ClipboardAction::Status => ClipboardRequest::Status,
        ClipboardAction::Clear => ClipboardRequest::Clear,
    })
}

/// `sill debug launcher-key KEY`: the command line's spelling of [`FakeLauncherKey`].
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

/// The IPC key for `key`.
pub fn launcher_key(key: LauncherKeyArg) -> FakeLauncherKey {
    match key {
        LauncherKeyArg::Up => FakeLauncherKey::Up,
        LauncherKeyArg::Down => FakeLauncherKey::Down,
        LauncherKeyArg::Enter => FakeLauncherKey::Enter,
        LauncherKeyArg::Escape => FakeLauncherKey::Escape,
        LauncherKeyArg::Tab => FakeLauncherKey::Tab,
        LauncherKeyArg::ShiftTab => FakeLauncherKey::ShiftTab,
        LauncherKeyArg::Actions => FakeLauncherKey::Actions,
        LauncherKeyArg::Space => FakeLauncherKey::Space,
        LauncherKeyArg::QuickLook => FakeLauncherKey::QuickLook,
        LauncherKeyArg::Right => FakeLauncherKey::Right,
        LauncherKeyArg::Left => FakeLauncherKey::Left,
        LauncherKeyArg::Reveal => FakeLauncherKey::Reveal,
    }
}
