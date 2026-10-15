//! The eyedropper's pure half: which pixel of a still picture of an output the pointer is over
//! and its colour (`sample`), where the magnifier goes and what part of the picture it shows
//! (`loupe`), and what the shell holds while a colour is asked for (`frozen`).

mod frozen;
mod loupe;
mod sample;

pub use frozen::{EyedropIn, EyedropOut, Frozen, eyedrop_step};
pub use loupe::{CELL, GRID, Loupe, SIDE, loupe_at};
pub use sample::{ShotPixel, colour_at, pick_at, pixel_under};
