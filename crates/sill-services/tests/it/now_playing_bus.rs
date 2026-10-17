//! Now Playing over a private `dbus-daemon`, never the session bus: a media player that never
//! answers is given up on after its deadline and the players behind it are still listed, and a
//! player's words are cut to a card's size. Without a `dbus-daemon` the test says so and skips.

use std::collections::HashMap;
use std::time::Duration;

use sill_model::now_playing::{Media, TEXT_MAX_CHARS};
use sill_services::NowPlayingService;
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::service::spawn;
use sill_testkit::{PrivateBus, TempDir};
use zbus::interface;
use zbus::zvariant::{OwnedValue, Value};

use crate::notifications_bus::wait_for;

const PATH: &str = "/org/mpris/MediaPlayer2";

/// How long the whole read may take: a hung player's deadline, with room for the rest.
const READ: Duration = Duration::from_secs(20);

/// The player's root object; `identity` is what it says, or never answers.
struct Root {
    identity: Option<String>,
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    #[zbus(property)]
    async fn identity(&self) -> String {
        match &self.identity {
            Some(identity) => identity.clone(),
            None => std::future::pending().await,
        }
    }
}

/// The player object of a player playing `title`.
struct Player {
    title: String,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    #[zbus(property)]
    fn playback_status(&self) -> String {
        "Playing".to_owned()
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let title = Value::from(self.title.as_str())
            .try_into_owned()
            .unwrap_or_else(|e| panic!("{e}"));
        HashMap::from([("xesam:title".to_owned(), title)])
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        0
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

    fn play_pause(&self) {}

    fn next(&self) {}

    fn previous(&self) {}
}

/// A player on `bus` under `name`: it answers, or (`identity` none) never answers about itself.
async fn serve_player(
    bus: &PrivateBus,
    name: &str,
    identity: Option<&str>,
    title: &str,
) -> zbus::Connection {
    zbus::connection::Builder::address(bus.address.as_str())
        .and_then(|builder| builder.name(name.to_owned()))
        .and_then(|builder| {
            builder.serve_at(
                PATH,
                Root {
                    identity: identity.map(str::to_owned),
                },
            )
        })
        .and_then(|builder| {
            builder.serve_at(
                PATH,
                Player {
                    title: title.to_owned(),
                },
            )
        })
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("serving {name}: {e}"))
}

#[test]
fn a_player_that_never_answers_does_not_hide_the_others_and_long_titles_are_cut() {
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("now-playing");
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(scratch.path())
    };
    let long_title = "t".repeat(TEXT_MAX_CHARS * 10);
    let _players = runtime.block_on(async {
        (
            serve_player(&bus, "org.mpris.MediaPlayer2.hung", None, "never").await,
            serve_player(
                &bus,
                "org.mpris.MediaPlayer2.good",
                Some("Good"),
                &long_title,
            )
            .await,
        )
    });
    let mut service = spawn::<NowPlayingService>(runtime.handle(), &env, ());
    let listed = runtime.block_on(wait_for(&mut service.state, READ, |media: &Media| {
        !media.players.is_empty()
    }));
    assert!(listed, "no player was listed: the hung one held the read");
    let media = service.state.borrow().clone();
    let ids: Vec<&str> = media.players.iter().map(|p| p.id.0.as_str()).collect();
    assert_eq!(
        ids,
        ["org.mpris.MediaPlayer2.good"],
        "the hung one is not listed"
    );
    let title = &media.players[0].track.title;
    assert!(
        title.chars().count() <= TEXT_MAX_CHARS + 1,
        "the title keeps {} characters",
        title.chars().count()
    );
}
