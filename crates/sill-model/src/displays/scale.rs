//! The default scale for a display, from its physical density: aim every display at the same
//! logical density (Apple's 27-inch one, 2560 x 1440 points on 27 inches, about 109 points per
//! inch) and round to a step the compositor renders well. Pure and table-tested.
//!
//! `display.scale_rounding` chooses how the rounding goes: [`ScaleRounding::Nearest`] takes
//! whichever step (whole or quarter) sits closest, for the most logical space; the compositor
//! still resamples a fractional scale, which softens edges. [`ScaleRounding::PreferWhole`] is
//! macOS's own default for a sharp external display - a 4K screen defaults to 2x there, "looks
//! like 1920x1080" - trading some space for a scale the renderer draws pixel for pixel.

use ds_core::word::Word;
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

    /// `1.5`, `1.25`, `2`: how a person writes it.
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

/// The narrowest logical width [`ScaleRounding::PreferWhole`] will still round up for: below
/// this, the whole scale it would pick leaves too little space to use, so it falls back to
/// [`ScaleRounding::Nearest`] instead. 1280 is the width of the smallest common laptop panel
/// (1280 x 720/800), so nothing that already fits a real desktop is pushed narrower than that.
pub const MIN_LOGICAL_WIDTH: u32 = 1280;

/// How [`default_scale`] rounds a display's ideal (`display.scale_rounding`; module doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum ScaleRounding {
    /// Round up to a whole scale (1, 2 or 3) when the ideal is at least 1.5 and the display
    /// stays at least [`MIN_LOGICAL_WIDTH`] px wide at that scale; [`Nearest`](Self::Nearest)
    /// otherwise.
    #[default]
    PreferWhole,
    /// The nearest step, pulled to an integer within [`INTEGER_PULL`]: sill's original
    /// behaviour, and more logical space than a whole scale gives.
    Nearest,
}

/// The physical density of `pixels` spread over `size`, along the diagonal; `None` when the
/// size is missing or implausible.
pub fn density(pixels: Pixels, size: Option<Millimetres>) -> Option<Ppi> {
    exact_density(pixels, size).map(|ppi| Ppi(ppi.round() as u16))
}

/// The default scale for a display of `pixels` over `size`, aimed at `target` and rounded by
/// `rounding` (see the module doc). With no usable size, 2 for a panel at least 3840 px wide (a
/// 4K TV or projector is always viewed from further than its size would say) and 1 otherwise -
/// `rounding` plays no part, since there is no density to round.
pub fn default_scale(
    pixels: Pixels,
    size: Option<Millimetres>,
    target: Ppi,
    rounding: ScaleRounding,
) -> DisplayScale {
    match exact_density(pixels, size) {
        Some(ppi) if target.0 > 0 => {
            let ideal = (ppi * 100.0 / f64::from(target.0)).round() as u32;
            match rounding {
                ScaleRounding::Nearest => snap(ideal),
                ScaleRounding::PreferWhole => prefer_whole(ideal, pixels.width),
            }
        }
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

/// `ideal` (hundredths) rounded half-up to the nearest whole scale, clamped to 1..=3: both what
/// [`snap`] pulls a near-integer `ideal` to, and what [`ScaleRounding::PreferWhole`] tries first.
fn nearest_whole(ideal: u32) -> u32 {
    ((ideal + 50) / 100).clamp(1, 3)
}

/// `ideal` snapped: the integer when within [`INTEGER_PULL`] of one, else the nearest step (the
/// lower of two equally near), clamped to the steps' range.
fn snap(ideal: u32) -> DisplayScale {
    let integer = nearest_whole(ideal) * 100;
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

/// The largest whole scale [`ScaleRounding::PreferWhole`] picks: 3x on a 15.6-inch 4K laptop
/// leaves 1280 px across, tighter than any Mac laptop's default, so a denser panel stays at 2x
/// and nearest-step rounding remains the way to 2.5 or 3.
const MAX_WHOLE: u32 = 2;

/// [`ScaleRounding::PreferWhole`]: below 1.5, [`snap`]; at or above it, `ideal` rounded half-up
/// to a whole scale ([`nearest_whole`]) and capped at [`MAX_WHOLE`], so long as `width` stays
/// at least [`MIN_LOGICAL_WIDTH`] px across at that scale, else [`snap`].
fn prefer_whole(ideal: u32, width: u32) -> DisplayScale {
    if ideal < 150 {
        return snap(ideal);
    }
    let whole = nearest_whole(ideal).min(MAX_WHOLE);
    if width / whole >= MIN_LOGICAL_WIDTH {
        DisplayScale((whole * 100) as u16)
    } else {
        snap(ideal)
    }
}

#[cfg(test)]
mod tests;
