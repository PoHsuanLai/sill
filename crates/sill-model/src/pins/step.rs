//! The pins state's pure transition.

use super::{DockPins, PinAt, PinEvent};

/// `pins` after `event`.
pub fn step(pins: DockPins, event: PinEvent) -> DockPins {
    let DockPins { mut pinned } = pins;
    match event {
        PinEvent::Pin { app, at } => {
            pinned.retain(|a| a != &app);
            let slot = match at {
                PinAt::End => pinned.len(),
                PinAt::Index(i) => clamp(i, pinned.len()),
            };
            pinned.insert(slot, app);
        }
        PinEvent::Unpin(app) => pinned.retain(|a| a != &app),
        PinEvent::Move { app, to } => {
            if let Some(from) = pinned.iter().position(|a| a == &app) {
                let moved = pinned.remove(from);
                pinned.insert(clamp(to, pinned.len()), moved);
            }
        }
        PinEvent::Replaced(all) => return all,
    }
    DockPins { pinned }
}

fn clamp(index: u32, len: usize) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX).min(len)
}
