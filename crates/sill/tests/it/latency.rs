//! The launcher latency arithmetic: formatting `launcher.open_latency_ms=<ms>`, the
//! line `dev/accept/launcher.sh` greps out of the daemon's stderr for its p50/p95/max over 20
//! toggles. No wall clock: every case is a fixed `Duration`.

use std::time::Duration;

use sill::ipc::latency::open_latency_line;

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
