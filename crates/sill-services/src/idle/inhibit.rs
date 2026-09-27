//! Idle inhibitors that are sill's to honour (sill FINDINGS "sill idle (Q420 B)", F905). Wayland's
//! `zwp_idle_inhibit_manager_v1` is the compositor's: cosmic-comp stops sending `idled` for
//! `get_idle_notification` while a visible surface inhibits, so sill sees nothing of it. Two
//! other kinds are sill's:
//!
//! - **`org.freedesktop.ScreenSaver.Inhibit(app, reason) -> cookie`** and `UnInhibit(cookie)` on
//!   the session bus (at `/org/freedesktop/ScreenSaver` and `/ScreenSaver`), which browsers and
//!   video players call and which cosmic-idle served. An inhibit also ends when its caller's
//!   unique name leaves the bus.
//! - **logind's idle inhibitors** (`systemd-inhibit --what=idle`), read from the Manager's
//!   `BlockInhibited` property (a `:`-separated list that contains `idle`).
//!
//! While either holds, sill drops its idle notifications (no dim, no screen off, no idle
//! suspend), as cosmic-idle did, and holds them again when both are free.

/// A cookie `Inhibit` handed out. Never zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InhibitCookie(pub u32);

/// A D-Bus caller's unique name (`:1.42`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BusPeer(pub String);

/// One held `ScreenSaver` inhibit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Inhibitor {
    pub cookie: InhibitCookie,
    pub peer: BusPeer,
    /// The `application_name` argument, for the log and a later status.
    pub app: String,
    pub reason: String,
}

/// Every held `ScreenSaver` inhibit, and the last cookie given.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inhibitors {
    pub held: Vec<Inhibitor>,
    pub last: u32,
}

/// What changes the held set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InhibitMsg {
    Inhibit {
        peer: BusPeer,
        app: String,
        reason: String,
    },
    UnInhibit {
        peer: BusPeer,
        cookie: InhibitCookie,
    },
    /// `NameOwnerChanged` with an empty new owner.
    PeerGone(BusPeer),
}

/// What the D-Bus side answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InhibitReply {
    /// `Inhibit`'s return value.
    Cookie(InhibitCookie),
    /// `UnInhibit` of a held cookie, or a peer's departure.
    Done,
    /// `UnInhibit` of a cookie that peer does not hold (ignored, as the spec's servers do).
    Unknown,
}

/// Whether idle is held off by an inhibitor sill honours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Inhibition {
    #[default]
    Free,
    Held,
}

/// Whether logind's `BlockInhibited` names `idle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LogindIdle {
    #[default]
    Free,
    Blocked,
}

/// `inhibitors` after `msg`, and the reply. Cookies count up from 1 and skip 0 on wrap; an
/// `UnInhibit` only releases a cookie its own peer holds.
pub fn inhibit_step(inhibitors: Inhibitors, msg: InhibitMsg) -> (Inhibitors, InhibitReply) {
    let _ = (inhibitors, msg);
    todo!("idle fill: F905")
}

/// `BlockInhibited`'s text (`"handle-power-key:idle:sleep"`) read as [`LogindIdle`].
pub fn logind_idle(block_inhibited: &str) -> LogindIdle {
    let _ = block_inhibited;
    todo!("idle fill: F905")
}

/// Held when any `ScreenSaver` inhibit is held or logind blocks idle.
pub fn inhibition(inhibitors: &Inhibitors, logind: LogindIdle) -> Inhibition {
    let _ = (inhibitors, logind);
    todo!("idle fill: F905")
}

#[cfg(test)]
#[path = "inhibit_tests.rs"]
mod tests;
