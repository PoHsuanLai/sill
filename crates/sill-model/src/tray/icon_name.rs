//! The name of a theme icon, as another program may send it. A status item or a menu row names
//! an icon the way a freedesktop theme does ("mail-unread"); it never names a file. The theme
//! lookup joins the name onto its own folders, so a name with a slash or `..` in it would leave
//! them and open whatever file the sender pointed at. A name that is not a plain theme name is
//! refused where it enters, before any lookup.

use super::limits::TRAY_ICON_NAME_MAX_BYTES;

/// `raw` when it is a plain theme icon name, else `None`: not empty, at most
/// [`TRAY_ICON_NAME_MAX_BYTES`] bytes, no path separator, no `..`, no leading `.` and no control
/// character.
pub fn theme_icon_name(raw: &str) -> Option<&str> {
    let plain = !raw.is_empty()
        && raw.len() <= TRAY_ICON_NAME_MAX_BYTES
        && !raw.starts_with('.')
        && !raw.contains("..")
        && !raw
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'));
    plain.then_some(raw)
}

#[cfg(test)]
mod tests {
    use super::theme_icon_name;
    use crate::tray::TRAY_ICON_NAME_MAX_BYTES;

    #[test]
    fn only_a_plain_theme_name_is_let_through() {
        let too_long = "a".repeat(TRAY_ICON_NAME_MAX_BYTES + 1);
        let longest = "a".repeat(TRAY_ICON_NAME_MAX_BYTES);
        let cases: &[(&str, bool)] = &[
            ("mail-unread", true),
            ("input-keyboard-symbolic", true),
            ("org.kde.app", true),
            (longest.as_str(), true),
            ("", false),
            (too_long.as_str(), false),
            ("/home/u/.var/app/x/spin", false),
            ("../../etc/passwd", false),
            ("a/b", false),
            ("a\\b", false),
            ("..", false),
            ("a..b", false),
            (".hidden", false),
            ("mail\nunread", false),
            ("mail\0", false),
        ];
        for &(name, want) in cases {
            assert_eq!(theme_icon_name(name).is_some(), want, "{name:?}");
        }
    }
}
