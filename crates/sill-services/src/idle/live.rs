//! The idle service's loop (sill FINDINGS "sill idle (Q420 B)", F903-F906). **A freeze stub**:
//! it returns at once, so `Services::spawn` starts nothing and every `Control::send` answers
//! `Gone`. The fill:
//!
//! - stays passive (no Wayland connection, no bus name, no logind call) until the first
//!   `Configure` or `Use`, and again whenever the policy's mode is `Off`;
//! - on `Configure`: `resolve_mode(Host::of(..), cosmic-idle running?)` and, for
//!   `TimesChoice::Cosmic`, `cosmic_times` over the user's and the system's config directories,
//!   re-read when a file there changes;
//! - binds `ext_idle_notifier_v1` (v2) and `zwlr_output_power_manager_v1` on `Setup.display`,
//!   one `zwlr_output_power_v1` per `wl_output` as outputs come and go;
//! - owns `org.freedesktop.ScreenSaver` on `Setup.session` while `Own` (released when `Off`),
//!   and on `Setup.system` reads logind's `BlockInhibited` and `PrepareForSleep` and calls
//!   `Suspend(false)` and `SetIdleHint`;
//! - follows `IdleLinks.lock` and `IdleLinks.battery` into `IdleEvent::{Lock, Power}`;
//! - steps `reduce` and performs its effects in order, publishing [`super::Idle`].

use tokio::sync::{mpsc, watch};

use super::model::{Idle, IdleCommand, IdleLinks, Setup};

/// Runs until `commands` closes (the fill); the freeze returns at once.
pub(super) async fn run(
    setup: Setup,
    links: IdleLinks,
    commands: mpsc::UnboundedReceiver<IdleCommand>,
    tx: watch::Sender<Idle>,
) {
    let _ = (setup, links, commands, tx);
}
