//! The pictures on the Spaces bar's thumbnails: what has arrived, and when to ask for another.
//! Pure; the daemon carries out the requests [`thumbs_wanted`] returns.

use std::collections::BTreeMap;
use std::sync::Arc;

use casement_protocols::vocab::{Shown, WindowStateKind};
use porter_core::SpaceId;

use super::bar_layout::BarLayouts;
use super::{DesktopView, SpacePlaceView};

/// What a request for a picture of a Space came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpacePicture {
    /// The picture, as PNG bytes.
    Png(Vec<u8>),
    /// The compositor could not make one.
    Failed,
}

/// The latest answer for one Space of one output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarPicture {
    /// Counts up from 1 with each answer, a failed one too.
    pub seq: u64,
    /// The last picture that arrived; a failure leaves it as it was.
    pub png: Option<Arc<[u8]>>,
}

/// Every picture that has arrived.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarPictures(BTreeMap<(String, SpaceId), BarPicture>);

impl BarPictures {
    /// The same pictures with `answer` for `space` of `output` taken in.
    pub fn with(mut self, output: &str, space: SpaceId, answer: &SpacePicture) -> BarPictures {
        let key = (output.to_owned(), space);
        let before = self.0.get(&key);
        let seq = before.map_or(1, |picture| picture.seq + 1);
        let png = match answer {
            SpacePicture::Png(bytes) => Some(Arc::from(bytes.as_slice())),
            SpacePicture::Failed => before.and_then(|picture| picture.png.clone()),
        };
        self.0.insert(key, BarPicture { seq, png });
        self
    }

    /// How many answers `space` of `output` has had.
    pub fn seq_of(&self, output: &str, space: &SpaceId) -> u64 {
        self.get(output, space).map_or(0, |picture| picture.seq)
    }

    /// The picture to show for `space` of `output`, if one has arrived.
    pub fn png_of(&self, output: &str, space: &SpaceId) -> Option<Arc<[u8]>> {
        self.get(output, space)
            .and_then(|picture| picture.png.clone())
    }

    fn get(&self, output: &str, space: &SpaceId) -> Option<&BarPicture> {
        self.0.get(&(output.to_owned(), space.clone()))
    }
}

/// A picture of a Space to ask the compositor for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbRequest {
    /// The output.
    pub output: String,
    /// The Space it shows now.
    pub space: SpaceId,
    /// The picture's size in device pixels.
    pub size: (u32, u32),
}

/// What about a window decides how the Space looks.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowMark {
    identifier: String,
    state: WindowStateKind,
    geometry: Option<(i32, i32, u32, u32)>,
}

/// What was asked for on one output and what the Space looked like then.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    space: SpaceId,
    looked: Vec<WindowMark>,
    /// How many answers the Space had when asked: a request is out until there are more.
    seq: u64,
}

/// The requests in flight or answered, by output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThumbAsks(BTreeMap<String, Ask>);

/// The Space `output` shows now.
fn shown_on(view: &DesktopView, output: &str) -> Option<SpaceId> {
    view.spaces
        .iter()
        .find(|space| {
            space.shown == Shown::Shown && (space.output.is_empty() || space.output == output)
        })
        .map(|space| space.id.clone())
}

/// The marks of the windows `space` shows.
fn looks_of(view: &DesktopView, space: &SpaceId) -> Vec<WindowMark> {
    view.windows
        .iter()
        .filter(|window| match &window.space {
            SpacePlaceView::On(on) => on == space,
            SpacePlaceView::AllSpaces => true,
            SpacePlaceView::Unknown => false,
        })
        .map(|window| WindowMark {
            identifier: window.identifier.clone(),
            state: window.state,
            geometry: window.geometry,
        })
        .collect()
}

/// The pictures to ask for now and the asks that follow. Each laid-out bar asks for the Space its
/// output shows (the compositor pictures only that one), once when the bar comes up, and again
/// only after the last answer came and the Space changed or looks different: a held request is the
/// compositor's damage gate, and asking in a loop for a picture that did not change would spin. A
/// bar that went away forgets its ask, so the next opening asks again.
pub fn thumbs_wanted(
    asks: &ThumbAsks,
    layouts: &BarLayouts,
    view: &DesktopView,
    pictures: &BarPictures,
) -> (ThumbAsks, Vec<ThumbRequest>) {
    let mut next = BTreeMap::new();
    let mut requests = Vec::new();
    for (output, layout) in &layouts.0 {
        let Some(space) = shown_on(view, output) else {
            continue;
        };
        let looked = looks_of(view, &space);
        let ask = asks.0.get(output);
        let due = ask.is_none_or(|ask| {
            let answered = pictures.seq_of(output, &ask.space) > ask.seq;
            answered && (ask.space != space || ask.looked != looked)
        });
        match (due, ask) {
            (false, Some(ask)) => {
                next.insert(output.clone(), ask.clone());
            }
            _ => {
                next.insert(
                    output.clone(),
                    Ask {
                        seq: pictures.seq_of(output, &space),
                        space: space.clone(),
                        looked,
                    },
                );
                requests.push(ThumbRequest {
                    output: output.clone(),
                    space,
                    size: layout.thumb,
                });
            }
        }
    }
    (ThumbAsks(next), requests)
}
