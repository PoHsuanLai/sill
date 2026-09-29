//! The Now Playing module: `Media::current`'s track on the module's panel, drawn by the view it
//! shares with the Now Playing widget ([`crate::widgets::now_playing::MediaView`]); a press sends
//! `MediaCommand::Press`. Reads `use_service::<NowPlayingService>`; asks the service to re-read the
//! position when it opens, so the bar starts from where the track is now.

use crate::surfaces::control_center::modules::use_plate;
use crate::widgets::now_playing::{MediaView, use_media};
use dioxus::prelude::*;
use ds::ModulePanel;

/// The Now Playing module.
#[component]
pub fn NowPlayingModule() -> Element {
    let plate = use_plate();
    let (face, onpress) = use_media();
    rsx! {
        ModulePanel { plate,
            MediaView { face, onpress }
        }
    }
}
