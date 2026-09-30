//! Where a widget's card is drawn and how big it is there.

use dioxus::prelude::EventHandler;
use ds_shell::{CardPresence, Lift, WidgetHost, WidgetSize};
use sill_settings::WidgetKind;

use super::{OnProvider, WidgetProvider, with_provider};

/// The one size `kind` takes in `host`: a widget has one size, nothing picks another. In the
/// notification center every tile spans the column at Medium (344 fits its 384 less the
/// padding): the month is the reference's Medium Calendar, today beside the compact month. On the
/// desktop the compact month and the batteries are Small, the lists and the clocks' dials
/// Medium.
pub fn size_of(kind: WidgetKind, host: WidgetHost) -> WidgetSize {
    struct DesktopSize;
    impl OnProvider for DesktopSize {
        type Out = WidgetSize;
        fn on<P: WidgetProvider>(self) -> WidgetSize {
            P::DESKTOP_SIZE
        }
    }
    match host {
        WidgetHost::Tile => WidgetSize::Medium,
        WidgetHost::Desktop => with_provider(kind, DesktopSize),
    }
}

/// `kind`'s one size on the desktop.
pub fn desktop_size(kind: WidgetKind) -> WidgetSize {
    size_of(kind, WidgetHost::Desktop)
}

/// Where a provider's card is drawn: the host, the size and the lift, and whether it is on its
/// way out (and who to tell once it has gone).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub host: WidgetHost,
    pub size: WidgetSize,
    pub lift: Lift,
    pub presence: CardPresence,
    pub on_gone: Option<EventHandler<()>>,
}
