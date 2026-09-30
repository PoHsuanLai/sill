//! The one root component every spawned surface has: it builds `S::root`.

use std::fmt;
use std::marker::PhantomData;

use dioxus::prelude::{Element, use_context_provider};
use shell_host::OutputInfo;

use super::Surface;
use crate::root::{SurfaceName, use_host_signals};

/// What [`mount`] is given: the output and the slot of the surface it draws.
pub struct MountProps<S: Surface> {
    pub output: OutputInfo,
    pub slot: S::Slot,
    surface: PhantomData<fn() -> S>,
}

impl<S: Surface> MountProps<S> {
    /// The props for `slot` of `S` on `output`.
    pub fn new(output: OutputInfo, slot: S::Slot) -> MountProps<S> {
        MountProps {
            output,
            slot,
            surface: PhantomData,
        }
    }
}

// By hand: a derive would ask `S` itself to be `Clone`.
impl<S: Surface> Clone for MountProps<S> {
    fn clone(&self) -> Self {
        MountProps::new(self.output.clone(), self.slot)
    }
}

// By hand: a derive would ask `S` itself to be `Debug`.
impl<S: Surface> fmt::Debug for MountProps<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MountProps")
            .field("surface", &S::NAME)
            .field("output", &self.output)
            .field("slot", &self.slot)
            .finish()
    }
}

/// The root component of `S`: quire's document host and the seat's signals, then its `root`, in
/// the component's scope.
pub fn mount<S: Surface>(props: MountProps<S>) -> Element {
    ds_native::provide_host();
    use_context_provider(|| SurfaceName(S::NAME));
    use_host_signals();
    S::root(props.output, props.slot)
}
