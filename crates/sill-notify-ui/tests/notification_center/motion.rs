//! The notification center's list motion (design/26 5.4, design/20 1.6), on the
//! virtual clock with a pinned appearance: the center's rows are one quire `LeavingList`. They
//! are there as the center opens; the cards an opened group adds enter; "Show less", Clear and
//! Clear All fade the rows the model dropped and the rows below close the gap; an arrival
//! while the center is open enters. Every one of them ends at rest, asking for no frame.

use super::{Running, advance, commands_until, fake, note, settle, start, two_apps, until};
use ds_harness::Harness;
use ds_harness::harness::assert_settles_to_zero_frames;
use sill_model::notifications::{AppGroup, AppKey, Clear, History, NotificationsCommand};
use sill_settings::ShellFile;

/// A group's toggle ("N more", "Show less") and Clear, in its header.
const TOGGLE: &str = ".ds-group-header-actions .ds-button:first-child";
const CLEAR: &str = ".ds-group-header-actions .ds-button:last-child";

/// The list, and its rows.
const LIST: &str = ".sill-nc-list > .ds-list";
const ROW: &str = ".ds-list-item";

/// The center opened over `history`, as soon as its panel is drawn.
fn opening(history: History) -> Running {
    let mut running = start(ShellFile::default(), history);
    advance(&mut running, 40);
    running.link.open(fake::output().id);
    settle(&mut running.harness, |h| h.count(".ds-side-panel") == 1);
    running
}

/// How many rows are in `presence` (`entering`, `present`, `leaving`, `healing`).
fn rows_in(h: &Harness, presence: &str) -> usize {
    h.count(&format!("{ROW}[*|data-presence={presence}]"))
}

/// The rows the list draws.
fn rows(h: &Harness) -> usize {
    h.count(ROW)
}

/// Opened over [`two_apps`], at rest: Mail's head and stack, Chat's head and card.
fn at_rest() -> Running {
    let mut running = opening(two_apps());
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(rows(&running.harness), 4);
    running
}

/// A group's control, `control`, in the header of `group`.
fn in_head(group: &str, control: &str) -> String {
    format!(".sill-nc-item[*|data-group={group}] {control}")
}

#[test]
fn opening_shows_the_rows_at_once_then_rests() {
    let mut running = opening(two_apps());
    let h = &running.harness;
    assert_eq!(
        rows_in(h, "present"),
        4,
        "the rows are simply there as the center opens"
    );
    assert_eq!(rows_in(h, "entering"), 0);
    assert_settles_to_zero_frames(&mut running.harness);
    // A re-render (the same history again) replays nothing.
    running
        .feeds
        .notifications
        .send_modify(|n| n.history = two_apps());
    advance(&mut running, 40);
    assert_eq!(rows_in(&running.harness, "entering"), 0);
    assert_settles_to_zero_frames(&mut running.harness);
}

#[test]
fn opening_a_group_enters_the_cards_it_adds_and_show_less_removes_and_closes_the_gap() {
    let mut running = at_rest();
    let toggle = in_head("mail", TOGGLE);
    let more = running.harness.centre(&toggle).expect("N more");
    running.harness.click(more);
    until(&mut running, |h| rows(h) == 6);
    let h = &running.harness;
    assert_eq!(rows_in(h, "entering"), 2, "Mail's two older cards enter");
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(rows_in(&running.harness, "present"), 6);
    // "Show less": the two cards it hides fold as one batch; Chat's rows then heal up.
    let less = running.harness.centre(&toggle).expect("Show less");
    running.harness.click(less);
    until(&mut running, |h| rows_in(h, "leaving") == 2);
    assert_eq!(
        running
            .harness
            .count(&format!("{ROW}[*|data-presence=leaving][*|data-exit=row]")),
        2,
        "they fade and slide up"
    );
    assert_eq!(rows(&running.harness), 6, "still drawn while they fold");
    until(&mut running, |h| rows_in(h, "healing") == 2);
    assert_eq!(rows(&running.harness), 4, "dropped together");
    let dy = heal_distance(&running.harness);
    assert!(dy > 100.0, "Chat heals by the two cards' height: {dy}");
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(rows_in(&running.harness, "present"), 4);
}

/// The first healing row's `--dy`, in px.
fn heal_distance(h: &Harness) -> f32 {
    let style = h
        .attr(&format!("{ROW}[*|data-presence=healing]"), "style")
        .unwrap_or_default();
    style
        .split(';')
        .find_map(|part| part.trim().strip_prefix("--dy:"))
        .and_then(|px| px.trim().trim_end_matches("px").parse().ok())
        .unwrap_or(0.0)
}

