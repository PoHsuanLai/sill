//! The shell protocol's events as [`ShellNote`]s.

use desktop_client::quire::ShellEvent;
use porter_core::SpaceId;
use sill_model::desktop::{Consent, DisplayTrial, OverviewNote, ShellNote, SwitcherNote};

/// `event` in the shell's words. `None` for what is state the desktop client keeps and a
/// consumer reads from `DesktopState::quire` (one overview slot, one Space slot), and for an
/// event this build does not know.
pub fn note_of(event: &ShellEvent) -> Option<ShellNote> {
    Some(match event {
        ShellEvent::Modifiers { mods, .. } => ShellNote::Modifiers(*mods),
        ShellEvent::KeyGesture(gesture) => ShellNote::Gesture(*gesture),
        ShellEvent::SwitcherApp { index, app } => ShellNote::Switcher(SwitcherNote::App {
            index: *index,
            app: app.as_str().to_owned(),
        }),
        ShellEvent::SwitcherShown { selected } => ShellNote::Switcher(SwitcherNote::Shown {
            selected: *selected,
        }),
        ShellEvent::SwitcherSelected(index) => ShellNote::Switcher(SwitcherNote::Selected(*index)),
        ShellEvent::SwitcherHidden => ShellNote::Switcher(SwitcherNote::Hidden),
        ShellEvent::Overview {
            mode,
            state,
            progress,
        } => ShellNote::Overview(OverviewNote::Moved {
            mode: *mode,
            state: *state,
            progress: *progress,
        }),
        ShellEvent::OverviewDone => ShellNote::Overview(OverviewNote::Laid),
        ShellEvent::HotCorner(corner) => ShellNote::HotCorner(*corner),
        ShellEvent::Reveal { edge, shown } => ShellNote::Reveal {
            edge: *edge,
            shown: *shown,
        },
        ShellEvent::MenuFlash { app, item } => ShellNote::MenuFlash {
            app: app.as_str().to_owned(),
            item: *item,
        },
        ShellEvent::Bell(app) => ShellNote::Bell(app.as_str().to_owned()),
        ShellEvent::Reopen(app) => ShellNote::Reopen(app.as_str().to_owned()),
        ShellEvent::WindowMenu {
            window,
            at,
            enabled,
        } => ShellNote::WindowMenu {
            window: window.as_str().to_owned(),
            at: (at.x, at.y),
            enabled: enabled.clone(),
        },
        ShellEvent::SplitChoose { first, side } => ShellNote::SplitChoose {
            first: first.as_str().to_owned(),
            side: *side,
        },
        ShellEvent::SplitCandidate(window) => ShellNote::SplitCandidate(window.as_str().to_owned()),
        ShellEvent::SplitChoiceEnd(first) => ShellNote::SplitChoiceEnd(first.as_str().to_owned()),
        ShellEvent::OverviewOutput { output, shown } => ShellNote::OverviewOutput {
            output: output.as_str().to_owned(),
            shown: *shown,
        },
        ShellEvent::DropTarget { output, space } => ShellNote::DropTarget {
            output: output.as_str().to_owned(),
            space: match space {
                Some(space) => Some(SpaceId::parse(space.as_str()).ok()?),
                None => None,
            },
        },
        ShellEvent::DropTargetNew { output, over } => ShellNote::DropTargetNew {
            output: output.as_str().to_owned(),
            over: *over,
        },
        ShellEvent::WindowDragBegin { output, window } => ShellNote::WindowDragBegin {
            output: output.as_str().to_owned(),
            window: window.as_str().to_owned(),
        },
        ShellEvent::WindowDragEnd {
            output,
            window,
            outcome,
        } => ShellNote::WindowDragEnd {
            output: output.as_str().to_owned(),
            window: window.as_str().to_owned(),
            outcome: *outcome,
        },
        ShellEvent::PermissionAsk(ask) => ShellNote::PermissionAsk(Consent::from_wire(
            ask.ask,
            ask.power,
            ask.subject_kind,
            &ask.subject,
            ask.memory,
        )),
        ShellEvent::PermissionWithdraw(ask) => ShellNote::PermissionWithdraw(*ask),
        ShellEvent::DisplaysTrial(trial) => ShellNote::DisplaysTrial(DisplayTrial {
            trial: trial.trial,
            seconds: trial.seconds,
        }),
        ShellEvent::DisplaysTrialEnd(trial) => ShellNote::DisplaysTrialEnd(*trial),
        ShellEvent::Killed => ShellNote::Killed,
        // Slots are collected into the overview's layout until `OverviewDone`.
        _ => return None,
    })
}
