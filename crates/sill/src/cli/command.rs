//! What to do: every verb the command line has, one variant per verb, its grammar in the verb
//! group's file.

use clap::Subcommand;

use super::debug::DebugAction;
use super::desktop::{ControlCenterAction, DockAction, PowerMenuAction, ShotArg, WidgetsAction};
use super::display::DisplayAction;
use super::launcher::{ClipboardAction, LauncherAction};
use super::notifications::NotificationsAction;
use super::session::{IdleAction, LockVerb, PolkitAction, SwitcherAction};

/// What to do.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Run the shell (the daemon); exits 0 if one is already running.
    Run(crate::run_args::RunArgs),
    /// Show, hide or seed the launcher.
    Launcher {
        #[command(subcommand)]
        action: LauncherAction,
    },
    /// Pin or unpin a dock app.
    Dock {
        #[command(subcommand)]
        action: DockAction,
    },
    /// Re-read settings, apps and pins.
    Reload,
    /// Stop the daemon.
    Quit,
    /// Is the daemon alive?
    Ping,
    /// Development probes.
    Debug {
        #[command(subcommand)]
        action: DebugAction,
    },
    /// The brightness and volume keys.
    Display {
        #[command(subcommand)]
        action: DisplayAction,
    },
    /// Show or hide the power menu (Log Out, Restart, Shut Down, Suspend).
    PowerMenu {
        #[command(subcommand)]
        action: PowerMenuAction,
    },
    /// Open or close the control center.
    ControlCenter {
        #[command(subcommand)]
        action: ControlCenterAction,
    },
    /// The notification center and the banners.
    Notifications {
        #[command(subcommand)]
        action: NotificationsAction,
    },
    /// Lock the screen now, or say what the lock screen is doing.
    Lock {
        #[arg(value_enum, default_value_t = LockVerb::Now)]
        verb: LockVerb,
    },
    /// The app switcher's chord (COSMIC's window switcher shortcuts run these).
    Switcher {
        #[command(subcommand)]
        action: SwitcherAction,
    },
    /// sill's own polkit agent.
    Polkit {
        #[command(subcommand)]
        action: PolkitAction,
    },
    /// sill's own idle: the dim, the displays off, the lock at screen off and the idle suspend.
    Idle {
        #[command(subcommand)]
        action: IdleAction,
    },
    /// The launcher's clipboard history.
    Clipboard {
        #[command(subcommand)]
        action: ClipboardAction,
    },
    /// The lock screen's PAM helper: reads one request on stdin, checks it, answers on stdout
    /// (the daemon runs it; not for people).
    #[command(hide = true)]
    AuthCheck,
    /// Take a screenshot with `screenshot.tool`; its thumbnail shows at the bottom right.
    Screenshot {
        #[arg(value_enum, default_value_t = ShotArg::Area)]
        kind: ShotArg,
    },
    /// Give the keyboard to the newest transient card (the screenshot thumbnail): Enter opens it,
    /// Delete trashes it, Escape dismisses it. Bind it in COSMIC's shortcuts.
    FocusLatest,
    /// The widgets.
    Widgets {
        #[command(subcommand)]
        action: WidgetsAction,
    },
}
