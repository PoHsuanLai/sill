//! The default scale for a display, from its physical density: aim every display at the same
//! logical density (Apple's 27-inch one, 2560 x 1440 points on 27 inches, about 109 points per
//! inch) and snap to a step the compositor renders well. Pure and table-tested.

use serde::{Deserialize, Serialize};

use super::edid::{Millimetres, Pixels};

/// An output scale in hundredths (`150` is 1.5).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct DisplayScale(pub u16);

impl DisplayScale {
    pub const ONE: DisplayScale = DisplayScale(100);
    pub const TWO: DisplayScale = DisplayScale(200);

    /// `1.5`, `1.25`, `2`: how `cosmic-randr --scale` and a person write it.
    pub fn decimal(self) -> String {
        let (whole, cents) = (self.0 / 100, self.0 % 100);
        match cents {
            0 => whole.to_string(),
            c if c % 10 == 0 => format!("{whole}.{}", c / 10),
            c => format!("{whole}.{c:02}"),
        }
    }

    /// `1.5` or `1.50` or `2` back to hundredths; `None` for anything that is not a positive
    /// decimal with at most two places.
    pub fn parse(text: &str) -> Option<DisplayScale> {
        let text = text.trim();
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        if whole.is_empty() || fraction.len() > 2 || !fraction.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let whole: u16 = whole.parse().ok()?;
        let cents: u16 = format!("{fraction:0<2}").parse().ok()?;
        let value = whole.checked_mul(100)?.checked_add(cents)?;
        (value > 0).then_some(DisplayScale(value))
    }
}

/// Points per inch, whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ppi(pub u16);

impl Ppi {
    /// `display.density_target_ppi`'s default: 2560 px across a 27-inch 16:9 panel.
    pub const APPLE_27: Ppi = Ppi(109);
}

/// The steps a default may land on.
const STEPS: [u16; 7] = [100, 125, 150, 175, 200, 250, 300];

/// How close to an integer the ideal scale must be to take the integer (sharper, and the
/// compositor skips resampling): 0.15.
const INTEGER_PULL: u32 = 15;

/// A density outside this range is a size the EDID got wrong (a TV reporting 16 x 9 cm, a
/// placeholder), not a display: fall back as if there were no size. 600 clears the densest
/// phone panels Linux runs on (about 560).
const PLAUSIBLE_PPI: std::ops::RangeInclusive<u32> = 40..=600;

/// The physical density of `pixels` spread over `size`, along the diagonal; `None` when the
/// size is missing or implausible.
pub fn density(pixels: Pixels, size: Option<Millimetres>) -> Option<Ppi> {
    exact_density(pixels, size).map(|ppi| Ppi(ppi.round() as u16))
}

/// The default scale for a display of `pixels` over `size`, aimed at `target` (see the module
/// doc). With no usable size, 2 for a panel at least 3840 px wide (a 4K TV or projector is
/// always viewed from further than its size would say) and 1 otherwise.
pub fn default_scale(pixels: Pixels, size: Option<Millimetres>, target: Ppi) -> DisplayScale {
    match exact_density(pixels, size) {
        Some(ppi) if target.0 > 0 => snap((ppi * 100.0 / f64::from(target.0)).round() as u32),
        _ if pixels.width >= 3840 => DisplayScale::TWO,
        _ => DisplayScale::ONE,
    }
}

/// Diagonal pixels over diagonal inches, within [`PLAUSIBLE_PPI`]. The float never leaves
/// this module's arithmetic.
fn exact_density(pixels: Pixels, size: Option<Millimetres>) -> Option<f64> {
    let size = size?;
    let diagonal_px = f64::from(pixels.width).hypot(f64::from(pixels.height));
    let diagonal_in = f64::from(size.width).hypot(f64::from(size.height)) / 25.4;
    let ppi = diagonal_px / diagonal_in;
    (diagonal_in > 0.0 && PLAUSIBLE_PPI.contains(&(ppi.round() as u32))).then_some(ppi)
}

/// `ideal` snapped: the integer when within [`INTEGER_PULL`] of one, else the nearest step (the
/// lower of two equally near), clamped to the steps' range.
fn snap(ideal: u32) -> DisplayScale {
    let integer = ((ideal + 50) / 100).clamp(1, 3) * 100;
    if ideal.abs_diff(integer) <= INTEGER_PULL {
        return DisplayScale(integer as u16);
    }
    let nearest = STEPS
        .iter()
        .copied()
        .min_by_key(|step| u32::from(*step).abs_diff(ideal))
        .unwrap_or(100);
    DisplayScale(nearest)
}

#[cfg(test)]
mod tests;
