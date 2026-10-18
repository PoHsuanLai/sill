//! What a call from a backend to another process can fail at: the bus, a JSON body, a value the
//! peer sent that is not what the contract names, or a peer that is not there.

/// A bus call that did not give an answer.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CallError {
    /// The bus refused the call or the peer did not answer.
    #[error("{0}")]
    Bus(#[from] zbus::Error),
    /// The intents router did not give the reply the request wanted.
    #[error("{0}")]
    Router(#[from] docket_client::ClientError),
    /// A service ran out of time or is gone.
    #[error("{0}")]
    Service(#[from] crate::error::ServiceError),
    /// A JSON body would not be written or read.
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    /// A value the contract names (an id, a path, a state) is not well formed.
    #[error("{0}")]
    Invalid(String),
    /// The peer answered and said no, in its own words.
    #[error("{0}")]
    Declined(String),
    /// The peer is not there, or not ready, and the text says so.
    #[error("{0}")]
    Absent(&'static str),
}

impl CallError {
    /// A value that did not parse, with the parser's words.
    pub(crate) fn invalid(error: impl std::fmt::Display) -> Self {
        Self::Invalid(error.to_string())
    }
}
