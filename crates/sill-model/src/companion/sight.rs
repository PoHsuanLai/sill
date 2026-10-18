//! Whether the model that works apps can see the screen, as cuad says beside computer use.

/// The most characters of a note that are kept.
const NOTE_LIMIT: usize = 240;

/// The sentence cuad says about the screen model: never empty, at most [`NOTE_LIMIT`] characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteText(String);

impl NoteText {
    /// The note `text` says; `None` when it says nothing.
    pub fn new(text: &str) -> Option<NoteText> {
        let kept: String = text.trim().chars().take(NOTE_LIMIT).collect();
        (!kept.is_empty()).then_some(NoteText(kept))
    }

    /// The words.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether the screen can be seen. It stays `Sees` while cuad is away: nothing is warned about
/// what is not known.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SightState {
    /// A model that reads images serves computer use, or cuad is not there to say.
    #[default]
    Sees,
    /// None does, and this is what to tell the person.
    CannotSee(NoteText),
}

impl SightState {
    /// The state cuad's `SightNote` property names: empty is `Sees`.
    pub fn of(note: &str) -> SightState {
        NoteText::new(note).map_or(SightState::Sees, SightState::CannotSee)
    }
}
