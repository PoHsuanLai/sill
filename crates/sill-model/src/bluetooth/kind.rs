//! A device's kind from BlueZ's `Device1.Icon`: the freedesktop icon name BlueZ derives from
//! the device class (`audio-headphones`, `input-keyboard`, ...). Compared whole: an icon name
//! is a token, not a substring (CONVENTIONS "Substrings are not tokens").

use super::BtKind;

/// The kind `icon` names; `Other` for an icon this list does not know, or none at all.
pub fn device_kind(icon: &str) -> BtKind {
    match icon {
        "audio-headphones" => BtKind::Headphones,
        "audio-headset" => BtKind::Headset,
        "audio-speakers" | "audio-card" => BtKind::Speaker,
        "input-keyboard" => BtKind::Keyboard,
        "input-mouse" | "input-tablet" => BtKind::Mouse,
        "input-gaming" => BtKind::Gamepad,
        "phone" => BtKind::Phone,
        "computer" => BtKind::Computer,
        _ => BtKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::device_kind;
    use crate::bluetooth::BtKind;

    #[test]
    fn bluez_icon_names_map_to_kinds() {
        const CASES: &[(&str, BtKind)] = &[
            ("audio-headphones", BtKind::Headphones),
            ("audio-headset", BtKind::Headset),
            ("audio-speakers", BtKind::Speaker),
            ("audio-card", BtKind::Speaker),
            ("input-keyboard", BtKind::Keyboard),
            ("input-mouse", BtKind::Mouse),
            ("input-tablet", BtKind::Mouse),
            ("input-gaming", BtKind::Gamepad),
            ("phone", BtKind::Phone),
            ("computer", BtKind::Computer),
            ("audio-headphones-extra", BtKind::Other),
            ("", BtKind::Other),
        ];
        for &(icon, want) in CASES {
            assert_eq!(device_kind(icon), want, "{icon:?}");
        }
    }
}
