//! The brightness and volume keys (`sill display ...`): the level change goes to the displays or
//! the volume service, and the OSD follows a change that was carried out.

use sill_ipc::act::{DisplayRequest, LevelRequest, VolumeRequest};
use sill_ipc::response::Response;
use sill_model::displays::{DisplaysCommand, LevelChange};
use sill_model::units::Percent;
use sill_model::volume::{Mute, Step, VolumeCommand};

use super::reach::Reach;
use super::reply::sent;

/// A display key's change. It shows on the OSD once the service has been told.
pub fn press(reach: &Reach, request: DisplayRequest) -> Response {
    let services = reach.services;
    let response = match request {
        DisplayRequest::Brightness(level) => {
            let at = services.cosmic_wl.state.borrow().toplevels.focused_output();
            sent(
                services.displays.control.send(DisplaysCommand::Brightness {
                    at,
                    change: level_change(level),
                }),
                "displays",
            )
        }
        DisplayRequest::Volume(volume) => sent(
            services.volume.control.send(volume_command(volume)),
            "volume",
        ),
    };
    let file = reach.settings.borrow();
    crate::osd::after_display(
        reach.osd,
        services,
        reach.outputs,
        &file,
        request,
        &response,
    );
    response
}

/// A percentage from the wire, held to 0-100.
fn percent(wire: u8) -> Percent {
    Percent(u16::from(wire.min(100)))
}

fn level_change(level: LevelRequest) -> LevelChange {
    match level {
        LevelRequest::Up => LevelChange::Step(Step::Up),
        LevelRequest::Down => LevelChange::Step(Step::Down),
        LevelRequest::Set { percent: wire } => LevelChange::Set(percent(wire)),
    }
}

fn volume_command(request: VolumeRequest) -> VolumeCommand {
    match request {
        VolumeRequest::Level(LevelRequest::Up) => VolumeCommand::Step(Step::Up),
        VolumeRequest::Level(LevelRequest::Down) => VolumeCommand::Step(Step::Down),
        VolumeRequest::Level(LevelRequest::Set { percent: wire }) => {
            VolumeCommand::SetLevel(percent(wire))
        }
        VolumeRequest::Mute => VolumeCommand::SetMute(Mute::Muted),
        VolumeRequest::Unmute => VolumeCommand::SetMute(Mute::Audible),
        VolumeRequest::ToggleMute => VolumeCommand::ToggleMute,
    }
}

#[cfg(test)]
mod tests {
    use sill_ipc::act::{LevelRequest, VolumeRequest};
    use sill_model::displays::LevelChange;
    use sill_model::units::Percent;
    use sill_model::volume::{Mute, Step, VolumeCommand};

    use super::{level_change, volume_command};

    #[test]
    fn a_level_change_holds_a_percentage_to_100() {
        let cases = [
            (LevelRequest::Up, LevelChange::Step(Step::Up)),
            (LevelRequest::Down, LevelChange::Step(Step::Down)),
            (
                LevelRequest::Set { percent: 40 },
                LevelChange::Set(Percent(40)),
            ),
            (
                LevelRequest::Set { percent: 250 },
                LevelChange::Set(Percent(100)),
            ),
        ];
        for (request, want) in cases {
            assert_eq!(level_change(request), want, "{request:?}");
        }
    }

    #[test]
    fn volume_keys_are_the_volume_services_commands() {
        let cases = [
            (
                VolumeRequest::Level(LevelRequest::Up),
                VolumeCommand::Step(Step::Up),
            ),
            (
                VolumeRequest::Level(LevelRequest::Down),
                VolumeCommand::Step(Step::Down),
            ),
            (
                VolumeRequest::Level(LevelRequest::Set { percent: 250 }),
                VolumeCommand::SetLevel(Percent(100)),
            ),
            (VolumeRequest::Mute, VolumeCommand::SetMute(Mute::Muted)),
            (VolumeRequest::Unmute, VolumeCommand::SetMute(Mute::Audible)),
            (VolumeRequest::ToggleMute, VolumeCommand::ToggleMute),
        ];
        for (request, want) in cases {
            assert_eq!(volume_command(request), want, "{request:?}");
        }
    }
}
