use shell_host::{OutputId, OutputInfo, Rect, Scale120};
use sill_ipc::act::{DisplayRequest, LevelRequest, VolumeRequest};
use sill_model::displays::ddc::Endpoint;
use sill_model::displays::{Controls, DdcDisplay, Displays, KeysTarget, Level as DdcLevel};
use sill_model::ids::{DdcId, OutputName};
use sill_model::units::Percent;
use sill_model::volume::Mute;
use sill_ui_kit::links::{Level, OsdKind};

use super::{fake_level, kind_of, osd_outputs};

fn output(id: u32, name: Option<&str>) -> OutputInfo {
    OutputInfo {
        id: OutputId(id),
        name: name.map(str::to_owned),
        description: None,
        logical: Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        scale: Scale120::ONE,
        refresh_mhz: 60_000,
    }
}

/// A monitor on `HDMI-A-2` that answers DDC/CI brightness; `DP-1` has none.
fn displays() -> Displays {
    Displays {
        ddc: vec![DdcDisplay {
            endpoint: Endpoint {
                id: DdcId(5),
                connector: Some(OutputName("HDMI-A-2".to_owned())),
                product: "Monitor".to_owned(),
            },
            controls: Controls {
                brightness: Some(DdcLevel {
                    value: 49,
                    max: 100,
                }),
                ..Controls::default()
            },
        }],
        ..Displays::default()
    }
}

/// One row: its name, the change, the keys' target, the focused output and the outputs that show.
type Case<'a> = (
    &'a str,
    OsdKind,
    Option<Level>,
    KeysTarget,
    Option<&'a str>,
    &'a [u32],
);

#[test]
fn each_change_shows_on_the_outputs_it_moved() {
    let outputs = [
        output(1, Some("DP-1")),
        output(2, Some("HDMI-A-2")),
        output(3, None),
    ];
    let fixed = Some(Level {
        percent: Percent(50),
        mute: Mute::Audible,
    });
    let live: Option<Level> = None;
    let pointer = KeysTarget::PointerOutput;
    let all = KeysTarget::AllOutputs;
    let cases: &[Case] = &[
        (
            "volume, focused",
            OsdKind::Volume,
            live,
            pointer,
            Some("HDMI-A-2"),
            &[2],
        ),
        (
            "volume, no focus",
            OsdKind::Volume,
            live,
            pointer,
            None,
            &[1],
        ),
        (
            "volume, all keys",
            OsdKind::Volume,
            live,
            all,
            Some("HDMI-A-2"),
            &[2],
        ),
        (
            "volume, unknown focus",
            OsdKind::Volume,
            live,
            pointer,
            Some("X"),
            &[1],
        ),
        (
            "brightness, focused",
            OsdKind::Brightness,
            live,
            pointer,
            Some("HDMI-A-2"),
            &[2],
        ),
        (
            "brightness, none to show",
            OsdKind::Brightness,
            live,
            pointer,
            Some("DP-1"),
            &[],
        ),
        (
            "brightness, every output",
            OsdKind::Brightness,
            live,
            all,
            None,
            &[2],
        ),
        (
            "brightness faked",
            OsdKind::Brightness,
            fixed,
            pointer,
            Some("DP-1"),
            &[1],
        ),
        (
            "brightness faked, all",
            OsdKind::Brightness,
            fixed,
            all,
            None,
            &[1, 2],
        ),
    ];
    for &(name, kind, level, keys, focus, want) in cases {
        let focus = focus.map(|f| OutputName(f.to_owned()));
        let got = osd_outputs(kind, level, keys, focus.as_ref(), &outputs, &displays());
        let want: Vec<OutputId> = want.iter().map(|id| OutputId(*id)).collect();
        assert_eq!(got, want, "{name}");
    }
}

#[test]
fn an_unnamed_output_still_shows_the_volume() {
    let outputs = [output(7, None)];
    let got = osd_outputs(
        OsdKind::Volume,
        None,
        KeysTarget::PointerOutput,
        None,
        &outputs,
        &Displays::default(),
    );
    assert_eq!(got, vec![OutputId(7)]);
    let none = osd_outputs(
        OsdKind::Volume,
        None,
        KeysTarget::PointerOutput,
        None,
        &[],
        &Displays::default(),
    );
    assert_eq!(none, Vec::<OutputId>::new());
}

#[test]
fn requests_name_their_kind_and_fakes_their_level() {
    assert_eq!(
        kind_of(DisplayRequest::Brightness(LevelRequest::Up)),
        OsdKind::Brightness
    );
    assert_eq!(
        kind_of(DisplayRequest::Volume(VolumeRequest::ToggleMute)),
        OsdKind::Volume
    );
    let cases = [
        ((Percent(40), Mute::Muted), (40, Mute::Muted)),
        ((Percent(250), Mute::Audible), (100, Mute::Audible)),
    ];
    for ((asked, asked_mute), (percent, mute)) in cases {
        assert_eq!(
            fake_level(asked, asked_mute),
            Level {
                percent: Percent(percent),
                mute
            },
            "{asked:?}"
        );
    }
}
