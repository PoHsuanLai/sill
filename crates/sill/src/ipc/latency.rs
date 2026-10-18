//! The launcher toggle's latency line: `launcher.open_latency_ms=<ms>` on stderr, timed from the
//! IPC request's arrival, which `dev/accept/launcher.sh` greps out for its p50/p95/max over 20
//! toggles.

use std::time::Duration;

/// Prints [`open_latency_line`] on stderr.
pub fn log_open_latency(elapsed: Duration) {
    eprintln!("{}", open_latency_line(elapsed));
}

/// The stderr line for one `sill launcher toggle`'s latency, `elapsed` since the IPC request
/// arrived. Its own function (rather than inlined in [`log_open_latency`]) so
/// `crates/sill/tests/latency.rs` can check the arithmetic without a wall clock.
pub fn open_latency_line(elapsed: Duration) -> String {
    format!(
        "launcher.open_latency_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    )
}
