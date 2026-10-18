//! The effects end of the picker: follow the [`SplitPickerLink`] onto this output's surface
//! (mapped while an offer shows here and has not been answered), and write the person's answer
//! back to the link. An answer hides the picker at once; the compositor's end of the offer
//! (which the daemon puts on the link) arrives after.

use dioxus::prelude::*;
use shell_host::{OutputInfo, SurfaceHandle, Visibility, use_watch};
use sill_ui_kit::links::{OfferSerial, SplitAnswer, SplitOffer, SplitPickerLink};

use super::model::shows_on;

/// Everything the picker's handlers reach. Clone is cheap.
#[derive(Clone)]
pub(super) struct Picker {
    link: SplitPickerLink,
    offer: ReadSignal<Option<SplitOffer>>,
    answered: Signal<OfferSerial>,
    name: Option<String>,
}

/// The picker on `output`.
pub(super) fn use_picker(link: &SplitPickerLink, output: &OutputInfo) -> Picker {
    let picker = Picker {
        link: link.clone(),
        offer: use_watch(link.subscribe()),
        answered: use_signal(OfferSerial::default),
        name: output.name.clone(),
    };
    use_follow_visibility(picker.clone());
    picker
}

impl Picker {
    /// The offer to draw: open, for this output, and not yet answered.
    pub(super) fn shown(&self) -> Option<SplitOffer> {
        let answered = *self.answered.read();
        self.offer
            .read()
            .clone()
            .filter(|offer| offer.serial != answered)
            .filter(|offer| shows_on(&offer.output, self.name.as_deref()))
    }

    /// The person picked `window` for the other half.
    pub(super) fn pick(&self, window: &str) {
        self.answer(SplitAnswer::Picked(window.to_owned()));
    }

    /// The person turned the choice down.
    pub(super) fn cancel(&self) {
        self.answer(SplitAnswer::Cancelled);
    }

    /// Write `answer` for the offer shown and stop showing it.
    fn answer(&self, answer: SplitAnswer) {
        let Some(offer) = self.shown() else {
            return;
        };
        self.link.answer(offer.first, answer);
        let mut answered = self.answered;
        answered.set(offer.serial);
    }
}

/// Map this surface while an offer shows on its output, unmap it otherwise. Nothing to do
/// without a host (a test, an SSR render).
fn use_follow_visibility(picker: Picker) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(None::<Visibility>));
    use_effect(move || {
        let visible = match picker.shown() {
            Some(_) => Visibility::Shown,
            None => Visibility::Hidden,
        };
        if *applied.peek() == Some(visible) {
            return;
        }
        applied.set(Some(visible));
        if let Some(surface) = &surface {
            surface.set_visibility(visible);
        }
    });
}
