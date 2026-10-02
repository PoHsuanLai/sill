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
