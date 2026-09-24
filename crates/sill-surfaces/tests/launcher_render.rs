//! The panel's picture of fake results, in a `ds_native::Harness`: groups in order under their
//! headings, the top hit first, the empty query's Actions and Recent, the empty state, and the
//! row hints (`↵` on the top hit, `⌃K` where a row has actions).

mod launcher_harness;

use launcher_harness::{open, retrying_the_focus_race, selected_title, titles};

fn headings(html: &str) -> Vec<String> {
    [
        "Top hit",
        "Applications",
        "Windows",
        "Calculator",
        "Settings",
        "System",
        "Actions",
        "Recent",
    ]
    .iter()
    .filter_map(|h| html.find(&format!(">{h}<")).map(|at| (at, (*h).to_owned())))
    .collect::<std::collections::BTreeMap<_, _>>()
    .into_values()
    .collect()
}

#[test]
fn a_query_groups_under_the_top_hit() {
    retrying_the_focus_race(|| {
        let open = open("fi");
        let html = open.harness.html();
        assert_eq!(
            headings(&html),
            vec!["Top hit", "Applications", "Windows", "Settings"]
        );
        assert_eq!(
            titles(&open.harness),
            vec!["Firefox", "Files", "Firefox — sill launcher", "Wi-Fi"]
        );
        assert!(
            html.contains("Web Browser"),
            "the subtitle is the row's detail"
        );
    });
}

#[test]
fn the_top_hit_is_selected_and_trails_enter() {
    retrying_the_focus_race(|| {
        let open = open("fi");
        let h = &open.harness;
        assert_eq!(selected_title(h).as_deref(), Some("Firefox"));
        let html = h.html();
        assert!(html.contains('↵'), "the top hit trails Enter");
        assert!(html.contains("⌃K"), "Files trails Ctrl+K for its actions");
    });
}

#[test]
fn the_empty_query_lists_actions_then_recent() {
    retrying_the_focus_race(|| {
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
    });
}

#[test]
fn nothing_matching_shows_the_empty_state() {
    retrying_the_focus_race(|| {
        let open = open("zzz");
        assert_eq!(open.harness.count(".ds-menu-item"), 0);
        assert_eq!(
            open.harness.text_of(".ds-menu-empty").as_deref(),
            Some("Nothing matches \"zzz\".")
        );
    });
}

#[test]
fn a_calculation_is_the_top_hit() {
    retrying_the_focus_race(|| {
        let open = open("6*7");
        assert_eq!(titles(&open.harness), vec!["= 42"]);
        assert_eq!(headings(&open.harness.html()), vec!["Top hit"]);
    });
}
