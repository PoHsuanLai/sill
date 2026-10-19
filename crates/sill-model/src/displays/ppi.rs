//! A display's physical density, from its pixels and its EDID size. What the font-rendering
//! default follows; pure and table-tested.

use serde::{Deserialize, Serialize};

use super::edid::{Millimetres, Pixels};

/// Points per inch, whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ppi(pub u16);

/// A density outside this range is a size the EDID got wrong (a TV reporting 16 x 9 cm, a
/// placeholder), not a display: treat it as no size. 600 clears the densest phone panels Linux
/// runs on (about 560).
const PLAUSIBLE_PPI: std::ops::RangeInclusive<u32> = 40..=600;

/// The physical density of `pixels` spread over `size`, along the diagonal; `None` when the
/// size is missing or implausible.
pub fn density(pixels: Pixels, size: Option<Millimetres>) -> Option<Ppi> {
    let size = size?;
    let diagonal_px = f64::from(pixels.width).hypot(f64::from(pixels.height));
    let diagonal_in = f64::from(size.width).hypot(f64::from(size.height)) / 25.4;
    let ppi = (diagonal_px / diagonal_in).round();
    (diagonal_in > 0.0 && PLAUSIBLE_PPI.contains(&(ppi as u32))).then_some(Ppi(ppi as u16))
}

#[cfg(test)]
mod tests {
    use super::{Ppi, density};
    use crate::displays::edid::{Millimetres, Pixels};

    #[test]
    fn density_follows_the_diagonal_and_refuses_nonsense() {
        let px = |width, height| Pixels { width, height };
        let mm = |width, height| Some(Millimetres { width, height });
        let cases = [
            // The MA270U's own EDID millimetres.
            ("BenQ MA270U", px(3840, 2160), mm(597, 336), Some(Ppi(163))),
            ("14in laptop", px(2880, 1800), mm(302, 189), Some(Ppi(242))),
            ("no size", px(3840, 2160), None, None),
            (
                "a TV reporting 16 x 9 cm",
                px(3840, 2160),
                mm(160, 90),
                None,
            ),
            ("a zero size", px(1920, 1080), mm(0, 0), None),
        ];
        for (label, pixels, size, want) in cases {
            assert_eq!(density(pixels, size), want, "{label}");
        }
    }
}
