//! Where the wallpaper's picture comes from: the `wallpaper.picture` setting as the daemon sends
//! it, and, while that is empty, the old `sill/wallpaper-source` file (read, never written).

use std::path::{Path, PathBuf};
use std::time::Duration;

use sill_model::ids::OutputName;
use sill_model::wallpaper::{WallpaperCommand, WallpaperPicture, Wallpapers};
use sill_services::WallpaperService;
use sill_services::env::Env;
use sill_services::service::spawn;
use sill_testkit::TempDir;
use tokio::sync::watch;

/// A small PNG of one `shade` in `dir`.
fn png(dir: &Path, name: &str, shade: u8) -> PathBuf {
    let path = dir.join(name);
    image::RgbImage::from_pixel(4, 4, image::Rgb([shade, shade, shade]))
        .save(&path)
        .unwrap_or_else(|e| panic!("{e}"));
    path
}

/// The next published state, within a few seconds.
async fn next(rx: &mut watch::Receiver<Wallpapers>) -> Wallpapers {
    tokio::time::timeout(Duration::from_secs(10), rx.changed())
        .await
        .unwrap_or_else(|_| panic!("no wallpaper was published"))
        .unwrap_or_else(|e| panic!("{e}"));
    rx.borrow_and_update().clone()
}

/// The one output's light image, if the state has it.
fn light_of(wallpapers: &Wallpapers) -> Option<PathBuf> {
    wallpapers.outputs.first().map(|image| image.light.clone())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_setting_wins_and_the_old_file_serves_while_it_is_empty() {
    let scratch = TempDir::new("wallpaper-source");
    let env = Env::isolated(scratch.path());
    let old = png(scratch.path(), "old.png", 40);
    let chosen = png(scratch.path(), "chosen.png", 200);
    let old_file = env.dirs.config.join("sill/wallpaper-source");
    std::fs::create_dir_all(old_file.parent().unwrap_or_else(|| panic!("no parent")))
        .unwrap_or_else(|e| panic!("{e}"));
    let old_text = format!("{}\n", old.display());
    std::fs::write(&old_file, &old_text).unwrap_or_else(|e| panic!("{e}"));

    let service = spawn::<WallpaperService>(&tokio::runtime::Handle::current(), &env, ());
    let (mut state, control) = (service.state, service.control);
    let send = |command| control.send(command).unwrap_or_else(|e| panic!("{e}"));
    send(WallpaperCommand::Prepare {
        output: OutputName("DP-1".to_owned()),
        width: 8,
        height: 8,
    });
    let from_old_file = light_of(&next(&mut state).await);
    assert!(from_old_file.is_some(), "the old file's picture is shown");

    send(WallpaperCommand::Picture(WallpaperPicture::Chosen(chosen)));
    let from_setting = light_of(&next(&mut state).await);
    assert!(from_setting.is_some());
    assert_ne!(
        from_setting, from_old_file,
        "the setting beats the old file"
    );

    send(WallpaperCommand::Picture(WallpaperPicture::Chosen(
        scratch.path().join("gone.png"),
    )));
    assert_eq!(
        light_of(&next(&mut state).await),
        None,
        "a chosen picture that is missing is not replaced by the old file's"
    );

    send(WallpaperCommand::Picture(WallpaperPicture::Unset));
    assert_eq!(light_of(&next(&mut state).await), from_old_file);
    assert_eq!(
        std::fs::read_to_string(&old_file).unwrap_or_else(|e| panic!("{e}")),
        old_text,
        "the old file is only read"
    );
}
