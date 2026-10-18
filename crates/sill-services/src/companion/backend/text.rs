//! The small conversions every call of the backend makes: JSON bodies.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::call_error::CallError;

/// A value as the JSON a body carries.
pub(super) fn to_json<T: Serialize>(value: &T) -> Result<String, CallError> {
    Ok(serde_json::to_string(value)?)
}

/// A body read as `T`.
pub(super) fn from_json<T: DeserializeOwned>(text: &str) -> Result<T, CallError> {
    Ok(serde_json::from_str(text)?)
}
