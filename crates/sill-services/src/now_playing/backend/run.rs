//! MPRIS over the session bus: finds every `org.mpris.MediaPlayer2.*` name, follows their
//! `PropertiesChanged` and `Seeked`, brings cover art into the cache, feeds every change
//! through [`sill_model::now_playing::step`], and carries out [`MediaCommand`]s.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use ds_core::time::stamp::Stamp;
use sill_model::now_playing::{
    Abilities, Art, Media, MediaCommand, MediaEvent, MediaPlayer, MediaVerb, Micros, Playback,
    PlayerId, Timeline, Track, step,
};
use sill_model::units::Availability;
use tokio::sync::{mpsc, watch};
use zbus::fdo::{DBusProxy, NameOwnerChanged};
use zbus::message::{Message, Type as MessageType};
use zbus::names::{BusName, OwnedUniqueName};
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, MessageStream};

use super::art::{ArtFetcher, ArtJob, apply_art, resolve_art};
use super::mpris::{MediaPlayer2Proxy, PLAYER_INTERFACE, PlayerProxy};
use crate::env::Bus;
use crate::error::ServiceError;
use crate::stream::next;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";

/// Runs until `commands` closes. Without a session bus, `tx` stays at its default.
pub(crate) async fn run(
    bus: Bus,
    cache_dir: PathBuf,
    tx: watch::Sender<Media>,
    mut commands: mpsc::UnboundedReceiver<MediaCommand>,
) -> Result<(), ServiceError> {
    let connection = crate::supervise::connect_when_up(&bus).await?;
    let dbus = DBusProxy::new(&connection).await?;
    let mut owner_changes = dbus.receive_name_owner_changed().await?;
    let mut prop_changes =
        MessageStream::for_match_rule(properties_changed_rule()?, &connection, None).await?;
    let mut seeks = MessageStream::for_match_rule(seeked_rule()?, &connection, None).await?;

    let origin = Instant::now();
    let (art, mut art_rx) = ArtFetcher::new();
    let mut owners: HashMap<OwnedUniqueName, PlayerId> = HashMap::new();

    discover(
        &connection,
        &dbus,
        &tx,
        &art,
        &mut owners,
        Some(cache_dir.as_path()),
        origin,
    )
    .await;

    loop {
        tokio::select! {
            Some(signal) = next(&mut owner_changes) => {
                handle_owner_change(&signal, &connection, &tx, &art, &mut owners, Some(cache_dir.as_path()), origin).await;
            }
            Some(Ok(message)) = next(&mut prop_changes) => {
                handle_properties_changed(&message, &connection, &tx, &art, &owners, Some(cache_dir.as_path()), origin).await;
            }
            Some(Ok(message)) = next(&mut seeks) => {
                handle_seeked(&message, &connection, &tx, &owners, origin).await;
            }
            Some((id, path)) = art_rx.recv() => {
                apply_art(&tx, id, path);
            }
            command = commands.recv() => match command {
                Some(MediaCommand::Press { player, verb }) => press(&connection, &tx, &player, verb).await,
                Some(MediaCommand::Refresh) => {
                    discover(&connection, &dbus, &tx, &art, &mut owners, Some(cache_dir.as_path()), origin).await;
                }
                Some(MediaCommand::Position(id)) => {
                    reread_timeline(&connection, &tx, id, origin).await;
                }
                None => break,
            }
        }
    }
    Ok(())
}

/// A bus-wide match on every `PropertiesChanged` signal, whoever sends it (any interface; only
/// [`PLAYER_INTERFACE`]'s are kept).
fn properties_changed_rule() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .build())
}

/// A bus-wide match on every `Seeked` signal.
fn seeked_rule() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(PLAYER_INTERFACE)?
        .member("Seeked")?
        .build())
}

/// Whether `name` is an MPRIS player's well-known bus name.
fn is_mpris_name(name: &str) -> bool {
    name.starts_with(MPRIS_PREFIX)
}

/// Lists every MPRIS name on the bus, snapshots each, and replaces `owners` and the published
/// `Media` wholesale (a resume, or the first read).
async fn discover(
    connection: &Connection,
    dbus: &DBusProxy<'_>,
    tx: &watch::Sender<Media>,
    art: &ArtFetcher,
    owners: &mut HashMap<OwnedUniqueName, PlayerId>,
    cache_dir: Option<&Path>,
    origin: Instant,
) {
    owners.clear();
    let mut media = Media::default();
    let Ok(names) = dbus.list_names().await else {
        let _ = tx.send(media);
        return;
    };
    for name in names.into_iter().filter(|n| is_mpris_name(n.as_str())) {
        let Ok(bus_name) = BusName::try_from(name.as_str().to_owned()) else {
            continue;
        };
        if let Ok(owner) = dbus.get_name_owner(bus_name).await {
            owners.insert(owner, PlayerId(name.as_str().to_owned()));
        }
        let Some((player, pending)) =
            snapshot(connection, name.as_str(), stamp(origin), cache_dir).await
        else {
            continue;
        };
        if let Some(job) = pending {
            art.spawn(player.id.clone(), job);
        }
        media.players.push(player);
    }
    let _ = tx.send(media);
}

