//! The effects end of Edit Widgets: follow the [`EditWidgetsLink`] through the notification
//! center's phase machine (`notification_center::phase`: enter, stay, play the exit, then
//! unmap) onto the surface's visibility, and work out which places an edit writes to the state
//! file. Once hidden nothing runs and nothing asks for a frame.

use dioxus::prelude::*;
use shell_host::{OutputId, SurfaceHandle, SurfaceOp, Visibility, use_watch};

use super::link::EditWidgetsLink;
use crate::surfaces::desktop_widgets::Placement;
use crate::surfaces::notification_center::phase::{
    Phase, PhaseInput, PhaseOut, Wanted, step_phase,
};

/// The panel's phase on one output, and the surface it maps. Clone is cheap.
#[derive(Clone)]
pub(super) struct EditPhase {
    phase: Signal<Phase>,
    surface: Option<SurfaceHandle>,
}

impl EditPhase {
    /// Where the panel is (read reactively).
    pub fn now(&self) -> Phase {
        (self.phase)()
    }

    /// The panel's exit has settled: unmap.
    pub fn settled(&self) {
        drive(self.phase, self.surface.clone(), PhaseInput::ExitSettled);
    }
}

/// The panel's phase on `output`, following the link.
pub(super) fn use_phase(output: OutputId) -> EditPhase {
    let link = use_context::<EditWidgetsLink>();
    let shown = use_watch(link.subscribe());
    let surface = try_use_context::<SurfaceHandle>();
    let phase = use_signal(Phase::default);
    let follow = surface.clone();
    use_effect(move || {
        let wanted = match *shown.read() {
            super::EditShown::Open(on) if on == output => Wanted::Here,
            super::EditShown::Open(_) | super::EditShown::Closed => Wanted::NotHere,
        };
        drive(phase, follow.clone(), PhaseInput::Link(wanted));
    });
    EditPhase { phase, surface }
}

/// `input` through the machine.
fn drive(mut phase: Signal<Phase>, surface: Option<SurfaceHandle>, input: PhaseInput) {
    let (next, out) = step_phase(*phase.peek(), input);
    if next != *phase.peek() {
        phase.set(next);
    }
    let visibility = match out {
        Some(PhaseOut::Map) => Visibility::Shown,
        Some(PhaseOut::Unmap) => Visibility::Hidden,
        Some(PhaseOut::PlayExit) | None => return,
    };
    eprintln!("edit_widgets.visible={}", visibility == Visibility::Shown);
    if let Some(surface) = &surface {
        surface.apply(SurfaceOp::SetVisibility(visibility));
    }
}

/// The places an edit writes for its output: each widget still on the desktop at its new cell,
/// except that one whose cell did not change keeps its saved entry (a place clamped onto a
/// smaller output is never written over, F882); then every saved place of a kind the desktop no
/// longer shows (a widget taken away keeps its place for when it comes back, as a drop does).
pub(super) fn places_to_save(
    before: &[Placement],
    saved: &[Placement],
    after: &[Placement],
) -> Vec<Placement> {
    let kept = after.iter().map(|now| {
        let unmoved = before.contains(now);
        match (unmoved, saved.iter().find(|s| s.kind == now.kind)) {
            (true, Some(entry)) => *entry,
            (true, None) | (false, _) => *now,
        }
    });
    let others = saved
        .iter()
        .filter(|s| after.iter().all(|now| now.kind != s.kind))
        .copied();
    kept.chain(others).collect()
}

#[cfg(test)]
mod tests {
    use sill_settings::WidgetKind;

    use super::places_to_save;
    use crate::surfaces::desktop_widgets::{GridSlot, Placement};

    fn at(kind: WidgetKind, column: u16, row: u16) -> Placement {
        Placement {
            kind,
            slot: GridSlot { column, row },
        }
    }

    #[test]
    fn an_edit_keeps_saved_places_it_did_not_move() {
        use WidgetKind::{Battery, Calendar, WorldClock};
        // (name, before, saved, after, written)
        let cases = [
            (
                "an added widget is recorded; a clamped one keeps its saved place",
                vec![at(Calendar, 4, 0)],
                vec![at(Calendar, 9, 0)],
                vec![at(Calendar, 4, 0), at(WorldClock, 2, 0)],
                vec![at(Calendar, 9, 0), at(WorldClock, 2, 0)],
            ),
            (
                "a resize that moved it writes the new cell",
                vec![at(Calendar, 4, 0), at(Battery, 4, 1)],
                vec![at(Calendar, 4, 0), at(Battery, 4, 1)],
                vec![at(Calendar, 2, 0), at(Battery, 4, 1)],
                vec![at(Calendar, 2, 0), at(Battery, 4, 1)],
            ),
            (
                "a removed widget keeps its saved place",
                vec![at(Calendar, 4, 0), at(Battery, 4, 1)],
                vec![at(Calendar, 4, 0), at(Battery, 4, 1)],
                vec![at(Battery, 4, 1)],
                vec![at(Battery, 4, 1), at(Calendar, 4, 0)],
            ),
        ];
        for (name, before, saved, after, want) in cases {
            assert_eq!(places_to_save(&before, &saved, &after), want, "{name}");
        }
    }
}
