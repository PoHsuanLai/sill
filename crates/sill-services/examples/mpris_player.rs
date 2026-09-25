//! A fake MPRIS player for exercising Now Playing without a real one:
//!
//! ```sh
//! cargo run -p sill-services --example mpris_player -- <suffix> \
//!     [--title T] [--artist A] [--album AL] [--paused]
//! ```
//!
//! Claims `org.mpris.MediaPlayer2.<suffix>` on the session bus (a unique suffix per run, so
//! parallel runs and a real player never collide), implements `org.mpris.MediaPlayer2` and
//! `.Player` enough for `now_playing::live` to find it, and answers `PlayPause` by toggling
//! `PlaybackStatus` and emitting `PropertiesChanged`. Runs until killed.

use std::collections::HashMap;
use std::process::ExitCode;

use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYING: &str = "Playing";
const PAUSED: &str = "Paused";

#[tokio::main]
async fn main() -> ExitCode {
    let Some(config) = Config::from_args(std::env::args().skip(1)) else {
        eprintln!("usage: mpris_player <suffix> [--title T] [--artist A] [--album AL] [--paused]");
        return ExitCode::FAILURE;
    };

    let name = format!("org.mpris.MediaPlayer2.{}", config.suffix);
    let root = Root {
        identity: format!("sill test player ({})", config.suffix),
    };
    let player = Player {
        playback: if config.paused { PAUSED } else { PLAYING },
        metadata: config.metadata(),
        position_us: 0,
    };

    let build = async {
        let builder = zbus::connection::Builder::session()?
            .name(name.clone())?
            .serve_at(PATH, root)?
            .serve_at(PATH, player)?;
        builder.build().await
    };
    let connection = match build.await {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("mpris_player: {error}");
            return ExitCode::FAILURE;
        }
    };

    eprintln!("mpris_player: {name} up at {PATH} until killed");
    std::future::pending::<()>().await;
    drop(connection);
    ExitCode::SUCCESS
}

/// `org.mpris.MediaPlayer2`: just enough for `now_playing::mpris::MediaPlayer2Proxy`
/// (`Identity`).
struct Root {
    identity: String,
}

#[interface(interface = "org.mpris.MediaPlayer2")]
impl Root {
    #[zbus(property)]
    fn identity(&self) -> String {
        self.identity.clone()
    }
}

/// `org.mpris.MediaPlayer2.Player`: a fixed track, a playback status that flips on `PlayPause`,
/// and every ability on (so a live test can press every verb).
struct Player {
    playback: &'static str,
    metadata: HashMap<String, OwnedValue>,
    position_us: i64,
}

#[interface(interface = "org.mpris.MediaPlayer2.Player")]
impl Player {
    async fn play_pause(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.playback = if self.playback == PLAYING {
            PAUSED
        } else {
            PLAYING
        };
        eprintln!("mpris_player: PlayPause -> {}", self.playback);
        let _ = self.playback_status_changed(&emitter).await;
    }

    async fn next(&self) {
        eprintln!("mpris_player: Next");
    }

    async fn previous(&self) {
        eprintln!("mpris_player: Previous");
    }

    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.playback.to_owned()
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        self.metadata.clone()
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        self.position_us
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(signal)]
    async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;
}

/// This run's fixed track, parsed once from argv.
struct Config {
    suffix: String,
    title: String,
    artist: String,
    album: String,
    paused: bool,
}

impl Config {
    /// Parses `<suffix> [--title T] [--artist A] [--album AL] [--paused]`; `None` on a missing
    /// suffix, an unrecognised flag or a flag missing its value.
    fn from_args(mut args: impl Iterator<Item = String>) -> Option<Config> {
        let suffix = args.next()?;
        let mut title = "Test Track".to_owned();
        let mut artist = "Test Artist".to_owned();
        let mut album = "Test Album".to_owned();
        let mut paused = false;
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--title" => title = args.next()?,
                "--artist" => artist = args.next()?,
                "--album" => album = args.next()?,
                "--paused" => paused = true,
                _ => return None,
            }
        }
        Some(Config {
            suffix,
            title,
            artist,
            album,
            paused,
        })
    }

    /// The `Metadata` dictionary MPRIS clients read: a title, an artist, an album and a fixed
    /// three-minute length, enough for `now_playing::live::track_of` and `timeline_of`.
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let track_id = ObjectPath::from_static_str_unchecked("/org/sill/MprisPlayer/track1");
        let mut map = HashMap::new();
        map.insert("mpris:trackid".to_owned(), OwnedValue::from(track_id));
        map.insert(
            "xesam:title".to_owned(),
            owned(Value::from(self.title.clone())),
        );
        map.insert(
            "xesam:artist".to_owned(),
            owned(Value::from(vec![self.artist.clone()])),
        );
        map.insert(
            "xesam:album".to_owned(),
            owned(Value::from(self.album.clone())),
        );
        // Three minutes, in microseconds (MPRIS's unit for `mpris:length`).
        map.insert("mpris:length".to_owned(), OwnedValue::from(180_000_000_i64));
        map
    }
}

/// Owns `value`: infallible for the `Str` and `Array<Str>` shapes this file ever builds (the
/// only failure mode, an unclonable file descriptor, cannot occur here).
fn owned(value: Value<'_>) -> OwnedValue {
    OwnedValue::try_from(value).expect("no file descriptor to clone")
}
