//! On which outputs Mission Control is up, as the compositor reports it per output, and so on
//! which outputs the Spaces bar is drawn. Pure.

use std::collections::BTreeMap;

use casement_protocols::vocab::Shown;

use super::BarShow;

/// What the compositor last said of Mission Control on each output. Empty until the first report
/// (a compositor that does not report per output): then every output follows the overview as a
/// whole.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarOutputs(BTreeMap<String, Shown>);

impl BarOutputs {
    /// The same reports with `output` now `shown`.
    pub fn with(mut self, output: &str, shown: Shown) -> BarOutputs {
        self.0.insert(output.to_owned(), shown);
        self
    }

    /// The bar on the output called `output` (`None` for an unnamed one), given the overview as a
    /// whole is `overall`: up only when both say so. With no report at all the overview as a
    /// whole decides; once there are reports, an output they do not name has no bar.
    pub fn bar_on(&self, overall: BarShow, output: Option<&str>) -> BarShow {
        let here = match (self.0.is_empty(), output.and_then(|name| self.0.get(name))) {
            (true, _) | (false, Some(Shown::Shown)) => BarShow::Show,
            (false, _) => BarShow::Hide,
        };
        match (overall, here) {
            (BarShow::Show, BarShow::Show) => BarShow::Show,
            _ => BarShow::Hide,
        }
    }
}
