//! What the session says about the release the shell runs from: whether it fell back to the safe
//! compositor, and where the release's own programs are. Both are read once at the edge
//! ([`crate::env::Env::from_process`]).

use std::path::{Path, PathBuf};

/// The environment variable the desktop session sets to say how it started the compositor.
pub const SESSION_MODE_VAR: &str = "QUIRE_SESSION_MODE";

/// How the session started: with its usual compositor, or with the safe one it falls back to
/// when the usual one keeps exiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionMode {
    #[default]
    Normal,
    /// The session fell back: a shell running here says nothing about whether the release works.
    Safe,
}

impl SessionMode {
    /// The mode `value` of [`SESSION_MODE_VAR`] names: `safe` is [`SessionMode::Safe`]; unset,
    /// empty or any other word is [`SessionMode::Normal`].
    pub fn from_value(value: Option<&str>) -> SessionMode {
        match value {
            Some("safe") => SessionMode::Safe,
            Some(_) | None => SessionMode::Normal,
        }
    }

    /// The mode of this process's environment.
    pub fn from_process() -> SessionMode {
        SessionMode::from_value(std::env::var(SESSION_MODE_VAR).ok().as_deref())
    }
}

/// The folder of the program this process runs: the `bin` of its release, which holds the
/// release's other programs beside it. `None` when the system does not say.
pub fn running_bin_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.parent().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::SessionMode;

    #[test]
    fn only_the_word_safe_is_safe_mode() {
        const CASES: &[(Option<&str>, SessionMode)] = &[
            (None, SessionMode::Normal),
            (Some(""), SessionMode::Normal),
            (Some("normal"), SessionMode::Normal),
            (Some("safe"), SessionMode::Safe),
            (Some("SAFE"), SessionMode::Normal),
            (Some("safe "), SessionMode::Normal),
            (Some("unsafe"), SessionMode::Normal),
            (Some("1"), SessionMode::Normal),
        ];
        for &(value, want) in CASES {
            assert_eq!(SessionMode::from_value(value), want, "{value:?}");
        }
    }
}
