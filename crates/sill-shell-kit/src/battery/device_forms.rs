//! The filled device glyphs' geometry (design/23-WIDGETS.md section 4.4, settled 2026-09-27):
//! abstract solids on the Lucide 24 grid, one form per [`Device`], as data. A form is a list of
//! pieces joined into one path; under [`FillRule::EvenOdd`] a piece inside another is a hole
//! (a keyboard's keys, a mouse's wheel slot), under [`FillRule::NonZero`] overlapping pieces
//! merge (an earbud's head and stem). No outline and no stroke anywhere: the reference's device
//! symbols are filled, and ours are drawn in our own geometry, not traced from its.

use crate::battery::device_glyph::Device;
use ds::base::word::Word;

/// A rounded rectangle on the 24 grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Slab {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) r: f32,
}

/// One piece of a form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Piece {
    /// A rounded rectangle.
    Slab(Slab),
    /// A disc: centre x, centre y, radius.
    Disc(f32, f32, f32),
    /// Any other outline, as path data (closed).
    Path(&'static str),
}

/// How overlapping pieces combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub(crate) enum FillRule {
    /// A piece inside another cuts a hole.
    #[word(slug = "evenodd")]
    EvenOdd,
    /// Pieces merge wherever they overlap.
    #[word(slug = "nonzero")]
    NonZero,
}

/// A device's form: its pieces and how they combine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Form {
    pub(crate) rule: FillRule,
    pub(crate) pieces: &'static [Piece],
}

const fn slab(x: f32, y: f32, w: f32, h: f32, r: f32) -> Piece {
    Piece::Slab(Slab { x, y, w, h, r })
}

const fn form(rule: FillRule, pieces: &'static [Piece]) -> Form {
    Form { rule, pieces }
}

/// A laptop: a solid screen over a wider base, a unit and a half of air between them.
const LAPTOP: Form = form(
    FillRule::NonZero,
    &[
        slab(4.0, 4.5, 16.0, 11.5, 2.0),
        slab(1.5, 17.5, 21.0, 2.5, 1.25),
    ],
);

/// A desktop (this computer without a battery of its own): a screen on a short stand and foot.
const DESKTOP: Form = form(
    FillRule::NonZero,
    &[
        slab(2.0, 3.0, 20.0, 13.5, 2.5),
        slab(10.25, 16.0, 3.5, 3.5, 0.0),
        slab(7.0, 19.0, 10.0, 2.0, 1.0),
    ],
);

/// A phone: a tall slab with a speaker slot cut near its top.
const PHONE: Form = form(
    FillRule::EvenOdd,
    &[
        slab(6.0, 1.5, 12.0, 21.0, 3.0),
        slab(10.0, 3.75, 4.0, 1.25, 0.625),
    ],
);

/// A tablet: a wide slab with a round mark cut near its foot.
const TABLET: Form = form(
    FillRule::EvenOdd,
    &[
        slab(3.5, 2.0, 17.0, 20.0, 2.5),
        Piece::Disc(12.0, 19.25, 0.9),
    ],
);

/// A watch: a rounded case between two band stubs, the crown at its right.
const WATCH: Form = form(
    FillRule::NonZero,
    &[
        slab(8.0, 1.5, 8.0, 4.5, 1.25),
        slab(5.5, 5.0, 13.0, 14.0, 3.75),
        slab(8.0, 18.0, 8.0, 4.5, 1.25),
        slab(17.5, 10.0, 2.5, 4.0, 1.0),
    ],
);

/// Headphones: a thick band over two ear cups.
const HEADPHONES: Form = form(
    FillRule::NonZero,
    &[
        Piece::Path("M3 14.5V13a9 9 0 0 1 18 0v1.5h-2.25V13a6.75 6.75 0 0 0-13.5 0v1.5z"),
        slab(2.5, 13.0, 5.5, 8.5, 2.0),
        slab(16.0, 13.0, 5.5, 8.5, 2.0),
    ],
);

/// Earbuds: two rounded heads, each on a stem.
const EARBUDS: Form = form(
    FillRule::NonZero,
    &[
        Piece::Disc(7.0, 7.0, 3.75),
        slab(7.25, 8.0, 3.0, 13.0, 1.5),
        Piece::Disc(17.0, 7.0, 3.75),
        slab(13.75, 8.0, 3.0, 13.0, 1.5),
    ],
);

/// A mouse: a capsule with its wheel slot cut in the top half.
const MOUSE: Form = form(
    FillRule::EvenOdd,
    &[
        slab(6.0, 2.0, 12.0, 20.0, 6.0),
        slab(10.75, 5.0, 2.5, 5.0, 1.25),
    ],
);

/// A keyboard: a wide slab with two rows of five keys and a space bar cut through it.
const KEYBOARD: Form = form(
    FillRule::EvenOdd,
    &[
        slab(1.5, 5.5, 21.0, 13.0, 2.5),
        slab(4.0, 8.25, 2.0, 2.0, 0.5),
        slab(7.5, 8.25, 2.0, 2.0, 0.5),
        slab(11.0, 8.25, 2.0, 2.0, 0.5),
        slab(14.5, 8.25, 2.0, 2.0, 0.5),
        slab(18.0, 8.25, 2.0, 2.0, 0.5),
        slab(4.0, 11.5, 2.0, 2.0, 0.5),
        slab(7.5, 11.5, 2.0, 2.0, 0.5),
        slab(11.0, 11.5, 2.0, 2.0, 0.5),
        slab(14.5, 11.5, 2.0, 2.0, 0.5),
        slab(18.0, 11.5, 2.0, 2.0, 0.5),
        slab(7.0, 14.75, 10.0, 1.75, 0.875),
    ],
);