/// A name gained or lost an owner: a player appeared (or replaced its process) or left.
async fn handle_owner_change(
    signal: &NameOwnerChanged,
    connection: &Connection,
    tx: &watch::Sender<Media>,
    art: &ArtFetcher,
    owners: &mut HashMap<OwnedUniqueName, PlayerId>,
    cache_dir: Option<&Path>,
    origin: Instant,
) {
    let Ok(args) = signal.args() else {
        return;
    };
    let name = args.name().as_str();
    if !is_mpris_name(name) {
        return;
    }
    let id = PlayerId(name.to_owned());
    owners.retain(|_, existing| existing != &id);
    match args.new_owner().as_ref() {
        Some(owner) => {
            owners.insert(owner.to_owned().into(), id.clone());
            if let Some((player, pending)) =
                snapshot(connection, name, stamp(origin), cache_dir).await
            {
                apply(tx, MediaEvent::Player(player));
                if let Some(job) = pending {
                    art.spawn(id, job);
                }
            }
        }
        None => apply(tx, MediaEvent::Removed(id)),
    }
}

/// A tracked player's `PropertiesChanged`: re-snapshots it and, if it differs, publishes the
/// fresh player (keeping `changed` unless `playback` itself moved).
async fn handle_properties_changed(
    message: &Message,
    connection: &Connection,
    tx: &watch::Sender<Media>,
    art: &ArtFetcher,
    owners: &HashMap<OwnedUniqueName, PlayerId>,
    cache_dir: Option<&Path>,
    origin: Instant,
) {
    let Some(id) = sender_player(message, owners) else {
        return;
    };
    let Ok((iface, _changed, _invalidated)) =
        message
            .body()
            .deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
    else {
        return;
    };
    if iface != PLAYER_INTERFACE {
        return;
    }
    let Some(before) = tx.borrow().players.iter().find(|p| p.id == id).cloned() else {
        return;
    };
    let Some((mut fresh, pending)) = snapshot(connection, &id.0, stamp(origin), cache_dir).await
    else {
        return;
    };
    if fresh.playback == before.playback {
        fresh.changed = before.changed;
    }
    if fresh != before {
        apply(tx, MediaEvent::Player(fresh));
    }
    if let Some(job) = pending {
        art.spawn(id, job);
    }
}

/// A `Seeked` signal: reads `Position` fresh and republishes just the timeline.
async fn handle_seeked(
    message: &Message,
    connection: &Connection,
    tx: &watch::Sender<Media>,
    owners: &HashMap<OwnedUniqueName, PlayerId>,
    origin: Instant,
) {
    let Some(id) = sender_player(message, owners) else {
        return;
    };
    reread_timeline(connection, tx, id, origin).await;
}

/// Reads `id`'s `Position` and `mpris:length` fresh (property reads only) and republishes just
/// its timeline; a player that is not listed is dropped by the reducer.
async fn reread_timeline(
    connection: &Connection,
    tx: &watch::Sender<Media>,
    id: PlayerId,
    origin: Instant,
) {
    let Some(player) = player_proxy(connection, &id).await else {
        return;
    };
    let metadata = player.metadata().await.unwrap_or_default();
    let timeline = timeline_of(&player, &metadata, stamp(origin)).await;
    apply(tx, MediaEvent::Timeline { id, timeline });
}

/// The tracked player that sent `message`, if any.
fn sender_player(
    message: &Message,
    owners: &HashMap<OwnedUniqueName, PlayerId>,
) -> Option<PlayerId> {
    let header = message.header();
    let sender: OwnedUniqueName = header.sender()?.to_owned().into();
    owners.get(&sender).cloned()
}

/// Carries out `verb` on `player`, dropped without a proxy or when the player's last known
/// abilities disallow it.
async fn press(
    connection: &Connection,
    tx: &watch::Sender<Media>,
    player: &PlayerId,
    verb: MediaVerb,
) {
    let allowed = tx
        .borrow()
        .players
        .iter()
        .find(|p| &p.id == player)
        .is_some_and(|p| ability_allows(&p.abilities, verb));
    if !allowed {
        return;
    }
    let Some(proxy) = player_proxy(connection, player).await else {
        return;
    };
    let _ = match verb {
        MediaVerb::PlayPause => proxy.play_pause().await,
        MediaVerb::Next => proxy.next().await,
        MediaVerb::Previous => proxy.previous().await,
    };
}

