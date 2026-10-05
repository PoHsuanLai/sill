//! The small conversions every call of the backend makes: JSON bodies and error text.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// A value as the JSON a body carries.
pub(super) fn to_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}

/// A body read as `T`.
pub(super) fn from_json<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|error| error.to_string())
}

/// Any error as its text.
pub(super) fn text<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}
