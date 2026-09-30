//! The Display module: a [`crate::control_center::LevelModule`] with the `Sun` glyph,
//! "Display" as its title, and the brightness. It reads no service: the caller
//! ([`super::Module`], the bar's display dropdown) passes the bar output's brightness
//! (`Displays::target`, then `Displays::brightness`), sends `DisplaysCommand::SetBrightness {
//! output, level }` from `on_change`, and passes `Disabled` when the target is
//! `BrightnessTarget::None`.

use dioxus::prelude::*;
use ds::{Availability, Fraction, Icon, LevelGlyph};

use super::cc_module::{BarItemFace, CcCx, CcModule, Span};
use super::wired::WiredDisplay;
use crate::bar::{BarMenu, StatusIcon};
use crate::control_center::LevelModule;
use sill_settings::{ControlCenterSettings, InMenuBar};

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
            level_glyph: LevelGlyph::Brightness,
            title: "Display".to_owned(),
            level,
            on_change,
            availability,
        }
    }
}

/// The Display module as the control center and the bar draw it.
pub(crate) struct DisplayCc;

impl CcModule for DisplayCc {
    fn span(&self) -> Span {
        Span::Row
    }

    fn tile(&self, cx: &CcCx) -> Element {
        rsx! { WiredDisplay { output: cx.output.clone() } }
    }

    fn menu_bar(&self, settings: &ControlCenterSettings) -> InMenuBar {
        settings.menu_bar_display
    }

    fn bar_item(&self) -> Option<BarItemFace> {
        Some(BarItemFace {
            menu: BarMenu::Display,
            item: || {
                rsx! {
                    StatusIcon {
                        menu: BarMenu::Display,
                        icon: Icon::Sun,
                        tooltip: "Display".to_owned(),
                    }
                }
            },
        })
    }
}
