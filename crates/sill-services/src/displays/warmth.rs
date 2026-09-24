//! Night warmth through a monitor's own RGB gains (VCP 0x16/0x18/0x1A): a colour temperature,
//! chosen by a strength, turned into three per-mille multipliers of the monitor's gains as the
//! person left them. Pure. No schedule yet: the service applies whatever the setting says.
//!
//! The temperature-to-RGB curve is the well-known blackbody fit to CIE 1964 10-degree colour
//! matching data (in the public domain), evaluated here per channel and normalised so that
//! 6500 K is exactly neutral: at strength 0 the gains are untouched.

use serde::{Deserialize, Serialize};

use crate::units::{Percent, Permille};

/// A colour temperature in kelvin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Kelvin(pub u16);

/// Neutral: a D65-calibrated monitor's own white.
pub const NEUTRAL: Kelvin = Kelvin(6500);
/// Full strength: about a warm incandescent bulb.
pub const WARMEST: Kelvin = Kelvin(2700);

/// Three multipliers of a monitor's gains, each at most [`Permille::ONE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Gains {
    pub red: Permille,
    pub green: Permille,
    pub blue: Permille,
}

impl Gains {
    pub const NEUTRAL: Gains = Gains {
        red: Permille::ONE,
        green: Permille::ONE,
        blue: Permille::ONE,
    };
}

/// The temperature a strength stands for: linear in mireds (1e6 / K), which is how evenly
/// warmer light looks to the eye, from [`NEUTRAL`] at 0 to [`WARMEST`] at 100.
pub fn temperature(strength: Percent) -> Kelvin {
    let s = f64::from(strength.0.min(100)) / 100.0;
    let mired = |k: Kelvin| 1e6 / f64::from(k.0);
    let m = mired(NEUTRAL) + s * (mired(WARMEST) - mired(NEUTRAL));
    Kelvin((1e6 / m).round() as u16)
}

/// The multipliers for `kelvin`, normalised to 6500 K, and clamped to 1000-6500 K.
pub fn gains(kelvin: Kelvin) -> Gains {
    let k = f64::from(kelvin.0.clamp(1000, NEUTRAL.0));
    let (r, g, b) = blackbody(k);
    let (r0, g0, b0) = blackbody(f64::from(NEUTRAL.0));
    let per_mille = |v: f64, v0: f64| Permille(((v / v0).clamp(0.0, 1.0) * 1000.0).round() as u32);
    Gains {
        red: per_mille(r, r0),
        green: per_mille(g, g0),
        blue: per_mille(b, b0),
    }
}

/// The gains for night warmth at `strength`.
pub fn warmth_gains(strength: Percent) -> Gains {
    gains(temperature(strength))
}

/// A monitor gain `original` (its raw VCP value) scaled by `factor`.
pub fn scaled(original: u16, factor: Permille) -> u16 {
    let v = (u32::from(original) * factor.0 + 500) / 1000;
    u16::try_from(v).unwrap_or(u16::MAX)
}

/// The blackbody fit, per channel on a 0-255 scale, for 1000-6600 K (the range used here).
fn blackbody(kelvin: f64) -> (f64, f64, f64) {
    let t = kelvin / 100.0;
    let red = 255.0;
    let green = (99.470_802_586_1 * t.ln() - 161.119_568_166_1).clamp(0.0, 255.0);
    let blue = if t <= 19.0 {
        0.0
    } else {
        (138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7).clamp(0.0, 255.0)
    };
    (red, green, blue)
}

#[cfg(test)]
mod tests {
    use super::{Gains, Kelvin, gains, scaled, temperature, warmth_gains};
    use crate::units::{Percent, Permille};

    #[test]
    fn strength_maps_evenly_in_mireds() {
        const CASES: &[(u16, u16)] = &[(0, 6500), (50, 3815), (100, 2700), (150, 2700)];
        for &(strength, kelvin) in CASES {
            assert_eq!(
                temperature(Percent(strength)),
                Kelvin(kelvin),
                "{strength}%"
            );
        }
    }

    #[test]
    fn warmer_light_takes_blue_then_green_away() {
        let g = |k| gains(Kelvin(k));
        assert_eq!(g(6500), Gains::NEUTRAL);
        assert_eq!(g(9000), Gains::NEUTRAL, "never cooler than neutral");
        let cases: &[(u16, (u32, u32, u32))] = &[
            (5000, (1000, 897, 824)),
            (3400, (1000, 746, 541)),
            (2700, (1000, 656, 350)),
            (1000, (1000, 267, 0)),
        ];
        for &(kelvin, (r, gr, b)) in cases {
            assert_eq!(
                g(kelvin),
                Gains {
                    red: Permille(r),
                    green: Permille(gr),
                    blue: Permille(b),
                },
                "{kelvin} K"
            );
        }
        assert_eq!(warmth_gains(Percent(0)), Gains::NEUTRAL);
    }

    #[test]
    fn a_gain_scales_from_the_monitors_own_value() {
        const CASES: &[(u16, u32, u16)] =
            &[(100, 1000, 100), (100, 656, 66), (50, 350, 18), (0, 500, 0)];
        for &(original, factor, want) in CASES {
            assert_eq!(
                scaled(original, Permille(factor)),
                want,
                "{original} x {factor}"
            );
        }
    }
}
