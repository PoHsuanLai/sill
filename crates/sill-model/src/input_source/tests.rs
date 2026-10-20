use super::next_of;

fn names(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn the_next_input_method_wraps_round() {
    const CASES: &[(&str, &[&str], Option<&str>)] = &[
        ("keyboard-us", &["keyboard-us", "pinyin", "mozc"], Some("pinyin")),
        ("pinyin", &["keyboard-us", "pinyin", "mozc"], Some("mozc")),
        ("mozc", &["keyboard-us", "pinyin", "mozc"], Some("keyboard-us")),
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
