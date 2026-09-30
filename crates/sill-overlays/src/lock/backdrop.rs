//! The lock screen's backdrop: the blurred copy the wallpaper service writes beside each
//! prepared image (`sill_model::wallpaper::backdrop_file`). Pure.

use ds::prelude::*;
use std::path::PathBuf;

use sill_model::ids::OutputName;
use sill_model::wallpaper::{Wallpapers, backdrop_file};

/// The backdrop file for `output` in `scheme`: the blurred copy of that output's prepared light
/// or dark image; `None` when the output has no prepared image (the surface paints `--paper`).
pub fn backdrop_for(
    wallpapers: &Wallpapers,
    output: &OutputName,
    scheme: Scheme,
) -> Option<PathBuf> {
    wallpapers
        .outputs
        .iter()
        .find(|image| &image.output == output)
        .map(|image| match scheme {
            Scheme::Light => backdrop_file(&image.light),
            Scheme::Dark => backdrop_file(&image.dark),
        })
}

#[cfg(test)]
mod tests {
    use ds::prelude::*;
    use std::path::PathBuf;

    use sill_model::ids::OutputName;
    use sill_model::wallpaper::{WallpaperImage, Wallpapers};

    use super::backdrop_for;

    #[test]
    fn each_output_and_scheme_has_its_backdrop() {
        let wallpapers = Wallpapers {
            outputs: vec![WallpaperImage {
                output: OutputName("DP-1".to_owned()),
                light: PathBuf::from("/c/aa-1920x1080.png"),
                dark: PathBuf::from("/c/bb-1920x1080.png"),
            }],
        };
        let dp1 = OutputName("DP-1".to_owned());
        let cases = [
            (
                dp1.clone(),
                Scheme::Light,
                Some(PathBuf::from("/c/aa-1920x1080-backdrop.png")),
            ),
            (
                dp1,
                Scheme::Dark,
                Some(PathBuf::from("/c/bb-1920x1080-backdrop.png")),
            ),
            (OutputName("HDMI-A-1".to_owned()), Scheme::Light, None),
        ];
        for (output, scheme, want) in cases {
            assert_eq!(
                backdrop_for(&wallpapers, &output, scheme),
                want,
                "{output:?}"
            );
        }
    }
}
