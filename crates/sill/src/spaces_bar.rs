//! The daemon's half of Mission Control's Spaces bar: the compositor's notes raise and lower the
//! bar on the link per output, bring the pictures of Spaces and say which Space a dragged window
//! is over ([`apply_note`], called from [`crate::daemon::compositor::apply`]), the bar surfaces
//! draw it, and a click comes back on the link. [`Outgoing`] sends each click's commands to the
//! host for the desktop arm; [`spawn_outbox`] follows the bars' layouts
//! and sends the drop targets and the requests for pictures.

use shell_host::HostHandle;
use sill_model::desktop::{
    BarPick, DesktopCommand, Outbox, OverviewNote, ShellNote, Shown, bar_show, commands_of,
    outgoing,
};
use sill_ui_kit::links::{Links, SpacesBarLink, SpacesBarPicked};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;
use crate::outgoing::Outgoing;

/// What an overview note does to the bar on `link`: Mission Control opening raises it, closing
/// lowers it. The layout (`Laid`) is the compositor's, and changes nothing here.
///
/// ```
/// use sill::spaces_bar::apply;
/// use sill_model::desktop::{BarShow, OverviewMode, OverviewNote, OverviewState};
/// use sill_ui_kit::links::SpacesBarLink;
///
/// let link = SpacesBarLink::new();
/// let open = OverviewNote::Moved {
///     mode: OverviewMode::MissionControl,
///     state: OverviewState::Entering,
///     progress: 0,
/// };
/// apply(&link, &open);
/// assert_eq!(link.latest().show, BarShow::Show);
/// ```
pub fn apply(link: &SpacesBarLink, note: &OverviewNote) {
    if let OverviewNote::Moved { mode, state, .. } = note {
        link.set_show(bar_show(*mode, *state));
    }
}

/// What a note of the compositor's shell protocol does to the bar on `link`: the per-output
/// overview, the tile under a dragged window, where a window drag begins and ends, and an answer to a request for a Space's picture
/// are written to it; the overview as a whole goes through [`apply`]; every other note is not the
/// bar's.
///
/// ```
/// use sill::spaces_bar::apply_note;
/// use sill_model::desktop::{BarShow, ShellNote, Shown};
/// use sill_ui_kit::links::SpacesBarLink;
///
/// let link = SpacesBarLink::new();
/// apply_note(&link, &ShellNote::OverviewOutput { output: "DP-1".to_owned(), shown: Shown::Hidden });
/// assert_eq!(link.latest().outputs.bar_on(BarShow::Show, Some("DP-1")), BarShow::Hide);
/// ```
pub fn apply_note(link: &SpacesBarLink, note: &ShellNote) {
    match note {
        ShellNote::Overview(note) => apply(link, note),
        ShellNote::OverviewOutput { output, shown } => link.set_output(output, *shown),
        ShellNote::DropTarget { output, space } => link.set_drop(output, space.clone()),
        ShellNote::DropTargetNew { output, over } => link.set_drop_new(output, *over),
        ShellNote::WindowDragBegin { output, .. } => link.set_dragging(output, Shown::Shown),
        ShellNote::WindowDragEnd { output, .. } => link.set_dragging(output, Shown::Hidden),
        ShellNote::SpacePicture {
            output,
            space,
            picture,
        } => link.set_picture(output, space.clone(), picture),
        _ => {}
    }
}

/// Follow the bars' feed on `links` on `rt` and send what [`outgoing`] says to the host, until
/// the host is gone.
pub fn spawn_outbox(rt: &Handle, links: &Links, host: HostHandle<ShellMsg>) {
    let mut feed = links.spaces_bar.subscribe();
    rt.spawn(async move {
        let mut outbox = Outbox::default();
        loop {
            let latest = feed.borrow_and_update().clone();
            let (next, commands) =
                outgoing(outbox, &latest.layouts, &latest.view, &latest.pictures);
            outbox = next;
            for command in commands {
                if host.send(ShellMsg::Command(command)).is_err() {
                    return;
                }
            }
            if feed.changed().await.is_err() {
                return;
            }
        }
    });
}

impl Outgoing for SpacesBarLink {
    type Item = BarPick;

    fn follow(&self) -> watch::Receiver<Option<SpacesBarPicked>> {
        self.subscribe_picks()
    }

    fn commands(picked: &SpacesBarPicked) -> Vec<DesktopCommand> {
        commands_of(&picked.item)
    }
}

#[cfg(test)]
mod tests {
    use sill_model::desktop::{
        BarDrops, BarShow, DragOutcome, OverviewMode, OverviewNote, OverviewState, ShellNote, Shown,
    };
    use sill_ui_kit::links::SpacesBarLink;

    use super::{apply, apply_note};

    fn moved(mode: OverviewMode, state: OverviewState) -> OverviewNote {
        OverviewNote::Moved {
            mode,
            state,
            progress: 0,
        }
    }

    #[test]
    fn the_bar_follows_mission_control_open_to_closed_and_ignores_the_layout() {
        use OverviewMode::{AppExpose, MissionControl};
        use OverviewState::{Closed, Entering, Leaving, Open};
        let link = SpacesBarLink::new();
        let steps = [
            (moved(MissionControl, Entering), BarShow::Show),
            (moved(MissionControl, Open), BarShow::Show),
            (OverviewNote::Laid, BarShow::Show),
            (moved(MissionControl, Leaving), BarShow::Hide),
            (moved(MissionControl, Closed), BarShow::Hide),
            (moved(AppExpose, Open), BarShow::Hide),
            (moved(MissionControl, Open), BarShow::Show),
            (moved(MissionControl, Closed), BarShow::Hide),
        ];
        for (note, want) in steps {
            apply(&link, &note);
            assert_eq!(link.latest().show, want, "{note:?}");
        }
    }

    #[test]
    fn the_notes_of_the_bar_reach_the_link_and_the_others_do_not() {
        let link = SpacesBarLink::new();
        let notes = [
            ShellNote::OverviewOutput {
                output: "DP-2".to_owned(),
                shown: Shown::Shown,
            },
            ShellNote::DropTarget {
                output: "DP-2".to_owned(),
                space: None,
            },
            ShellNote::DropTargetNew {
                output: "DP-2".to_owned(),
                over: Shown::Shown,
            },
            ShellNote::WindowDragBegin {
                output: "DP-2".to_owned(),
                window: "id-1".to_owned(),
            },
            ShellNote::Killed,
        ];
        for note in &notes {
            apply_note(&link, note);
        }
        let outputs = link.latest().outputs;
        assert_eq!(outputs.bar_on(BarShow::Show, Some("DP-2")), BarShow::Show);
        assert_eq!(outputs.bar_on(BarShow::Show, Some("DP-1")), BarShow::Hide);
        assert_eq!(
            link.latest().drops,
            BarDrops::default()
                .with_new("DP-2", Shown::Shown)
                .with_dragging("DP-2", Shown::Shown)
        );
        apply_note(
            &link,
            &ShellNote::WindowDragEnd {
                output: "DP-2".to_owned(),
                window: "id-1".to_owned(),
                outcome: DragOutcome::Cancelled,
            },
        );
        assert_eq!(
            link.latest().drops,
            BarDrops::default().with_new("DP-2", Shown::Shown)
        );
    }
}
