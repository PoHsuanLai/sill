//! `sill style check`: its report per stylesheet, its exit code, and its grammar.

use std::process::ExitCode;

use clap::Parser;

use super::{StyleAction, check};
use crate::cli::{Cli, Command};

/// A stylesheet, the report lines it earns under the name `s.css`, and whether it exits
/// non-zero.
struct Case {
    name: &'static str,
    css: &'static str,
    lines: &'static [&'static str],
    fails: bool,
}

const CASES: &[Case] = &[
    Case {
        name: "an empty file",
        css: "",
        lines: &["s.css: nothing to report"],
        fails: false,
    },
    Case {
        name: "token overrides are clean",
        css: ".ds { --accent: red }",
        lines: &["s.css: nothing to report"],
        fails: false,
    },
    Case {
        name: "a note carries its line and column and does not fail",
        css: ".ds {\n  color: var(--nope);\n}",
        lines: &[
            "s.css:2:14: note: `--nope` is not a design-system token or declared here",
            "s.css: 1 note(s)",
        ],
        fails: false,
    },
    Case {
        name: "text that is not CSS fails",
        css: ".ds { color: red }\n}",
        lines: &[
            "s.css:2:1: error: `}` closes nothing",
            "s.css: 1 note(s), 1 not well-formed CSS",
        ],
        fails: true,
    },
];

#[test]
fn each_stylesheet_prints_its_report_and_exits_by_its_parse_errors() {
    for case in CASES {
        let report = check(case.css);
        assert_eq!(report.lines("s.css"), case.lines, "{}", case.name);
        assert_eq!(
            report.exit_code() == ExitCode::FAILURE,
            case.fails,
            "{}",
            case.name
        );
    }
}

#[test]
fn the_shells_own_surfaces_are_public_selectors() {
    let report = check("[*|data-surface=bar] .ds-button { color: red }");
    assert_eq!(report.notes, vec![]);
}

#[test]
fn the_grammar_takes_an_optional_file() {
    let parsed = |args: &[&str]| {
        let mut argv = vec!["sill"];
        argv.extend_from_slice(args);
        Cli::try_parse_from(argv).map(|cli| cli.command)
    };
    assert_eq!(
        parsed(&["style", "check"]).ok(),
        Some(Some(Command::Style {
            action: StyleAction::Check { file: None }
        }))
    );
    assert_eq!(
        parsed(&["style", "check", "a.css"]).ok(),
        Some(Some(Command::Style {
            action: StyleAction::Check {
                file: Some("a.css".into())
            }
        }))
    );
    assert!(parsed(&["style"]).is_err(), "a verb is required");
    assert!(
        parsed(&["style", "check", "a", "b"]).is_err(),
        "one file at most"
    );
}
