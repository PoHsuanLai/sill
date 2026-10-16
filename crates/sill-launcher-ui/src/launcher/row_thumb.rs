//! A picture file's thumbnail in a launcher row.
//!
//! The shell's host draws a `file:` picture only from the folders it is given (`sill_ui_kit::root`),
//! and a person's pictures can be anywhere in their home folder. The freedesktop thumbnail
//! specification is the shared answer: a file manager or a thumbnailer keeps a small picture of a
//! file under `<cache>/thumbnails/<size>/<md5 of the file's address>.png`, for any program to
//! show instead of the file. So a row draws that picture, from a folder the host is given
//! (`Dirs::thumbnails`), and only while it was made for this very version of the file
//! (`FreedesktopThumbnails::lookup` checks the address and the time the picture says). A file
//! with no thumbnail has none in its row. A file in the Pictures folder, which the host may
//! read, is drawn from itself, as before. Nothing is copied: a copy would hold the whole picture
//! in sill's cache and need scaling, which the specification's cache already is.

use std::path::{Path, PathBuf};

use anyview_core::{ByteLen, FilePath, FileStamp, ModTime};
use anyview_platform::linux::FreedesktopThumbnails;
use anyview_platform::{Env, ThumbSize, ThumbnailCache, file_uri};
use dioxus::prelude::*;
use ds::components::content::image_source::ImageSource;
use md5::{Digest, Md5};
use sill_services::env::Dirs;
use sill_ui_kit::services::Services;

/// The picture types a row shows a thumbnail of.
const PICTURES: [&str; 4] = ["image/png", "image/jpeg", "image/webp", "image/gif"];

/// The sizes tried and the folder each is kept in, the smaller first: a row's picture is a few
/// dozen pixels across.
const SIZES: [(ThumbSize, &str); 2] = [(ThumbSize::Normal, "normal"), (ThumbSize::Large, "large")];

/// Where a row's picture of a file comes from.
#[derive(Debug, Clone)]
pub struct RowThumbs(Source);

#[derive(Debug, Clone)]
enum Source {
    /// The file itself: a pane mounted without services, whose host was given its own folders.
    Direct,
    /// The thumbnail cache, but the Pictures folder from itself.
    Cached(Folders),
}

/// The person's thumbnail cache and Pictures folder.
#[derive(Debug, Clone)]
struct Folders {
    lookup: FreedesktopThumbnails,
    thumbnails: PathBuf,
    pictures: Option<PathBuf>,
}

/// The row pictures for this component: from the services' directories, else the file itself.
pub fn use_row_thumbs() -> RowThumbs {
    use_hook(|| {
        try_consume_context::<Services>().map_or_else(RowThumbs::direct, |services| {
            RowThumbs::in_dirs(&services.dirs)
        })
    })
}

impl RowThumbs {
    /// Every picture from the file itself.
    pub fn direct() -> RowThumbs {
        RowThumbs(Source::Direct)
    }

    /// The cache of `dirs`.
    pub fn in_dirs(dirs: &Dirs) -> RowThumbs {
        // The platform crate reads only the cache directory of its `Env`; the rest is isolated.
        let mut env = Env::isolated(&dirs.cache);
        env.dirs.cache = dirs.cache.clone();
        RowThumbs(Source::Cached(Folders {
            lookup: FreedesktopThumbnails::new(&env),
            thumbnails: dirs.thumbnails(),
            pictures: dirs.pictures.clone(),
        }))
    }

    /// The picture for a row of a file of type `mime` at `path`, if it has one.
    pub fn of(&self, mime: &str, path: &Path) -> Option<ImageSource> {
        if !PICTURES.contains(&mime) {
            return None;
        }
        match &self.0 {
            Source::Direct => ImageSource::file(path).ok(),
            Source::Cached(folders) => folders.picture_of(path),
        }
    }
}

impl Folders {
    fn picture_of(&self, path: &Path) -> Option<ImageSource> {
        if self
            .pictures
            .as_deref()
            .is_some_and(|pictures| path.starts_with(pictures))
        {
            return ImageSource::file(path).ok();
        }
        let file = FilePath::new(path).ok()?;
        let stamp = stamp_of(path)?;
        SIZES.iter().find_map(|(size, folder)| {
            // Only a thumbnail made for this version of the file is drawn.
            self.lookup.lookup(&file, &stamp, *size).ok().flatten()?;
            ImageSource::file(&thumbnail_file(&self.thumbnails, folder, &file)).ok()
        })
    }
}

/// `path`'s size and modification time, as the specification compares them.
fn stamp_of(path: &Path) -> Option<FileStamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some(FileStamp {
        len: ByteLen(meta.len()),
        modified: ModTime::from_system_time(meta.modified().ok()?),
    })
}

