//! What to do: every verb the command line has, one variant per verb, its grammar in the verb
//! group's file.

use clap::Subcommand;

use super::debug::DebugAction;
use super::desktop::{
    CompanionAction, ControlCenterAction, DockAction, ForceQuitAction, PowerMenuAction, ShotArg,
    VoiceAction, WidgetsAction,
};
use super::display::DisplayAction;
use super::intents::IntentsAction;
use super::keycap::KeycapAction;
use super::launch::LaunchArgs;
use super::launcher::{ClipboardAction, LauncherAction};
use super::notifications::NotificationsAction;
use super::session::{IdleAction, LockVerb, PolkitAction, SwitcherAction, WorkspaceAction};
use super::style::StyleAction;

/// What to do.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Run the shell (the daemon); exits 0 if one is already running.
    Run(crate::run_args::RunArgs),
    /// Open an installed app the way a click in the dock does.
    Launch(LaunchArgs),
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
    /// Show or hide the Force Quit Applications panel.
    ForceQuit {
        #[command(subcommand)]
        action: ForceQuitAction,
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
    /// The app switcher's chord (the window switcher shortcuts run these).
    Switcher {
        #[command(subcommand)]
        action: SwitcherAction,
    },
    /// Step the workspace (palmrest's swipes run these).
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
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
    /// The Mac keyboard layout: whether keycap runs, restart it, or end its pause.
    Keycap {
        #[command(subcommand)]
        action: KeycapAction,
    },
    /// What intentd may ask of sill: its confirmation sheet, and the terminal's standing grants.
    Intents {
        #[command(subcommand)]
        action: IntentsAction,
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
    /// Give the keyboard to the newest transient card, the screenshot thumbnail or a notification
    /// banner, whichever appeared last: Enter opens it, Delete trashes the shot or closes the
    /// banner, Escape dismisses it, Up and Down move between banners. Bind it in the compositor's shortcuts.
    FocusLatest,
    /// The widgets.
    Widgets {
        #[command(subcommand)]
        action: WidgetsAction,
    },
    /// The assistant: summon it, put it away, or pause what it is doing.
    Companion {
        #[command(subcommand)]
        action: CompanionAction,
    },
    /// Hold-to-talk: the edges of the hold, for a key source that detects it itself.
    Voice {
        #[command(subcommand)]
        action: VoiceAction,
    },
    /// Your own stylesheet, `style.css`.
    Style {
        #[command(subcommand)]
        action: StyleAction,
    },
}
