//! The brightness each display is at, as the shell's bus lists it (`org.quire.Displays1`).
//! Pure: the outputs whose brightness can be read, in the compositor's order.

use super::Displays;
use crate::ids::OutputName;
use crate::units::Percent;

/// One output and its brightness now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputLevel {
    pub output: OutputName,
    pub percent: Percent,
}

impl OutputLevel {
    /// The bus's `(su)`: the output's name and its percentage, 0 to 100.
    pub fn wire(&self) -> (String, u32) {
        (self.output.0.clone(), u32::from(self.percent.0.min(100)))
    }
}

impl Displays {
    /// Every output whose brightness sill can read (a built-in panel's backlight, a monitor that
    /// answers DDC/CI), in the compositor's order. An output with nothing to read is not listed.
    pub fn levels(&self) -> Vec<OutputLevel> {
        self.outputs
            .iter()
            .filter_map(|o| {
                self.brightness(&self.target(&o.name))
                    .map(|percent| OutputLevel {
                        output: o.name.clone(),
                        percent,
                    })
            })
            .collect()
    }
}

/// The bus's answer for `levels`: `a(su)`.
pub fn wire_of(levels: &[OutputLevel]) -> Vec<(String, u32)> {
    levels.iter().map(OutputLevel::wire).collect()
}
