use super::brightness::{Backlight, BacklightKind, BacklightName, BrightnessTarget};
use super::ddc::{Controls, DdcDisplay, Endpoint, Level};
use super::{Displays, DisplaysEvent, step};
use crate::ids::{DdcId, OutputName};
use crate::units::Percent;

fn monitor() -> DdcDisplay {
    DdcDisplay {
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
    }
}

fn panel() -> Backlight {
    Backlight {
        name: BacklightName("intel_backlight".to_owned()),
        kind: BacklightKind::Raw,
        value: 9600,
        max: 19200,
    }
}

#[test]
fn each_source_replaces_its_own_part() {
    let with_ddc = step(Displays::default(), DisplaysEvent::Ddc(vec![monitor()]));
    let both = step(with_ddc.clone(), DisplaysEvent::Backlights(vec![panel()]));
    assert_eq!(both.ddc, with_ddc.ddc);
    assert_eq!(both.backlights, vec![panel()]);
    let cleared = step(both, DisplaysEvent::Ddc(Vec::new()));
    assert_eq!(cleared.ddc, vec![]);
    assert_eq!(cleared.backlights, vec![panel()]);
}

#[test]
fn targets_and_levels_follow_the_outputs() {
    let displays = Displays {
        ddc: vec![monitor()],
        backlights: vec![panel()],
        ..Displays::default()
    };
    let name = |n: &str| OutputName(n.to_owned());
    let hdmi = displays.target(&name("HDMI-A-2"));
    let panel_target = displays.target(&name("eDP-1"));
    assert_eq!(hdmi, BrightnessTarget::Ddc(DdcId(5)));
    assert_eq!(
        panel_target,
        BrightnessTarget::Backlight(BacklightName("intel_backlight".to_owned()))
    );
    assert_eq!(displays.target(&name("DP-1")), BrightnessTarget::None);
    assert_eq!(displays.brightness(&hdmi), Some(Percent(49)));
    assert_eq!(displays.brightness(&panel_target), Some(Percent(50)));
    assert_eq!(displays.brightness(&BrightnessTarget::None), None);
}
