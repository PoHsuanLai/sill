//! The now playing state's pure transition.

use super::{Media, MediaEvent, PLAYERS_MAX};

/// `media` after `event`. A change for an id that is not listed is dropped, and so is a player
/// that would be more than [`PLAYERS_MAX`].
pub fn step(mut media: Media, event: MediaEvent) -> Media {
    match event {
        MediaEvent::Player(player) => {
            match media.players.iter_mut().find(|p| p.id == player.id) {
                Some(existing) => *existing = player,
                None if media.players.len() < PLAYERS_MAX => media.players.push(player),
                None => {}
            }
            media
        }
        MediaEvent::Playback { id, playback, at } => {
            if let Some(player) = media.players.iter_mut().find(|p| p.id == id) {
                player.playback = playback;
                player.changed = at;
            }
            media
        }
        MediaEvent::Track { id, track } => {
            if let Some(player) = media.players.iter_mut().find(|p| p.id == id) {
                player.track = track;
            }
            media
        }
        MediaEvent::Timeline { id, timeline } => {
            if let Some(player) = media.players.iter_mut().find(|p| p.id == id) {
                player.timeline = timeline;
            }
            media
        }
        MediaEvent::Abilities { id, abilities } => {
            if let Some(player) = media.players.iter_mut().find(|p| p.id == id) {
                player.abilities = abilities;
            }
            media
        }
        MediaEvent::Removed(id) => {
            media.players.retain(|p| p.id != id);
            media
        }
        MediaEvent::Gone => Media::default(),
    }
}
