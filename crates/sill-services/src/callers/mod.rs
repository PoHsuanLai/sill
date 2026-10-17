//! Who may call the shell's bus objects, and whose menu the bar believes (SILLS-07, QUIRE-12).
//!
//! Owning a well-known name proves nothing on the session bus: any process may claim a name that
//! nobody owns yet. Every check here asks the system who the caller is ([`peephole_bus`]: the
//! process behind the connection, held by a pidfd, its unit as the unit manager states it, judged
//! against the root-owned table at `/etc/quire/desktop-callers.toml`) and compares that with the
//! role the object answers:
//!
//! | object | who may call |
//! |---|---|
//! | `org.quire.Shell1` (the picker, `AskPermission`) | the portal backend (role `portal`) |
//! | `org.quire.AccountsSheet1` | accountd, inferd or syncd (roles `accounts`, `inference`, `sync`) |
//! | `org.quire.Confirm1` | intentd (role `router`) |
//! | `org.quire.Voice1.Utterance.Ended` (a signal to the shell), and the ask before `Begin` | voiced (role `voice`, owner of `org.quire.Voice1`) |
//!
//! and the caller must also be the owner of the well-known name of the service whose role it
//! holds (`admit`): a sync service that is not the owner of `org.quire.Sync1` is a stranger.
//! A caller that passes is known by its unique name, which is the only place an answer is sent to.
//! A refusal is `AccessDenied`, with words that do not say which check failed. A signal has no
//! one to answer, so a refused signal's sender is said once and then skipped unchecked
//! (`SignalSenders`).
//!
//! `menu_shown` is the same question about the owner of an app's menu name: who it is, compared
//! with the window's program (`sill_model::app_menu::menu_trust`) before the app-menu service
//! reads, shows or clicks its menu.

mod claim;
#[cfg(feature = "dev-callers")]
mod dev;
mod menu;
mod signal;
mod source;

pub(crate) use claim::{Claim, admit};
pub(crate) use menu::menu_shown;
pub(crate) use signal::SignalSenders;
pub use source::{CallerSource, DesktopCallers, Peers};
