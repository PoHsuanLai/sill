//! `Notify`'s `a{sv}` hints read into [`Hints`] and the rest of the call into a [`Posting`]
//! (`dbus.rs`'s `posting_from_wire` contract). A hint of the wrong type is as if absent.

use std::collections::HashMap;
use std::path::PathBuf;

use zbus::zvariant::{OwnedValue, Value};

use super::content::{
    AppRef, Expiry, Hints, IconRef, NoteImage, NotifyUrgency, Pixels, Posting, Residency,
    SoundHint, Transience, pair_actions,
};
use super::dbus::WireNotify;
use super::ids::NotificationId;
use super::markup::parse_body;

type HintMap = HashMap<String, OwnedValue>;

/// The posting `call` stands for.
pub(super) fn posting(call: WireNotify) -> Posting {
    let hints = &call.hints;
    Posting {
        replaces: NotificationId::replacing(call.replaces_id),
        app: AppRef {
            name: call.app_name,
            icon: IconRef::from_wire(&call.app_icon),
            desktop_entry: text(hints, "desktop-entry").map(|entry| {
                entry
                    .strip_suffix(".desktop")
                    .map_or(entry.clone(), str::to_owned)
            }),
        },
        summary: call.summary,
        body: parse_body(&call.body),
        actions: pair_actions(&call.actions),
        hints: hints_of(hints),
        expiry: Expiry::from_wire(call.expire_timeout),
    }
}

fn hints_of(hints: &HintMap) -> Hints {
    Hints {
        urgency: match value(hints, "urgency") {
            Some(Value::U8(byte)) => NotifyUrgency::from_wire(*byte),
            _ => NotifyUrgency::Normal,
        },
        category: text(hints, "category"),
        image: image(hints),
        transience: match flag(hints, "transient") {
            Flag::Set => Transience::Transient,
            Flag::Unset => Transience::Kept,
        },
        residency: match flag(hints, "resident") {
            Flag::Set => Residency::Resident,
            Flag::Unset => Residency::ClosesOnAction,
        },
        sound: sound(hints),
    }
}

/// A boolean hint, as sent or as a non-zero number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flag {
    Set,
    Unset,
}

fn flag(hints: &HintMap, key: &str) -> Flag {
    let set = match value(hints, key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::U8(n)) => *n != 0,
        Some(Value::I16(n)) => *n != 0,
        Some(Value::U16(n)) => *n != 0,
        Some(Value::I32(n)) => *n != 0,
        Some(Value::U32(n)) => *n != 0,
        Some(Value::I64(n)) => *n != 0,
        Some(Value::U64(n)) => *n != 0,
        _ => false,
    };
    if set { Flag::Set } else { Flag::Unset }
}

fn sound(hints: &HintMap) -> SoundHint {
    if flag(hints, "suppress-sound") == Flag::Set {
        return SoundHint::Suppressed;
    }
    match (text(hints, "sound-file"), text(hints, "sound-name")) {
        (Some(file), _) if !file.is_empty() => {
            SoundHint::File(PathBuf::from(file.strip_prefix("file://").unwrap_or(&file)))
        }
        (_, Some(name)) if !name.is_empty() => SoundHint::Named(name),
        _ => SoundHint::Default,
    }
}

/// `image-data` (or its two older names) as pixels, else `image-path`.
fn image(hints: &HintMap) -> NoteImage {
    let data = ["image-data", "image_data", "icon_data"]
        .into_iter()
        .find_map(|key| value(hints, key));
    match data {
        Some(data) => pixels(data).map_or(NoteImage::None, NoteImage::Pixels),
        None => match text(hints, "image-path").or_else(|| text(hints, "image_path")) {
            Some(path) => match IconRef::from_wire(&path) {
                IconRef::None => NoteImage::None,
                icon => NoteImage::Icon(icon),
            },
            None => NoteImage::None,
        },
    }
}

/// `(iiibiiay)`: width, height, rowstride, has alpha, bits per sample, channels, data; 8-bit
/// RGB or RGBA only. `None` for anything malformed.
fn pixels(data: &Value<'_>) -> Option<Pixels> {
    let Value::Structure(fields) = data else {
        return None;
    };
    let [
        Value::I32(width),
        Value::I32(height),
        Value::I32(rowstride),
        Value::Bool(alpha),
        Value::I32(8),
        Value::I32(channels),
        Value::Array(bytes),
    ] = fields.fields()
    else {
        return None;
    };
    let bytes: Vec<u8> = bytes
        .inner()
        .iter()
        .map(|b| match b {
            Value::U8(b) => Some(*b),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let shape = Shape {
        width: usize::try_from(*width).ok()?,
        height: usize::try_from(*height).ok()?,
        rowstride: usize::try_from(*rowstride).ok()?,
        channels: usize::try_from(*channels).ok()?,
        alpha: if *alpha { Alpha::Has } else { Alpha::None },
    };
    let rgba = shape.rgba(&bytes)?;
    Some(Pixels {
        width: u32::try_from(shape.width).ok()?,
        height: u32::try_from(shape.height).ok()?,
        rgba,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alpha {
    Has,
    None,
}

/// How `image-data`'s bytes are laid out.
#[derive(Debug, Clone, Copy)]
struct Shape {
    width: usize,
    height: usize,
    rowstride: usize,
    channels: usize,
    alpha: Alpha,
}

impl Shape {
    /// The bytes as straight RGBA, rows packed; `None` when they are too few or the channels
    /// do not match.
    fn rgba(self, bytes: &[u8]) -> Option<Vec<u8>> {
        let wanted = match self.alpha {
            Alpha::Has => 4,
            Alpha::None => 3,
        };
        if self.channels != wanted || self.width == 0 || self.height == 0 {
            return None;
        }
        let row = self.width.checked_mul(self.channels)?;
        let last = self
            .rowstride
            .checked_mul(self.height - 1)?
            .checked_add(row)?;
        if self.rowstride < row || bytes.len() < last {
            return None;
        }
        let mut out = Vec::with_capacity(self.width * self.height * 4);
        for y in 0..self.height {
            let start = y * self.rowstride;
            for pixel in bytes[start..start + row].chunks_exact(self.channels) {
                out.extend_from_slice(&pixel[..3]);
                out.push(pixel.get(3).copied().unwrap_or(u8::MAX));
            }
        }
        Some(out)
    }
}

/// A hint's value, a nested variant unwrapped.
fn value<'h>(hints: &'h HintMap, key: &str) -> Option<&'h Value<'static>> {
    let mut value: &Value<'static> = hints.get(key)?;
    while let Value::Value(inner) = value {
        value = inner;
    }
    Some(value)
}

fn text(hints: &HintMap, key: &str) -> Option<String> {
    match value(hints, key)? {
        Value::Str(s) => Some(s.as_str().to_owned()),
        Value::ObjectPath(p) => Some(p.as_str().to_owned()),
        _ => None,
    }
}