#[test]
fn the_rows_keep_one_gap_inside_themselves() {
    let mut running = at_rest();
    let more = running
        .harness
        .centre(&in_head("mail", TOGGLE))
        .expect("N more");
    running.harness.click(more);
    until(&mut running, |h| rows(h) == 6);
    assert_settles_to_zero_frames(&mut running.harness);
    let h = &running.harness;
    let tops: Vec<(f32, f32)> = (1..=6)
        .map(|n| {
            let rect = h
                .rect(&format!("{ROW}:nth-child({n}) > .sill-nc-item > *"))
                .expect("a row's content");
            (rect.origin.y.0, rect.origin.y.0 + rect.size.height.0)
        })
        .collect();
    let gaps: Vec<f32> = tops.windows(2).map(|w| w[1].0 - w[0].1).collect();
    assert!(
        gaps.iter()
            .all(|gap| *gap > 0.5 && (gap - gaps[0]).abs() < 0.5),
        "one gap between every row, head or card: {gaps:?}"
    );
}

#[test]
fn clear_fades_the_group_and_the_groups_below_close_the_gap() {
    let mut running = at_rest();
    let clear = running
        .harness
        .centre(&in_head("mail", CLEAR))
        .expect("Mail's Clear");
    running.harness.click(clear);
    assert_eq!(
        commands_until(&mut running, 1),
        vec![NotificationsCommand::ClearHistory(Clear::App(AppKey(
            "mail".to_owned()
        )))]
    );
    // The service's one update: Mail's group is gone.
    let mut left = two_apps();
    left.groups.remove(0);
    running
        .feeds
        .notifications
        .send_modify(|n| n.history = left);
    until(&mut running, |h| rows_in(h, "leaving") == 2);
    let h = &running.harness;
    assert_eq!(rows(h), 4, "Mail's head and stack fade where they stood");
    assert_eq!(
        h.attr(&format!("{ROW}[*|data-presence=leaving]"), "data-exit")
            .as_deref(),
        Some("row")
    );
    until(&mut running, |h| rows_in(h, "healing") == 2);
    assert_eq!(rows(&running.harness), 2);
    assert!(heal_distance(&running.harness) > 60.0, "Chat heals up");
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(running.harness.count(".sill-nc-group-head"), 1);
}

#[test]
fn clear_all_fades_every_row_together_then_says_the_list_is_empty() {
    let mut running = at_rest();
    let all = running
        .harness
        .centre(".sill-nc-head .ds-button")
        .expect("Clear All");
    running.harness.click(all);
    assert_eq!(
        commands_until(&mut running, 1),
        vec![NotificationsCommand::ClearHistory(Clear::All)]
    );
    running
        .feeds
        .notifications
        .send_modify(|n| n.history = History::default());
    until(&mut running, |h| rows_in(h, "leaving") == 4);
    let h = &running.harness;
    assert_eq!(h.count(".sill-nc-empty"), 0, "not under the leaving rows");
    let styles: Vec<Option<String>> = (1..=4)
        .map(|n| h.attr(&format!("{ROW}:nth-child({n})"), "style"))
        .collect();
    assert_eq!(styles, vec![None; 4], "all together, none after another");
    until(&mut running, |h| h.count(".sill-nc-empty") == 1);
    assert_eq!(rows(&running.harness), 0);
    assert_settles_to_zero_frames(&mut running.harness);
    // Clear All went with the history; the column has the keyboard again.
    assert!(running.harness.is_focused(".sill-nc-column"));
}

#[test]
fn a_clear_gives_the_column_the_keyboard_back() {
    let mut running = at_rest();
    let clear = running
        .harness
        .centre(&in_head("chat", CLEAR))
        .expect("Chat's Clear");
    running.harness.click(clear);
    let mut left = two_apps();
    left.groups.truncate(1);
    running
        .feeds
        .notifications
        .send_modify(|n| n.history = left);
    until(&mut running, |h| rows(h) == 2);
    assert_settles_to_zero_frames(&mut running.harness);
    assert!(
        running.harness.is_focused(".sill-nc-column"),
        "Escape reaches the column, not the gone button"
    );
}

#[test]
fn an_arrival_while_open_enters() {
    let mut running = at_rest();
    let mut more = two_apps();
    more.groups.insert(
        0,
        AppGroup {
            key: AppKey("ci".to_owned()),
            notes: vec![note(9, "CI", "Passed")],
        },
    );
    running
        .feeds
        .notifications
        .send_modify(|n| n.history = more);
    until(&mut running, |h| rows(h) == 6);
    let h = &running.harness;
    assert_eq!(rows_in(h, "entering"), 2, "CI's head and card enter");
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(rows_in(&running.harness, "present"), 6);
}
