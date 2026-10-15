//! The magnifier: a round window on the picture that follows the pointer and shows the pixels
//! around it enlarged, the pixel under the pointer in the middle. Pure.
//!
//! The picture is drawn once at [`CELL`] logical pixels per device pixel behind a clip of
//! [`SIDE`] logical pixels, and the magnifier only moves that picture so the pixel under the
//! pointer lies in the centre cell.

use super::sample::pixel_under;

/// Device pixels the magnifier shows across; odd, so one pixel is in the centre.
pub const GRID: u32 = 11;

/// Logical pixels one device pixel is enlarged to.
pub const CELL: u32 = 12;

/// The magnifier's width and height in logical pixels.
pub const SIDE: u32 = GRID * CELL;

/// Where the magnifier is and how the picture lies inside it, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loupe {
    /// The magnifier's left edge on the surface: centred on the pointer.
    pub left: i32,
    /// The magnifier's top edge on the surface.
    pub top: i32,
    /// The enlarged picture's left edge, relative to the magnifier.
    pub picture_left: i32,
    /// The enlarged picture's top edge, relative to the magnifier.
    pub picture_top: i32,
    /// The enlarged picture's width.
    pub picture_width: u32,
    /// The enlarged picture's height.
    pub picture_height: u32,
}

/// The magnifier for a pointer at `at` on a surface `surface` logical pixels across and down that
/// shows a picture `shot` device pixels across and down; none when the pointer is off the picture.
pub fn loupe_at(shot: (u32, u32), surface: (u32, u32), at: (f64, f64)) -> Option<Loupe> {
    let pixel = pixel_under(shot, surface, at)?;
    let centre = (GRID / 2) as i32;
    let cell = CELL as i32;
    let half = (SIDE / 2) as i32;
    Some(Loupe {
        left: at.0.round() as i32 - half,
        top: at.1.round() as i32 - half,
        picture_left: (centre - pixel.x as i32) * cell,
        picture_top: (centre - pixel.y as i32) * cell,
        picture_width: shot.0 * CELL,
        picture_height: shot.1 * CELL,
    })
}
