//! The control center's render: the page state ([`super::step_page`]), the module grid laid out
//! by [`super::place`] (each module from `crate::surfaces::control_center::modules`), the detail
//! pages, the slide between them, and Escape.

use dioxus::prelude::*;
use sill_services::OutputName;

use super::page::CcPage;

/// The panel. M5 freeze stub, filled by M5 agent (c) control-center (FINDINGS "M5 freeze"). It is
/// only mounted once the bar opens a `PopupBody::ControlCenter`, which nothing does before (c).
pub(crate) fn control_center(
    output: Option<OutputName>,
    on_close: EventHandler<()>,
    on_page: EventHandler<CcPage>,
) -> Element {
    let _ = (output, on_close, on_page);
    todo!("M5 fill: control-center (c)")
}
