//! The ECB daily file, off the keystroke path: `refresh_if_due` starts a background fetch at most
//! once a day (or, after a failed attempt, at most once an hour — a
//! blocked or offline `curl` must not start again on every keystroke that follows) and lands it
//! in the shared rates once it arrives; nothing here runs synchronously inside a query.
//!
//! [`due`] is pure and table-tested; the two facts it decides from — the cache file's own
//! `mtime` and the last attempt's time and outcome — are read by [`refresh_if_due`], the only
//! effectful part, which also writes the attempt back (a sibling stamp file next to the cache;
//! `CalculatorProvider`'s own fields stay at `policy`/`rates`, so this needed no new one).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sill_launcher::UnixTime;
use tokio::process::Command;

use super::rates::{ECB_DAILY_URL, Rates, SharedRates, parse_ecb};

/// How long a failed attempt holds off the next one.
const FAILURE_BACKOFF_SECS: u64 = Duration::from_secs(3600).as_secs();

/// Why the last fetch attempt ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failure,
}

/// The last time a fetch was attempted, and how it went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempt {
    pub at: UnixTime,
    pub outcome: Outcome,
}

/// `ecb-rates.xml` in sill's folder of `dirs`' cache directory.
pub fn cache_path(dirs: &crate::env::Dirs) -> PathBuf {
    dirs.sill_cache().join("ecb-rates.xml")
}

/// The attempt stamp beside `cache_path`'s file: `ecb-rates.attempt`.
fn attempt_path(cache_path: &Path) -> PathBuf {
    cache_path.with_extension("attempt")
}

/// Whether it is worth trying again at `now`: no cache is always worth it; a successful
/// cache is worth it once it turns a day old; a failed attempt holds off retrying for
/// [`FAILURE_BACKOFF_SECS`] regardless of the cache's age, so an offline or blocked `curl`
/// tries again once an hour, not once a keystroke.
pub fn due(cached_at: Option<UnixTime>, last_attempt: Option<Attempt>, now: UnixTime) -> bool {
    if let Some(Attempt {
        at,
        outcome: Outcome::Failure,
    }) = last_attempt
        && now.0.saturating_sub(at.0) < FAILURE_BACKOFF_SECS
    {
        return false;
    }
    match cached_at {
        Some(cached_at) => now.days_since(cached_at) >= 1,
        None => true,
    }
}

/// Set while a fetch is in flight, so two keystrokes in a row never start a second one.
static FETCHING: AtomicBool = AtomicBool::new(false);

/// If [`due`] (reading `path`'s own `mtime` and the attempt stamp beside it), fetch the ECB's
/// daily file with `curl` on its own task, record the attempt (its outcome included) and land
/// a successful fetch in `rates`; otherwise (or while one is already in flight) does nothing.
/// Never blocks its caller: the network call happens after this function has already returned.
pub fn refresh_if_due(rates: SharedRates, path: PathBuf, now: UnixTime) {
    let cached_at = mtime(&path);
    let attempt_path = attempt_path(&path);
    let last_attempt = read_attempt(&attempt_path);
    if !due(cached_at, last_attempt, now) {
        return;
    }
    if FETCHING.swap(true, Ordering::AcqRel) {
        return;
    }
    tokio::spawn(async move {
        let outcome = match fetch_into(&path).await {
            Ok(()) => Outcome::Success,
            Err(_) => Outcome::Failure,
        };
        write_attempt(&attempt_path, Attempt { at: now, outcome });
        if let Some(loaded) = load(&path)
            && let Ok(mut guard) = rates.lock()
        {
            *guard = loaded;
        }
        FETCHING.store(false, Ordering::Release);
    });
}

/// Runs `curl` into a temporary file beside `path`, then renames it in: a reader never sees a
/// half-written file. `Err` on a non-zero exit (offline, blocked, a proxy refusing it), not
/// only on an I/O error, so [`refresh_if_due`] records it as a [`Outcome::Failure`] and backs
/// off.
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
    if !status.success() {
        return Err(std::io::Error::other(format!("curl exited {status}")));
    }
    tokio::fs::rename(&temp, path).await?;
    Ok(())
}

/// `path`'s rates, if the file exists and parses; a missing or damaged file is no rates, never
/// a panic (a lost conversion is not worth a failed launcher).
pub fn load(path: &Path) -> Option<Rates> {
    let text = std::fs::read_to_string(path).ok()?;
    parse_ecb(&text).ok()
}

/// `path`'s own last-modified time, if it exists and is readable.
fn mtime(path: &Path) -> Option<UnixTime> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let seconds = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(UnixTime(seconds))
}

/// `path`'s attempt stamp (`<unix seconds> success|failure`), if it exists and parses; a
/// missing or damaged stamp is no attempt, never a panic.
fn read_attempt(path: &Path) -> Option<Attempt> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut words = text.split_whitespace();
    let at = words.next()?.parse::<u64>().ok()?;
    let outcome = match words.next()? {
        "success" => Outcome::Success,
        "failure" => Outcome::Failure,
        _ => return None,
    };
    Some(Attempt {
        at: UnixTime(at),
        outcome,
    })
}

/// Writes `attempt` to `path`; failing to (a read-only cache directory) is not a reason to
/// panic a background task — it only means the next keystroke reads the same stale attempt
/// and tries again.
fn write_attempt(path: &Path, attempt: Attempt) {
    let outcome = match attempt.outcome {
        Outcome::Success => "success",
        Outcome::Failure => "failure",
    };
    let _ = std::fs::write(path, format!("{} {outcome}", attempt.at.0));
}

#[cfg(test)]
mod tests {
    use sill_launcher::UnixTime;

    use super::{Attempt, Outcome, due};

    const NOW: UnixTime = UnixTime(1_790_000_000);
    const HOUR: u64 = 3_600;
    const DAY: u64 = 86_400;

    fn ago(seconds: u64) -> UnixTime {
        UnixTime(NOW.0 - seconds)
    }

    fn failed(seconds_ago: u64) -> Option<Attempt> {
        Some(Attempt {
            at: ago(seconds_ago),
            outcome: Outcome::Failure,
        })
    }

    #[test]
    fn a_fetch_is_due_exactly_when_no_recent_cache_or_failure_explains_skipping_it() {
        let cases: &[(&str, Option<UnixTime>, Option<Attempt>, bool)] = &[
            ("missing cache, no attempt", None, None, true),
            // A failed attempt backs off for an hour, whatever the cache's own age.
            (
                "a failed attempt 10 minutes ago",
                None,
                failed(10 * 60),
                false,
            ),
            ("a failed attempt 2 hours ago", None, failed(2 * HOUR), true),
            ("a fresh cache", Some(ago(60)), None, false),
            ("a day-old cache", Some(ago(DAY)), None, true),
        ];
        for &(label, cached_at, last_attempt, want) in cases {
            assert_eq!(due(cached_at, last_attempt, NOW), want, "{label}");
        }
    }
}
