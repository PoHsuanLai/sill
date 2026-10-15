//! The effects end of the panel: follow the link onto the stage (and so the surface's mapping),
//! and read the apps the desktop lists.

use dioxus::prelude::*;
use shell_host::{OutputInfo, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_model::desktop::ForceQuitRow;
use sill_ui_kit::links::ForceQuitLink;

use super::stage::{Mapped, Stage, StageIn, advance};

/// The stage on `output`, following the link, and the surface's mapping following the stage.
/// Nothing to map without a host (a test, an SSR render).
pub(super) fn use_stage(output: &OutputInfo) -> Signal<Stage> {
    let link = use_context::<ForceQuitLink>();
    let shown = use_watch(link.subscribe());
    let mut stage = use_signal(Stage::default);
    let id = output.id;
    use_effect(move || {
        let wanted = shown
            .read()
            .opening()
            .filter(|opening| opening.output == id)
            .copied();
        let next = advance(*stage.peek(), StageIn::Want(wanted));
        if *stage.peek() != next {
            stage.set(next);
        }
    });
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(Mapped::No));
    use_effect(move || {
        let mapped = stage.read().mapped();
        if *applied.peek() == mapped {
            return;
        }
        applied.set(mapped);
        if let Some(surface) = &surface {
            surface.apply(SurfaceOp::SetVisibility(match mapped {
                Mapped::Yes => Visibility::Shown,
                Mapped::No => Visibility::Hidden,
            }));
        }
    });
    stage
}

/// The apps the desktop lists now.
pub(super) fn use_rows() -> ReadSignal<Vec<ForceQuitRow>> {
    let link = use_context::<ForceQuitLink>();
    use_watch(link.subscribe_rows())
}
