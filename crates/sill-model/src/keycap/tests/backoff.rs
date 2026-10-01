use std::time::Duration;

use crate::keycap::reconnect_delay;

#[test]
fn the_wait_starts_at_one_second_and_doubles_to_thirty() {
    let seconds = |s| Some(Duration::from_secs(s));
    let cases: &[(&str, Option<u64>, u64)] = &[
        ("first", None, 1),
        ("second", Some(1), 2),
        ("third", Some(2), 4),
        ("fifth", Some(8), 16),
        ("capped", Some(16), 30),
        ("stays capped", Some(30), 30),
    ];
    for &(case, previous, want) in cases {
        assert_eq!(
            reconnect_delay(previous.and_then(seconds)),
            Duration::from_secs(want),
            "{case}"
        );
    }
}

#[test]
fn the_waits_from_the_first_run_one_two_four_eight_sixteen_thirty() {
    let waits: Vec<u64> = std::iter::successors(Some(reconnect_delay(None)), |previous| {
        Some(reconnect_delay(Some(*previous)))
    })
    .take(7)
    .map(|wait| wait.as_secs())
    .collect();
    assert_eq!(waits, [1, 2, 4, 8, 16, 30, 30]);
}
