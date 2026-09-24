use super::{follow, monitor_for_sink, monitor_in_sink};
use crate::cosmic_wl::OutputName;
use crate::displays::ddc::{Controls, DdcDisplay, DdcId, Endpoint, Feature, Level, Write};
use crate::displays::policy::HardwareVolume;
use crate::units::{Percent, Presence};
use crate::volume::{Mute, Volume};

/// The MA270U's speaker volume as it answers VCP 0x62.
const SPEAKERS: Option<Level> = Some(Level { value: 20, max: 50 });

/// The MA270U's sink as pipewire-pulse describes it.
const SINK: &str = "GB203 High Definition Audio Controller Digital Stereo (HDMI) [BenQ MA270U]";

fn ma270u(volume: Option<Level>) -> DdcDisplay {
    DdcDisplay {
        endpoint: Endpoint {
            id: DdcId(5),
            connector: Some(OutputName("HDMI-A-2".to_owned())),
            product: "BenQ MA270U".to_owned(),
        },
        controls: Controls {
            volume,
            mute: Some(Mute::Audible),
            ..Controls::default()
        },
    }
}

fn sink(description: &str, level: u16, mute: Mute) -> Volume {
    Volume {
        presence: Presence::Present,
        level: Percent(level),
        mute,
        sink: description.to_owned(),
    }
}

#[test]
fn the_monitor_name_is_the_bracketed_suffix() {
    let cases: &[(&str, Option<&str>)] = &[
        (SINK, Some("BenQ MA270U")),
        (
            "HDA Intel Digital Stereo (HDMI 2) [DELL U2723QE] ",
            Some("DELL U2723QE"),
        ),
        ("Built-in Audio Analog Stereo", None),
        ("USB Audio [ ]", None),
        ("[BenQ MA270U] headphones", None),
    ];
    for &(description, want) in cases {
        assert_eq!(monitor_in_sink(description), want, "{description}");
    }
    let displays = [ma270u(SPEAKERS)];
    assert!(monitor_for_sink(SINK, &displays).is_some());
    assert!(
        monitor_for_sink(SINK, &[ma270u(None)]).is_none(),
        "no VCP 0x62"
    );
    assert!(
        monitor_for_sink("x [BenQ MA270]", &displays).is_none(),
        "whole names only"
    );
}

#[test]
fn only_a_change_on_the_monitor_sink_follows_through() {
    use HardwareVolume::{Off, WhenMonitorIsOutput as On};
    let volume = |value| Write {
        id: DdcId(5),
        feature: Feature::SpeakerVolume,
        value,
    };
    let mute = |value| Write {
        id: DdcId(5),
        feature: Feature::AudioMute,
        value,
    };
    let speakers = "Built-in Audio Analog Stereo";
    let displays = [ma270u(SPEAKERS)];
    let cases: Vec<(&str, HardwareVolume, Volume, Volume, Vec<Write>)> = vec![
        (
            "level up",
            On,
            sink(SINK, 40, Mute::Audible),
            sink(SINK, 45, Mute::Audible),
            vec![volume(23)],
        ),
        (
            "to 100",
            On,
            sink(SINK, 95, Mute::Audible),
            sink(SINK, 100, Mute::Audible),
            vec![volume(50)],
        ),
        (
            "above 100 stays at the monitor's max",
            On,
            sink(SINK, 100, Mute::Audible),
            sink(SINK, 130, Mute::Audible),
            vec![volume(50)],
        ),
        (
            "mute",
            On,
            sink(SINK, 40, Mute::Audible),
            sink(SINK, 40, Mute::Muted),
            vec![mute(1)],
        ),
        (
            "unmute",
            On,
            sink(SINK, 40, Mute::Muted),
            sink(SINK, 40, Mute::Audible),
            vec![mute(2)],
        ),
        (
            "off",
            Off,
            sink(SINK, 40, Mute::Audible),
            sink(SINK, 45, Mute::Audible),
            vec![],
        ),
        (
            "another sink",
            On,
            sink(speakers, 40, Mute::Audible),
            sink(speakers, 45, Mute::Audible),
            vec![],
        ),
        (
            "switching to the monitor sends nothing",
            On,
            sink(speakers, 40, Mute::Audible),
            sink(SINK, 80, Mute::Audible),
            vec![],
        ),
        (
            "the sink appearing sends nothing",
            On,
            Volume::default(),
            sink(SINK, 80, Mute::Audible),
            vec![],
        ),
        (
            "nothing changed",
            On,
            sink(SINK, 40, Mute::Audible),
            sink(SINK, 40, Mute::Audible),
            vec![],
        ),
    ];
    for (label, policy, before, after, want) in cases {
        assert_eq!(follow(policy, &before, &after, &displays), want, "{label}");
    }
}
