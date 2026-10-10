//! Output heads from `zwlr-output-management` as the displays service reads them, and the
//! configuration that sets one head's scale.

use desktop_client::output::{
    ConfigError, Enabled, Head as WireHead, HeadChangeRequest, HeadConfig, HeadSettings,
    OutputConfig, Outputs, ScaleMilli,
};
use sill_model::displays::randr::{Enablement, Head};
use sill_model::displays::scale::DisplayScale;
use sill_model::displays::{Millimetres, Mode, Pixels};
use sill_model::ids::OutputName;

/// Why a scale could not become a configuration.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScaleRefusal {
    /// The compositor lists no head of this name.
    #[error("no output {0}")]
    UnknownHead(String),
    /// The configuration does not fit the heads as they are.
    #[error("{0}")]
    Invalid(#[from] ConfigError),
}

/// Every head the compositor lists, as the displays service describes an output.
pub fn heads_of(outputs: &Outputs) -> Vec<Head> {
    outputs.heads().map(head).collect()
}

fn head(wire: &WireHead) -> Head {
    Head {
        name: OutputName(wire.name.as_str().to_owned()),
        enablement: match wire.enabled {
            Enabled::On => Enablement::Enabled,
            Enabled::Off => Enablement::Disabled,
        },
        make: wire.make.clone().unwrap_or_default(),
        model: wire.model.clone().unwrap_or_default(),
        physical: wire
            .physical_size_mm
            .filter(|size| size.w > 0 && size.h > 0)
            .map(|size| Millimetres {
                width: size.w,
                height: size.h,
            }),
        current: wire.current().map(|mode| Mode {
            pixels: Pixels {
                width: mode.size.w,
                height: mode.size.h,
            },
            refresh_mhz: mode.refresh.get(),
        }),
        scale: display_scale(wire.scale),
    }
}

/// Hundredths, rounded: the wire's thousandths are finer than a person sets.
fn display_scale(scale: ScaleMilli) -> DisplayScale {
    DisplayScale(u16::try_from((scale.get() + 5) / 10).unwrap_or(u16::MAX))
}

/// The configuration that runs the head named `name` at `scale`, keeping everything else.
pub fn config_for_scale(
    outputs: &Outputs,
    name: &OutputName,
    scale: DisplayScale,
) -> Result<OutputConfig, ScaleRefusal> {
    let head = outputs
        .heads()
        .find(|head| head.name.as_str() == name.0)
        .ok_or_else(|| ScaleRefusal::UnknownHead(name.0.clone()))?;
    let settings = HeadSettings::keep().scaled(ScaleMilli::new(u32::from(scale.0) * 10));
    let config = OutputConfig::new(vec![HeadConfig::new(
        head.key,
        HeadChangeRequest::Enable(settings),
    )]);
    config.validate(outputs)?;
    Ok(config)
}
