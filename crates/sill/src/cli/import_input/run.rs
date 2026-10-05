//! The effects half of `sill session import-input`: read `kcminputrc`, write cosmic-comp's
//! `input_devices` when it does not exist yet, and say what happened.

use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Args as ClapArgs;

use super::plasma::{import, render};
use crate::cli::stdout;

/// Where the imported settings go, relative to the config home.
const TARGET: &str = "cosmic/com.system76.CosmicComp/v1/input_devices";

/// The arguments of `sill session import-input`.
#[derive(Debug, Clone, PartialEq, Eq, ClapArgs)]
pub struct Args {
    /// Read Plasma's `$XDG_CONFIG_HOME/kcminputrc`: natural scroll, scroll factor and pointer
    /// acceleration, per device name.
    #[arg(long, required = true)]
    pub from_plasma: bool,
    /// Print what would be written (or merged) and write nothing.
    #[arg(long)]
    pub dry_run: bool,
}

/// Whether to touch the disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Write,
    DryRun,
}

/// What the run came to.
#[derive(Debug)]
enum Outcome {
    Wrote {
        path: PathBuf,
        ron: String,
    },
    WouldWrite {
        path: PathBuf,
        ron: String,
    },
    /// The file exists: nothing was changed; this is what a merge would add.
    Exists {
        path: PathBuf,
        ron: String,
    },
    NothingToImport {
        source: PathBuf,
    },
    NoSource {
        source: PathBuf,
        error: io::Error,
    },
    Failed {
        path: PathBuf,
        error: io::Error,
    },
}

impl Outcome {
    /// The run could not do what was asked (an existing file is not that: it is left alone).
    fn failed(&self) -> bool {
        matches!(self, Outcome::NoSource { .. } | Outcome::Failed { .. })
    }

    fn exit_code(&self) -> ExitCode {
        match self {
            Outcome::Wrote { .. }
            | Outcome::WouldWrite { .. }
            | Outcome::NothingToImport { .. } => ExitCode::SUCCESS,
            Outcome::Exists { .. } => ExitCode::from(2),
            Outcome::NoSource { .. } | Outcome::Failed { .. } => ExitCode::FAILURE,
        }
    }

    /// What to tell the person: the first line says what happened, the rest is the file's text.
    fn report(&self) -> String {
        match self {
            Outcome::Wrote { path, ron } => format!("wrote {}\n{ron}\n", path.display()),
            Outcome::WouldWrite { path, ron } => {
                format!("would write {} (--dry-run)\n{ron}\n", path.display())
            }
            Outcome::Exists { path, ron } => format!(
                "{} exists and was left alone; merge these devices into it by hand:\n{ron}\n",
                path.display()
            ),
            Outcome::NothingToImport { source } => {
                format!("{}: no per-device settings to import\n", source.display())
            }
            Outcome::NoSource { source, error } => {
                format!("{}: {error}\n", source.display())
            }
            Outcome::Failed { path, error } => format!("{}: {error}\n", path.display()),
        }
    }
}

/// Run the import under the person's config home (`$XDG_CONFIG_HOME`, else `~/.config`): the
/// report on stdout, a failure on stderr.
pub fn main(args: &Args) -> ExitCode {
    let home = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    match home {
        Some(config_home) => main_in(args, &config_home),
        None => {
            eprintln!("sill: neither XDG_CONFIG_HOME nor HOME is set");
            ExitCode::FAILURE
        }
    }
}

fn main_in(args: &Args, config_home: &Path) -> ExitCode {
    let mode = match args.dry_run {
        true => Mode::DryRun,
        false => Mode::Write,
    };
    let outcome = run(config_home, mode);
    let report = outcome.report();
    if outcome.failed() {
        eprintln!("sill: {}", report.trim_end());
        return outcome.exit_code();
    }
    match stdout::print(&report) {
        stdout::Emitted::Failed(_) => ExitCode::FAILURE,
        stdout::Emitted::Done | stdout::Emitted::ReaderGone => outcome.exit_code(),
    }
}

fn run(config_home: &Path, mode: Mode) -> Outcome {
    let source = config_home.join("kcminputrc");
    let text = match fs::read_to_string(&source) {
        Ok(text) => text,
        Err(error) => return Outcome::NoSource { source, error },
    };
    let devices = import(&text);
    if devices.is_empty() {
        return Outcome::NothingToImport { source };
    }
    let path = config_home.join(TARGET);
    let ron = match render(&devices) {
        Ok(ron) => ron,
        Err(error) => {
            let error = io::Error::other(error);
            return Outcome::Failed { path, error };
        }
    };
    match (path.exists(), mode) {
        (true, _) => Outcome::Exists { path, ron },
        (false, Mode::DryRun) => Outcome::WouldWrite { path, ron },
        (false, Mode::Write) => match write_new(&path, &ron) {
            Ok(()) => Outcome::Wrote { path, ron },
            Err(error) if error.kind() == ErrorKind::AlreadyExists => Outcome::Exists { path, ron },
            Err(error) => Outcome::Failed { path, error },
        },
    }
}

/// Create `path` (and its directory) with `text`; an existing file is an `AlreadyExists` error.
fn write_new(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(text.as_bytes())
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
