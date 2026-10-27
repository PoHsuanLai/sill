//! What to do: every verb the command line has, one variant per verb, its grammar in the verb
//! group's file.

use clap::Subcommand;
use ds_core::word::Word;
use sill_ipc::act::{SwitcherRequest, WorkspaceRequest};
use sill_ipc::show::{CompanionRequest, Reveal, VoiceRequest};
use sill_model::screenshots::ShotKind;

use super::action::DoArgs;
use super::debug::DebugAction;
use super::desktop::{ControlCenterAction, DockAction};
use super::display::DisplayAction;
use super::intents::IntentsAction;
use super::keycap::KeycapAction;
use super::launch::LaunchArgs;
use super::launcher::{ClipboardAction, LauncherAction};
use super::notifications::NotificationsAction;
use super::session::{IdleAction, LockVerb, PolkitAction};
use super::style::StyleAction;
use super::word::word;

/// What to do.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Run the shell (the daemon); exits 0 if one is already running.
    Run(crate::run_args::RunArgs),
    /// Open an installed app the way a click in the dock does.
    Launch(LaunchArgs),
    /// Do any action of the shell by its id, as a menu or a chord would.
    Do(DoArgs),
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
        /// Open it, close it, or flip it.
        #[arg(value_parser = word::<Reveal>())]
        action: Reveal,
    },
    /// Show or hide the Force Quit Applications panel.
    ForceQuit {
        /// Open it, close it, or flip it.
        #[arg(value_parser = word::<Reveal>())]
        action: Reveal,
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
        /// `next` is the chord (Cmd+Tab: arm the switcher, or move the selection on), `prev` the
        /// reverse chord (Cmd+Shift+Tab); `cancel` closes it and switches to nothing, `commit`
        /// switches to the selection and closes it.
        #[arg(value_parser = word::<SwitcherRequest>())]
        action: SwitcherRequest,
    },
    /// Step the workspace (palmrest's swipes run these).
    Workspace {
        /// `prev` and `next` go to the workspace before or after the active one; `overview` shows
        /// every window and workspace at once (Mission Control; fails on a session that is not
        /// ours).
        #[arg(value_parser = word::<WorkspaceRequest>())]
        action: WorkspaceRequest,
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
    /// A verb the compositor sends, read by `shell_verbs::ShellVerb::parse` (not a clap word:
    /// the same words are also clap's grammar above, which has to agree with it).
    #[command(skip)]
    Verb(shell_verbs::ShellVerb),
    /// Take a screenshot with the portal; its thumbnail shows at the bottom right.
    Screenshot {
        /// `area` is a rectangle you drag out and `window` one window (both the picker); `screen`
        /// is the whole screen, at once.
        #[arg(value_parser = word::<ShotKind>(), default_value = ShotKind::Area.slug())]
        kind: ShotKind,
    },
    /// Give the keyboard to the newest transient card, the screenshot thumbnail or a notification
    /// banner, whichever appeared last: Enter opens it, Delete trashes the shot or closes the
    /// banner, Escape dismisses it, Up and Down move between banners. Bind it in the compositor's shortcuts.
    FocusLatest,
    /// Bring forward the next window of the front app (Command+` on a Mac).
    NextWindow,
    /// The assistant: summon it, put it away, or pause what it is doing.
    Companion {
        /// `tap` is one tap of the double-tap key (the second within the double-tap window
        /// summons); `summon` summons now (a double tap already detected); `dismiss` puts the
        /// assistant away; `stop-all` pauses every run in the focused Space.
        #[arg(value_parser = word::<CompanionRequest>())]
        action: CompanionRequest,
    },
    /// Hold-to-talk: the edges of the hold, for a key source that detects it itself.
    Voice {
        /// `hold-start`: the hold began, open the assistant and listen; `hold-end`: the key was let
        /// go, finish and send what was said; `hold-cancel`: another input spoiled the hold,
        /// discard what was said.
        #[arg(value_parser = word::<VoiceRequest>())]
        action: VoiceRequest,
    },
    /// Your own stylesheet, `style.css`.
    Style {
        #[command(subcommand)]
        action: StyleAction,
    },
}
