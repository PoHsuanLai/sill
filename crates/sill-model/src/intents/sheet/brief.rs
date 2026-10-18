//! The brief sheet's data: the text a helper would work from, the spans the router flagged in it,
//! and the pieces it falls into. The router writes the flags with a plain word
//! list, never a model, so a page cannot talk them out of showing; sill only draws them.

/// How many bytes a brief may be: the router refuses a longer answer, so the sheet does too.
pub const BRIEF_MAX_BYTES: usize = 4000;

/// What a flagged span is, each with its plain sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BriefKind {
    Link,
    Address,
    Path,
    Order,
    Credentials,
}

impl BriefKind {
    /// Why the span is listed, in the person's words.
    pub fn sentence(self) -> &'static str {
        match self {
            BriefKind::Link => "This opens something outside.",
            BriefKind::Address => "This sends something outside.",
            BriefKind::Path => "This names a file on this computer.",
            BriefKind::Order => "This tells helpers to do something.",
            BriefKind::Credentials => "This mentions a password or a key.",
        }
    }
}

/// A run of characters (not bytes) of the text, `start` included and `end` excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BriefSpan {
    pub start: usize,
    pub end: usize,
}

/// One flagged span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BriefFlag {
    pub span: BriefSpan,
    pub kind: BriefKind,
}

/// The brief as the sheet draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefView {
    /// The planner's words, to be edited.
    pub text: String,
    pub flags: Vec<BriefFlag>,
    /// The pieces of work, read-only, one per line.
    pub pieces: Vec<String>,
}
