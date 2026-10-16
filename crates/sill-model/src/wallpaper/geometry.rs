//! Cover-fit geometry: the part of the source that, scaled up or down evenly, fills the target on
//! both axes, centred on both (CSS `background-size: cover`, centered). Pure, so the crop stays
//! symmetric: the source's own centre pixel always lands on the target's. The window is cut from
//! the source first and only that is resized, so the work is bounded by the target and the
//! source, whatever the source's aspect ratio.

/// The `width` x `height` region at `x`, `y` of the source that fills the target: resize it to
/// the target's size and nothing more is needed. The same margin is left on either side, give or
/// take the one pixel an odd overflow cannot split evenly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverWindow {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The window of `source` that covers `target`. A zero dimension on either side is floored to 1
/// so the geometry is always finite (a zero-sized source or target never occurs for a real
/// image or output, but this keeps the function total rather than panicking on one). The window
/// is never empty and never leaves the source.
pub fn cover_window(source: (u32, u32), target: (u32, u32)) -> CoverWindow {
    let (sw, sh) = (u64::from(source.0.max(1)), u64::from(source.1.max(1)));
    let (tw, th) = (u64::from(target.0.max(1)), u64::from(target.1.max(1)));
    // The target is relatively wider than the source when tw/th > sw/sh: the whole source width
    // is used, and only the height that has the target's aspect.
    let (width, height) = if tw * sh >= th * sw {
        (sw, ((sw * th + tw / 2) / tw).clamp(1, sh))
    } else {
        (((sh * tw + th / 2) / th).clamp(1, sw), sh)
    };
    CoverWindow {
        x: to_u32((sw - width) / 2),
        y: to_u32((sh - height) / 2),
        width: to_u32(width),
        height: to_u32(height),
    }
}

/// `value`, which is at most a `u32` the caller started from.
fn to_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{CoverWindow, cover_window};

    const fn window(x: u32, y: u32, width: u32, height: u32) -> CoverWindow {
        CoverWindow {
            x,
            y,
            width,
            height,
        }
    }

    /// name, source, target, expected.
    type Case = (&'static str, (u32, u32), (u32, u32), CoverWindow);

    #[test]
    fn the_window_is_the_part_of_the_source_that_fills_the_target() {
        const CASES: &[Case] = &[
            (
                "already the target size",
                (1920, 1080),
                (1920, 1080),
                window(0, 0, 1920, 1080),
            ),
            (
                "a 2x source at the same aspect",
                (3840, 2160),
                (1920, 1080),
                window(0, 0, 3840, 2160),
            ),
            ("one pixel to one pixel", (1, 1), (1, 1), window(0, 0, 1, 1)),
            (
                "a wider source than the target loses its sides evenly",
                (4000, 1000),
                (1000, 1000),
                window(1500, 0, 1000, 1000),
            ),
            (
                "a 2:1 source into a square loses its sides",
                (1000, 500),
                (2000, 2000),
                window(250, 0, 500, 500),
            ),
            (
                "a taller source than the target loses its top and bottom",
                (500, 1000),
                (2000, 2000),
                window(0, 250, 500, 500),
            ),
            (
                "a source smaller than the target is still only itself",
                (100, 100),
                (250, 250),
                window(0, 0, 100, 100),
            ),
            (
                "a hair-thin panorama keeps at least one pixel each way",
                (5000, 1),
                (3840, 2160),
                window(2499, 0, 2, 1),
            ),
            (
                "a hair-thin column keeps at least one pixel each way",
                (1, 5000),
                (3840, 2160),
                window(0, 2499, 1, 1),
            ),
        ];
        for (name, source, target, want) in CASES {
            assert_eq!(cover_window(*source, *target), *want, "{name}");
        }
    }

    #[test]
    fn the_window_never_leaves_the_source_and_is_centred_within_one_pixel() {
        const SOURCES: &[(u32, u32)] = &[
            (1080, 1920),
            (2000, 500),
            (333, 777),
            (4096, 4096),
            (5000, 1),
            (1, 5000),
        ];
        const TARGETS: &[(u32, u32)] = &[(1920, 1080), (2560, 1440), (1280, 720), (3840, 2160)];
        for &source in SOURCES {
            for &target in TARGETS {
                let fit = cover_window(source, target);
                let said = format!("{source:?} -> {target:?}: {fit:?}");
                assert!(fit.width >= 1 && fit.height >= 1, "{said}");
                assert!(fit.x + fit.width <= source.0, "{said}");
                assert!(fit.y + fit.height <= source.1, "{said}");
                let far_x = source.0 - fit.x - fit.width;
                let far_y = source.1 - fit.y - fit.height;
                assert!(far_x.abs_diff(fit.x) <= 1, "{said}");
                assert!(far_y.abs_diff(fit.y) <= 1, "{said}");
            }
        }
    }
}
