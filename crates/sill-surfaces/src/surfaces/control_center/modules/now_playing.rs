//! The Now Playing module: `Media::current`'s art (quire `IconView` with an
//! `IconUrl::file`, `IconSize::Tile48`), [`crate::surfaces::control_center::track_lines`], and
//! Previous, Play/Pause and Next as quire `IconButton`s, each disabled per `Abilities`, sending
//! `MediaCommand::Press`. Nothing playing: the module shows "Not Playing" and no buttons.
//! Reads `use_now_playing`.

use dioxus::prelude::*;

/// The Now Playing module.
#[component]
pub fn NowPlayingModule() -> Element {
    todo!("M5 fill: control-center (c)")
}
