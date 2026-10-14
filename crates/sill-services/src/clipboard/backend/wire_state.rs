//! The live loop's state and what it does on each protocol event: follow `selection` (never
//! `primary_selection`), ask [`offer_verdict`] before receiving anything, read the chosen type
//! through a pipe, and offer an entry back through a source. It says what happened to the
//! service's task as [`ClipEvent`]s; the only things it prints are fault kinds.

use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::os::fd::{AsFd, OwnedFd};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sill_launcher::UnixTime;
use sill_model::clipboard::{
    ClipContent, ClipEvent, ImageCap, ImagesKept, MimePick, OfferVerdict, Skipped, offer_verdict,
};
use smithay_client_toolkit::reexports::calloop::{LoopHandle, LoopSignal};
use wayland_client::backend::ObjectId;
use wayland_client::{Connection, QueueHandle};

use super::content::{content_of, limit_for};
use super::pipes::{ReadEnd, ReadId, ReadOwner, Reading, read_whole, write_whole};
use super::wire::Tell;
use super::wl::{Device, Manager, Offer, Source};

/// The types an offered-back text is announced under (what readers ask for).
const TEXT_OFFERS: &[&str] = &[
    "text/plain;charset=utf-8",
    "text/plain",
    "UTF8_STRING",
    "STRING",
    "TEXT",
];

/// The manager and the first seat's device, when the compositor has one of the protocols.
#[derive(Debug)]
pub(crate) enum Binding {
    Bound { manager: Manager, device: Device },
    Unsupported,
}

/// An entry the service is offering as the selection, with the bytes a paste receives.
#[derive(Debug)]
struct Owned {
    source: Source,
    bytes: Arc<[u8]>,
}

/// Everything the loop's callbacks reach.
pub(crate) struct WireState {
    pub(crate) conn: Connection,
    pub(crate) qh: QueueHandle<WireState>,
    pub(crate) handle: LoopHandle<'static, WireState>,
    pub(crate) signal: LoopSignal,
    pub(crate) binding: Binding,
    pub(crate) images: ImageCap,
    pub(crate) tell: Tell,
    /// The types each live offer announced, until it is chosen or dropped.
    offers: HashMap<ObjectId, Vec<String>>,
    reading: Option<(Reading, MimePick)>,
    reads: u64,
    owned: Option<Owned>,
}

impl WireState {
    pub(crate) fn new(
        conn: Connection,
        qh: QueueHandle<WireState>,
        handle: LoopHandle<'static, WireState>,
        signal: LoopSignal,
        binding: Binding,
        images: ImageCap,
        tell: Tell,
    ) -> WireState {
        WireState {
            conn,
            qh,
            handle,
            signal,
            binding,
            images,
            tell,
            offers: HashMap::new(),
            reading: None,
            reads: 0,
            owned: None,
        }
    }

    /// A new offer object; its types follow.
    pub(crate) fn offered(&mut self, offer: &Offer) {
        self.offers.insert(offer.id(), Vec::new());
    }

    pub(crate) fn offer_type(&mut self, offer: &ObjectId, mime: String) {
        if let Some(types) = self.offers.get_mut(offer) {
            types.push(mime);
        }
    }

    /// The selection changed: decide on its types, and read it only when it may be kept.
    pub(crate) fn selection(&mut self, offer: Option<Offer>) {
        self.cancel_read();
        let Some(offer) = offer else { return };
        let types = self.offers.remove(&offer.id()).unwrap_or_default();
        let kept = match self.images {
            ImageCap(0) => ImagesKept::Skipped,
            ImageCap(_) => ImagesKept::Kept,
        };
        match offer_verdict(&types, kept) {
            OfferVerdict::Secret => self.tell.say(ClipEvent::Skipped(Skipped::Secret)),
            OfferVerdict::Nothing => self.tell.say(ClipEvent::Skipped(Skipped::NoKnownType)),
            OfferVerdict::Read(pick) => self.start_read(&offer, pick),
        }
        offer.destroy();
    }

    /// The primary selection is never followed: its offer is dropped unread.
    pub(crate) fn primary_selection(&mut self, offer: Option<Offer>) {
        if let Some(offer) = offer {
            self.offers.remove(&offer.id());
            offer.destroy();
        }
    }

