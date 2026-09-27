//! Quire's `Osd` in a Blitz harness, on the virtual clock (quire Q380): a change fades it in with the live level
//! and its real title, a new level moves the bar, the hold (`osd.hold_ms`) ends in the fade out
//! and then the hidden look, a change during the fade takes it back, and a faked change shows its
//! own level without touching the services.

mod osd_support;

use std::time::{Duration, Instant};

use ds_settings::Ms;
use osd_support::{fake, label, level, look, start};
use sill_services::{Mute, Percent, Volume};
use sill_settings::{OsdSettings, ShellFile};
use sill_surfaces::surfaces::osd::{Level, LevelSource, OsdKind};

const HOLD_MS: u64 = 600;

fn file() -> ShellFile {
    ShellFile {
        osd: OsdSettings {
            hold_ms: Ms(HOLD_MS as u16),
            ..OsdSettings::default()
        },
        ..ShellFile::default()
    }
}

/// Advance in 20 ms steps for `span`, recording each new look and when it appeared (ms since
/// `from`, on the harness's clock).
fn looks_over(
    harness: &mut ds_native::Harness,
    from: Instant,
    span: Duration,
) -> Vec<(u64, String)> {
    let mut seen: Vec<(u64, String)> = Vec::new();
    let until = harness.now() + span;
    while harness.now() < until {
        harness.advance(Duration::from_millis(20));
        let now = look(harness).unwrap_or_default();
        if seen.last().map(|(_, l)| l) != Some(&now) {
            let since = harness.now().saturating_duration_since(from);
            seen.push((u64::try_from(since.as_millis()).unwrap_or(u64::MAX), now));
        }
    }
    seen
}

/// Show `kind` on the test output from `source`, taken at this instant (the next `advance` runs
/// the woken surface first, quire G311); returns when it was asked, on the harness's clock.
fn show(running: &mut osd_support::Running, kind: OsdKind, source: LevelSource) -> Instant {
    running.link.show(kind, vec![fake::output().id], source);
    running.harness.now()
}

#[test]
fn a_change_holds_then_fades_then_hides() {
    let mut running = start(file());
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(look(&running.harness).as_deref(), Some("hidden"));
    let changed = show(&mut running, OsdKind::Volume, LevelSource::Live);
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(look(&running.harness).as_deref(), Some("entering"));
    assert_eq!(level(&running.harness), Some(40), "the fake sink's level");
    assert_eq!(
        label(&running.harness).as_deref(),
        Some("Built-in Audio Analog Stereo"),
        "the fake sink's own name"
    );
    running.feeds.volume.send_replace(Volume {
        level: Percent(55),
        ..fake::volume()
    });
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        level(&running.harness),
        Some(55),
        "the bar follows the service"
    );
    let seen = looks_over(&mut running.harness, changed, Duration::from_millis(1500));
    let names: Vec<&str> = seen.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(
        names,
        ["entering", "present", "leaving", "hidden"],
        "{seen:?}"
    );
    assert_eq!(
        seen[2].0, HOLD_MS,
        "the fade starts at the hold's end: {seen:?}"
    );
}

#[test]
fn a_change_during_the_fade_takes_it_back_and_holds_again() {
    let mut running = start(file());
    let shown = show(&mut running, OsdKind::Volume, LevelSource::Live);
    running.harness.advance(Duration::from_millis(HOLD_MS - 1));
    assert_eq!(
        look(&running.harness).as_deref(),
        Some("present"),
        "held a millisecond before the hold's end"
    );
    running.harness.advance(Duration::from_millis(1));
    assert_eq!(
        look(&running.harness).as_deref(),
        Some("leaving"),
        "fading at it"
    );
    assert_eq!(
        running.harness.now() - shown,
        Duration::from_millis(HOLD_MS)
    );
    running.harness.advance(Duration::from_millis(20));
    let again = show(&mut running, OsdKind::Volume, LevelSource::Live);
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        look(&running.harness).as_deref(),
        Some("present"),
        "the hide is taken back at once, no second entrance"
    );
    let seen = looks_over(&mut running.harness, again, Duration::from_millis(1400));
    let names: Vec<&str> = seen.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(names, ["present", "leaving", "hidden"], "{seen:?}");
    assert_eq!(seen[1].0, HOLD_MS, "a fresh hold: {seen:?}");
}

#[test]
fn a_faked_change_shows_its_own_level_and_another_outputs_does_not_show() {
    let mut running = start(file());
    running.link.show(
        OsdKind::Volume,
        vec![shell_host::OutputId(99)],
        LevelSource::Live,
    );
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(
        look(&running.harness).as_deref(),
        Some("hidden"),
        "another output's change"
    );
    running.link.show(
        OsdKind::Brightness,
        vec![fake::output().id],
        LevelSource::Fixed(Level {
            percent: Percent(70),
            mute: Mute::Audible,
        }),
    );
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(look(&running.harness).as_deref(), Some("entering"));
    assert_eq!(level(&running.harness), Some(70));
    assert_eq!(
        label(&running.harness).as_deref(),
        Some("Display"),
        "no DDC monitor behind the fake output"
    );
}
