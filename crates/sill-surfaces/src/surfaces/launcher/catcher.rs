//! The catcher (design/20 §1.3): a transparent Overlay over the whole output, shown with the
//! panel, painted once per showing and never animated or dimmed (plan "shell risks"), whose
//! press anywhere outside the panel closes the launcher. Its input region leaves the panel's
//! rectangle out, so a press on the panel reaches the panel whichever surface is on top.

use dioxus::prelude::*;
use shell_host::{OutputInfo, RegionSpec, SurfaceHandle};

use crate::hooks::use_settings;
use crate::surfaces::launcher::follow::{use_follow_visibility, use_shown};
use crate::surfaces::launcher::layer::catcher_input;
use crate::surfaces::launcher::panel::LAUNCHER_CSS;
use crate::surfaces::launcher::presence::LauncherLink;

/// The catcher on `output`.
pub(crate) fn catcher(output: OutputInfo) -> Element {
    let link = use_context::<LauncherLink>();
    let shown = use_shown(&link);
    use_follow_visibility(shown, output.id);
    let file = use_settings()();
    follow_input(catcher_input(&output, &file.bar, &file.launcher));
    rsx! {
        style { {LAUNCHER_CSS} }
        div {
            class: "sill-launcher-catcher",
            onmousedown: move |_| link.close(),
        }
    }
}

/// Keep the input region on `region` as the settings move the panel.
fn follow_input(region: RegionSpec) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut last = use_hook(|| CopyValue::new(None::<RegionSpec>));
    if last.peek().as_ref() == Some(&region) {
        return;
    }
    if let Some(surface) = surface {
        surface.set_input_region(region.clone());
    }
    last.set(Some(region));
}
