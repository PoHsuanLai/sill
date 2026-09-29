//! The wifi state's pure transition.

use super::{Radio, Wifi, WifiEvent};

/// `wifi` after `event`. Radio `Off` or `Blocked` empties the access points (NetworkManager
/// keeps stale ones for a while).
pub fn step(wifi: Wifi, event: WifiEvent) -> Wifi {
    match event {
        WifiEvent::Device(device) => Wifi { device, ..wifi },
        WifiEvent::Radio(radio) => {
            let access_points = match radio {
                Radio::On => wifi.access_points,
                Radio::Off | Radio::Blocked => Vec::new(),
            };
            Wifi {
                radio,
                access_points,
                ..wifi
            }
        }
        WifiEvent::AccessPoints(access_points) => Wifi {
            access_points,
            ..wifi
        },
        WifiEvent::Saved(saved) => Wifi { saved, ..wifi },
        WifiEvent::Gone => Wifi::default(),
    }
}
