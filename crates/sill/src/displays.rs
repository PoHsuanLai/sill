//! The daemon's half of the displays service: the `display.*` settings translated into the
//! service's [`DisplayPolicy`] (sill-services may not see sill-settings, which brings dioxus),
//! outputs changes forwarded as rescans, and `sill display ...` answered.

use shell_host::HostEvent;
use sill_ipc::{DisplayRequest, LevelRequest, Response, VolumeRequest};
use sill_services::displays::ScaleOverride;
use sill_services::{
    DisplayPolicy, DisplaysCommand, FontRenderingMode, HardwareVolume, KeysTarget, LevelChange,
    NightWarmth, OutputName, Ppi, ScaleMode, ScaleRounding, Services, Step, Toplevels,
    VolumeCommand,
};

use crate::msg::ShellMsg;
use sill_settings::{
    BrightnessKeysTarget, DisplayScaleMode, DisplaySettings, FontRenderingSetting,
    HardwareVolumeMode, NightWarmthMode, ScaleRounding as ScaleRoundingSetting, ShellFile,
};

/// `display.*` as the service's policy.
pub fn policy_from(settings: &DisplaySettings) -> DisplayPolicy {
    DisplayPolicy {
        scale: match settings.scale {
            DisplayScaleMode::Auto => ScaleMode::Auto,
            DisplayScaleMode::Manual => ScaleMode::Manual,
        },
        scale_overrides: settings
            .scale_overrides
            .split(';')
            .filter_map(ScaleOverride::parse)
            .collect(),
        density_target: Ppi(settings.density_target_ppi.0),
        scale_rounding: match settings.scale_rounding {
            ScaleRoundingSetting::PreferWhole => ScaleRounding::PreferWhole,
            ScaleRoundingSetting::Nearest => ScaleRounding::Nearest,
        },
        keys_target: match settings.brightness_keys_target {
            BrightnessKeysTarget::PointerOutput => KeysTarget::PointerOutput,
            BrightnessKeysTarget::AllOutputs => KeysTarget::AllOutputs,
        },
        hardware_volume: match settings.hardware_volume {
            HardwareVolumeMode::Off => HardwareVolume::Off,
            HardwareVolumeMode::WhenMonitorIsOutput => HardwareVolume::WhenMonitorIsOutput,
        },
        night_warmth: match settings.night_warmth {
            NightWarmthMode::Off => NightWarmth::Off,
            NightWarmthMode::Hardware => NightWarmth::Hardware(sill_services::Percent(u16::from(
                settings.night_warmth_strength.0,
            ))),
        },
        font_rendering: match settings.font_rendering {
            FontRenderingSetting::Auto => FontRenderingMode::Auto,
            FontRenderingSetting::Off => FontRenderingMode::Off,
        },
    }
}

/// Send the settings' policy (at start, and on every settled settings change).
pub fn send_policy(services: &Services, file: &ShellFile) {
    let _ = services
        .displays_control
        .send(DisplaysCommand::Policy(policy_from(&file.display)));
}

/// The host events the displays service follows: outputs coming, going or changing (a rescan)
/// and a settled settings file (a new policy).
pub fn on_host_event(services: &Services, event: &HostEvent<ShellMsg>) {
    match event {
        HostEvent::Started
        | HostEvent::OutputAdded(_)
        | HostEvent::OutputChanged(_)
        | HostEvent::OutputRemoved(_) => {
            let _ = services.displays_control.send(DisplaysCommand::Rescan);
        }
        HostEvent::App(ShellMsg::Settings(file)) => send_policy(services, file),
        _ => {}
    }
}

/// The output of the active window: the daemon's stand-in for "under the pointer", which only
/// the compositor knows (FINDINGS F140).
pub fn focused_output(toplevels: &Toplevels) -> Option<OutputName> {
    toplevels
        .windows
        .iter()
        .find(|w| w.activity == sill_services::Activity::Active)
        .and_then(|w| w.outputs.first().cloned())
}

fn change(level: LevelRequest) -> LevelChange {
    match level {
        LevelRequest::Up => LevelChange::Step(Step::Up),
        LevelRequest::Down => LevelChange::Step(Step::Down),
        LevelRequest::Set { percent } => {
            LevelChange::Set(sill_services::Percent(u16::from(percent.min(100))))
        }
    }
}

