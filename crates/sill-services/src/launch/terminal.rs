//! Which terminal wraps a `Terminal=true` entry's `Exec` (plan "Launch": "Terminal=true
//! wrapper"). `$TERMINAL`, if it names something on `PATH`, else the first of ghostty, wezterm,
//! foot, cosmic-term found there (FINDINGS.md "M2 launch": the user prefers ghostty, and it is
//! what this machine has).

use std::path::{Path, PathBuf};

/// The binaries tried after `$TERMINAL`, in order.
const FALLBACKS: [&str; 4] = ["ghostty", "wezterm", "foot", "cosmic-term"];

/// `env_terminal` (trimmed, if non-blank) then [`FALLBACKS`], first occurrence kept.
fn candidates(env_terminal: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    let named = env_terminal
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .into_iter()
        .chain(FALLBACKS);
    for name in named {
        if !out.iter().any(|kept: &String| kept == name) {
            out.push(name.to_owned());
        }
    }
    out
}

/// The first of `candidates` found as an executable file in `path_dirs`, wrapped `[bin, "-e"]`;
/// empty when none is (the caller then runs the bare `Exec`, unwrapped).
fn resolve(candidates: &[String], path_dirs: &[PathBuf]) -> Vec<String> {
    candidates
        .iter()
        .find(|bin| path_dirs.iter().any(|dir| is_executable(&dir.join(bin))))
        .map(|bin| vec![bin.clone(), "-e".to_owned()])
        .unwrap_or_default()
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// The terminal wrapper for this machine, read from `$TERMINAL` and `$PATH`.
pub fn detect() -> Vec<String> {
    let env_terminal = std::env::var("TERMINAL").ok();
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    resolve(&candidates(env_terminal.as_deref()), &path_dirs)
}

#[cfg(test)]
mod tests {
    use super::{candidates, resolve};
    use crate::launch::test_support::TempDir;

    #[test]
    fn candidates_put_the_env_terminal_first_and_drop_blanks_and_duplicates() {
        assert_eq!(
            candidates(None),
            vec!["ghostty", "wezterm", "foot", "cosmic-term"]
        );
        assert_eq!(
            candidates(Some("  ")),
            vec!["ghostty", "wezterm", "foot", "cosmic-term"],
            "a blank $TERMINAL is ignored"
        );
        assert_eq!(
            candidates(Some(" alacritty ")),
            vec!["alacritty", "ghostty", "wezterm", "foot", "cosmic-term"],
            "trimmed and first"
        );
        assert_eq!(
            candidates(Some("foot")),
            vec!["foot", "ghostty", "wezterm", "cosmic-term"],
            "a duplicate of a fallback is not repeated"
        );
    }

    fn make_executable(dir: &TempDir, name: &str) {
        let path = dir.path().join(name);
        std::fs::write(&path, b"#!/bin/sh\n").unwrap_or_else(|e| panic!("{e}"));
        let mut perms = std::fs::metadata(&path)
            .unwrap_or_else(|e| panic!("{e}"))
            .permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        std::fs::set_permissions(&path, perms).unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn resolve_picks_the_first_candidate_present_and_executable() {
        let dir = TempDir::new("terminal-resolve");
        make_executable(&dir, "foot");
        std::fs::write(dir.path().join("cosmic-term"), b"not executable")
            .unwrap_or_else(|e| panic!("{e}"));

        let path_dirs = vec![dir.path().to_path_buf()];
        assert_eq!(
            resolve(&["ghostty".to_owned(), "foot".to_owned()], &path_dirs),
            vec!["foot".to_owned(), "-e".to_owned()],
            "ghostty is missing, foot is present"
        );
        assert_eq!(
            resolve(&["cosmic-term".to_owned()], &path_dirs),
            Vec::<String>::new(),
            "present but not executable does not count"
        );
        assert_eq!(
            resolve(&["nope".to_owned()], &path_dirs),
            Vec::<String>::new(),
            "nothing on PATH"
        );
    }
}
