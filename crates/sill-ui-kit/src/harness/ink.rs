//! Reading what a surface really paints: the contrast of an element's ink against the ground
//! behind it, off the rendered pixels. Computed colours cannot see a glyph the renderer kept in
//! a stale colour (a dark glyph on a dark bar after the scheme changed), so a legibility test
//! asks the picture.

use std::collections::HashMap;

use ds_harness::{Backdrop, Harness, Query, Srgba};

use super::SurfaceHarness;
use crate::surface::Surface;

impl<S: Surface> SurfaceHarness<S> {
    /// [`ink_contrast`] of this surface's document.
    pub fn ink_contrast(&mut self, selector: &str) -> Option<f32> {
        ink_contrast(&mut self.harness, self.viewport.scale_percent, selector)
    }
}

/// The WCAG contrast between the ground behind the first element matching `selector` (the colour
/// most of its box shows) and the pixel that differs most from it: how well its glyph or text
/// reads as painted, in a document drawn at `scale_percent`. `None` when nothing matches or the
/// render fails; about 1 for an element that paints nothing visible against its ground.
pub fn ink_contrast(harness: &mut Harness, scale_percent: u16, selector: &str) -> Option<f32> {
    let rect = harness.rect(selector)?;
    let image = harness.render_over(Backdrop::Scheme).ok()?;
    let scale = f32::from(scale_percent) / 100.0;
    let (left, top) = (
        (rect.origin.x.0 * scale) as u32,
        (rect.origin.y.0 * scale) as u32,
    );
    let (right, bottom) = (
        (((rect.origin.x.0 + rect.size.width.0) * scale).ceil() as u32).min(image.width()),
        (((rect.origin.y.0 + rect.size.height.0) * scale).ceil() as u32).min(image.height()),
    );
    let pixels: Vec<[u8; 3]> = (top..bottom)
        .flat_map(|y| (left..right).map(move |x| (x, y)))
        .map(|(x, y)| {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            [r, g, b]
        })
        .collect();
    let ground = most_common(&pixels)?;
    pixels
        .iter()
        .map(|pixel| srgb(*pixel).contrast(srgb(ground)))
        .reduce(f32::max)
}

fn most_common(pixels: &[[u8; 3]]) -> Option<[u8; 3]> {
    let mut seen: HashMap<[u8; 3], usize> = HashMap::new();
    for pixel in pixels {
        *seen.entry(*pixel).or_default() += 1;
    }
    seen.into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(pixel, _)| pixel)
}

fn srgb([r, g, b]: [u8; 3]) -> Srgba {
    Srgba([
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        1.0,
    ])
}
