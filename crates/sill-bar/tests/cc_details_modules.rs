//! Module details on the real control center (design/26 5.2.7, 5.2.9, 5.2.10): Now Playing's
//! play/pause, track and position,
//! the Battery module's device rings and the Keyboard Brightness module, driven by the fake
//! services on `Clock::Virtual`. Every state change ends at 0 frames (R3). Nothing here reaches a
//! real player or backlight.

mod cc_support;

use cc_support::details::{center, figure, ms, press};
use cc_support::{fake, live};

use ds_native::harness::{assert_settles_to_zero_frames, settle_until};
use sill_model::kbd_backlight::{KbdBacklight, KbdCommand, KbdLevel};
use sill_model::now_playing::{MediaCommand, MediaVerb, Micros, Playback, Timeline};
use sill_model::units::{Percent, Presence, Stamp};

// ---- Now Playing ----------------------------------------------------------------------------

fn playing(playback: Playback, title: &str) -> sill_model::now_playing::Media {
    let mut media = fake::media();
    let player = &mut media.players[0];
    player.playback = playback;
    player.track.title = title.to_owned();
    player.timeline = Timeline::Known {
        position: Micros(65_000_000),
        length: Micros(301_000_000),
        at: Stamp(1),
    };
    media
}

const TOGGLE: &str = ".sill-cc-media-buttons [*|data-playback]";

#[test]
fn now_playing_morphs_its_button_cross_fades_the_track_and_steps_the_position() {
    let mut running = center(|feeds| {
        feeds
            .media
            .send_replace(playing(Playback::Playing, "So What"));
    });
    let player = fake::media().players[0].id.clone();
    assert_eq!(
        running.feeds.media_commands.try_recv().ok(),
        Some(MediaCommand::Position(player.clone())),
        "opening asks where the track is now"
    );
    let harness = &mut running.harness;
    assert_eq!(harness.attr(TOGGLE, "aria-label").as_deref(), Some("Pause"));
    assert_eq!(harness.text_of(".ds-track-time").as_deref(), Some("1:05"));
    harness.advance(ms(1_000));
    assert_eq!(harness.text_of(".ds-track-time").as_deref(), Some("1:06"));
    // A press asks the player (a fake: nothing real plays or pauses).
    press(harness, TOGGLE);
    harness.advance(ms(0));
    assert_eq!(
        running.feeds.media_commands.try_recv().ok(),
        Some(MediaCommand::Press {
            player,
            verb: MediaVerb::PlayPause
        })
    );
    // The player reports paused: the button offers Play, the next action growing in.
    running
        .feeds
        .media
        .send_replace(playing(Playback::Paused, "So What"));
    let harness = &mut running.harness;
    harness.advance(ms(0));
    assert_eq!(harness.attr(TOGGLE, "aria-label").as_deref(), Some("Play"));
    assert_eq!(harness.count(".sill-cc-media-buttons [*|data-morph=in]"), 1);
    assert_settles_to_zero_frames(harness);
    let held = harness.text_of(".ds-track-time");
    harness.advance(ms(3_000));
    assert_eq!(harness.text_of(".ds-track-time"), held, "paused, it holds");
    // A new track cross-fades in.
    running
        .feeds
        .media
        .send_replace(playing(Playback::Paused, "Freddie Freeloader"));
    let harness = &mut running.harness;
    harness.advance(ms(0));
    assert_eq!(harness.count(".ds-track-words [*|data-morph=out]"), 1);
    assert_settles_to_zero_frames(harness);
    assert_eq!(
        harness.text_of(".ds-track-title").as_deref(),
        Some("Freddie Freeloader")
    );
}

#[test]
fn a_player_without_a_length_shows_no_position() {
    let mut running = center(|feeds| {
        feeds.media.send_replace(fake::media());
    });
    assert_eq!(running.harness.count(".ds-track-position"), 0);
    assert_eq!(running.harness.count(".ds-track"), 1);
    assert_settles_to_zero_frames(&mut running.harness);
}

// ---- Battery --------------------------------------------------------------------------------

const RING: &str = ".sill-cc-batteries > .ds-device-battery";

#[test]
fn each_devices_ring_stands_at_its_level_when_the_center_opens() {
    let (services, feeds) = fake::services();
    let mut running = live::start(services, feeds);
    let harness = &mut running.harness;
    settle_until(harness, |h| h.count(RING) == 2);
    let this = format!("{RING}:nth-child(1) .ds-battery-figure");
    let buds = format!("{RING}:nth-child(2) .ds-battery-figure");
    // Nothing sweeps in on first show (quire design/30 section 1.3): the figures are the levels.
    assert_eq!(
        (figure(harness, &this), figure(harness, &buds)),
        (Some(64), Some(80))
    );
    assert_settles_to_zero_frames(harness);
    assert_eq!(
        (figure(harness, &this), figure(harness, &buds)),
        (Some(64), Some(80))
    );
    // A change moves the ring; the figure changes at once.
    running
        .feeds
        .battery
        .send_modify(|battery| battery.level = Percent(60));
    running.harness.advance(ms(0));
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(figure(&running.harness, &this), Some(60));
}

// ---- Keyboard brightness --------------------------------------------------------------------

const KEYS: &str = ".ds-level[*|aria-label=\"Keyboard Brightness\"]";

#[test]
fn the_keyboard_module_follows_the_backlight_and_hides_without_one() {
    let mut running = center(|_| {});
    assert_eq!(running.harness.count(KEYS), 0, "no backlight, no module");
    assert_settles_to_zero_frames(&mut running.harness);
    running
        .feeds
        .kbd_backlight
        .send_replace(fake::kbd_backlight());
    let harness = &mut running.harness;
    settle_until(harness, |h| h.count(KEYS) == 1);
    assert_eq!(harness.attr(KEYS, "aria-valuenow").as_deref(), Some("67"));
    assert!(harness.count(&format!("{KEYS} [*|data-part=key-rays]")) >= 1);
    assert_settles_to_zero_frames(harness);
    // The laptop's own key: the service reports the top step.
    running.feeds.kbd_backlight.send_replace(KbdBacklight {
        presence: Presence::Present,
        level: KbdLevel { value: 3, max: 3 },
    });
    let harness = &mut running.harness;
    settle_until(harness, |h| {
        h.attr(KEYS, "aria-valuenow").as_deref() == Some("100")
    });
    assert_settles_to_zero_frames(harness);
    // A press on the level asks the service (a fake: no real backlight changes).
    press(harness, KEYS);
    harness.advance(ms(0));
    assert!(
        matches!(
            running.feeds.kbd_commands.try_recv().ok(),
            Some(KbdCommand::Set(_))
        ),
        "a press sets the backlight"
    );
    assert_settles_to_zero_frames(&mut running.harness);
    running
        .feeds
        .kbd_backlight
        .send_replace(KbdBacklight::default());
    settle_until(&mut running.harness, |h| h.count(KEYS) == 0);
    assert_settles_to_zero_frames(&mut running.harness);
}
