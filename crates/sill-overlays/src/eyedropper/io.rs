//! The effects end of the eyedropper: follow the [`EyedropperLink`] onto this output's surface
//! (mapped while the link holds a picture of this output for a question not yet answered here),
//! tell the link the surface exists, and send the person's answer to the capture service. An
//! answer hides the surface at once; the daemon's end of the question arrives after.

use dioxus::prelude::*;
use shell_host::{OutputInfo, SurfaceHandle, Visibility, use_watch};
use sill_model::capture_pick::{AskId, CaptureCommand, ColorOutcome};
use sill_model::desktop::OutputShot;
use sill_services::Control;
use sill_services::capture_pick::CaptureService;
use sill_ui_kit::hooks::use_control;
use sill_ui_kit::links::{EyedropFeed, EyedropperLink};

use super::model::shown_on;

/// Everything the eyedropper's handlers reach. Clone is cheap.
#[derive(Clone)]
pub(super) struct Sampler {
    feed: ReadSignal<EyedropFeed>,
    answered: Signal<Option<AskId>>,
    control: Control<CaptureCommand>,
    name: Option<String>,
}

/// The eyedropper on `output`.
pub(super) fn use_sampler(link: &EyedropperLink, output: &OutputInfo) -> Sampler {
    let sampler = Sampler {
        feed: use_watch(link.subscribe()),
        answered: use_signal(|| None),
        control: use_control::<CaptureService>(),
        name: output.name.clone(),
    };
    use_exists(link);
    use_follow_visibility(sampler.clone());
    sampler
}

impl Sampler {
    /// The question and the picture to draw: this output's, for a question not answered here.
    pub(super) fn shown(&self) -> Option<(AskId, OutputShot)> {
        let answered = *self.answered.read();
        shown_on(&self.feed.read(), self.name.as_deref(), answered)
    }

    /// End question `id` with `outcome` and stop drawing it.
    pub(super) fn answer(&self, id: AskId, outcome: ColorOutcome) {
        // The service may be gone; nobody is left to ask the person then.
        let _ = self.control.send(CaptureCommand::ColorAnswer(id, outcome));
        let mut answered = self.answered;
        answered.set(Some(id));
    }
}

/// The link counts this surface from its first render to its last.
fn use_exists(link: &EyedropperLink) {
    let link = link.clone();
    use_hook({
        let link = link.clone();
        move || link.attach()
    });
    use_drop(move || link.detach());
}

/// Map this surface while there is something to draw on it, unmap it otherwise. Nothing to do
/// without a host (a test, an SSR render).
fn use_follow_visibility(sampler: Sampler) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(None::<Visibility>));
    use_effect(move || {
        let visible = match sampler.shown() {
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
