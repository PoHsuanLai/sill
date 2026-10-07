//! A module tile's disc: the glyph on its paper or accent disc and the ring while the module is
//! busy (design/26-DETAILS.md 5.2.2, 5.2.5).

use dioxus::prelude::*;
use ds::base::vocab::{Availability, Check};
use ds::components::content::icon_source::IconSource;
use ds::components::content::icon_view::IconView;
use ds::components::controls::progress::model::{Progress, ProgressStyle};
use ds::components::controls::progress::view::ProgressIndicator;
use ds::motion::detail::{
    detailed::Detailed, moment::Moment, touch::Touch, use_detail::use_detail,
    use_operation::use_operation,
};
use ds::style::icon::render::IconSize;

/// Where the disc stands: off, on, or working towards a state it has not reached (connecting,
/// scanning). What the disc plays follows from the change between two of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Lighting {
    /// A paper disc with ink.
    Off,
    /// The disc in `--accent`.
    On,
    /// Its ring turns.
    Busy,
}

impl Lighting {
    /// The disc's lighting for a module's value and availability: a busy module is neither on
    /// nor off yet.
    pub(crate) fn of(value: Check, availability: Availability) -> Self {
        match (availability, value) {
            (Availability::Busy, _) => Lighting::Busy,
            (Availability::Enabled | Availability::Disabled, Check::On) => Lighting::On,
            (Availability::Enabled | Availability::Disabled, Check::Off | Check::Mixed) => {
                Lighting::Off
            }
        }
    }
}

impl Detailed for Lighting {
    fn moment(from: &Self, to: &Self) -> Moment {
        match (from, to) {
            (Lighting::Off | Lighting::On | Lighting::Busy, Lighting::Busy) => Moment::Pending,
            (Lighting::Off | Lighting::Busy, Lighting::On) => Moment::Success,
            (Lighting::On, Lighting::On) => Moment::Rest,
            (Lighting::Off | Lighting::On | Lighting::Busy, Lighting::Off) => Moment::Change,
        }
    }

    fn first(state: &Self) -> Moment {
        match state {
            Lighting::Busy => Moment::Pending,
            Lighting::Off | Lighting::On => Moment::Rest,
        }
    }
}

/// `span.ds-module-disc`: `glyph` on the disc, with the pending ring while `lighting` is busy.
#[component]
pub(crate) fn ModuleDisc(glyph: IconSource, lighting: Lighting) -> Element {
    let detail = use_detail(lighting, Touch::Remote);
    // Busy is an operation the tile's own availability starts.
    let operation = use_operation(detail.cue());
    rsx! {
        span { class: "ds-module-disc",
            IconView { source: glyph, size: IconSize::Base }
            if lighting == Lighting::Busy {
                ProgressIndicator {
                    style: ProgressStyle::Ring,
                    progress: Progress::Unknown(operation),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Lighting;
    use super::Lighting::{Busy, Off, On};
    use ds::base::vocab::{Availability, Check};
    use ds::motion::detail::{
        detailed::{first_table, moment_table},
        moment::Moment,
    };

    #[test]
    fn the_disc_reads_coming_on_as_its_success() {
        moment_table(&[
            (Off, On, Moment::Success),
            (Busy, On, Moment::Success),
            (Off, Busy, Moment::Pending),
            (On, Busy, Moment::Pending),
            (On, Off, Moment::Change),
            (Busy, Off, Moment::Change),
            (On, On, Moment::Rest),
        ]);
        first_table(&[
            (Busy, Moment::Pending),
            (On, Moment::Rest),
            (Off, Moment::Rest),
        ]);
    }

    #[test]
    fn a_busy_module_is_neither_on_nor_off() {
        const CASES: &[(Check, Availability, Lighting)] = &[
            (Check::On, Availability::Enabled, On),
            (Check::Off, Availability::Enabled, Off),
            (Check::On, Availability::Disabled, On),
            (Check::On, Availability::Busy, Busy),
            (Check::Off, Availability::Busy, Busy),
        ];
        for &(value, availability, want) in CASES {
            assert_eq!(
                Lighting::of(value, availability),
                want,
                "{value:?} {availability:?}"
            );
        }
    }
}
