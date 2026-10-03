//! A video's picture in the pane comes from the freedesktop thumbnail cache of the injected cache
//! directory, and only while the thumbnail is of this very version of the file.
#![cfg(feature = "media")]
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use anyview_peek::Body;
use md5::{Digest, Md5};
use sill_launcher_ui::launcher::preview::frames::ThumbnailFrames;
use sill_launcher_ui::launcher::preview::load::{Look, ViewerLook};
use sill_testkit::TempDir;

/// A recording with no cover of its own, in anyview's fixtures.
const CLIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../anyview/crates/anyview-peek/tests/fixtures/clip.mp4"
);

/// A scratch `XDG_CACHE_HOME` and a copy of the clip next to it.
struct Scene {
    _dir: TempDir,
    cache: PathBuf,
    video: PathBuf,
}

fn scene() -> Scene {
    let dir = TempDir::new("launcher-video-frames");
    let cache = dir.path().join("cache");
    let video = dir.path().join("clip.mp4");
    std::fs::copy(CLIP, &video).unwrap();
    Scene {
        cache,
        video,
        _dir: dir,
    }
}

/// The freedesktop thumbnail of `video` at `size`, as another program writes it: a 2 x 2 PNG named
/// by the MD5 of the file's URI, saying whose it is and which version of it.
fn write_thumbnail(cache: &Path, video: &Path, size: &str, uri: &str, mtime: u64) {
    let name: String = Md5::digest(uri.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let folder = cache.join("thumbnails").join(size);
    std::fs::create_dir_all(&folder).unwrap();
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .add_text_chunk("Thumb::URI".into(), uri.into())
        .unwrap();
    encoder
        .add_text_chunk("Thumb::MTime".into(), mtime.to_string())
        .unwrap();
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[200; 16]).unwrap();
    writer.finish().unwrap();
    std::fs::write(folder.join(format!("{name}.png")), bytes).unwrap();
    assert!(video.exists());
}

fn uri_of(video: &Path) -> String {
    format!("file://{}", video.display())
}

fn mtime_of(video: &Path) -> u64 {
    let modified = std::fs::metadata(video).unwrap().modified().unwrap();
    modified.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn body_of(scene: &Scene) -> &'static str {
    let look = ViewerLook::new(Arc::new(ThumbnailFrames::in_cache(&scene.cache)));
    look.at(&scene.video).unwrap().body.slug()
}

#[test]
fn a_cached_thumbnail_of_the_video_is_its_picture() {
    let scene = scene();
    for size in ["large", "normal"] {
        let video = scene.video.clone();
        write_thumbnail(
            &scene.cache,
            &video,
            size,
            &uri_of(&video),
            mtime_of(&video),
        );
        let look = ViewerLook::new(Arc::new(ThumbnailFrames::in_cache(&scene.cache)));
        let peeked = look.at(&video).unwrap();
        let Body::Picture(frame) = &peeked.body else {
            panic!("{size}: expected the frame, got {}", peeked.body.slug());
        };
        assert_eq!(frame.picture.size().width.0, 2, "{size}");
        std::fs::remove_dir_all(scene.cache.join("thumbnails")).unwrap();
    }
}

#[test]
fn a_missing_or_stale_thumbnail_leaves_the_facts_card() {
    let scene = scene();
    assert_eq!(body_of(&scene), "facts", "no thumbnail at all");

    let uri = uri_of(&scene.video);
    let mtime = mtime_of(&scene.video);
    write_thumbnail(&scene.cache, &scene.video, "large", &uri, mtime + 1);
    assert_eq!(body_of(&scene), "facts", "made for another mtime");

    write_thumbnail(
        &scene.cache,
        &scene.video,
        "large",
        "file:///elsewhere/clip.mp4",
        mtime,
    );
    // The name is the md5 of the URI it claims, so write the other file's name by hand: the chunk
    // that names another file under this file's name is stale too.
    let name: String = Md5::digest(uri.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let liar = scene
        .cache
        .join("thumbnails/large")
        .join(format!("{name}.png"));
    let other: String = Md5::digest(b"file:///elsewhere/clip.mp4")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    std::fs::rename(
        scene
            .cache
            .join("thumbnails/large")
            .join(format!("{other}.png")),
        liar,
    )
    .unwrap();
    assert_eq!(body_of(&scene), "facts", "its chunk names another file");
}
