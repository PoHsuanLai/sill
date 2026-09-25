//! The column's input and blur regions follow its cards (layer.rs's module doc; G60): each card
//! slot's element is kept as it mounts, and a frame after anything that moves or resizes a card
//! (one arriving, entering, leaving, expanding, a count) the slots are measured through quire's
//! host measurer and the surface's regions set from [`card_regions`]. While a card's actions
//! menu is open the column takes the whole surface's input, since the menu hangs outside the
//! cards; the blur stays on the cards.

use std::collections::BTreeMap;
use std::rc::Rc;

use dioxus::prelude::*;
use ds::{FRAME_SLACK, HostMeasure, Measured, sleep};
use shell_host::{BlurSpec, RegionSpec, SurfaceHandle};

use super::boxes::{CLOSE_REACH, card_box, reached};
use super::deck::EntryKey;
use super::regions::{CardBox, card_regions};
use crate::surfaces::notification_card::MenuOpen;

/// How many frames a read waits for a busy or not yet laid out document.
const ATTEMPTS: usize = 8;

/// The slots' elements, by card.
#[derive(Clone, Copy)]
pub(crate) struct Slots(CopyValue<BTreeMap<EntryKey, Rc<MountedData>>>);

impl Slots {
    /// Keep `key`'s slot element.
    pub fn mounted(self, key: EntryKey, event: Event<MountedData>) {
        let mut slots = self.0;
        slots.write().insert(key, event.data());
    }
}

/// Everything the regions follow, as the view draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegionKey {
    pub cards: Vec<(EntryKey, String)>,
    pub menu: MenuOpen,
}

/// Keep the surface's regions on the cards whenever `key` changes.
pub(crate) fn use_card_regions(key: RegionKey) -> Slots {
    let slots = Slots(use_hook(|| CopyValue::new(BTreeMap::new())));
    let surface = try_use_context::<SurfaceHandle>();
    let measure = try_use_context::<HostMeasure>();
    let mut last = use_hook(|| CopyValue::new(None::<RegionKey>));
    if last.peek().as_ref() == Some(&key) {
        return slots;
    }
    last.set(Some(key.clone()));
    let mut kept = slots.0;
    kept.write()
        .retain(|drawn, _| key.cards.iter().any(|(k, _)| k == drawn));
    if let (Some(surface), Some(HostMeasure(read))) = (surface, measure) {
        spawn(publish(slots, read, surface, key.menu));
    }
    slots
}

/// Measure every slot a frame from now (again while the document is busy), then set the
/// regions.
async fn publish(
    slots: Slots,
    read: fn(&MountedData) -> Measured,
    surface: SurfaceHandle,
    menu: MenuOpen,
) {
    for _ in 0..ATTEMPTS {
        sleep(FRAME_SLACK).await;
        let elements: Vec<Rc<MountedData>> = slots.0.peek().values().cloned().collect();
        let measured: Vec<Measured> = elements.iter().map(|e| read(e)).collect();
        if measured.iter().any(|m| matches!(m, Measured::Busy)) {
            continue;
        }
        let boxes: Vec<CardBox> = measured
            .into_iter()
            .filter_map(|m| match m {
                Measured::At(rect) => Some(card_box(rect)),
                Measured::Busy | Measured::Unknown => None,
            })
            .collect();
        let reach: Vec<CardBox> = boxes.iter().map(|b| reached(*b, CLOSE_REACH)).collect();
        let input = match menu {
            MenuOpen::Yes => RegionSpec::Whole,
            MenuOpen::No => card_regions(&reach),
        };
        surface.set_input_region(input);
        surface.set_blur(match card_regions(&boxes) {
            RegionSpec::Empty => BlurSpec::None,
            region => BlurSpec::Region(region),
        });
        return;
    }
}
