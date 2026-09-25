//! The Now Playing module: `Media::current`'s art (quire `IconView` with an
//! `IconUrl::file`, `IconSize::Tile48`), [`crate::surfaces::control_center::track_lines`], and
//! Previous, Play/Pause and Next as quire `IconButton`s, each disabled per `Abilities`, sending
//! `MediaCommand::Press`. Nothing playing: the module shows "Not Playing" and no buttons.
//! Reads `use_now_playing`.

use dioxus::prelude::*;
use ds::{
    Availability, ExternalIcon, Glyph, Icon, IconButton, IconButtonVariant, IconSize, IconSource,
    IconUrl, IconView, ModulePanel,
};
use sill_services::{Art, MediaCommand, MediaVerb, Playback};

use crate::hooks::{use_now_playing, use_services};
use crate::surfaces::control_center::modules::use_plate;
use crate::surfaces::control_center::track_lines;

/// The Now Playing module.
#[component]
pub fn NowPlayingModule() -> Element {
    let media = use_now_playing()();
    let control = use_services().media_control;
    let playing = media.current().cloned();
    let plate = use_plate();
    let Some(player) = playing else {
        return rsx! {
            ModulePanel { plate,
                div { class: "sill-cc-media",
                    ArtView { art: Art::None }
                    div { class: "sill-cc-media-text",
                        span { class: "sill-cc-title", "Not Playing" }
                    }
                }
            }
        };
    };
    let (title, by) = track_lines(&player);
    let press = move |verb: MediaVerb| {
        let control = control.clone();
        let player = player.id.clone();
        move |_| {
            // A stopped service drops it: the module shows what it last published.
            let _gone = control.send(MediaCommand::Press {
                player: player.clone(),
                verb,
            });
        }
    };
    let (toggle, toggle_label) = match player.playback {
        Playback::Playing => (Icon::Pause, "Pause"),
        Playback::Paused | Playback::Stopped => (Icon::Play, "Play"),
    };
    let abilities = player.abilities;
    rsx! {
        ModulePanel { plate,
            div { class: "sill-cc-media",
                ArtView { art: player.track.art.clone() }
                div { class: "sill-cc-media-text",
                    span { class: "sill-cc-title ds-truncate", "{title}" }
                    span { class: "sill-cc-status ds-truncate", "{by}" }
                }
                div { class: "sill-cc-media-buttons",
                    IconButton {
                        variant: IconButtonVariant::Tool,
                        icon: Icon::SkipBack,
                        label: "Previous".to_owned(),
                        availability: available(abilities.previous),
                        onclick: press(MediaVerb::Previous),
                    }
                    IconButton {
                        variant: IconButtonVariant::Tool,
                        icon: toggle,
                        label: toggle_label.to_owned(),
                        availability: available(abilities.play_pause),
                        onclick: press(MediaVerb::PlayPause),
                    }
                    IconButton {
                        variant: IconButtonVariant::Tool,
                        icon: Icon::SkipForward,
                        label: "Next".to_owned(),
                        availability: available(abilities.next),
                        onclick: press(MediaVerb::Next),
                    }
                }
            }
        }
    }
}

/// The service's word for "can do it" as quire's.
fn available(can: sill_services::Availability) -> Availability {
    match can {
        sill_services::Availability::Enabled => Availability::Enabled,
        sill_services::Availability::Disabled => Availability::Disabled,
    }
}

/// The track's art at 48, or the Now Playing glyph on the art's plate.
#[component]
fn ArtView(art: Art) -> Element {
    let image = match &art {
        Art::File(path) => IconUrl::file(path).ok(),
        Art::None => None,
    };
    match image {
        Some(url) => rsx! {
            span { class: "sill-cc-art",
                IconView {
                    source: IconSource::Image(ExternalIcon { url, size: IconSize::Tile48 }),
                    size: IconSize::Tile48,
                }
            }
        },
        None => rsx! {
            span { class: "sill-cc-art", "data-art": "none",
                Glyph { icon: Icon::Play }
            }
        },
    }
}
