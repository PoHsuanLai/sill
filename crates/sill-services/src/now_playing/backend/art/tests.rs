//! `resolve_art` against scratch folders: where each kind of `artUrl` goes, and what is refused.

use std::collections::HashMap;
use std::os::unix::net::UnixListener;
use std::path::Path;

use sill_model::now_playing::{Art, art_cache_name};
use sill_testkit::TempDir;
use zbus::zvariant::{OwnedValue, Str};

use super::cache::copy_cover;
use super::{ART_MAX_BYTES, ArtJob, resolve_art};

fn metadata(url: &str) -> HashMap<String, OwnedValue> {
    HashMap::from([(
        "mpris:artUrl".to_owned(),
        OwnedValue::from(Str::from(url.to_owned())),
    )])
}

fn file_url(path: &Path) -> String {
    format!("file://{}", path.display())
}

/// The job a cover came to, or a failure naming what was there instead.
fn job_of(art: Art, job: Option<ArtJob>) -> ArtJob {
    job.unwrap_or_else(|| panic!("expected a job, got {art:?}"))
}

#[test]
fn a_remote_cover_is_a_job_until_it_is_in_the_cache() {
    let dir = TempDir::new("art-remote");
    let url = "https://example.com/cover.png";
    let (art, job) = resolve_art(&metadata(url), Some(dir.path()));
    let cache_path = dir.path().join(art_cache_name(url));
    assert_eq!(art, Art::None);
    assert_eq!(
        job,
        Some(ArtJob::Fetch {
            url: url.to_owned(),
            cache_path: cache_path.clone(),
        })
    );
    std::fs::write(&cache_path, b"cover").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        resolve_art(&metadata(url), Some(dir.path())),
        (Art::File(cache_path), None)
    );
}

#[test]
fn a_local_cover_is_copied_into_the_cache_and_drawn_from_there() {
    let dir = TempDir::new("art-local");
    let cache = dir.path().join("art");
    let source = dir.path().join("elsewhere").join("cover.png");
    std::fs::create_dir_all(source.parent().unwrap_or(dir.path()))
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&source, b"cover").unwrap_or_else(|e| panic!("{e}"));

    let (art, job) = resolve_art(&metadata(&file_url(&source)), Some(&cache));
    let job = job_of(art, job);
    let ArtJob::Copy { from, cache_path } = &job else {
        panic!("a local file is copied, got {job:?}");
    };
    assert_eq!(from, &source);
    assert!(cache_path.starts_with(&cache), "{cache_path:?}");
    assert_ne!(
        cache_path, &source,
        "never drawn from where the player put it"
    );

    copy_cover(from, cache_path).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        resolve_art(&metadata(&file_url(&source)), Some(&cache)),
        (Art::File(cache_path.clone()), None),
        "once copied, it is the copy that is drawn"
    );
}

#[test]
fn a_player_that_rewrites_one_file_for_each_track_gets_a_new_copy_each_time() {
    let dir = TempDir::new("art-rewritten");
    let source = dir.path().join("cover.png");
    std::fs::write(&source, b"first").unwrap_or_else(|e| panic!("{e}"));
    let url = file_url(&source);
    let first = job_of(Art::None, resolve_art(&metadata(&url), Some(dir.path())).1);
    std::fs::write(&source, b"second cover").unwrap_or_else(|e| panic!("{e}"));
    let second = job_of(Art::None, resolve_art(&metadata(&url), Some(dir.path())).1);
    assert_ne!(first.cache_path(), second.cache_path());
}

#[test]
fn only_a_regular_file_of_a_sane_size_is_a_local_cover() {
    let dir = TempDir::new("art-refused");
    let big = dir.path().join("big.png");
    std::fs::File::create(&big)
        .and_then(|file| file.set_len(ART_MAX_BYTES + 1))
        .unwrap_or_else(|e| panic!("{e}"));
    let socket = dir.path().join("cover.sock");
    let _listener = UnixListener::bind(&socket).unwrap_or_else(|e| panic!("{e}"));
    let cache = dir.path().join("art");
    let cases = [
        ("a folder", file_url(dir.path())),
        ("a socket", file_url(&socket)),
        ("past the size limit", file_url(&big)),
        ("missing", file_url(&dir.path().join("missing.png"))),
        ("not an address at all", "cover.png".to_owned()),
        ("a data address", "data:image/png;base64,AAAA".to_owned()),
    ];
    for (name, url) in cases {
        assert_eq!(
            resolve_art(&metadata(&url), Some(&cache)),
            (Art::None, None),
            "{name}"
        );
    }
}

#[test]
fn without_a_cache_folder_no_cover_is_taken() {
    let dir = TempDir::new("art-no-cache");
    let source = dir.path().join("cover.png");
    std::fs::write(&source, b"cover").unwrap_or_else(|e| panic!("{e}"));
    for url in [file_url(&source), "https://example.com/a.png".to_owned()] {
        assert_eq!(
            resolve_art(&metadata(&url), None),
            (Art::None, None),
            "{url}"
        );
    }
}
