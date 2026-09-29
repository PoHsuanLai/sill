//! Now Playing: every MPRIS player on the session bus (`org.mpris.MediaPlayer2.*`) and which one
//! the control center shows (design/20 §1.5, design/13 §13.3.7: the Now Playing module).
//!
//! Spoken through hand-written `zbus` proxies for `org.mpris.MediaPlayer2` and
//! `org.mpris.MediaPlayer2.Player`, found by `ListNames` and followed by `NameOwnerChanged`
//! (the `mpris` crate runs on the `dbus` crate and libdbus). Album art reaches the surface only as
//! a local file: a `file://` URL is used as it is, an `http(s)://`
//! one is downloaded once into `$XDG_CACHE_HOME/sill/art/` (`sill_model::now_playing::art_source`), since quire's `IconUrl`
//! takes `file:` and `data:` only.

mod backend;

use sill_model::now_playing::{Media, MediaCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// Now Playing (MPRIS) as a [`Service`]. Without a session bus the state stays at
/// `Media::default()` (no players), and the module shows nothing playing.
#[derive(Debug)]
pub enum NowPlayingService {}

impl Service for NowPlayingService {
    const NAME: &'static str = "now_playing";
    type State = Media;
    type Command = MediaCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(
            env.buses.session.clone(),
            env.dirs.sill_cache().join("art"),
            io.state,
            io.commands,
        )
    }
}
