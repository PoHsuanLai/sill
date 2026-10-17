use super::{Abilities, Media, MediaEvent, MediaPlayer, Playback, PlayerId, Timeline, Track, step};
use ds_core::time::stamp::Stamp;

fn player(name: &str, playback: Playback, changed: u64) -> MediaPlayer {
    MediaPlayer {
        id: PlayerId(format!("org.mpris.MediaPlayer2.{name}")),
        identity: name.to_owned(),
        playback,
        changed: Stamp(changed),
        track: Track::default(),
        timeline: Timeline::Unknown,
        abilities: Abilities::default(),
    }
}

fn current(players: Vec<MediaPlayer>) -> Option<String> {
    Media { players }.current().map(|p| p.identity.clone())
}

#[test]
fn the_most_recent_player_to_play_is_current() {
    use Playback::{Paused, Playing, Stopped};
    let cases: Vec<(&str, Vec<MediaPlayer>, Option<&str>)> = vec![
        ("nothing on the bus", Vec::new(), None),
        ("stopped only", vec![player("a", Stopped, 9)], None),
        (
            "playing beats a later pause",
            vec![player("a", Playing, 1), player("b", Paused, 9)],
            Some("a"),
        ),
        (
            "the later of two playing",
            vec![player("a", Playing, 1), player("b", Playing, 5)],
            Some("b"),
        ),
        (
            "the later of two paused",
            vec![player("a", Paused, 7), player("b", Paused, 5)],
            Some("a"),
        ),
    ];
    for (name, players, want) in cases {
        assert_eq!(current(players), want.map(str::to_owned), "{name}");
    }
}

#[test]
fn events_reduce_to_the_expected_media() {
    let a = || player("a", Playback::Paused, 1);
    let with = |players| Media { players };
    let cases: Vec<(&str, Media, MediaEvent, Media)> = vec![
        (
            "a player appears",
            Media::default(),
            MediaEvent::Player(a()),
            with(vec![a()]),
        ),
        (
            "it starts playing",
            with(vec![a()]),
            MediaEvent::Playback {
                id: a().id,
                playback: Playback::Playing,
                at: Stamp(4),
            },
            with(vec![player("a", Playback::Playing, 4)]),
        ),
        (
            "an unknown player's change is dropped",
            with(vec![a()]),
            MediaEvent::Playback {
                id: PlayerId("nobody".to_owned()),
                playback: Playback::Playing,
                at: Stamp(4),
            },
            with(vec![a()]),
        ),
        (
            "it leaves",
            with(vec![a()]),
            MediaEvent::Removed(a().id),
            Media::default(),
        ),
        (
            "the bus goes",
            with(vec![a()]),
            MediaEvent::Gone,
            Media::default(),
        ),
    ];
    for (name, before, event, want) in cases {
        assert_eq!(step(before, event), want, "{name}");
    }
}

#[test]
fn players_past_the_cap_are_not_listed_but_a_listed_one_still_updates() {
    use super::PLAYERS_MAX;
    let full = Media {
        players: (0..PLAYERS_MAX)
            .map(|n| player(&format!("p{n}"), Playback::Paused, 1))
            .collect(),
    };
    let extra = player("extra", Playback::Playing, 9);
    let after = step(full.clone(), MediaEvent::Player(extra.clone()));
    assert_eq!(after, full, "the {}th player is dropped", PLAYERS_MAX + 1);
    let listed = player("p0", Playback::Playing, 9);
    let after = step(full, MediaEvent::Player(listed.clone()));
    assert_eq!(after.players.len(), PLAYERS_MAX);
    assert_eq!(after.players[0], listed);
}
