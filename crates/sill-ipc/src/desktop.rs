//! M10's requests (sill FINDINGS "M10 freeze"): `sill screenshot`, `sill widgets`, and the
//! debug requests that fake a screenshot and a hot corner's hit.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// `sill screenshot [area|window|screen]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ScreenshotRequest {
    /// Run `screenshot.tool` for this kind; the file it saves shows as the thumbnail through the
    /// folder watch. Answered `Done` once the tool is started.
    Take(ShotArea),
}

/// What a screenshot covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShotArea {
    /// A rectangle the person drags out.
    #[default]
    Area,
    /// One window.
    Window,
    /// The whole screen, at once.
    Screen,
}

/// `sill widgets ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WidgetsRequest {
    /// Flip `widgets.desktop` in `settings.toml` (the file stays the one truth; the daemon's
    /// watch picks the change up).
    ToggleDesktop,
    /// Open Edit Widgets (the widget gallery) on the first output that has its surface.
    Edit,
    /// Close Edit Widgets wherever it is open.
    EditClose,
}

/// `sill debug screenshot PATH`: show the thumbnail for `path` as if the folder watch had seen
/// it, without taking a screenshot of anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FakeScreenshot {
    pub path: PathBuf,
}

/// A screen corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CornerName {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// `sill debug corner CORNER`: carry out `corner`'s action on the first output, as if its dwell
/// had just run out (the trigger machine is proven by its own tests and by real input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FakeCorner {
    pub corner: CornerName,
}
