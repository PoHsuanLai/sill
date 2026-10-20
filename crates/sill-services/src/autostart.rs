//! Open at Login: the freedesktop autostart entry `$XDG_CONFIG_HOME/autostart/<app id>.desktop`.
//!
//! Turning it on copies the app's own desktop entry there, with one line added under
//! `[Desktop Entry]` that says sill wrote it ([`MARK_LINE`]). Turning it off deletes the file only
//! when that line is in it: a file the person or another program put there is shown as on and
//! left alone. The state is always read from the folder, never remembered.

use std::path::{Path, PathBuf};

use sill_launcher::AppId;

use crate::env::Dirs;
use crate::error::ServiceError;

/// The key that marks a file sill wrote.
pub const MARK_KEY: &str = "X-Quire-Autostart";

/// The line added to a copied entry.
const MARK_LINE: &str = "X-Quire-Autostart=true";

/// The group header the mark goes under.
const HEADER: &str = "[Desktop Entry]";

/// Whether an app starts at login, as the autostart folder says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Login {
    /// No autostart entry for the app.
    Off,
    /// An entry sill wrote: sill may remove it.
    OurEntry,
    /// An entry someone else wrote: shown as on, never removed here.
    OthersEntry,
}

impl Login {
    /// Whether the app starts at login.
    pub fn is_on(self) -> bool {
        self != Login::Off
    }
}

/// `app`'s autostart file under `dirs`' config directory.
pub fn file_of(dirs: &Dirs, app: &AppId) -> PathBuf {
    dirs.config.join("autostart").join(format!("{}.desktop", app.0))
}

/// Whether `text` carries sill's mark.
pub fn is_marked(text: &str) -> bool {
    text.lines().any(|line| line.trim() == MARK_LINE)
}

/// `entry` with the mark under its `[Desktop Entry]` header; none when it has no such header.
pub fn with_mark(entry: &str) -> Option<String> {
    let mut out = String::with_capacity(entry.len() + MARK_LINE.len() + 2);
    let mut placed = false;
    for line in entry.split_inclusive('\n') {
        out.push_str(line);
        if !placed && line.trim_end() == HEADER {
            if !line.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(MARK_LINE);
            out.push('\n');
            placed = true;
        }
    }
    placed.then_some(out)
}

/// What the folder says about `app`.
pub fn login_of(dirs: &Dirs, app: &AppId) -> Login {
    match std::fs::read_to_string(file_of(dirs, app)) {
        Ok(text) if is_marked(&text) => Login::OurEntry,
        Ok(_) => Login::OthersEntry,
        // An unreadable file is somebody's: it is there, and not ours to remove.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Login::Off,
        Err(_) => Login::OthersEntry,
    }
}

/// Start `app` at login: copy its desktop entry at `entry` into the autostart folder. An entry
/// already there, ours or not, stays as it is.
pub fn turn_on(dirs: &Dirs, app: &AppId, entry: &Path) -> Result<(), ServiceError> {
    if login_of(dirs, app).is_on() {
        return Ok(());
    }
    let target = file_of(dirs, app);
    let text = std::fs::read_to_string(entry).map_err(|e| io(entry, e))?;
    let marked = with_mark(&text).ok_or_else(|| {
        io(
            entry,
            std::io::Error::new(std::io::ErrorKind::InvalidData, "no [Desktop Entry] group"),
        )
    })?;
    if let Some(folder) = target.parent() {
        std::fs::create_dir_all(folder).map_err(|e| io(folder, e))?;
    }
    std::fs::write(&target, marked).map_err(|e| io(&target, e))
}

/// Stop starting `app` at login: remove the entry when sill wrote it, and only then.
pub fn turn_off(dirs: &Dirs, app: &AppId) -> Result<(), ServiceError> {
    match login_of(dirs, app) {
        Login::OurEntry => {
            let target = file_of(dirs, app);
            std::fs::remove_file(&target).map_err(|e| io(&target, e))
        }
        Login::Off | Login::OthersEntry => Ok(()),
    }
}

/// [`turn_on`] on a thread of its own; a failure is logged, there being nobody to tell.
pub fn turn_on_in_background(dirs: Dirs, app: AppId, entry: PathBuf) {
    std::thread::spawn(move || {
        if let Err(error) = turn_on(&dirs, &app, &entry) {
            eprintln!("sill: Open at Login: {error}");
        }
    });
}

/// [`turn_off`] on a thread of its own; a failure is logged.
pub fn turn_off_in_background(dirs: Dirs, app: AppId) {
    std::thread::spawn(move || {
        if let Err(error) = turn_off(&dirs, &app) {
            eprintln!("sill: Open at Login: {error}");
        }
    });
}

fn io(path: &Path, source: std::io::Error) -> ServiceError {
    ServiceError::Io {
        path: path.to_path_buf(),
        source,
    }
}
