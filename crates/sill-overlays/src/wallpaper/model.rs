//! What the wallpaper shows, from what the service published.

use ds::prelude::Scheme;
use sill_model::ids::OutputName;
use sill_model::wallpaper::{WallpaperImage, Wallpapers};

/// `wallpapers`' entry for `output`, if it has one yet (a fresh output, or a source that never
/// resolved, has none).
fn image_for<'a>(
    wallpapers: &'a Wallpapers,
    output: Option<&OutputName>,
) -> Option<&'a WallpaperImage> {
    let output = output?;
    wallpapers
        .outputs
        .iter()
        .find(|image| &image.output == output)
}

/// A cache path as the `file://` URL shell-host's `LocalNet` reads directly (mirrors
/// `sill_services::tray::icon::named_icon_file_url`).
fn file_url(path: &std::path::Path) -> String {
    format!("file://{}", path.display())
}

/// The `file://` URL of the image `output` shows in `scheme`, if it has one yet.
pub(crate) fn url_for(
    wallpapers: &Wallpapers,
    output: Option<&OutputName>,
    scheme: Scheme,
) -> Option<String> {
    image_for(wallpapers, output).map(|image| {
        file_url(match scheme {
            Scheme::Light => &image.light,
            Scheme::Dark => &image.dark,
        })
    })
}
