//! What can go wrong reading or writing one message.

/// A message that could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    /// The stream failed.
    #[error("ipc stream: {0}")]
    Io(#[from] std::io::Error),
    /// The peer closed the stream before a whole line arrived.
    #[error("ipc stream closed before a message arrived")]
    Closed,
    /// A line longer than [`crate::MAX_LINE`]: refused before it is parsed.
    #[error("ipc line longer than {limit} bytes")]
    TooLong { limit: usize },
    /// A line that is not exactly one message of the expected type.
    #[error("malformed ipc line: {0}")]
    Malformed(#[from] serde_json::Error),
}
