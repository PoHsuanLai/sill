//! `sill style check [FILE]`: what `ds_lint::user_stylesheet` says about the person's `style.css`
//! (or `FILE`), one line per note with its line and column. Reading the file is the only effect;
//! the report is [`check`], pure over the text. It never asks the daemon, which loads the file
//! whatever this says. The exit code is non-zero only for text that is not well-formed CSS.

use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Subcommand;
use ds_lint::{UserStyleNote, user_stylesheet};
use ds_settings::{ConfigRoot, Store, UserStyle};
use sill_settings::APP;

/// `sill style ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum StyleAction {
    /// Report what is wrong or worth knowing in your style.css: text that is not CSS (exit 1),
    /// variables nothing declares, selectors outside the public surface, `!important`, remote
    /// URLs. The daemon loads the file whatever this says.
    Check {
        /// The file to check; your `style.css` in sill's config directory when left out.
        file: Option<PathBuf>,
    },
}

/// Where the file to check came from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    /// Named on the command line: it must exist.
    Given(PathBuf),
    /// The person's `style.css`: a missing one is an empty style, which has nothing to say.
    Own(PathBuf),
}

/// What [`check`] found in a stylesheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleCheck {
    /// Every note, in source order.
    pub notes: Vec<UserStyleNote>,
}

/// The notes on `css` against the shell's own kits.
pub fn check(css: &str) -> StyleCheck {
    StyleCheck {
        notes: user_stylesheet(css, &ds_shell::kits()),
    }
}

impl StyleCheck {
    /// How many notes are text that is not well-formed CSS.
    pub fn parse_errors(&self) -> usize {
        self.notes.iter().filter(|n| n.is_parse_error()).count()
    }

    /// The report: `name:line:column: ...` per note, then a summary line.
    pub fn lines(&self, name: &str) -> Vec<String> {
        let errors = self.parse_errors();
        let summary = match (self.notes.len(), errors) {
            (0, _) => format!("{name}: nothing to report"),
            (all, 0) => format!("{name}: {all} note(s)"),
            (all, errors) => format!("{name}: {all} note(s), {errors} not well-formed CSS"),
        };
        self.notes
            .iter()
            .map(|note| format!("{name}:{note}"))
            .chain([summary])
            .collect()
    }

    /// Non-zero only when some text is not well-formed CSS.
    pub fn exit_code(&self) -> ExitCode {
        match self.parse_errors() {
            0 => ExitCode::SUCCESS,
            _ => ExitCode::FAILURE,
        }
    }
}

/// Run `action`: read the file and print its report.
pub fn main(action: &StyleAction) -> ExitCode {
    let StyleAction::Check { file } = action;
    let source = match file {
        Some(path) => Source::Given(path.clone()),
        None => match Store::new(ConfigRoot::Xdg, APP).path::<UserStyle>() {
            Some(path) => Source::Own(path),
            None => {
                eprintln!("sill: no config directory to find style.css in");
                return ExitCode::from(2);
            }
        },
    };
    let (Source::Given(path) | Source::Own(path)) = &source;
    let name = path.display().to_string();
    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(css) => {
                let report = check(&css);
                report
                    .lines(&name)
                    .iter()
                    .for_each(|line| println!("{line}"));
                report.exit_code()
            }
            Err(error) => {
                eprintln!("sill: {name}: not UTF-8, the daemon keeps the last good text: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) if error.kind() == ErrorKind::NotFound && matches!(source, Source::Own(_)) => {
            println!("{name}: no file, an empty style");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("sill: {name}: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[path = "style_tests.rs"]
mod tests;
