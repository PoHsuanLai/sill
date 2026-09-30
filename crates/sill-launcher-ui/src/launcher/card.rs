//! Measuring the palette's card (the macOS polish pass: it is as tall as its content). An empty
//! foot element follows the card in the panel's column, so its top is the card's bottom edge;
//! the panel reads it through quire's host measurer a frame after anything that can change the
//! card's height (an opening, the query, the rows, the actions menu) and publishes what the
//! panel takes ([`PanelHit`]) on the shared link, where the panel's own input region and the
//! catcher's follow it.

use std::rc::Rc;

use dioxus::prelude::*;
use ds::{DocumentHost, FRAME_SLACK, Measured, sleep, use_document_host};
use shell_host::{RegionSpec, SurfaceHandle, use_watch};

use crate::launcher::hit::{ActionsShown, hit_for, panel_input};
use sill_ui_kit::links::PanelHit;
use sill_ui_kit::links::{LauncherLink, OpenSerial};

/// How many frames a read waits for a busy or not yet laid out document.
const ATTEMPTS: usize = 8;

/// Everything the card's height follows: a change is a reason to measure again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CardKey {
    /// The launcher is hidden here: the panel takes nothing it could measure.
    Hidden,
    Shown {
        serial: OpenSerial,
        query: String,
        rows: usize,
        actions: ActionsShown,
    },
}

/// The foot element's mount, which the panel hands [`Foot::on_mounted`].
#[derive(Clone, Copy)]
pub(crate) struct Foot(Signal<Option<Rc<MountedData>>>);

impl Foot {
    /// Keep the foot element to read it later.
    pub fn on_mounted(mut self, event: Event<MountedData>) {
        self.0.set(Some(event.data()));
    }
}

/// Measure the card again whenever `key` changes, publish what the panel takes on `link`, and
/// keep this surface's input region on it.
pub(crate) fn use_card_hit(link: &LauncherLink, key: CardKey) -> Foot {
    let foot = Foot(use_signal(|| None));
    let host = use_hook(use_document_host);
    let mut last = use_hook(|| CopyValue::new(None::<CardKey>));
    if last.peek().as_ref() != Some(&key) {
        last.set(Some(key.clone()));
        match key {
            CardKey::Hidden
            | CardKey::Shown {
                actions: ActionsShown::Open,
                ..
            } => link.set_hit(PanelHit::Whole),
            CardKey::Shown { .. } => {
                spawn(publish(foot, Rc::clone(&host), link.clone()));
            }
        }
    }
    follow_hit(link);
    foot
}

/// Read the foot a frame from now (and again while the document is busy or not laid out), then
/// publish the card's part.
async fn publish(foot: Foot, host: Rc<dyn DocumentHost>, link: LauncherLink) {
    for _ in 0..ATTEMPTS {
        sleep(FRAME_SLACK).await;
        let Some(element) = foot.0.peek().clone() else {
            continue;
        };
        match host.geometry().measure(&element) {
            Measured::At(rect) if rect.origin.y.0 >= 1.0 => {
                link.set_hit(hit_for(rect.origin.y.0, ActionsShown::Closed));
                return;
            }
            Measured::At(_) | Measured::Busy => {}
            Measured::Unknown => return,
        }
    }
}

/// Keep this surface's input region on what the panel takes.
fn follow_hit(link: &LauncherLink) {
    let hit = use_watch(link.subscribe_hit());
    follow_input(panel_input(hit()));
}

/// Keep this surface's input region on `region`, sending it only when it changes. Nothing to
/// do without a host (a test, an SSR render).
pub(crate) fn follow_input(region: RegionSpec) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut last = use_hook(|| CopyValue::new(None::<RegionSpec>));
    if last.peek().as_ref() == Some(&region) {
        return;
    }
    if let Some(surface) = surface {
        surface.set_input_region(region.clone());
    }
    last.set(Some(region));
}
