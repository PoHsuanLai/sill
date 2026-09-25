//! The glyphs the control center wants and quire's catalogue lacks (Q81): media transport and
//! Bluetooth device kinds. Each is the nearest glyph quire has, named for the one it stands in
//! for, so the switch to quire's `Icon::Play`, `Pause`, `SkipBack`, `SkipForward`,
//! `Headphones`, `Speaker`, `Mouse`, `Gamepad` and `Phone` is this file alone.

use ds::Icon;
use sill_services::BtKind;

/// Play (Q81: `Icon::Play`): the paper plane points the way a play triangle does.
pub(crate) const PLAY: Icon = Icon::Send;
/// Pause (Q81: `Icon::Pause`): two columns.
pub(crate) const PAUSE: Icon = Icon::Columns;
/// Previous track (Q81: `Icon::SkipBack`).
pub(crate) const SKIP_BACK: Icon = Icon::ChevronLeft;
/// Next track (Q81: `Icon::SkipForward`).
pub(crate) const SKIP_FORWARD: Icon = Icon::ChevronRight;
/// Now Playing's own bar item and its empty art (Q81: `Icon::Play` there too).
pub(crate) const NOW_PLAYING: Icon = Icon::Volume2;

/// A Bluetooth device's glyph by kind (Q81: `Headphones`, `Speaker`, `Mouse`, `Gamepad`,
/// `Phone`; `Keyboard` and `Monitor` exist).
pub(crate) fn device_glyph(kind: BtKind) -> Icon {
    match kind {
        BtKind::Headphones | BtKind::Headset | BtKind::Speaker => Icon::Volume2,
        BtKind::Keyboard => Icon::Keyboard,
        BtKind::Mouse | BtKind::Gamepad => Icon::Command,
        BtKind::Phone | BtKind::Computer => Icon::Monitor,
        BtKind::Other => Icon::Bluetooth,
    }
}
