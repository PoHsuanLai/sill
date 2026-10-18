//! The effects end of the stage: follow a wanted value onto it, map the surface to match, and
//! settle it when the sheet's exit has played.

use dioxus::prelude::*;
use shell_host::{SurfaceHandle, Visibility};

use super::model::{Mapped, SheetStage, StageIn};
use super::step::advance;

/// The stage following `wanted` (read inside an effect, so the signals it reads drive it), and
/// the surface's mapping following the stage. Nothing to map without a host (a test, an SSR
/// render).
pub fn use_sheet_stage<T>(wanted: impl Fn() -> Option<T> + 'static) -> Signal<SheetStage<T>>
where
    T: Clone + PartialEq + 'static,
{
    let stage = use_signal(|| SheetStage::<T>::Empty);
    use_effect(move || step(stage, StageIn::Want(wanted())));
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(Mapped::No));
    use_effect(move || {
        let mapped = stage.read().mapped();
        if *applied.peek() == mapped {
            return;
        }
        applied.set(mapped);
        if let Some(surface) = &surface {
            surface.set_visibility(match mapped {
                Mapped::Yes => Visibility::Shown,
                Mapped::No => Visibility::Hidden,
            });
        }
    });
    stage
}

/// The sheet's exit settled (its `on_hidden`): an exit that played unmaps the surface.
pub fn settle<T>(stage: Signal<SheetStage<T>>)
where
    T: Clone + PartialEq + 'static,
{
    step(stage, StageIn::Settled);
}

fn step<T>(mut stage: Signal<SheetStage<T>>, input: StageIn<T>)
where
    T: Clone + PartialEq + 'static,
{
    let next = advance(SheetStage::clone(&stage.peek()), input);
    if *stage.peek() != next {
        stage.set(next);
    }
}