/// Where the thumbnail of `file` is in `size_folder` of the thumbnail cache `root`: the MD5 of the
/// file's address, as the specification names it (the same name the cache's writer gives).
fn thumbnail_file(root: &Path, size_folder: &str, file: &FilePath) -> PathBuf {
    let name: String = Md5::digest(file_uri(file).as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    root.join(size_folder).join(format!("{name}.png"))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    use anyview_core::{FilePath, PixelLen, PixelSize};
    use anyview_platform::linux::FreedesktopThumbnails;
    use anyview_platform::{Env, ThumbPixels, ThumbSize, ThumbnailCache};
    use ds::components::content::image_source::ImageSource;
    use sill_services::env::Dirs;
    use sill_testkit::TempDir;

    use super::{RowThumbs, stamp_of, thumbnail_file};

    /// A scratch home with a picture outside Pictures, and the thumbnail cache of its directories.
    struct Scene {
        _dir: TempDir,
        dirs: Dirs,
        picture: PathBuf,
    }

    fn scene() -> Scene {
        let dir = TempDir::new("row-thumbs");
        let dirs = Dirs {
            pictures: Some(dir.path().join("home/Pictures")),
            ..Dirs::under(dir.path())
        };
        let picture = dir.path().join("home/Documents/scan.png");
        std::fs::create_dir_all(picture.parent().unwrap_or(dir.path()))
            .unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(&picture, b"not really a png").unwrap_or_else(|e| panic!("{e}"));
        Scene {
            _dir: dir,
            dirs,
            picture,
        }
    }

    /// The thumbnail of `scene`'s picture in `size`, as another program's writer keeps it.
    fn make_thumbnail(scene: &Scene, size: ThumbSize) {
        let mut env = Env::isolated(&scene.dirs.cache);
        env.dirs.cache = scene.dirs.cache.clone();
        let writer = FreedesktopThumbnails::new(&env);
        let file = FilePath::new(&scene.picture).unwrap_or_else(|e| panic!("{e}"));
        let stamp = stamp_of(&scene.picture).unwrap_or_else(|| panic!("a stamp"));
        let pixels = ThumbPixels::new(
            PixelSize {
                width: PixelLen(2),
                height: PixelLen(2),
            },
            vec![200; 16],
        )
        .unwrap_or_else(|| panic!("pixels"));
        writer
            .store(&file, &stamp, size, &pixels)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    fn row(scene: &Scene, mime: &str, path: &Path) -> Option<ImageSource> {
        RowThumbs::in_dirs(&scene.dirs).of(mime, path)
    }

    #[test]
    fn a_picture_outside_pictures_is_drawn_from_its_thumbnail_in_the_cache() {
        let scene = scene();
        for (size, folder) in [(ThumbSize::Large, "large"), (ThumbSize::Normal, "normal")] {
            make_thumbnail(&scene, size);
            let file = FilePath::new(&scene.picture).unwrap_or_else(|e| panic!("{e}"));
            let kept = thumbnail_file(&scene.dirs.thumbnails(), folder, &file);
            assert!(
                kept.is_file(),
                "{folder}: the writer's name is the one looked up"
            );
            assert_eq!(
                row(&scene, "image/png", &scene.picture),
                ImageSource::file(&kept).ok(),
                "{folder}"
            );
        }
    }

    #[test]
    fn no_thumbnail_a_stale_one_or_another_kind_of_file_has_no_picture() {
        let scene = scene();
        assert_eq!(row(&scene, "image/png", &scene.picture), None, "none made");

        make_thumbnail(&scene, ThumbSize::Normal);
        assert_eq!(
            row(&scene, "application/pdf", &scene.picture),
            None,
            "not a picture type"
        );

        // The file is edited after its thumbnail was made.
        let later = SystemTime::now() + Duration::from_secs(60);
        std::fs::File::options()
            .write(true)
            .open(&scene.picture)
            .and_then(|file| file.set_modified(later))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            row(&scene, "image/png", &scene.picture),
            None,
            "made for another version"
        );
    }

    #[test]
    fn a_picture_in_the_pictures_folder_is_drawn_from_itself() {
        let scene = scene();
        let inside = scene
            .dirs
            .pictures
            .clone()
            .unwrap_or_else(|| panic!("a Pictures folder"));
        std::fs::create_dir_all(&inside).unwrap_or_else(|e| panic!("{e}"));
        let shot = inside.join("shot.png");
        std::fs::write(&shot, b"png").unwrap_or_else(|e| panic!("{e}"));
        let drawn = row(&scene, "image/png", &shot);
        assert!(drawn.is_some(), "no thumbnail was made, and it is drawn");
        assert_eq!(drawn, ImageSource::file(&shot).ok());
    }

    #[test]
    fn a_pane_without_services_draws_the_file_itself() {
        let path = Path::new("/s/home/a.jpg");
        let direct = RowThumbs::direct();
        assert_eq!(direct.of("image/jpeg", path), ImageSource::file(path).ok());
        assert_eq!(direct.of("text/plain", path), None);
    }
}
