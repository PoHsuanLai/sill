//! The notification parts as markup: every specimen matches its golden under
//! `tests/snapshots/notifications/`, lints clean, and uses only `ds-` classes the stylesheet
//! styles.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it notifications_ssr` rewrites the goldens.

#[path = "notifications/cases.rs"]
mod cases;
use crate::support::specimens::Specimen;

use dioxus::prelude::*;

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(make);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

pub(crate) fn specimens() -> Vec<Specimen> {
    cases::SPECIMENS
        .iter()
        .map(|(name, make)| Specimen::of(&format!("notifications/{name}.html"), render(*make)))
        .collect()
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
