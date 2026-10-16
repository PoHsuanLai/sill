//! Downloading one remote cover (effects: a child process, the filesystem).

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::ART_MAX_BYTES;

/// How long one fetch may take, connecting included.
const FETCH_TIME_LIMIT: Duration = Duration::from_secs(20);

/// How many redirects one fetch follows.
const REDIRECTS_MAX: &str = "5";

/// Whether `url` was brought to `cache_path`: fetched to a part file within the time and size
/// limits (http and https only, also across redirects, no globbing of the address), then renamed
/// into place, so a stopped fetch never leaves a half cover that looks cached. `curl` is a runtime
/// requirement of the desktop; without it no remote cover is fetched.
pub(super) async fn fetch(url: &str, cache_path: &Path) -> bool {
    let Some(dir) = cache_path.parent() else {
        return false;
    };
    let _ = tokio::fs::create_dir_all(dir).await;
    let mut part = cache_path.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let (max_time, max_size) = (
        FETCH_TIME_LIMIT.as_secs().to_string(),
        ART_MAX_BYTES.to_string(),
    );
    // curl's own limit is the first line; this one covers a curl that does not return.
    let limit = FETCH_TIME_LIMIT + Duration::from_secs(5);
    let status = tokio::time::timeout(
        limit,
        tokio::process::Command::new("curl")
            .args(["--silent", "--fail", "--location", "--globoff"])
            .args(["--proto", "=http,https", "--proto-redir", "=http,https"])
            .args(["--max-redirs", REDIRECTS_MAX])
            .args(["--max-time", max_time.as_str()])
            .args(["--max-filesize", max_size.as_str()])
            .arg("--output")
            .arg(&part)
            .arg("--")
            .arg(url)
            .kill_on_drop(true)
            .status(),
    )
    .await;
    let fetched = matches!(status, Ok(Ok(status)) if status.success()) && within_size(&part).await;
    if fetched && tokio::fs::rename(&part, cache_path).await.is_ok() {
        return true;
    }
    let _ = tokio::fs::remove_file(&part).await;
    false
}

/// Whether the file at `path` is no larger than [`ART_MAX_BYTES`] (curl's own limit cannot see a
/// reply of unknown length).
async fn within_size(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|meta| meta.is_file() && meta.len() <= ART_MAX_BYTES)
}