/// The volume service's command for a volume key. The monitor's own speakers follow the
/// software level inside the displays service (`hw_volume`), so this is the whole of it.
pub fn volume_command(request: VolumeRequest) -> VolumeCommand {
    match request {
        VolumeRequest::Level(LevelRequest::Up) => VolumeCommand::Step(Step::Up),
        VolumeRequest::Level(LevelRequest::Down) => VolumeCommand::Step(Step::Down),
        VolumeRequest::Level(LevelRequest::Set { percent }) => {
            VolumeCommand::SetLevel(sill_services::Percent(u16::from(percent.min(100))))
        }
        VolumeRequest::Mute => VolumeCommand::SetMute(sill_services::Mute::Muted),
        VolumeRequest::Unmute => VolumeCommand::SetMute(sill_services::Mute::Audible),
        VolumeRequest::ToggleMute => VolumeCommand::ToggleMute,
    }
}

/// Answer `sill display ...`.
pub fn dispatch(services: &Services, request: DisplayRequest) -> Response {
    let sent = match request {
        DisplayRequest::Brightness(level) => {
            let at = focused_output(&services.toplevels.borrow());
            services.displays_control.send(DisplaysCommand::Brightness {
                at,
                change: change(level),
            })
        }
        DisplayRequest::Volume(volume) => services.volume_control.send(volume_command(volume)),
    };
    match sent {
        Ok(()) => Response::Done,
        Err(error) => Response::Failed(sill_ipc::Failure {
            kind: sill_ipc::FailureKind::Internal,
            message: error.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use sill_ipc::{LevelRequest, VolumeRequest};
    use sill_services::displays::ScaleOverride;
    use sill_services::{
        DisplayPolicy, FontRenderingMode, HardwareVolume, KeysTarget, Mute, NightWarmth, Percent,
        Ppi, ScaleMode, ScaleRounding, Step, VolumeCommand,
    };
    use sill_settings::{
        BrightnessKeysTarget, DisplayScaleMode, DisplaySettings, FontRenderingSetting,
        HardwareVolumeMode, NightWarmthMode, ScaleRounding as ScaleRoundingSetting,
    };

    use super::{policy_from, volume_command};

    #[test]
    fn the_default_settings_are_the_default_policy() {
        assert_eq!(
            policy_from(&DisplaySettings::default()),
            DisplayPolicy::default()
        );
    }

    #[test]
    fn every_setting_reaches_the_policy() {
        let settings = DisplaySettings {
            scale: DisplayScaleMode::Manual,
            scale_overrides: "BNQ-8074-00ff = 1.75; nonsense; DEL-a0b1-1=2".to_owned(),
            density_target_ppi: sill_settings::Count(120),
            scale_rounding: ScaleRoundingSetting::Nearest,
            brightness_keys_target: BrightnessKeysTarget::AllOutputs,
            hardware_volume: HardwareVolumeMode::Off,
            night_warmth: NightWarmthMode::Hardware,
            night_warmth_strength: sill_settings::Percent(30),
            font_rendering: FontRenderingSetting::Off,
            ..DisplaySettings::default()
        };
        assert_eq!(
            policy_from(&settings),
            DisplayPolicy {
                scale: ScaleMode::Manual,
                scale_overrides: ["BNQ-8074-00ff = 1.75", "DEL-a0b1-1=2"]
                    .iter()
                    .filter_map(|r| ScaleOverride::parse(r))
                    .collect(),
                density_target: Ppi(120),
                scale_rounding: ScaleRounding::Nearest,
                keys_target: KeysTarget::AllOutputs,
                hardware_volume: HardwareVolume::Off,
                night_warmth: NightWarmth::Hardware(Percent(30)),
                font_rendering: FontRenderingMode::Off,
            }
        );
    }

    #[test]
    fn the_focused_output_is_the_active_windows_first() {
        use sill_services::{Activity, OutputName, Toplevel, Toplevels};
        let window = |key, activity, outputs: &[&str]| Toplevel {
            key: sill_launcher::WindowKey(key),
            app_id: "foot".to_owned(),
            title: String::new(),
            activity,
            window: Default::default(),
            sizing: Default::default(),
            workspaces: Vec::new(),
            outputs: outputs
                .iter()
                .map(|o| OutputName((*o).to_owned()))
                .collect(),
        };
        let toplevels = |windows| Toplevels { windows };
        let cases = [
            (toplevels(vec![]), None),
            (
                toplevels(vec![
                    window(1, Activity::Inactive, &["eDP-1"]),
                    window(2, Activity::Active, &["HDMI-A-2", "eDP-1"]),
                ]),
                Some("HDMI-A-2"),
            ),
            (toplevels(vec![window(1, Activity::Active, &[])]), None),
        ];
        for (toplevels, want) in cases {
            assert_eq!(
                super::focused_output(&toplevels),
                want.map(|w| OutputName(w.to_owned()))
            );
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
