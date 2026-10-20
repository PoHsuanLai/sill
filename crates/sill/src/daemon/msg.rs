//! The daemon's host message: what reaches `Host::run`'s event handler from other threads.

use std::sync::mpsc::SyncSender;

use ds_settings::{Environment, UserStyle};
use sill_ipc::request::Request;
use sill_ipc::response::Response;
use sill_settings::ShellFile;

/// One message to the host thread (`HostHandle::send`).
#[derive(Debug)]
pub enum ShellMsg {
    /// An IPC request; answer on `reply` within the IPC server's timeout.
    Ipc {
        request: Request,
        reply: SyncSender<Response>,
    },
    /// An action a surface or a corner asked for (`crate::actions::spawn_asks` sends it).
    Action(sill_ui_kit::links::Asked),
    /// `settings.toml` settled on a new value.
    Settings(Box<ShellFile>),
    /// `style.css` settled on a new text (empty when the file is gone).
    UserStyle(Box<UserStyle>),
    /// The appearance (`appearance.toml` and the portal) settled on a new value.
    Appearance(Box<Environment>),
    /// Hold or drop shell-host's session lock (`crate::lock::spawn_follower` sends it).
    Lock(crate::lock::LockWish),
    /// Map or unmap the idle dim surfaces (`crate::idle_dim::spawn_follower` sends it).
    Dim(sill_model::idle::DimShown),
    /// A source of questions changed; the sheets' latch looks again (`crate::sheet_latch`).
    Sheets,
    /// Open, switch or close the launcher for a summon (`crate::companion::spawn_follower`
    /// sends it).
    Summon(sill_model::companion::SummonOut),
    /// One note of the desktop's own compositor's shell protocol (`compositor::ShellSeat`'s
    /// forwarder sends it).
    Note(sill_model::desktop::ShellNote),
    /// Where the docks' icons rest now (`targets::forward` sends it).
    DockRects(sill_model::desktop::target::OutputRects),
    /// A command for the compositor's arm (`crate::outgoing` sends a pick of a
    /// window's menu).
    Command(sill_model::desktop::DesktopCommand),
    /// The desktop's view changed (`targets::nudge` sends it); the seat reads the view it holds.
    Desktop,
}
