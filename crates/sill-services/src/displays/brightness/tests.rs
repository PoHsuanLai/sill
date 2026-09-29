use super::{
    Backlight, BacklightKind, BacklightName, BrightnessTarget, LevelChange, backlight_value,
    percent_of, route, stepped, targets, value_of,
};
use crate::cosmic_wl::OutputName;
use crate::displays::drm::Placement;
use crate::displays::policy::KeysTarget;
use crate::env::ddc::DdcId;
use crate::units::Percent;
use crate::volume::Step;

fn name(n: &str) -> OutputName {
    OutputName(n.to_owned())
}

fn backlight(n: &str, kind: BacklightKind) -> Backlight {
    Backlight {
        name: BacklightName(n.to_owned()),
        kind,
        value: 100,
        max: 255,
    }
}

#[test]
fn each_output_routes_to_what_can_dim_it() {
    let ddc = vec![(DdcId(5), Some(name("HDMI-A-2"))), (DdcId(7), None)];
    let lights = vec![
        backlight("amdgpu_bl1", BacklightKind::Raw),
        backlight("acpi_video0", BacklightKind::Firmware),
    ];
    let cases: Vec<(&str, Placement, &[Backlight], BrightnessTarget)> = vec![
        (
            "HDMI-A-2",
            Placement::External,
            &lights,
            BrightnessTarget::Ddc(DdcId(5)),
        ),
        ("DP-1", Placement::External, &lights, BrightnessTarget::None),
        (
            "eDP-1",
            Placement::Internal,
            &lights,
            BrightnessTarget::Backlight(BacklightName("acpi_video0".to_owned())),
        ),
        ("eDP-1", Placement::Internal, &[], BrightnessTarget::None),
    ];
    for (output, placement, backlights, want) in cases {
        assert_eq!(
            route(&name(output), placement, backlights, &ddc),
            want,
            "{output}"
        );
    }
}

#[test]
fn keys_act_on_the_pointer_output_or_all() {
    let outputs = vec![name("eDP-1"), name("HDMI-A-2")];
    let pick = |keys, at: Option<&str>| -> Vec<String> {
        let at = at.map(name);
        targets(keys, at.as_ref(), &outputs)
            .into_iter()
            .map(|o| o.0.clone())
            .collect()
    };
    assert_eq!(
        pick(KeysTarget::PointerOutput, Some("HDMI-A-2")),
        ["HDMI-A-2"]
    );
    assert_eq!(pick(KeysTarget::PointerOutput, None), ["eDP-1"]);
    assert_eq!(pick(KeysTarget::PointerOutput, Some("gone")), ["eDP-1"]);
    assert_eq!(
        pick(KeysTarget::AllOutputs, Some("HDMI-A-2")),
        ["eDP-1", "HDMI-A-2"]
    );
    assert!(targets(KeysTarget::PointerOutput, None, &[]).is_empty());
}

#[test]
fn a_key_press_climbs_the_sixteen_step_ladder() {
    use LevelChange::{Set, Step as S};
    const CASES: &[(u16, LevelChange, u16)] = &[
        (49, S(Step::Up), 50),
        (50, S(Step::Up), 56),
        (49, S(Step::Down), 44),
        (50, S(Step::Down), 44),
        (0, S(Step::Down), 0),
        (0, S(Step::Up), 6),
        (100, S(Step::Up), 100),
        (100, S(Step::Down), 94),
        (97, S(Step::Up), 100),
        (3, S(Step::Down), 0),
        (40, Set(Percent(73)), 73),
        (40, Set(Percent(140)), 100),
    ];
    for &(level, change, want) in CASES {
        assert_eq!(
            stepped(Percent(level), change),
            Percent(want),
            "{level} {change:?}"
        );
    }
}

#[test]
fn percentages_convert_to_device_ranges_and_back() {
    const CASES: &[(u32, u32, u16)] = &[
        (49, 100, 49),
        (20, 50, 40),
        (128, 255, 50),
        (0, 0, 0),
        (300, 255, 100),
    ];
    for &(value, max, percent) in CASES {
        assert_eq!(percent_of(value, max), Percent(percent), "{value}/{max}");
    }
    assert_eq!(value_of(Percent(40), 50), 20);
    assert_eq!(value_of(Percent(50), 255), 128);
    assert_eq!(value_of(Percent(100), 96000), 96000);
    assert_eq!(backlight_value(Percent(0), 255), 1, "never fully dark");
    assert_eq!(backlight_value(Percent(0), 0), 0);
    assert_eq!(backlight_value(Percent(50), 255), 128);
}
