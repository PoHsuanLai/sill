//! The effects end of the stage: follow a wanted value onto it, map the surface to match, and
//! settle it when the sheet's exit has played. The surface's input region follows the stage too:
//! the preset's input while a value is up, nothing from the moment its exit starts.

use dioxus::prelude::*;
use shell_host::{RegionSpec, SurfaceHandle};

use super::model::{SheetStage, StageIn};
use super::step::advance;
use crate::hooks::use_follow_visibility;
use crate::surface::{LayerPreset, Pointer};

/// The stage following `wanted` (read inside an effect, so the signals it reads drive it), and
/// the surface's mapping and input (`preset`'s while up) following the stage. Nothing to map
/// without a host (a test, an SSR render).
pub fn use_sheet_stage<T>(
    preset: LayerPreset,
    wanted: impl Fn() -> Option<T> + 'static,
) -> Signal<SheetStage<T>>
where
    T: Clone + PartialEq + 'static,
{
    let stage = use_signal(|| SheetStage::<T>::Empty);
    use_effect(move || step(stage, StageIn::Want(wanted())));
    use_follow_pointer(preset, move || stage.read().pointer());
    use_follow_visibility(move || stage.read().mapped().visibility());
    stage
}

/// The surface's input region following `pointer` (read inside an effect, so the signals it reads
/// drive it): `preset`'s input while it takes the pointer, none while it passes it through.
/// Nothing to set without a host (a test, an SSR render).
pub fn use_follow_pointer(preset: LayerPreset, pointer: impl Fn() -> Pointer + 'static) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(None::<Pointer>));
    use_effect(move || {
        let now = pointer();
        if *applied.peek() == Some(now) {
            return;
        }
        applied.set(Some(now));
        if let Some(surface) = &surface {
            surface.set_input_region(match now {
                Pointer::Takes => preset.input(),
                Pointer::PassesThrough => RegionSpec::Empty,
            });
        }
    });
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
