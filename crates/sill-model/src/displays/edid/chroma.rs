//! The colour half of an EDID base block: the panel's primaries and white point (EDID 1.3/1.4
//! bytes 25-34), in the block's own 10-bit fixed point.

use serde::{Deserialize, Serialize};

/// A CIE 1931 xy chromaticity in 1/1024 units, as EDID stores it (`697` is x = 0.681).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Chromaticity {
    pub x: u16,
    pub y: u16,
}

impl Chromaticity {
    /// The point in thousandths, rounded: the unit the design docs and `edid-decode` quote.
    pub fn thousandths(self) -> (u16, u16) {
        let round = |v: u16| ((u32::from(v) * 1000 + 512) / 1024) as u16;
        (round(self.x), round(self.y))
    }

    /// The point as plain numbers for geometry. Never stored (CONVENTIONS §12).
    pub(crate) fn xy(self) -> (f64, f64) {
        (f64::from(self.x) / 1024.0, f64::from(self.y) / 1024.0)
    }
}

/// The three primaries and the white point a display reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Primaries {
    pub red: Chromaticity,
    pub green: Chromaticity,
    pub blue: Chromaticity,
    pub white: Chromaticity,
}

/// Bytes 25-34 of a base block: byte 25 holds the two low bits of red x/y and green x/y, byte
/// 26 those of blue and white, and bytes 27-34 the high eight bits of each coordinate in the
/// order red x, red y, green x, green y, blue x, blue y, white x, white y.
pub(crate) fn primaries(block: &[u8; 128]) -> Primaries {
    let low_rg = block[25];
    let low_bw = block[26];
    let point = |high_x: usize, high_y: usize, low: u8, shift: u8| Chromaticity {
        x: (u16::from(block[high_x]) << 2) | u16::from((low >> (shift + 2)) & 0b11),
        y: (u16::from(block[high_y]) << 2) | u16::from((low >> shift) & 0b11),
    };
    Primaries {
        red: point(27, 28, low_rg, 4),
        green: point(29, 30, low_rg, 0),
        blue: point(31, 32, low_bw, 4),
        white: point(33, 34, low_bw, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::Chromaticity;

    #[test]
    fn thousandths_round_to_nearest() {
        /// (10-bit x, y) and the thousandths they round to.
        type Case = ((u16, u16), (u16, u16));
        const CASES: &[Case] = &[
            ((697, 324), (681, 316)),
            ((0, 1024), (0, 1000)),
            ((321, 337), (313, 329)),
        ];
        for &((x, y), want) in CASES {
            assert_eq!(Chromaticity { x, y }.thousandths(), want, "{x},{y}");
        }
    }
}
