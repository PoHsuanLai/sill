//! The framing: one JSON value per line, `\n`-terminated, no embedded newline (JSON escapes
//! every newline inside a string, so a well-formed message never contains a raw one).

use std::io::{BufRead, Read, Write};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::IpcError;

/// The longest line either side accepts, newline included. Requests are tiny; a stats reply
/// for a few dozen surfaces is a few KiB.
pub const MAX_LINE: usize = 64 * 1024;

/// `message` as one line, trailing newline included.
pub fn encode<T: Serialize>(message: &T) -> Result<String, IpcError> {
    let mut line = serde_json::to_string(message)?;
    line.push('\n');
    Ok(line)
}

/// The one message `line` holds. Surrounding whitespace (the newline) is ignored; anything else
/// that is not exactly one `T` is `Malformed`.
pub fn decode<T: DeserializeOwned>(line: &str) -> Result<T, IpcError> {
    if line.len() > MAX_LINE {
        return Err(IpcError::TooLong { limit: MAX_LINE });
    }
    serde_json::from_str(line.trim_end_matches(['\n', '\r'])).map_err(IpcError::Malformed)
}

/// Read one line from `reader` and decode it. Reads at most [`MAX_LINE`] bytes.
pub fn read_message<T: DeserializeOwned>(reader: &mut impl BufRead) -> Result<T, IpcError> {
    let mut line = String::new();
    let limit = u64::try_from(MAX_LINE + 1).unwrap_or(u64::MAX);
    let read = <&mut _ as Read>::take(reader, limit).read_line(&mut line)?;
    if read == 0 {
        return Err(IpcError::Closed);
    }
    if read > MAX_LINE {
        return Err(IpcError::TooLong { limit: MAX_LINE });
    }
    decode(&line)
}

/// Encode `message` and write it as one line, then flush.
pub fn write_message<T: Serialize>(writer: &mut impl Write, message: &T) -> Result<(), IpcError> {
    writer.write_all(encode(message)?.as_bytes())?;
    writer.flush()?;
    Ok(())
}
