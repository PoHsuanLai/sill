//! The panel's picture of fake results, in a `ds_harness::Harness`: groups in order under their
//! headings, the top hit first, the empty query's Actions and Recent, the empty state, and the
//! row chord (Spotlight's hint: only the selected row shows it, quire's `RowChord`).

use crate::launcher_harness;

use ds_harness::Input;
use std::time::Duration;

use ds::prelude::*;
use launcher_harness::{open, selected_title, titles};

fn headings(html: &str) -> Vec<String> {
    [
        "Top Hit",
        "Applications",
        "Windows",
        "Calculator",
        "Settings",
        "System",
        "Actions",
        "Recent",
    ]
    .iter()
    .filter_map(|h| {
        html.find(&format!("ds-section-header-title\">{h}<"))
            .map(|at| (at, (*h).to_owned()))
    })
    .collect::<std::collections::BTreeMap<_, _>>()
    .into_values()
    .collect()
}

/// The selected row's markup: from its `aria-selected="true"` to the end of its element.
fn selected_row(html: &str) -> String {
    let rest = html
        .split("aria-selected=\"true\"")
        .nth(1)
        .unwrap_or_default();
    rest[..rest.find("</div>").unwrap_or(rest.len())].to_owned()
}

#[test]
fn a_query_groups_under_the_top_hit() {
    let open = open("fi");
    let html = open.harness.html();
    assert_eq!(
        headings(&html),
        vec!["Top Hit", "Applications", "Settings", "Windows"]
    );
    assert_eq!(
        titles(&open.harness),
        vec!["Firefox", "Files", "Wi-Fi", "Firefox — sill launcher"]
    );
    assert!(
        html.contains("Web Browser"),
        "the subtitle is the row's detail"
    );
}

/// Only the selected row shows its chord, as plain text (`.ds-key-equivalent`, no key caps), and
/// the chord moves with the selection; the other rows show only their data.
#[test]
fn only_the_selected_row_shows_its_chord() {
    let mut open = open("fi");
    let h = &open.harness;
    assert_eq!(selected_title(h).as_deref(), Some("Firefox"));
    assert_eq!(
        h.count(".ds-key-equivalent"),
        1,
        "one chord, on the selection"
    );
    assert!(
        selected_row(&h.html()).contains("ds-key-equivalent"),
        "on the selected row"
    );
    assert_eq!(
        h.count(".ds-row .ds-key-equivalent-key"),
        0,
        "no key caps on a row"
    );
    let top = h
        .text_of(".ds-key-equivalent")
        .expect("the top hit's chord");
    assert!(!top.is_empty(), "the top hit's chord");
    open.harness.send(Input::key(ShortcutKey::Down));
    open.harness.advance(Duration::from_millis(40));
    let h = &open.harness;
    assert_eq!(selected_title(h).as_deref(), Some("Files"));
    assert_eq!(
        h.count(".ds-key-equivalent"),
        1,
        "the chord moved, not copied"
    );
    assert!(
        selected_row(&h.html()).contains("ds-key-equivalent"),
        "on the selected row"
    );
    assert_eq!(
        h.text_of(".ds-key-equivalent").as_deref(),
        Some("⌘K"),
        "Files has actions without a chord of their own"
    );
}

#[test]
fn the_empty_query_lists_actions_then_recent() {
    let open = open("");
    assert_eq!(headings(&open.harness.html()), vec!["Actions", "Recent"]);
    assert_eq!(
        titles(&open.harness),
        vec![
            "Lock Screen",
            "Log Out",
            "Restart",
            "Firefox",
            "Files",
            "Firefox — sill launcher",
            "Terminal",
            "Wi-Fi"
        ]
    );
}

#[test]
fn nothing_matching_shows_the_empty_state() {
    let open = open("zzz");
    assert_eq!(open.harness.count(".ds-row"), 0);
    assert_eq!(
        open.harness.text_of(".ds-palette-empty").as_deref(),
        Some("Nothing matches \"zzz\".")
    );
}

#[test]
fn a_calculation_is_the_top_hit() {
    let open = open("6*7");
    assert_eq!(titles(&open.harness), vec!["= 42"]);
    assert_eq!(headings(&open.harness.html()), vec!["Top Hit"]);
}
