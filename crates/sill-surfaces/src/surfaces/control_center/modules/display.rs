//! The Display module: a [`crate::surfaces::control_center::LevelModule`] with the `Sun` glyph,
//! "Display" as its title, and the brightness. It reads no service: the caller
//! ([`super::Module`], the bar's display dropdown) passes the bar output's brightness
//! (`Displays::target`, then `Displays::brightness`), sends `DisplaysCommand::Brightness { at,
//! change: LevelChange::Set(..) }` from `on_change`, and passes `Disabled` when the target is
//! `BrightnessTarget::None`.

use dioxus::prelude::*;
use ds::{Availability, Fraction, Icon};

use crate::surfaces::control_center::LevelModule;

/// The Display module at `level`.
#[component]
pub fn DisplayModule(
    level: Fraction,
    on_change: EventHandler<Fraction>,
    #[props(default)] availability: Availability,
) -> Element {
    rsx! {
        LevelModule {
            glyph: Icon::Sun,
            title: "Display".to_owned(),
            level,
            on_change,
            availability,
        }
    }
}
