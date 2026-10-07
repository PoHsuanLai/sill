//! design/26's lint on quire's own sheets, one sheet at a time: no stylesheet loops
//! (`Rule::InfiniteLoop`) and every component times its motion with the grammar's tokens
//! (`Rule::OffGrammarTiming`, `Profile::Details`), except the sheets listed here, each with its
//! reason. A sheet on the list that no longer offends is a failure too, so the list only shrinks.

use ds_lint::{LintConfig, Profile, Rule, stylesheet};

/// Sheets allowed to break the details rules, and why. Each is mail's (mailo pins quire by tag,
/// so these change when mailo decides, design/05 section 12 item 4), or the keyframe table
/// itself (`a-turn` and the symbol effects' loops).
const ALLOWED: &[(&str, Rule, &str)] = &[
    (
        "motion",
        Rule::InfiniteLoop,
        "`a-turn`: a busy button's icon turns at a constant speed for as long as it is busy; a stepped clock jumps 30 degrees at a time, which reads as choppy on an arrow glyph. The `a-*-loop` classes are the symbol effects' `While` run mode (design/35 section 2): the effect runs for as long as its owner says it is active, so it has no count; Reduced runs it once",
    ),
    (
        "motion",
        Rule::OffGrammarTiming,
        "`a-turn` and the symbol loops: their period is `--t-turn`, a speed rather than a transition",
    ),
];

fn sheets() -> Vec<(&'static str, String)> {
    // The components' and the details' own sheets (the widgets' are linted with the widgets),
    // then the keyframes section, cut from the stylesheet.
    let mut all: Vec<(&str, String)> = sill_shell_kit::component_sheets()
        .iter()
        .filter(|(name, _)| !name.starts_with("widget_"))
        .map(|(name, css)| (*name, (*css).to_owned()))
        .collect();
    let sheet = sill_shell_kit::stylesheet();
    let motion = sheet
        .split_once("/* == motion == */\n")
        .and_then(|(_, rest)| rest.split_once("/* == utilities == */"))
        .map(|(motion, _)| motion.to_owned())
        .unwrap_or_else(|| panic!("the stylesheet has no motion section"));
    all.push(("motion", motion));
    all
}

fn details() -> LintConfig {
    LintConfig {
        profile: Profile::Details,
        ..LintConfig::new(&sill_shell_kit::kits())
    }
}

#[test]
fn no_sheet_loops_or_times_off_the_grammar_but_the_listed_ones() {
    let mut failures = Vec::new();
    let mut used = Vec::new();
    for (name, css) in sheets() {
        for offence in stylesheet(&css, &details()) {
            if !matches!(offence.rule, Rule::InfiniteLoop | Rule::OffGrammarTiming) {
                continue;
            }
            match ALLOWED
                .iter()
                .find(|(sheet, rule, _)| *sheet == name && *rule == offence.rule)
            {
                Some(allowed) => used.push((allowed.0, allowed.1)),
                None => failures.push(format!("{name}: {:?} {}", offence.rule, offence.text)),
            }
        }
    }
    for (sheet, rule, _) in ALLOWED {
        if !used.contains(&(*sheet, *rule)) {
            failures.push(format!(
                "{sheet}: {rule:?} is allowed but no longer offends"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_details_primitives_and_the_progress_indicator_never_loop() {
    for (name, css) in sheets() {
        if !(name.starts_with("detail_") || name == "spinner" || name == "progress") {
            continue;
        }
        let offences: Vec<_> = stylesheet(&css, &details())
            .into_iter()
            .filter(|offence| matches!(offence.rule, Rule::InfiniteLoop | Rule::OffGrammarTiming))
            .collect();
        assert!(
            offences.is_empty(),
            "{name}: {:?}",
            offences.iter().map(|o| &o.text).collect::<Vec<_>>()
        );
    }
}
