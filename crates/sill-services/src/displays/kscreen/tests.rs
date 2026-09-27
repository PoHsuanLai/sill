use super::{hundredths, parse_outputs, thousandths};
use crate::cosmic_wl::OutputName;
use crate::displays::brightness::LevelChange;
use crate::displays::ddc::{Controls, DdcDisplay, DdcId, Endpoint, Feature, Level, Write};
use crate::displays::drm::Connector;
use crate::displays::edid::tests::MA270U;
use crate::displays::edid::{Millimetres, Mode, Pixels};
use crate::displays::model::Displays;
use crate::displays::output::describe;
use crate::displays::plan::{BrightnessAction, brightness_actions};
use crate::displays::policy::KeysTarget;
use crate::displays::randr::{Enablement, Head};
use crate::displays::scale::{DisplayScale, Ppi, ScaleRounding};
use crate::volume::Step;

/// `kscreen-doctor --json` on the user's KWin (Plasma 6, wayland-0) with the BenQ MA270U on
/// HDMI-A-2 at 3840 x 2160 and scale 2, as printed on 2026-09-28. It carries no EDID or serial.
const MA270U_ON_KWIN: &str = include_str!("ma270u.json");

fn ma270u(scale: u16) -> Head {
    Head {
        name: OutputName("HDMI-A-2".to_owned()),
        enablement: Enablement::Enabled,
        make: String::new(),
        model: "HDMI-A-2".to_owned(),
        physical: Some(Millimetres {
            width: 597,
            height: 336,
        }),
        current: Some(Mode {
            pixels: Pixels {
                width: 3840,
                height: 2160,
            },
            refresh_mhz: 59997,
        }),
        scale: DisplayScale(scale),
    }
}

/// A two-output config in KScreen's shape: a disabled laptop panel with no current mode, an
/// external monitor at 1.25, and a disconnected connector that must not appear.
const TWO_AND_A_GHOST: &str = r#"{"outputs": [
  {"name": "eDP-1", "connected": true, "enabled": false, "currentModeId": "",
   "modes": [{"id": "1", "refreshRate": 60.0, "size": {"width": 2880, "height": 1800}}],
   "scale": 2, "sizeMM": {"width": 302, "height": 189}},
  {"name": "DP-3", "connected": true, "enabled": true, "currentModeId": "7",
   "modes": [{"id": "7", "refreshRate": 143.98500061035156, "size": {"width": 2560, "height": 1440}}],
   "scale": 1.25, "sizeMM": {"width": 0, "height": 0}},
  {"name": "DP-4", "connected": false, "enabled": false, "modes": [], "scale": 1}
]}"#;

#[test]
fn kscreen_configs_parse_to_heads() {
    let panel = Head {
        name: OutputName("eDP-1".to_owned()),
        enablement: Enablement::Disabled,
        model: "eDP-1".to_owned(),
        physical: Some(Millimetres {
            width: 302,
            height: 189,
        }),
        current: None,
        scale: DisplayScale(200),
        ..Head::default()
    };
    let monitor = Head {
        name: OutputName("DP-3".to_owned()),
        model: "DP-3".to_owned(),
        physical: None,
        current: Some(Mode {
            pixels: Pixels {
                width: 2560,
                height: 1440,
            },
            refresh_mhz: 143985,
        }),
        scale: DisplayScale(125),
        ..Head::default()
    };
    let cases: Vec<(&str, &str, Vec<Head>)> = vec![
        ("the user's KWin", MA270U_ON_KWIN, vec![ma270u(200)]),
        ("two and a ghost", TWO_AND_A_GHOST, vec![panel, monitor]),
        ("no outputs", r#"{"outputs": []}"#, Vec::new()),
        ("no outputs key", r#"{"screen": {}}"#, Vec::new()),
    ];
    for (what, json, want) in cases {
        let got = parse_outputs(json).unwrap_or_else(|e| panic!("{what}: {e}"));
        assert_eq!(got, want, "{what}");
    }
}

#[test]
fn text_that_is_not_a_config_is_an_error() {
    for text in ["", "kscreen-doctor: no backend", "[1, 2]"] {
        assert!(parse_outputs(text).is_err(), "{text:?}");
    }
}

#[test]
fn numbers_convert_or_fall_back() {
    let scales: &[(f64, Option<u16>)] = &[
        (1.0, Some(100)),
        (1.5, Some(150)),
        (1.25, Some(125)),
        (1.100_000_023_841_858, Some(110)),
        (0.0, None),
        (-1.0, None),
        (1e9, None),
    ];
    for &(scale, want) in scales {
        assert_eq!(hundredths(scale), want, "{scale}");
    }
    let rates: &[(f64, u32)] = &[
        (59.99700164794922, 59997),
        (60.0, 60000),
        (0.0, 0),
        (-5.0, 0),
    ];
    for &(hz, want) in rates {
        assert_eq!(thousandths(hz), want, "{hz}");
    }
}

/// The brightness keys on the user's KWin, end to end in values: the fixture's outputs, the
/// MA270U's EDID from sysfs, and a fake DDC display on HDMI-A-2 at 49 (what ddcutil read). A
/// key press up is one DDC write to 50, the step the user saw with the runner's stand-in; with
/// no outputs (cosmic-randr silent and no fallback, as before) it is nothing at all.
#[test]
fn the_brightness_keys_reach_the_monitor_by_its_connector() {
    let connector = Connector {
        name: OutputName("HDMI-A-2".to_owned()),
        edid: MA270U.to_vec(),
    };
    let outputs: Vec<_> = parse_outputs(MA270U_ON_KWIN)
        .unwrap_or_else(|e| panic!("{e}"))
        .iter()
        .map(|h| describe(h, Some(&connector), Ppi(109), ScaleRounding::PreferWhole))
        .collect();
    let ddc = vec![DdcDisplay {
        endpoint: Endpoint {
            id: DdcId(5),
            connector: Some(OutputName("HDMI-A-2".to_owned())),
            product: "BenQ MA270U".to_owned(),
        },
        controls: Controls {
            brightness: Some(Level {
                value: 49,
                max: 100,
            }),
            ..Controls::default()
        },
    }];
    let with_outputs = Displays {
        outputs,
        ddc: ddc.clone(),
        ..Displays::default()
    };
    let without = Displays {
        ddc,
        ..Displays::default()
    };
    let at = OutputName("HDMI-A-2".to_owned());
    let to_50 = vec![BrightnessAction::Ddc(Write {
        id: DdcId(5),
        feature: Feature::Brightness,
        value: 50,
    })];
    type Case<'a> = (
        &'a str,
        &'a Displays,
        KeysTarget,
        Option<&'a OutputName>,
        Vec<BrightnessAction>,
    );
    let cases: Vec<Case> = vec![
        (
            "pointer on the monitor",
            &with_outputs,
            KeysTarget::PointerOutput,
            Some(&at),
            to_50.clone(),
        ),
        (
            "pointer unknown",
            &with_outputs,
            KeysTarget::PointerOutput,
            None,
            to_50.clone(),
        ),
        (
            "all outputs",
            &with_outputs,
            KeysTarget::AllOutputs,
            None,
            to_50,
        ),
        (
            "no outputs (before the fallback)",
            &without,
            KeysTarget::PointerOutput,
            Some(&at),
            Vec::new(),
        ),
    ];
    for (what, displays, keys, at, want) in cases {
        let got = brightness_actions(displays, keys, at, LevelChange::Step(Step::Up));
        assert_eq!(got, want, "{what}");
    }
}
