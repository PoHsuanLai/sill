//! Coherence rules 1 and 2 for the polkit prompt: sill styles nothing
//! of it (no stylesheet of its own; quire's `.ds-polkit*` is quire's), and the real
//! `PolkitPrompt` goes through the markup lint hidden and showing a request.

mod polkit_prompt_support;

use std::time::Duration;

use ds::lint::{LintConfig, Profile, Rule, markup};
use polkit_prompt_support::{asking, fake, request, shown, start};
use sill_services::polkit_agent::LastTry;

fn strict() -> LintConfig {
    LintConfig {
        profile: Profile::Strict,
        ..LintConfig::default()
    }
}

#[test]
fn the_hidden_prompt_markup_is_clean() {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(40));
    let offences = markup(&running.harness.html(), ds::stylesheet(), &strict());
    assert!(offences.is_empty(), "{offences:#?}");
}

#[test]
fn the_shown_prompt_markup_is_clean_and_has_no_raw_markup() {
    let running = shown(request("c1", asking(), LastTry::None));
    let offences = markup(&running.harness.html(), ds::stylesheet(), &strict());
    assert!(
        offences.iter().all(|o| o.rule != Rule::RawMarkup),
        "{offences:#?}"
    );
    assert!(offences.is_empty(), "{offences:#?}");
}
