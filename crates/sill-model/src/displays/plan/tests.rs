use super::{
    BrightnessAction, Originals, Probe, brightness_actions, probe_needed, set_brightness_action,
    warmth_writes,
};
use crate::displays::Displays;
use crate::displays::brightness::{Backlight, BacklightKind, BacklightName, LevelChange};
use crate::displays::ddc::{Controls, DdcDisplay, Endpoint, Feature, Level, RgbGain, Write};
use crate::displays::output::describe;
use crate::displays::policy::{KeysTarget, NightWarmth};
use crate::displays::randr::Head;
use crate::ids::{DdcId, OutputName};
use crate::units::Percent;
use crate::volume::Step;

fn head(name: &str) -> Head {
    Head {
        name: OutputName(name.to_owned()),
        ..Head::default()
    }
}

fn gain(red: u16, green: u16, blue: u16) -> RgbGain {
    let level = |value| Level { value, max: 100 };
    RgbGain {
        red: level(red),
        green: level(green),
        blue: level(blue),
    }
}

fn monitor(id: u16, connector: &str, gains: Option<RgbGain>) -> DdcDisplay {
    DdcDisplay {
        endpoint: Endpoint {
            id: DdcId(id),
            connector: Some(OutputName(connector.to_owned())),
            product: format!("Monitor {id}"),
        },
        controls: Controls {
            brightness: Some(Level {
                value: 49,
                max: 100,
            }),
            gain: gains,
            ..Controls::default()
        },
    }
}

/// A laptop panel (eDP-1) and two monitors (HDMI-A-2 on bus 5, DP-1 on bus 6).
fn desk() -> Displays {
    Displays {
        outputs: ["eDP-1", "HDMI-A-2", "DP-1"]
            .iter()
            .map(|n| describe(&head(n), None))
            .collect(),
        ddc: vec![
            monitor(5, "HDMI-A-2", Some(gain(100, 100, 100))),
            monitor(6, "DP-1", None),
        ],
        backlights: vec![Backlight {
            name: BacklightName("intel_backlight".to_owned()),
            kind: BacklightKind::Raw,
            value: 9600,
            max: 19200,
        }],
    }
}

#[test]
fn brightness_goes_where_the_keys_point() {
    let up = LevelChange::Step(Step::Up);
    let ddc = |id, value| {
        BrightnessAction::Ddc(Write {
            id: DdcId(id),
            feature: Feature::Brightness,
            value,
        })
    };
    let panel = |value| BrightnessAction::Backlight {
        name: BacklightName("intel_backlight".to_owned()),
        value,
    };
    let at = |n: &str| Some(OutputName(n.to_owned()));
    type Case = (
        &'static str,
        KeysTarget,
        Option<OutputName>,
        LevelChange,
        Vec<BrightnessAction>,
    );
    let cases: Vec<Case> = vec![
        (
            "pointer on the monitor",
            KeysTarget::PointerOutput,
            at("HDMI-A-2"),
            up,
            vec![ddc(5, 50)],
        ),
        (
            "pointer on the laptop",
            KeysTarget::PointerOutput,
            at("eDP-1"),
            up,
            vec![panel(10752)],
        ),
        (
            "pointer unknown: the first output",
            KeysTarget::PointerOutput,
            None,
            up,
            vec![panel(10752)],
        ),
        (
            "all outputs",
            KeysTarget::AllOutputs,
            None,
            up,
            vec![panel(10752), ddc(5, 50), ddc(6, 50)],
        ),
        (
            "a slider",
            KeysTarget::PointerOutput,
            at("DP-1"),
            LevelChange::Set(Percent(80)),
            vec![ddc(6, 80)],
        ),
        (
            "down to the floor",
            KeysTarget::PointerOutput,
            at("eDP-1"),
            LevelChange::Set(Percent(0)),
            vec![panel(1)],
        ),
    ];
    for (label, keys, at, change, want) in cases {
        assert_eq!(
            brightness_actions(&desk(), keys, at.as_ref(), change),
            want,
            "{label}"
        );
    }
}