/// A speaker: a tall slab, a small tweeter and a ring for the woofer cut from it.
const SPEAKER: Form = form(
    FillRule::EvenOdd,
    &[
        slab(5.0, 2.0, 14.0, 20.0, 3.0),
        Piece::Disc(12.0, 6.75, 1.5),
        Piece::Disc(12.0, 14.75, 4.25),
        Piece::Disc(12.0, 14.75, 1.75),
    ],
);

/// A game controller: a wide capsule with a cross and two buttons cut from it.
const GAMEPAD: Form = form(
    FillRule::EvenOdd,
    &[
        slab(1.5, 6.5, 21.0, 11.5, 5.75),
        Piece::Path("M6.25 9.5h1.75v1.75h1.75V13H8v1.75H6.25V13H4.5v-1.75h1.75z"),
        Piece::Disc(15.5, 10.75, 1.1),
        Piece::Disc(18.0, 13.25, 1.1),
    ],
);

/// Any other device with a battery: a cell with its terminal.
const OTHER: Form = form(
    FillRule::NonZero,
    &[
        slab(2.0, 7.0, 17.5, 10.0, 2.5),
        slab(20.0, 10.0, 2.0, 4.0, 1.0),
    ],
);

/// `device`'s form.
pub(crate) fn form_of(device: Device) -> Form {
    match device {
        Device::Laptop => LAPTOP,
        Device::Desktop => DESKTOP,
        Device::Phone => PHONE,
        Device::Tablet => TABLET,
        Device::Watch => WATCH,
        Device::Headphones => HEADPHONES,
        Device::Earbuds => EARBUDS,
        Device::Mouse => MOUSE,
        Device::Keyboard => KEYBOARD,
        Device::Speaker => SPEAKER,
        Device::Gamepad => GAMEPAD,
        Device::Other => OTHER,
    }
}

/// `n` without a trailing `.0`, to at most three decimals.
fn num(n: f32) -> String {
    let text = format!("{n:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    match text {
        "-0" | "" => "0".to_owned(),
        _ => text.to_owned(),
    }
}

/// A rounded rectangle as a closed path, clockwise from its top edge.
fn slab_path(s: Slab) -> String {
    let r = s.r.min(s.w / 2.0).min(s.h / 2.0);
    let (x, y, w, h) = (
        num(s.x + r),
        num(s.y),
        num(s.w - 2.0 * r),
        num(s.h - 2.0 * r),
    );
    let r = num(r);
    format!(
        "M{x} {y}h{w}a{r} {r} 0 0 1 {r} {r}v{h}a{r} {r} 0 0 1 -{r} {r}h-{w}a{r} {r} 0 0 1 -{r} -{r}v-{h}a{r} {r} 0 0 1 {r} -{r}z"
    )
}

/// A disc as a closed path of two half arcs.
fn disc_path(cx: f32, cy: f32, radius: f32) -> String {
    let (x, y, r, d) = (num(cx - radius), num(cy), num(radius), num(2.0 * radius));
    format!("M{x} {y}a{r} {r} 0 1 0 {d} 0a{r} {r} 0 1 0 -{d} 0z")
}

/// One piece's path data.
fn piece_path(piece: Piece) -> String {
    match piece {
        Piece::Slab(slab) => slab_path(slab),
        Piece::Disc(cx, cy, r) => disc_path(cx, cy, r),
        Piece::Path(d) => d.to_owned(),
    }
}

/// The form's pieces as one path's data.
pub(crate) fn path_of(form: Form) -> String {
    form.pieces.iter().copied().map(piece_path).collect()
}

#[cfg(test)]
mod tests {
    use super::{Piece, Slab, form_of, num, path_of, slab_path};
    use crate::battery::device_glyph::Device;
    use ds::base::word::Word;

    /// A piece's bounds on the grid: (left, top, right, bottom); a free path reports none.
    fn bounds(piece: Piece) -> Option<(f32, f32, f32, f32)> {
        match piece {
            Piece::Slab(s) => Some((s.x, s.y, s.x + s.w, s.y + s.h)),
            Piece::Disc(cx, cy, r) => Some((cx - r, cy - r, cx + r, cy + r)),
            Piece::Path(_) => None,
        }
    }

    #[test]
    fn numbers_print_as_the_design_writes_them() {
        assert_eq!(num(2.0), "2");
        assert_eq!(num(0.625), "0.625");
        assert_eq!(num(12.5), "12.5");
        assert_eq!(num(-0.0), "0");
    }

    #[test]
    fn a_slab_is_one_closed_path() {
        let d = slab_path(Slab {
            x: 1.0,
            y: 2.0,
            w: 10.0,
            h: 6.0,
            r: 2.0,
        });
        assert_eq!(
            d,
            "M3 2h6a2 2 0 0 1 2 2v2a2 2 0 0 1 -2 2h-6a2 2 0 0 1 -2 -2v-2a2 2 0 0 1 2 -2z"
        );
    }

    /// Every form stays on the Lucide grid's live area: a unit of margin inside the 24 box,
    /// as Lucide keeps its own glyphs (design/08-ICONS.md section 1.3).
    #[test]
    fn every_form_is_on_the_grid() {
        for device in Device::ALL.iter().copied() {
            let form = form_of(device);
            assert!(!form.pieces.is_empty(), "{device:?}");
            assert!(path_of(form).ends_with('z'), "{device:?}");
            for piece in form.pieces {
                if let Some((left, top, right, bottom)) = bounds(*piece) {
                    assert!(left >= 1.0 && top >= 1.0, "{device:?} {piece:?}");
                    assert!(right <= 23.0 && bottom <= 23.0, "{device:?} {piece:?}");
                }
            }
        }
    }
}
