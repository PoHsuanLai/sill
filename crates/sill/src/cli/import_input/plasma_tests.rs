//! `import` over inline `kcminputrc` text with made-up devices, and `render` read back.

use super::{import, render};
use crate::cli::import_input::wire::{AccelProfile, InputConfig};

const KCMINPUTRC: &str = "\
[Keyboard]
RepeatDelay=400
RepeatRate=10

[Libinput][5][6][Acme Wheel Mouse]
NaturalScroll=true

[Libinput][7][8][Acme Pad Pro]
NaturalScroll=true
PointerAcceleration=-0.600
PointerAccelerationProfileFlat=true
ScrollFactor=1

[Libinput][9][10][Acme Tail Mouse]
ScrollFactor=2

[Libinput][11][12][ Acme Spaced Optical]
NaturalScroll=true
PointerAcceleration=0.400

[Libinput][13][14][Acme Idle Mouse]
Unrelated=1

[Plugin][Libinput]
NaturalScroll=true
";

fn imported(name: &str) -> InputConfig {
    import(KCMINPUTRC)
        .remove(name)
        .unwrap_or_else(|| panic!("{name} was not imported"))
}

fn scroll(name: &str) -> (Option<bool>, Option<f64>) {
    let config = imported(name).scroll_config.expect("scroll_config");
    (config.natural_scroll, config.scroll_factor)
}

#[test]
fn only_device_groups_with_a_known_key_are_imported() {
    let names: Vec<String> = import(KCMINPUTRC).into_keys().collect();
    assert_eq!(
        names,
        [
            " Acme Spaced Optical",
            "Acme Pad Pro",
            "Acme Tail Mouse",
            "Acme Wheel Mouse"
        ]
    );
}

#[test]
fn scroll_settings_follow_the_keys_and_natural_scroll_is_stated_outright() {
    const CASES: &[(&str, Option<bool>, Option<f64>)] = &[
        ("Acme Wheel Mouse", Some(true), None),
        ("Acme Pad Pro", Some(true), Some(1.0)),
        // Plasma wrote no NaturalScroll: libinput's default, which is off.
        ("Acme Tail Mouse", Some(false), Some(2.0)),
    ];
    for (name, natural, factor) in CASES {
        assert_eq!(scroll(name), (*natural, *factor), "{name}");
    }
}

#[test]
fn acceleration_takes_the_speed_and_the_profile() {
    let pad = imported("Acme Pad Pro").acceleration.expect("acceleration");
    assert_eq!((pad.speed, pad.profile), (-0.6, Some(AccelProfile::Flat)));
    let spaced = imported(" Acme Spaced Optical")
        .acceleration
        .expect("acceleration");
    assert_eq!((spaced.speed, spaced.profile), (0.4, None));
    assert_eq!(imported("Acme Wheel Mouse").acceleration, None);
}

#[test]
fn the_profile_keys_read_as_flat_or_adaptive() {
    const CASES: &[(&str, Option<AccelProfile>)] = &[
        (
            "PointerAccelerationProfileFlat=true",
            Some(AccelProfile::Flat),
        ),
        (
            "PointerAccelerationProfileAdaptive=true",
            Some(AccelProfile::Adaptive),
        ),
        (
            "PointerAccelerationProfileFlat=false",
            Some(AccelProfile::Adaptive),
        ),
        (
            "PointerAccelerationProfileAdaptive=false",
            Some(AccelProfile::Flat),
        ),
        ("PointerAccelerationProfileFlat=maybe", None),
    ];
    for (line, want) in CASES {
        let text = format!("[Libinput][1][2][Acme]\nPointerAcceleration=0.1\n{line}\n");
        let got = import(&text).remove("Acme").and_then(|c| c.acceleration);
        assert_eq!(got.map(|a| a.profile), Some(*want), "{line}");
    }
}

#[test]
fn malformed_group_headers_and_values_are_skipped() {
    const CASES: &[(&str, &str)] = &[
        ("[Libinput][x][2][Acme]", "NaturalScroll=true"),
        ("[Libinput][1][2][]", "NaturalScroll=true"),
        ("[Libinput][1][Acme]", "NaturalScroll=true"),
        ("[Libinput][1][2][Acme]", "ScrollFactor=fast"),
        ("[Libinput][1][2][Acme]", "PointerAcceleration=inf"),
    ];
    for (header, line) in CASES {
        assert!(
            import(&format!("{header}\n{line}\n")).is_empty(),
            "{header} {line}"
        );
    }
}

#[test]
fn names_keep_escaped_brackets() {
    let text = "[Libinput][1][2][Acme \\x5bUSB\\x5d]\nNaturalScroll=true\n";
    assert_eq!(import(text).into_keys().collect::<Vec<_>>(), ["Acme [USB]"]);
}

#[test]
fn the_rendered_file_reads_back_as_the_same_map() {
    let devices = import(KCMINPUTRC);
    let text = render(&devices).expect("renders");
    let back: std::collections::BTreeMap<String, InputConfig> =
        ron::from_str(&text).expect("cosmic-comp's RON");
    assert_eq!(back, devices);
    assert!(text.contains("natural_scroll: Some(true)"), "{text}");
}

#[test]
fn no_device_groups_render_an_empty_map() {
    assert!(import("[Keyboard]\nRepeatRate=10\n").is_empty());
    assert_eq!(render(&import("")).expect("renders").trim(), "{}");
}