#[test]
fn the_capsule_sets_its_own_outputs_monitor_with_or_without_the_outputs_list() {
    let name = |n: &str| OutputName(n.to_owned());
    let listed = desk();
    // Before (or without) an outputs answer: the DDC displays are known, the outputs not.
    let unlisted = Displays {
        outputs: Vec::new(),
        ..desk()
    };
    let ddc = |id, value| {
        Some(BrightnessAction::Ddc(Write {
            id: DdcId(id),
            feature: Feature::Brightness,
            value,
        }))
    };
    let cases = [
        ("the monitor, listed", &listed, "HDMI-A-2", 73, ddc(5, 73)),
        (
            "the monitor, not listed",
            &unlisted,
            "HDMI-A-2",
            73,
            ddc(5, 73),
        ),
        ("the other monitor only", &listed, "DP-1", 20, ddc(6, 20)),
        (
            "the laptop panel",
            &listed,
            "eDP-1",
            50,
            Some(BrightnessAction::Backlight {
                name: BacklightName("intel_backlight".to_owned()),
                value: 9600,
            }),
        ),
        ("an output with no target", &listed, "DP-9", 50, None),
    ];
    for (case, displays, output, level, want) in cases {
        assert_eq!(
            set_brightness_action(displays, &name(output), Percent(level)),
            want,
            "{case}"
        );
    }
    // The keys' path, by contrast, needs the outputs list.
    assert_eq!(
        brightness_actions(
            &unlisted,
            KeysTarget::PointerOutput,
            Some(&name("HDMI-A-2")),
            LevelChange::Set(Percent(73)),
        ),
        vec![]
    );
}

#[test]
fn a_monitor_missing_from_the_probe_is_probed_for_again() {
    let desk = desk();
    let one_missing = Displays {
        ddc: desk.ddc[..1].to_vec(),
        ..desk.clone()
    };
    let nothing = Displays::default();
    let panel_only = Displays {
        outputs: desk.outputs[..1].to_vec(),
        ddc: Vec::new(),
        ..desk.clone()
    };
    let unlisted = Displays {
        outputs: Vec::new(),
        ..desk.clone()
    };
    let cases = [
        ("every monitor answers", &desk, Probe::Done),
        ("DP-1 did not answer", &one_missing, Probe::Needed),
        ("nothing known", &nothing, Probe::Needed),
        ("only the laptop panel", &panel_only, Probe::Done),
        ("no outputs list, monitors known", &unlisted, Probe::Done),
    ];
    for (case, displays, want) in cases {
        assert_eq!(probe_needed(displays), want, "{case}");
    }
}

#[test]
fn warmth_scales_the_gains_and_off_restores_them() {
    let displays = desk();
    let (originals, writes) = warmth_writes(
        &displays,
        &Originals::new(),
        NightWarmth::Hardware(Percent(100)),
    );
    assert_eq!(originals.get(&DdcId(5)), Some(&gain(100, 100, 100)));
    assert_eq!(originals.len(), 1, "only displays with gains");
    let write = |feature, value| Write {
        id: DdcId(5),
        feature,
        value,
    };
    assert_eq!(
        writes,
        vec![write(Feature::GreenGain, 66), write(Feature::BlueGain, 35)],
        "red is already at its multiplier"
    );

    // The worker's cache now holds the warm gains; off puts the originals back.
    let warm = Displays {
        ddc: vec![
            monitor(5, "HDMI-A-2", Some(gain(100, 66, 35))),
            monitor(6, "DP-1", None),
        ],
        ..displays.clone()
    };
    let (kept, again) = warmth_writes(&warm, &originals, NightWarmth::Hardware(Percent(100)));
    assert_eq!(
        kept, originals,
        "originals are not re-read from warmed gains"
    );
    assert_eq!(again, vec![], "already warm");
    let (cleared, restore) = warmth_writes(&warm, &originals, NightWarmth::Off);
    assert_eq!(cleared, Originals::new());
    assert_eq!(
        restore,
        vec![
            write(Feature::GreenGain, 100),
            write(Feature::BlueGain, 100)
        ]
    );
    let (_, nothing) = warmth_writes(&displays, &Originals::new(), NightWarmth::Off);
    assert_eq!(nothing, vec![], "off with nothing changed writes nothing");
}
