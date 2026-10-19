//! Output heads from `zwlr-output-management` as the displays service reads them.

use desktop_client::output::{Enabled, Head as WireHead, Outputs, ScaleMilli};
use sill_model::displays::randr::{Enablement, Head};
use sill_model::displays::scale::DisplayScale;
use sill_model::displays::{Millimetres, Mode, Pixels};
use sill_model::ids::OutputName;

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
