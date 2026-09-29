//! The launcher toggle's latency line: `launcher.open_latency_ms=<ms>` on stderr, timed from the
//! IPC request's arrival, which `dev/accept-launcher.sh` greps out for its p50/p95/max over 20
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

/// The value [`open_latency_line`] printed, parsed back out of one line of the daemon's log.
/// `None` for any line that is not one of ours (including a malformed number, which a caller
/// should treat the same as "not present" rather than fail the whole log on).
pub fn parse_open_latency_ms(line: &str) -> Option<f64> {
    line.trim()
        .strip_prefix("launcher.open_latency_ms=")?
        .parse()
        .ok()
}
