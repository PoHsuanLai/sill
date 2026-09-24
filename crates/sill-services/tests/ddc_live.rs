//! DDC/CI against the monitors on this machine: probe, time every read, and time a brightness
//! and a speaker-volume write, putting each value back exactly as it was. Never touches the
//! input source (that would switch the screen away) or anything else.
//!
//! `cargo test -p sill-services --test ddc_live -- --ignored --nocapture`

use std::time::{Duration, Instant};

use sill_services::displays::ddc::{DdcBus, DdcId, Feature, I2cBuses, Level, retry};

fn millis(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1000.0)
}

/// One raw write of `to` (the bare transaction), then a verified write of `original` back
/// (`retry::write_verified`: write, read back, repeat until the monitor reports it), each
/// timed; the restore is asserted with a fresh read.
fn round_trip(bus: &mut I2cBuses, id: DdcId, feature: Feature, original: Level, to: u16) {
    let started = Instant::now();
    let raw = bus.write(id, feature, to);
    let wrote = started.elapsed();
    std::thread::sleep(Duration::from_millis(100));
    let seen = retry::read(bus, id, feature).map(|l| l.value);
    let started = Instant::now();
    let restored = retry::write_verified(bus, id, feature, original.value);
    let verified_in = started.elapsed();
    let now = retry::read(bus, id, feature).map(|l| l.value);
    println!(
        "  {feature:?}: raw write {to}: {raw:?} in {} (then reads {seen:?}); verified restore to {}: {restored:?} in {} -> {now:?}",
        millis(wrote),
        original.value,
        millis(verified_in),
    );
    assert_eq!(now, Ok(original.value), "{feature:?} was not restored");
}

#[test]
#[ignore = "talks to the real monitors over DDC/CI"]
fn probe_time_and_restore() {
    let mut bus = I2cBuses::default();
    let started = Instant::now();
    let found = bus.probe();
    println!(
        "probe: {} display(s) in {}",
        found.len(),
        millis(started.elapsed())
    );
    for endpoint in found {
        println!(
            "bus {} {:?} {:?}",
            endpoint.id.0, endpoint.connector, endpoint.product
        );
        let mut levels = Vec::new();
        for feature in Feature::ALL {
            let started = Instant::now();
            let read = bus.read(endpoint.id, feature);
            println!(
                "  read {feature:?}: {read:?} in {}",
                millis(started.elapsed())
            );
            levels.push((feature, read.ok()));
        }
        let level = |want| {
            levels
                .iter()
                .find(|(f, _)| *f == want)
                .and_then(|(_, l)| *l)
        };
        if let Some(brightness) = level(Feature::Brightness) {
            let to = if brightness.value >= 10 {
                brightness.value - 5
            } else {
                brightness.value + 5
            };
            round_trip(&mut bus, endpoint.id, Feature::Brightness, brightness, to);
        }
        if let Some(volume) = level(Feature::SpeakerVolume) {
            let to = if volume.value >= 2 {
                volume.value - 2
            } else {
                volume.value + 2
            };
            round_trip(&mut bus, endpoint.id, Feature::SpeakerVolume, volume, to);
        }
        // Ten back-to-back reads: the steady-state cost of one.
        let started = Instant::now();
        for _ in 0..10 {
            let _ = bus.read(endpoint.id, Feature::Brightness);
        }
        println!(
            "  10 brightness reads: {} each",
            millis(started.elapsed() / 10)
        );
    }
}
