//! What a client asks the daemon: one [`Request`] per connection.

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;
use sill_model::actions::Ask;
use sill_model::intents::IntentsAsk;
use sill_model::keycap::KeycapAsk;

use crate::act::DisplayRequest;
use crate::debug::DebugRequest;
use crate::show::{CompanionRequest, LauncherRequest, Reveal, VoiceRequest};
use crate::status::StatusRequest;

/// One request. A connection carries exactly one, then its [`crate::Response`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Request {
    /// Is the daemon alive? Answered with `Response::Pong`.
    Ping,
    /// Close every surface and exit the daemon.
    Quit,
    /// Re-read the settings without restarting.
    Reload,
    /// Do an action of the shell: lock, a screenshot, a panel, a step of the switcher, a pin
    /// (`sill lock`, `sill screenshot`, `sill launcher toggle`, ...). One request for every
    /// action; what each is lives in `sill_model::actions`.
    Do(Ask),
    /// Open, close or seed the launcher (`sill launcher {open,close}`; `toggle` is an action).
    Launcher(LauncherRequest),
    /// The power menu, opened, closed or flipped (`sill power-menu {open,close,toggle}`). An
    /// action only opens or flips a panel, so a close needs its own word.
    PowerMenu(Reveal),
    /// The Force Quit Applications panel (`sill force-quit {open,close,toggle}`), on the output
    /// of the active window.
    ForceQuit(Reveal),
    /// The notification center (`sill notifications {open,close}`).
    NotificationCenter(Reveal),
    /// The companion's summon (`sill companion {tap,summon,dismiss,stop-all}`).
    Companion(CompanionRequest),
    /// Hold-to-talk (`sill voice {hold-start,hold-end,hold-cancel}`).
    Voice(VoiceRequest),
    /// Read a state: answered with `Response::Status`.
    Status(StatusRequest),
    /// The brightness and volume keys (`sill display {brightness,volume} ...`), bound to
    /// XF86MonBrightness*/XF86Audio* in the compositor's shortcuts.
    Display(DisplayRequest),
    /// Forget every clipboard entry now (`sill clipboard clear`).
    ClearClipboard,
    /// Dismiss every banner on screen (each sender hears `NotificationClosed`, reason 2).
    DismissNotifications,
    /// Give the keyboard to the newest transient card: the screenshot thumbnail or the newest
    /// banner, whichever appeared last (`sill focus-latest`; bound in the compositor's shortcuts
    /// like the launcher's). Nothing happens when none is showing.
    FocusLatest,
    /// Restart keycap, or end its pause (`sill keycap retry|resume`).
    Keycap(KeycapAsk),
    /// Withdraw a standing terminal grant, or let a paused terminal go on (`sill intents
    /// revoke|resume`).
    Intents(IntentsAsk),
    /// Open an installed app the way a click in the dock does (`sill launch <desktop-id>`):
    /// answered `Response::Launched` once the app's process exists, or `Failed`.
    Launch(AppId),
    /// Development probes (`dev/accept/*.sh`); answered even in a release build, acted on only
    /// when the daemon was built with the `debug` feature.
    Debug(DebugRequest),
}