    /// The seat's device is gone: stop, and the thread says it disconnected.
    pub(crate) fn finished(&mut self) {
        self.signal.stop();
    }

    fn start_read(&mut self, offer: &Offer, pick: MimePick) {
        self.reads += 1;
        let id = ReadId(self.reads);
        let limit = limit_for(&pick, self.images);
        let started = std::io::pipe()
            .map_err(|error| error.kind())
            .and_then(|(reader, writer)| {
                let mime = match &pick {
                    MimePick::Text(mime) | MimePick::Image(mime) => mime.clone(),
                };
                offer.receive(mime, writer.as_fd());
                drop(writer);
                self.conn.flush().map_err(|_| ErrorKind::BrokenPipe)?;
                read_whole(&self.handle, File::from(OwnedFd::from(reader)), id, limit)
            });
        match started {
            Ok(reading) => self.reading = Some((reading, pick)),
            Err(kind) => self.fault("could not start reading a selection", kind),
        }
    }

    fn cancel_read(&mut self) {
        if let Some((reading, _)) = self.reading.take() {
            self.handle.remove(reading.pipe);
            self.handle.remove(reading.deadline);
        }
    }

    /// Offer `content` as the selection (an entry chosen in the launcher).
    pub(crate) fn offer_back(&mut self, content: ClipContent) {
        let Binding::Bound { manager, device } = &self.binding else {
            return;
        };
        let source = manager.source(&self.qh);
        let bytes: Arc<[u8]> = match &content {
            ClipContent::Text(text) => {
                TEXT_OFFERS.iter().for_each(|mime| source.offer(mime));
                Arc::from(text.0.as_bytes())
            }
            ClipContent::Image(image) => {
                source.offer(&image.mime.0);
                image.bytes.clone()
            }
        };
        device.set_selection(&source);
        if let Some(old) = self.owned.replace(Owned { source, bytes }) {
            old.source.destroy();
        }
    }

    /// Someone pastes the entry the service offers.
    pub(crate) fn send(&mut self, source: &ObjectId, fd: OwnedFd) {
        let Some(owned) = self.owned.as_ref().filter(|o| o.source.id() == *source) else {
            return;
        };
        if let Err(kind) = write_whole(&self.handle, fd, owned.bytes.clone()) {
            self.fault("could not offer an entry", kind);
        }
    }

    /// Another client took the selection.
    pub(crate) fn cancelled(&mut self, source: &Source) {
        if self
            .owned
            .as_ref()
            .is_some_and(|o| o.source.id() == source.id())
        {
            self.owned = None;
        }
        source.destroy();
    }

    /// A fault, by kind only (never what was being read or written).
    fn fault(&self, what: &str, kind: ErrorKind) {
        eprintln!("sill: clipboard: {what}: {kind}");
        self.tell.say(ClipEvent::Skipped(Skipped::Empty));
    }
}

impl ReadOwner for WireState {
    fn read_ended(&mut self, id: ReadId, end: ReadEnd) {
        let Some((reading, pick)) = self.reading.take_if(|(r, _)| r.id == id) else {
            return;
        };
        self.handle.remove(reading.deadline);
        let event = match end {
            ReadEnd::Whole(bytes) => match content_of(&pick, bytes) {
                Ok(content) => ClipEvent::Copied { content, at: now() },
                Err(skipped) => ClipEvent::Skipped(skipped),
            },
            ReadEnd::OverLimit => ClipEvent::Skipped(Skipped::TooLarge),
            ReadEnd::Broken(kind) => return self.fault("a selection's pipe failed", kind),
        };
        self.tell.say(event);
    }

    fn read_timed_out(&mut self, id: ReadId) {
        if let Some((reading, _)) = self.reading.take_if(|(r, _)| r.id == id) {
            self.handle.remove(reading.pipe);
            self.fault("a selection was not written in time", ErrorKind::TimedOut);
        }
    }
}

/// The wall clock, in whole seconds; a clock before 1970 reads as 1970.
fn now() -> UnixTime {
    UnixTime(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    )
}
