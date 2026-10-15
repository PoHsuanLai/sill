//! What the shell holds while one colour is asked for: the screens it is waiting on a picture of
//! and the pictures that came. One step per input, pure; the daemon carries out what it asks.

use std::collections::{BTreeMap, BTreeSet};

use super::super::step::AskId;
use crate::desktop::{OutputPicture, OutputShot};

/// The still pictures of the screens for the colour question `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frozen {
    pub id: AskId,
    /// The screens whose picture has not come yet.
    waiting: BTreeSet<String>,
    /// The pictures that came, by the screen's name.
    pub shots: BTreeMap<String, OutputShot>,
}

/// What reaches the step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EyedropIn {
    /// The colour question that is open now and the screens to take a picture of, or none when
    /// no colour is asked.
    Asked(Option<(AskId, Vec<String>)>),
    /// The picture of a screen came, or could not be made.
    Picture(String, OutputPicture),
}

/// What the step asks the daemon to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EyedropOut {
    /// Take a picture of this screen.
    Capture(String),
    /// End the question as cancelled: there is nothing to point at.
    Cancel(AskId),
}

/// `frozen` after `input`, and what to do about it. A question already held is not asked again,
/// a picture nobody waits for is dropped, and a question whose every picture failed is cancelled.
pub fn eyedrop_step(frozen: Option<Frozen>, input: EyedropIn) -> (Option<Frozen>, Vec<EyedropOut>) {
    match (frozen, input) {
        (_, EyedropIn::Asked(None)) => (None, Vec::new()),
        (Some(held), EyedropIn::Asked(Some((id, _)))) if held.id == id => (Some(held), Vec::new()),
        (_, EyedropIn::Asked(Some((id, screens)))) => ask(id, screens),
        (None, EyedropIn::Picture(..)) => (None, Vec::new()),
        (Some(held), EyedropIn::Picture(output, picture)) => arrive(held, output, picture),
    }
}

fn ask(id: AskId, screens: Vec<String>) -> (Option<Frozen>, Vec<EyedropOut>) {
    let waiting: BTreeSet<String> = screens.into_iter().collect();
    let out = match waiting.is_empty() {
        true => vec![EyedropOut::Cancel(id)],
        false => waiting.iter().cloned().map(EyedropOut::Capture).collect(),
    };
    let held = Frozen {
        id,
        waiting,
        shots: BTreeMap::new(),
    };
    (Some(held), out)
}

fn arrive(
    mut held: Frozen,
    output: String,
    picture: OutputPicture,
) -> (Option<Frozen>, Vec<EyedropOut>) {
    if !held.waiting.remove(&output) {
        return (Some(held), Vec::new());
    }
    if let OutputPicture::Shot(shot) = picture {
        held.shots.insert(output, shot);
    }
    let nothing_to_point_at = held.waiting.is_empty() && held.shots.is_empty();
    let out = if nothing_to_point_at {
        vec![EyedropOut::Cancel(held.id)]
    } else {
        Vec::new()
    };
    (Some(held), out)
}
