//! The Now Playing module: `Media::current`'s track on the module's panel, drawn by the view it
//! shares with the Now Playing widget ([`crate::widgets::now_playing::MediaView`]); a press sends
//! `MediaCommand::Press`. Reads `use_service::<NowPlayingService>`; asks the service to re-read the
//! position when it opens, so the bar starts from where the track is now.

use super::cc_module::{BarItemFace, CcCx, CcModule, Span};
use crate::hooks::use_service;
use crate::surfaces::bar::{BarMenu, StatusIcon};
use crate::surfaces::control_center::modules::use_plate;
use crate::widgets::now_playing::{MediaView, use_media};
use dioxus::prelude::*;
use ds::Icon;
use ds_shell::ModulePanel;
use sill_services::NowPlayingService;
use sill_settings::{ControlCenterSettings, InMenuBar};

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

/// The Now Playing item in the bar: shown while any player is up.
#[component]
fn NowPlayingItem() -> Element {
    let media = use_service::<NowPlayingService>()();
    if media.players.is_empty() {
        return rsx! {};
    }
    rsx! {
        StatusIcon { menu: BarMenu::NowPlaying, icon: Icon::Play, tooltip: "Now Playing".to_owned() }
    }
}

/// The Now Playing module as the control center and the bar draw it.
pub(crate) struct NowPlayingCc;

impl CcModule for NowPlayingCc {
    fn span(&self) -> Span {
        Span::Row
    }

    fn tile(&self, _cx: &CcCx) -> Element {
        rsx! { NowPlayingModule {} }
    }

    fn menu_bar(&self, settings: &ControlCenterSettings) -> InMenuBar {
        settings.menu_bar_now_playing
    }

    fn bar_item(&self) -> Option<BarItemFace> {
        Some(BarItemFace {
            menu: BarMenu::NowPlaying,
            item: || rsx! { NowPlayingItem {} },
        })
    }
}
