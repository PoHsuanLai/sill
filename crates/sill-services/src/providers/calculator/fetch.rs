//! The ECB daily file, off the keystroke path (M9 freeze F650): `refresh_if_due` starts a
//! background fetch at most once a day and lands it in the shared rates once it arrives;
//! nothing here runs synchronously inside a query.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use sill_launcher::UnixTime;
use tokio::process::Command;

use super::rates::{ECB_DAILY_URL, Rates, SharedRates, parse_ecb};

/// `$XDG_CACHE_HOME/sill/ecb-rates.xml` (F650).
pub fn cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sill")
        .join("ecb-rates.xml")
}

/// Whether `path`'s rates are worth fetching again: missing, unreadable, or a day (or more)
/// old. Never touches the network itself.
pub fn due(path: &Path, now: UnixTime) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return true;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    let Ok(age) = modified.duration_since(std::time::UNIX_EPOCH) else {
        return true;
    };
    now.days_since(UnixTime(age.as_secs())) >= 1
}

/// Set while a fetch is in flight, so two keystrokes in a row never start a second one.
static FETCHING: AtomicBool = AtomicBool::new(false);

/// If `path` is [`due`], fetch the ECB's daily file with `curl` on its own task and land it in
/// `rates` once it arrives; otherwise (or while one is already in flight) does nothing. Never
/// blocks its caller: the network call happens after this function has already returned.
pub fn refresh_if_due(rates: SharedRates, path: PathBuf, now: UnixTime) {
    if !due(&path, now) {
        return;
    }
    if FETCHING.swap(true, Ordering::AcqRel) {
        return;
    }
    tokio::spawn(async move {
        let _ = fetch_into(&path).await;
        if let Some(loaded) = load(&path)
            && let Ok(mut guard) = rates.lock()
        {
            *guard = loaded;
        }
        FETCHING.store(false, Ordering::Release);
    });
}

/// Runs `curl` into a temporary file beside `path`, then renames it in: a reader never sees a
/// half-written file.
async fn fetch_into(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    let temp = path.with_extension("xml.tmp");
    let status = Command::new("curl")
        .arg("--fail")
        .arg("--silent")
        .arg("--show-error")
        .arg("--output")
        .arg(&temp)
        .arg(ECB_DAILY_URL)
        .status()
        .await?;
    if status.success() {
        tokio::fs::rename(&temp, path).await?;
    }
    Ok(())
}

/// `path`'s rates, if the file exists and parses; a missing or damaged file is no rates, never
/// a panic (a lost conversion is not worth a failed launcher).
pub fn load(path: &Path) -> Option<Rates> {
    let text = std::fs::read_to_string(path).ok()?;
    parse_ecb(&text).ok()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::SystemTime;

    use sill_launcher::UnixTime;

    use super::due;

    #[test]
    fn a_missing_file_is_always_due() {
        assert!(due(
            Path::new("/nonexistent/sill-test/ecb-rates.xml"),
            UnixTime(1_790_000_000)
        ));
    }

    #[test]
    fn a_file_written_today_is_not_due_a_day_old_one_is() {
        let dir = std::env::temp_dir().join(format!("sill-rates-due-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let path = dir.join("ecb-rates.xml");
        std::fs::write(&path, "<x/>").unwrap_or_else(|e| panic!("{e}"));

        let now = UnixTime(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
        assert!(!due(&path, now), "just written");
        assert!(due(&path, UnixTime(now.0 + 2 * 86_400)), "two days later");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
