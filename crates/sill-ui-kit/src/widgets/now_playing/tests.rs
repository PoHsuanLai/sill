use super::face::{TrackTimes, playback_of, position_of, track_lines};
use ds::base::time::stamp::Stamp;
use ds_shell::now_playing::kind::Playback;
use sill_model::now_playing::{Abilities, MediaPlayer, PlayerId, Track};
use sill_model::now_playing::{Micros, Timeline};
use std::time::Duration;

#[test]
fn a_stopped_player_offers_play() {
    let cases = [
        (
            sill_model::now_playing::Playback::Playing,
            Playback::Playing,
        ),
        (sill_model::now_playing::Playback::Paused, Playback::Paused),
        (sill_model::now_playing::Playback::Stopped, Playback::Paused),
    ];
    for (from, want) in cases {
        assert_eq!(playback_of(from), want, "{from:?}");
    }
}

#[test]
fn the_bar_shows_only_when_both_times_are_known() {
    let known = Timeline::Known {
        position: Micros(65_000_000),
        length: Micros(301_000_000),
        at: Stamp(9),
    };
    let cases = [
        (
            known,
            Some(TrackTimes {
                at: Duration::from_secs(65),
                length: Duration::from_secs(301),
            }),
        ),
        (Timeline::Unknown, None),
    ];
    for (timeline, want) in cases {
        assert_eq!(position_of(timeline), want, "{timeline:?}");
    }
}

#[test]
fn now_playing_lines() {
    let player = |title: &str, artists: &[&str], album: &str| MediaPlayer {
        id: PlayerId("org.mpris.MediaPlayer2.spotify".to_owned()),
        identity: "Spotify".to_owned(),
        playback: sill_model::now_playing::Playback::Playing,
        changed: Stamp(0),
        track: Track {
            title: title.to_owned(),
            artists: artists.iter().map(|a| (*a).to_owned()).collect(),
            album: album.to_owned(),
            ..Track::default()
        },
        timeline: Timeline::Unknown,
        abilities: Abilities::default(),
    };
    let lines = |p: MediaPlayer| track_lines(&p);
    assert_eq!(
        lines(player("So What", &["Miles Davis"], "Kind of Blue")),
        (
            "So What".to_owned(),
            "Miles Davis — Kind of Blue".to_owned()
        )
    );
    assert_eq!(
        lines(player("Duet", &["A", "B"], "")),
        ("Duet".to_owned(), "A, B".to_owned())
    );
    assert_eq!(
        lines(player("", &[], "")),
        ("Spotify".to_owned(), String::new())
    );
}
