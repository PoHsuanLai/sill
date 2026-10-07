//! The notification parts as markup: every specimen matches its golden under
//! `tests/snapshots/notifications/`, lints clean, and uses only `ds-` classes the stylesheet
//! styles.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test notifications_ssr` rewrites the goldens.

#[path = "notifications/cases.rs"]
mod cases;
#[path = "support/golden.rs"]
mod golden;

use dioxus::prelude::*;
use ds_lint::{LintConfig, markup};

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(make);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn every_specimen_matches_its_golden() {
    let failures: Vec<String> = cases::SPECIMENS
        .iter()
        .filter_map(|(name, make)| {
            golden::check(&format!("notifications/{name}.html"), &render(*make)).err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_specimen_lints_clean_and_every_class_is_styled() {
    let sheet = sill_shell_kit::stylesheet();
    let mut failures = Vec::new();
    for (name, make) in cases::SPECIMENS {
        let html = render(*make);
        for offence in markup(&html, sheet, &LintConfig::new(&sill_shell_kit::kits())) {
            failures.push(format!("{name}: {:?} {}", offence.rule, offence.text));
        }
        for class in html
            .split("class=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .flat_map(str::split_whitespace)
            .filter(|class| class.starts_with("ds-"))
        {
            let needle = format!(".{class}");
            let styled = sheet.match_indices(&needle).any(|(at, _)| {
                !sheet[at + needle.len()..]
                    .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            });
            if !styled {
                failures.push(format!("{name}: .{class} is not styled"));
            }
        }
    }
    failures.dedup();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A body's link is an anchor with its target, and the toned runs carry their tone words.
#[test]
fn a_rich_body_draws_its_tones_and_links() {
    let html = render(cases::rich_runs);
    for want in [
        "data-tone=\"italic\"",
        "data-tone=\"underline\"",
        "data-tone=\"strong\"",
        "<a class=\"ds-run-link\" href=\"https://ci.example/runs/42\"",
    ] {
        assert!(html.contains(want), "{want} in {html}");
    }
}
