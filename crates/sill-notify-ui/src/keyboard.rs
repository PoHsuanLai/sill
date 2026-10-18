//! The input modality a shortcut-driven keyboard grab leaves the seat in.

use dioxus::prelude::*;
use ds::base::vocab::InputModality;
use ds::prelude::*;

/// Mark the seat as on the keyboard, so the focus ring is drawn. A grab asked for by a shortcut
/// means the person is on the keyboard, though the seat has seen no key yet; nothing without a
/// host (a test, an SSR render).
pub(crate) fn seat_on_keyboard(signals: CopyValue<Option<HostSignals>>) {
    if let Some(mut modality) = (*signals.peek()).map(|signals| signals.modality) {
        modality.set(InputModality::Keyboard);
    }
}
