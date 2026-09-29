//! Hand-written `zbus` proxies for `org.mpris.MediaPlayer2` and `.Player` (the `mpris` crate runs
//! on the `dbus` crate and libdbus, a second D-Bus stack beside every proxy elsewhere in this
//! crate). No `default_service`: each player owns a different
//! bus name, so every proxy is built with `.destination(name)`.

use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::OwnedValue;

/// `org.mpris.MediaPlayer2.Player`'s interface name, as it appears in a `PropertiesChanged`
/// signal's first argument.
pub(crate) const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

#[proxy(
    interface = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub(crate) trait MediaPlayer2 {
    #[zbus(property)]
    fn identity(&self) -> zbus::Result<String>;
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub(crate) trait Player {
    fn play_pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;
    fn previous(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedValue>>;
    #[zbus(property)]
    fn position(&self) -> zbus::Result<i64>;
    #[zbus(property)]
    fn can_play(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn can_pause(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn can_go_next(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn can_go_previous(&self) -> zbus::Result<bool>;
}
