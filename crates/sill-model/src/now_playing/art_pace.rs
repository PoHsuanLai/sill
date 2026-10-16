//! How fast one player may have covers fetched. Pure; the service sleeps until the instant this
//! names.

use std::time::{Duration, Instant};

/// When a cover wanted `now` may start, given when the player's last one was set to start and the
/// gap `every` a player's covers keep. A cover wanted while the last one is still waiting takes its
/// place (a player that changes cover faster than the gap gets only the newest fetched), so a
/// stream of changes is one fetch per gap, never a queue.
pub fn next_start(previous: Option<Instant>, now: Instant, every: Duration) -> Instant {
    match previous {
        None => now,
        Some(waiting) if waiting > now => waiting,
        Some(last) => (last + every).max(now),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::next_start;

    #[test]
    fn a_players_covers_start_a_gap_apart_and_a_waiting_one_is_replaced() {
        let base = Instant::now();
        let at = |ms: u64| base + Duration::from_millis(ms);
        let every = Duration::from_millis(3_000);
        // previous start (ms after base), now (ms after base), the start it gets
        let cases: [(&str, Option<u64>, u64, u64); 5] = [
            ("the first cover starts at once", None, 500, 500),
            (
                "one long after the last starts at once",
                Some(0),
                10_000,
                10_000,
            ),
            (
                "one soon after the last waits out the gap",
                Some(1_000),
                1_500,
                4_000,
            ),
            (
                "one exactly a gap after starts at once",
                Some(1_000),
                4_000,
                4_000,
            ),
            (
                "one while another waits takes its place",
                Some(4_000),
                1_500,
                4_000,
            ),
        ];
        for (name, previous, now, want) in cases {
            assert_eq!(
                next_start(previous.map(at), at(now), every),
                at(want),
                "{name}"
            );
        }
    }
}
