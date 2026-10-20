//! What the input source service can be told.

/// What the input source service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputCommand {
    /// Switch to the next input source of the running input-method framework.
    Next,
}
