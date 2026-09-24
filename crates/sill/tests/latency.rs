//! The launcher latency arithmetic (M4 perf-ime; FINDINGS.md "M4 perf-ime"): formatting and
//! parsing `launcher.open_latency_ms=<ms>`, the line `dev/accept-launcher.sh` greps out of the
//! daemon's stderr for its p50/p95/max over 20 toggles. No wall clock: every case is a fixed
//! `Duration`.

use std::time::Duration;

use sill::daemon::{open_latency_line, parse_open_latency_ms};

const FORMAT_CASES: &[(Duration, &str)] = &[
    (Duration::ZERO, "launcher.open_latency_ms=0.000"),
    (Duration::from_millis(1), "launcher.open_latency_ms=1.000"),
    (
        Duration::from_micros(42_500),
        "launcher.open_latency_ms=42.500",
    ),
    (
        Duration::from_millis(100),
        "launcher.open_latency_ms=100.000",
    ),
    (Duration::from_secs(1), "launcher.open_latency_ms=1000.000"),
    (
        Duration::from_nanos(123_456),
        "launcher.open_latency_ms=0.123",
    ),
];

#[test]
fn open_latency_line_formats_milliseconds_to_three_decimals() {
    for (elapsed, want) in FORMAT_CASES {
        assert_eq!(&open_latency_line(*elapsed), want, "{elapsed:?}");
    }
}

#[test]
fn parse_open_latency_ms_round_trips_every_formatted_line() {
    for (elapsed, _) in FORMAT_CASES {
        let line = open_latency_line(*elapsed);
        let parsed =
            parse_open_latency_ms(&line).unwrap_or_else(|| panic!("{line:?} did not parse"));
        assert!(
            (parsed - elapsed.as_secs_f64() * 1000.0).abs() < 0.001,
            "{line:?} parsed to {parsed}, wanted ~{}",
            elapsed.as_secs_f64() * 1000.0
        );
    }
}

#[test]
fn parse_open_latency_ms_ignores_lines_that_are_not_ours() {
    for line in [
        "",
        "launcher.open_latency_ms",
        "launcher.open_latency_ms=",
        "launcher.open_latency_ms=not-a-number",
        "sill: the launcher is not built yet (M4)",
        "dock.open_latency_ms=12.000",
        "launcher.open_latency_msextra=12.000",
    ] {
        assert_eq!(parse_open_latency_ms(line), None, "{line:?}");
    }
}

#[test]
fn parse_open_latency_ms_tolerates_surrounding_whitespace() {
    assert_eq!(
        parse_open_latency_ms("  launcher.open_latency_ms=12.500\n"),
        Some(12.5)
    );
}
