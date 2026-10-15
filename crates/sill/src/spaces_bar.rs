//! The daemon's half of Mission Control's Spaces bar: the compositor's overview notes raise and
//! lower the bar on the link ([`apply`], called from [`crate::daemon::compositor::apply`]), the
//! bar surfaces draw it, and a click comes back on the link. [`spawn_picks`] follows the clicks
//! and sends each one's commands to the host for the desktop arm.

use shell_host::HostHandle;
use sill_model::desktop::{OverviewNote, bar_show, commands_of};
use sill_ui_kit::links::{Links, SpacesBarLink};
use tokio::runtime::Handle;

use crate::daemon::msg::ShellMsg;

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

/// Follow the clicks on `links` on `rt` and send each one's commands to the host, until the host
/// is gone.
pub fn spawn_picks(rt: &Handle, links: &Links, host: HostHandle<ShellMsg>) {
    let mut picks = links.spaces_bar.subscribe_picks();
    rt.spawn(async move {
        loop {
            if picks.changed().await.is_err() {
                return;
            }
            let Some(picked) = picks.borrow_and_update().clone() else {
                continue;
            };
            for command in commands_of(&picked.pick) {
                if host.send(ShellMsg::Command(command)).is_err() {
                    return;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use sill_model::desktop::{BarShow, OverviewMode, OverviewNote, OverviewState};
    use sill_ui_kit::links::SpacesBarLink;

    use super::apply;

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
}
