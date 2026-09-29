//! Cover-fit geometry: scale a source image up until it fills the target on both axes, then
//! crop the overflow off both edges equally (CSS `background-size: cover`, centered). Pure, so
//! the crop stays symmetric — the source's own centre pixel always lands on the target's.

/// How to turn a `source`-sized image into exactly `target`: resize it to `scaled` with
/// Lanczos3, then crop `crop_x`/`crop_y` off the left and top (the same again is left on the
/// right and bottom, since `scaled` is chosen to be exactly `target` plus the even overflow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverFit {
    pub scaled: (u32, u32),
    pub crop_x: u32,
    pub crop_y: u32,
}

/// The cover-fit geometry for `source` filling `target`. A zero dimension on either side is
/// floored to 1 so the scale is always finite (a zero-sized source or target never occurs for a
/// real image or output, but this keeps the function total rather than panicking on one).
pub fn cover_fit(source: (u32, u32), target: (u32, u32)) -> CoverFit {
    let (sw, sh) = (source.0.max(1), source.1.max(1));
    let (tw, th) = (target.0.max(1), target.1.max(1));
    let scale = (f64::from(tw) / f64::from(sw)).max(f64::from(th) / f64::from(sh));
    let scaled_w = ((f64::from(sw) * scale).round() as u32).max(tw);
    let scaled_h = ((f64::from(sh) * scale).round() as u32).max(th);
    CoverFit {
        scaled: (scaled_w, scaled_h),
        crop_x: (scaled_w - tw) / 2,
        crop_y: (scaled_h - th) / 2,
    }
}

#[cfg(test)]
mod tests {
    use super::{CoverFit, cover_fit};

    /// name, source, target, expected.
    type Case = (&'static str, (u32, u32), (u32, u32), CoverFit);

    #[test]
    fn exact_and_uniformly_scaled_sources_crop_to_nothing() {
        const CASES: &[Case] = &[
            (
                "already the target size",
                (1920, 1080),
                (1920, 1080),
                CoverFit {
                    scaled: (1920, 1080),
                    crop_x: 0,
                    crop_y: 0,
                },
            ),
            (
                "a 2x source at the same aspect",
                (3840, 2160),
                (1920, 1080),
                CoverFit {
                    scaled: (1920, 1080),
                    crop_x: 0,
                    crop_y: 0,
                },
            ),
            (
                "one pixel to one pixel",
                (1, 1),
                (1, 1),
                CoverFit {
                    scaled: (1, 1),
                    crop_x: 0,
                    crop_y: 0,
                },
            ),
        ];
        for (name, source, target, want) in CASES {
            assert_eq!(cover_fit(*source, *target), *want, "{name}");
        }
    }

    #[test]
    fn a_wider_target_than_source_crops_the_top_and_bottom_evenly() {
        // A short, wide crop of a 2:1 source into a square target: the source's own width
        // becomes the target's, and the overflow is trimmed off the top and bottom equally.
        let fit = cover_fit((1000, 500), (2000, 2000));
        assert_eq!(fit.scaled, (4000, 2000));
        assert_eq!(fit.crop_x, 1000);
        assert_eq!(fit.crop_y, 0);
    }

    #[test]
    fn a_taller_target_than_source_crops_the_left_and_right_evenly() {
        let fit = cover_fit((500, 1000), (2000, 2000));
        assert_eq!(fit.scaled, (2000, 4000));
        assert_eq!(fit.crop_x, 0);
        assert_eq!(fit.crop_y, 1000);
    }

    #[test]
    fn the_crop_is_centred_within_one_pixel_so_the_source_centre_lands_on_the_targets() {
        const SOURCES: &[(u32, u32)] = &[(1080, 1920), (2000, 500), (333, 777), (4096, 4096)];
        const TARGETS: &[(u32, u32)] = &[(1920, 1080), (2560, 1440), (1280, 720)];
        for &source in SOURCES {
            for &target in TARGETS {
                let fit = cover_fit(source, target);
                assert!(
                    fit.scaled.0 >= target.0,
                    "{source:?} -> {target:?}: {fit:?}"
                );
                assert!(
                    fit.scaled.1 >= target.1,
                    "{source:?} -> {target:?}: {fit:?}"
                );
                // The far margin (scaled - crop - target) is the near margin (crop), give or
                // take the one pixel an odd overflow cannot split evenly.
                let far_x = fit.scaled.0 - fit.crop_x - target.0;
                let far_y = fit.scaled.1 - fit.crop_y - target.1;
                assert!(
                    far_x.abs_diff(fit.crop_x) <= 1,
                    "{source:?} -> {target:?}: {fit:?}"
                );
                assert!(
                    far_y.abs_diff(fit.crop_y) <= 1,
                    "{source:?} -> {target:?}: {fit:?}"
                );
            }
        }
    }
}
