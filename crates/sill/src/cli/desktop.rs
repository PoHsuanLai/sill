//! `sill screenshot [area|window|screen]`, `sill widgets toggle-desktop`, `sill debug screenshot
//! PATH` and `sill debug corner CORNER`, and their pure mappings to IPC requests.

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};
use sill_ipc::{
    CornerName, FakeCorner, FakeScreenshot, ScreenshotRequest, ShotArea, WidgetsRequest,
};

/// `sill screenshot KIND`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShotArg {
    /// A rectangle you drag out (COSMIC's picker).
    Area,
    /// One window (COSMIC's picker).
    Window,
    /// The whole screen, at once.
    Screen,
}

/// `sill widgets ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum WidgetsAction {
    /// Show the desktop widgets when hidden, hide them when shown.
    ToggleDesktop,
    /// Open Edit Widgets, the widget gallery.
    Edit,
    /// Close Edit Widgets.
    EditClose,
}

/// `sill debug corner CORNER`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CornerArg {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// The request `sill screenshot KIND` sends.
pub fn screenshot_request(kind: ShotArg) -> ScreenshotRequest {
    ScreenshotRequest::Take(match kind {
        ShotArg::Area => ShotArea::Area,
        ShotArg::Window => ShotArea::Window,
        ShotArg::Screen => ShotArea::Screen,
    })
}

/// The request `sill widgets ACTION` sends.
pub fn widgets_request(action: WidgetsAction) -> WidgetsRequest {
    match action {
        WidgetsAction::ToggleDesktop => WidgetsRequest::ToggleDesktop,
        WidgetsAction::Edit => WidgetsRequest::Edit,
        WidgetsAction::EditClose => WidgetsRequest::EditClose,
    }
}

/// `sill debug screenshot PATH`'s made-up screenshot.
pub fn fake_screenshot(path: PathBuf) -> FakeScreenshot {
    FakeScreenshot { path }
}

/// `sill debug corner CORNER`'s made-up hit.
pub fn fake_corner(corner: CornerArg) -> FakeCorner {
    FakeCorner {
        corner: match corner {
            CornerArg::TopLeft => CornerName::TopLeft,
            CornerArg::TopRight => CornerName::TopRight,
            CornerArg::BottomLeft => CornerName::BottomLeft,
            CornerArg::BottomRight => CornerName::BottomRight,
        },
    }
}
