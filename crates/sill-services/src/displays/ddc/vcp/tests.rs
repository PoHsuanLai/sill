use super::{ColorPreset, Controls, Feature, InputSource, Level, RgbGain, mute_value};
use sill_model::volume::Mute;

fn level(value: u16, max: u16) -> Option<Level> {
    Some(Level { value, max })
}

/// What the MA270U answered (ddcutil and sill's own probe agree).
fn ma270u() -> Vec<(Feature, Option<Level>)> {
    vec![
        (Feature::Brightness, level(49, 100)),
        (Feature::Contrast, level(50, 100)),
        (Feature::ColorPreset, level(0x0B, 0)),
        (Feature::RedGain, level(100, 100)),
        (Feature::GreenGain, level(100, 100)),
        (Feature::BlueGain, level(100, 100)),
        (Feature::InputSource, level(0x11, 0)),
        (Feature::SpeakerVolume, level(20, 50)),
        (Feature::AudioMute, level(0x02, 0)),
    ]
}

#[test]
fn readings_become_typed_controls() {
    let controls = Controls::from_readings(&ma270u());
    let gain = Level {
        value: 100,
        max: 100,
    };
    assert_eq!(
        controls,
        Controls {
            brightness: level(49, 100),
            contrast: level(50, 100),
            color_preset: Some(ColorPreset(0x0B)),
            gain: Some(RgbGain {
                red: gain,
                green: gain,
                blue: gain,
            }),
            input: Some(InputSource(0x11)),
            volume: level(20, 50),
            mute: Some(Mute::Audible),
        }
    );
    assert_eq!(controls.input.and_then(InputSource::name), Some("HDMI 1"));
}

#[test]
fn a_failed_read_is_a_missing_control() {
    let mut readings = ma270u();
    readings.retain(|(f, _)| *f != Feature::GreenGain && *f != Feature::SpeakerVolume);
    readings.push((Feature::SpeakerVolume, None));
    let controls = Controls::from_readings(&readings);
    assert_eq!(controls.gain, None, "two of three gains is no gain control");
    assert_eq!(controls.volume, None);
    assert_eq!(controls.brightness, level(49, 100));
    assert!(controls.any());
    assert!(!Controls::from_readings(&[(Feature::Brightness, None)]).any());
}

#[test]
fn a_write_moves_the_value_and_keeps_the_maximum() {
    let controls = Controls::from_readings(&ma270u());
    let cases: Vec<(Feature, u16, Controls)> = vec![
        (
            Feature::Brightness,
            44,
            Controls {
                brightness: level(44, 100),
                ..controls
            },
        ),
        (
            Feature::SpeakerVolume,
            25,
            Controls {
                volume: level(25, 50),
                ..controls
            },
        ),
        (
            Feature::AudioMute,
            mute_value(Mute::Muted),
            Controls {
                mute: Some(Mute::Muted),
                ..controls
            },
        ),
        (
            Feature::BlueGain,
            80,
            Controls {
                gain: controls.gain.map(|g| RgbGain {
                    blue: Level {
                        value: 80,
                        max: 100,
                    },
                    ..g
                }),
                ..controls
            },
        ),
    ];
    for (feature, value, want) in cases {
        assert_eq!(controls.written(feature, value), want, "{feature:?}");
    }
    let bare = Controls::default();
    for feature in Feature::ALL {
        assert_eq!(
            bare.written(feature, 1),
            bare,
            "{feature:?} on a display without it"
        );
    }
}

#[test]
fn codes_are_mccs() {
    let codes: Vec<u8> = Feature::ALL.iter().map(|f| f.code()).collect();
    assert_eq!(
        codes,
        [0x10, 0x12, 0x14, 0x16, 0x18, 0x1A, 0x60, 0x62, 0x8D]
    );
}
