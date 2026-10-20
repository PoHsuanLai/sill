use super::{InputFramework, InputRefusal, address_in, next_of};

fn names(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn the_next_input_method_wraps_round() {
    const CASES: &[(&str, &[&str], Option<&str>)] = &[
        (
            "keyboard-us",
            &["keyboard-us", "pinyin", "mozc"],
            Some("pinyin"),
        ),
        ("pinyin", &["keyboard-us", "pinyin", "mozc"], Some("mozc")),
        (
            "mozc",
            &["keyboard-us", "pinyin", "mozc"],
            Some("keyboard-us"),
        ),
        ("keyboard-us", &["keyboard-us"], Some("keyboard-us")),
        ("gone", &["keyboard-us", "pinyin"], Some("keyboard-us")),
        ("", &["keyboard-us", "pinyin"], Some("keyboard-us")),
        ("keyboard-us", &[], None),
    ];
    for (current, list, want) in CASES {
        let list = names(list);
        assert_eq!(next_of(current, &list), *want, "{current} in {list:?}");
    }
}

#[test]
fn ibus_address_is_the_line_that_names_it() {
    const CASES: &[(&str, Option<&str>)] = &[
        (
            "# This file is created by ibus-daemon\nIBUS_ADDRESS=unix:path=/run/x,guid=ab\nIBUS_DAEMON_PID=7\n",
            Some("unix:path=/run/x,guid=ab"),
        ),
        ("IBUS_ADDRESS=\n", None),
        ("IBUS_DAEMON_PID=7\n", None),
        ("", None),
    ];
    for (text, want) in CASES {
        assert_eq!(address_in(text), *want, "{text:?}");
    }
}

#[test]
fn a_refusal_is_told_in_a_plain_sentence() {
    assert_eq!(
        InputRefusal::NoFramework.sentence(),
        "No input sources to switch."
    );
    assert_eq!(
        InputRefusal::Unsupported(InputFramework::IBus).sentence(),
        "This input method framework is not supported."
    );
}
