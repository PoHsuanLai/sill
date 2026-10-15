//! Reading a colour from a still picture of an output. Pure.

use super::super::color::Rgb8;
use crate::desktop::OutputShot;

/// A pixel of a picture: its column and row, counted from the top left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShotPixel {
    pub x: u32,
    pub y: u32,
}

/// The pixel of a picture `shot` device pixels across and down that a pointer at `at` is over, on
/// a surface `surface` logical pixels across and down that shows the whole picture. The two sizes
/// give the scale on each axis, so a fractional scale (and the rounding of the logical size) is
/// exact; the pixel is the one the position falls in. None when the pointer is off the surface or
/// either size is empty.
pub fn pixel_under(shot: (u32, u32), surface: (u32, u32), at: (f64, f64)) -> Option<ShotPixel> {
    let across = |size: u32, extent: u32, position: f64| -> Option<u32> {
        let (size, extent) = (f64::from(size), f64::from(extent));
        // `contains` is false for a NaN position as well.
        (extent > 0.0 && size > 0.0 && (0.0..extent).contains(&position))
            .then(|| (position * size / extent).floor().min(size - 1.0) as u32)
    };
    Some(ShotPixel {
        x: across(shot.0, surface.0, at.0)?,
        y: across(shot.1, surface.1, at.1)?,
    })
}

/// The colour of `pixel` in `shot`, none when it lies outside the picture.
pub fn colour_at(shot: &OutputShot, pixel: ShotPixel) -> Option<Rgb8> {
    let (width, height) = shot.size;
    if pixel.x >= width || pixel.y >= height {
        return None;
    }
    let at = (pixel.y as usize * width as usize + pixel.x as usize) * 4;
    match shot.rgba.get(at..at + 3) {
        Some(&[red, green, blue]) => Some(Rgb8 { red, green, blue }),
        _ => None,
    }
}

/// The colour under a pointer at `at` on a surface `surface` logical pixels across and down that
/// shows `shot`.
pub fn pick_at(shot: &OutputShot, surface: (u32, u32), at: (f64, f64)) -> Option<Rgb8> {
    pixel_under(shot.size, surface, at).and_then(|pixel| colour_at(shot, pixel))
}
