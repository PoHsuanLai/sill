//! Screenshots: noticing a new one, so the floating thumbnail can show it (design/20 §1.13), and
//! taking one on `sill screenshot`.
//!
//! How sill learns a screenshot happened (researched): the desktop portal
//! (`org.freedesktop.portal.Screenshot`) saves the image as `Screenshot_%Y-%m-%d_%H-%M-%S.png`
//! in the Pictures folder, or a tool is told a folder to save into. Neither announces the file
//! on the bus: the portal's `Response` goes to the caller only. So the one route that sees
//! every screenshot, whoever took it (the Print key, `sill screenshot`, spectacle, grim), is
//! **an inotify watch of the screenshot folders** (`notify`, already a dependency): a new file
//! whose name reads as a screenshot (`names::is_screenshot_name`), seen when it is closed after
//! writing or renamed into place, becomes the current `Shot`. `sill screenshot
//! [area|window|screen]` only runs the tool (`tool::tool_command`); its file arrives through
//! the same watch, so there is one path to test. A clipboard-only screenshot has no file and no
//! thumbnail, as on macOS.
//!
//! The service is passive until the daemon's first [`ShotCommand::Configure`] (from
//! `screenshot.folders` and `screenshot.tool`): a test or a headless render never watches
//! `~/Pictures` or runs a tool. Tests configure temp folders and a stand-in tool.

mod backend;

use sill_model::screenshots::{ShotCommand, Shots};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// Screenshots as a [`Service`]: watching folders under `env`'s directories, its previews in their
/// cache. Passive until `Configure`.
#[derive(Debug)]
pub enum ScreenshotsService {}

impl Service for ScreenshotsService {
    const NAME: &'static str = "screenshots";
    type State = Shots;
    type Command = ShotCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let dirs = env.dirs.clone();
        let children = env.children.clone();
        async move {
            backend::run(io.state, io.commands, dirs, children).await;
            Ok(())
        }
    }
}