fn ability_allows(abilities: &Abilities, verb: MediaVerb) -> bool {
    let availability = match verb {
        MediaVerb::PlayPause => abilities.play_pause,
        MediaVerb::Next => abilities.next,
        MediaVerb::Previous => abilities.previous,
    };
    availability == Availability::Enabled
}

/// One player's full state, read fresh from the bus, and a cover job still to run (the cache had
/// no file for it yet).
async fn snapshot(
    connection: &Connection,
    name: &str,
    at: Stamp,
    cache_dir: Option<&Path>,
) -> Option<(MediaPlayer, Option<ArtJob>)> {
    let root = MediaPlayer2Proxy::builder(connection)
        .destination(name.to_owned())
        .ok()?
        .build()
        .await
        .ok()?;
    let player = player_proxy(connection, &PlayerId(name.to_owned())).await?;

    let identity = root.identity().await.unwrap_or_default();
    let playback = playback_of(&player.playback_status().await.unwrap_or_default());
    let metadata = player.metadata().await.unwrap_or_default();
    let mut track = track_of(&metadata);
    let (art, pending) = resolve_art(&metadata, cache_dir);
    track.art = art;
    let timeline = timeline_of(&player, &metadata, at).await;
    let abilities = abilities_of(&player).await;

    Some((
        MediaPlayer {
            id: PlayerId(name.to_owned()),
            identity,
            playback,
            changed: at,
            track,
            timeline,
            abilities,
        },
        pending,
    ))
}

async fn player_proxy<'a>(connection: &'a Connection, id: &PlayerId) -> Option<PlayerProxy<'a>> {
    PlayerProxy::builder(connection)
        .destination(id.0.clone())
        .ok()?
        .build()
        .await
        .ok()
}

fn playback_of(status: &str) -> Playback {
    match status {
        "Playing" => Playback::Playing,
        "Paused" => Playback::Paused,
        _ => Playback::Stopped,
    }
}

fn track_of(metadata: &HashMap<String, OwnedValue>) -> Track {
    Track {
        title: metadata_string(metadata, "xesam:title"),
        artists: metadata_strings(metadata, "xesam:artist"),
        album: metadata_string(metadata, "xesam:album"),
        art: Art::None,
    }
}

async fn timeline_of(
    player: &PlayerProxy<'_>,
    metadata: &HashMap<String, OwnedValue>,
    at: Stamp,
) -> Timeline {
    let Some(length) = metadata_micros(metadata, "mpris:length") else {
        return Timeline::Unknown;
    };
    let Ok(position) = player.position().await else {
        return Timeline::Unknown;
    };
    Timeline::Known {
        position: Micros(position.max(0).cast_unsigned()),
        length,
        at,
    }
}

async fn abilities_of(player: &PlayerProxy<'_>) -> Abilities {
    let can_play = player.can_play().await.unwrap_or(true);
    let can_pause = player.can_pause().await.unwrap_or(true);
    let can_next = player.can_go_next().await.unwrap_or(true);
    let can_previous = player.can_go_previous().await.unwrap_or(true);
    Abilities {
        play_pause: availability_of(can_play || can_pause),
        next: availability_of(can_next),
        previous: availability_of(can_previous),
    }
}

fn availability_of(can: bool) -> Availability {
    if can {
        Availability::Enabled
    } else {
        Availability::Disabled
    }
}

pub(super) fn metadata_string(metadata: &HashMap<String, OwnedValue>, key: &str) -> String {
    metadata
        .get(key)
        .and_then(|v| String::try_from(v.clone()).ok())
        .unwrap_or_default()
}

fn metadata_strings(metadata: &HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    metadata
        .get(key)
        .and_then(|v| Vec::<String>::try_from(v.clone()).ok())
        .unwrap_or_default()
}

fn metadata_micros(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<Micros> {
    metadata
        .get(key)
        .and_then(|v| i64::try_from(v.clone()).ok())
        .map(|v| Micros(v.max(0).cast_unsigned()))
}

fn stamp(origin: Instant) -> Stamp {
    Stamp(u64::try_from(origin.elapsed().as_millis()).unwrap_or(u64::MAX))
}

/// Reduces `event` onto the receiver's current value and publishes the result.
pub(super) fn apply(tx: &watch::Sender<Media>, event: MediaEvent) {
    let next = step(tx.borrow().clone(), event);
    let _ = tx.send(next);
}
