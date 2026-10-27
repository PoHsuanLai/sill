//! The effects end of one corner: feed the pointer's enter and leave to the trigger machine
//! (`ds_behaviour::hot_corner::Corner`, run by quire's `use_machine`: its dwell and re-arm are its
//! own deadlines), and write a firing
//! to the [`CornerLink`].

use dioxus::prelude::*;
use ds::machine::{MachineRef, use_machine};
use ds_behaviour::hot_corner::{Corner, CornerIn, CornerOut, CornerParams};
use shell_host::OutputId;
use sill_ui_kit::links::{Corner as WhichCorner, CornerLink};

/// One corner's machine. Cheap to clone: a ref to quire's machine hook.
#[derive(Clone, Copy)]
pub(super) struct Pointer {
    machine: MachineRef<Corner>,
}

/// Run `corner` of `output`: the machine on quire's fixed corner timing. Its dwell and re-arm are the machine's own deadlines, run by
/// quire's one timer; a firing goes to the link.
pub(super) fn use_hot_corner(output: OutputId, corner: WhichCorner) -> Pointer {
    let link = use_context::<CornerLink>();
    let machine = use_machine(
        |_| Corner::default(),
        CornerParams::default(),
        || (),
        move |CornerOut::Fire, _| link.hit(corner, output),
    );
    Pointer { machine }
}

impl Pointer {
    /// The pointer entered the corner.
    pub(super) fn enter(&self) {
        self.machine.send(CornerIn::Enter);
    }

    /// The pointer left the corner.
    pub(super) fn leave(&self) {
        self.machine.send(CornerIn::Leave);
    }
}
