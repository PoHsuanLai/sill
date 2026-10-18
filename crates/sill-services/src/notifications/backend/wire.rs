//! `Notify`'s `a{sv}` hints read into [`Hints`] and the rest of the call into a [`Posting`]
//! (`dbus.rs`'s `posting_from_wire` contract). A hint of the wrong type is as if absent. Every
//! field is cut to its ceiling (`sill_model::notifications::limits`) on the way in, so nothing
//! downstream holds more than a card has use for; a call with more hints than any sender has a
//! reason to send is refused.

use std::collections::HashMap;
use std::path::PathBuf;

use sill_model::notifications::{
    ACTION_KEY_MAX_BYTES, ACTION_LABEL_MAX_BYTES, ACTIONS_MAX, APP_NAME_MAX_BYTES, Action,
    ActionKey, AppRef, Expiry, HINT_PATH_MAX_BYTES, HINT_TEXT_MAX_BYTES, HINTS_MAX, Hints, IconRef,
    NoteImage, NotificationId, NotifyUrgency, Posting, Residency, SUMMARY_MAX_BYTES, SoundHint,
    Transience, app_entry, pair_actions, parse_body,
};
use sill_model::untrusted::{clip_bytes, cut_bytes};
use zbus::zvariant::{OwnedValue, Value};

use super::dbus::{WireNotify, WireRefusal};
use super::picture::pixels;

type HintMap = HashMap<String, OwnedValue>;

/// The posting `call` stands for, or why not.
pub(crate) fn posting(call: WireNotify) -> Result<Posting, WireRefusal> {
    if call.hints.len() > HINTS_MAX {
        return Err(WireRefusal::TooManyHints);
    }
    let hints = &call.hints;
    Ok(Posting {
        replaces: NotificationId::replacing(call.replaces_id),
        app: AppRef {
            name: clip_bytes(&call.app_name, APP_NAME_MAX_BYTES),
            icon: IconRef::from_wire(cut_bytes(&call.app_icon, HINT_PATH_MAX_BYTES)),
            desktop_entry: text(hints, "desktop-entry")
                .map(|entry| {
                    entry
                        .strip_suffix(".desktop")
                        .map_or_else(|| entry.clone(), str::to_owned)
                })
                .and_then(app_entry),
        },
        summary: clip_bytes(&call.summary, SUMMARY_MAX_BYTES),
        body: parse_body(&call.body),
        actions: actions_of(&call.actions),
        hints: hints_of(hints),
        expiry: Expiry::from_wire(call.expire_timeout),
    })
}

/// The first [`ACTIONS_MAX`] key and label pairs, each cut to its ceiling.
fn actions_of(flat: &[String]) -> Vec<Action> {
    let kept = &flat[..flat.len().min(ACTIONS_MAX * 2)];
    pair_actions(kept)
        .into_iter()
        .map(|action| Action {
            key: ActionKey(cut_bytes(&action.key.0, ACTION_KEY_MAX_BYTES).to_owned()),
            label: clip_bytes(&action.label, ACTION_LABEL_MAX_BYTES),
        })
        .collect()
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
    match (path_text(hints, "sound-file"), text(hints, "sound-name")) {
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
        None => match path_text(hints, "image-path").or_else(|| path_text(hints, "image_path")) {
            Some(path) => match IconRef::from_wire(&path) {
                IconRef::None => NoteImage::None,
                icon => NoteImage::Icon(icon),
            },
            None => NoteImage::None,
        },
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

/// A word-like hint (`category`, `desktop-entry`, `sound-name`), cut to [`HINT_TEXT_MAX_BYTES`].
fn text(hints: &HintMap, key: &str) -> Option<String> {
    string_up_to(hints, key, HINT_TEXT_MAX_BYTES)
}

/// A path-like hint (`image-path`, `sound-file`), cut to [`HINT_PATH_MAX_BYTES`].
fn path_text(hints: &HintMap, key: &str) -> Option<String> {
    string_up_to(hints, key, HINT_PATH_MAX_BYTES)
}

fn string_up_to(hints: &HintMap, key: &str, max: usize) -> Option<String> {
    let whole = match value(hints, key)? {
        Value::Str(s) => s.as_str(),
        Value::ObjectPath(p) => p.as_str(),
        _ => return None,
    };
    Some(cut_bytes(whole, max).to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use sill_model::notifications::{
        ACTION_LABEL_MAX_BYTES, ACTIONS_MAX, BODY_MAX_BYTES, HINTS_MAX, IMAGE_MAX_SIDE, NoteImage,
        SUMMARY_MAX_BYTES,
    };
    use zbus::zvariant::{OwnedValue, Value};

    use super::posting;
    use crate::notifications::backend::dbus::{WireNotify, WireRefusal};

    fn owned(value: Value<'_>) -> OwnedValue {
        value.try_into_owned().unwrap_or_else(|e| panic!("{e}"))
    }

    fn call() -> WireNotify {
        WireNotify {
            app_name: "Mail".to_owned(),
            replaces_id: 0,
            app_icon: String::new(),
            summary: "Ada".to_owned(),
            body: "hello".to_owned(),
            actions: Vec::new(),
            hints: HashMap::new(),
            expire_timeout: -1,
        }
    }

    fn rgba(side: i32, bytes: usize) -> OwnedValue {
        owned(Value::from((
            side,
            side,
            side * 4,
            true,
            8i32,
            4i32,
            vec![7u8; bytes],
        )))
    }

    #[test]
    fn text_is_cut_to_its_ceiling() {
        let long = "x".repeat(1 << 20);
        let mut wire = call();
        wire.app_name = long.clone();
        wire.summary = long.clone();
        wire.body = long.clone();
        wire.actions = ["k".repeat(5000), long.clone()].to_vec();
        wire.hints
            .insert("category".to_owned(), owned(Value::from(long.as_str())));
        let posting = posting(wire).unwrap_or_else(|e| panic!("{e:?}"));
        assert!(
            posting.app.name.len() <= 128 + 3,
            "{}",
            posting.app.name.len()
        );
        assert!(posting.summary.len() <= SUMMARY_MAX_BYTES + 3);
        assert!(posting.body.text().len() <= BODY_MAX_BYTES);
        assert!(posting.hints.category.is_some_and(|c| c.len() <= 256));
        assert_eq!(posting.actions.len(), 1);
        assert!(posting.actions[0].key.0.len() <= 128);
        assert!(posting.actions[0].label.len() <= ACTION_LABEL_MAX_BYTES + 3);
    }

    #[test]
    fn only_the_first_actions_are_kept() {
        let mut wire = call();
        wire.actions = (0..ACTIONS_MAX * 5).map(|n| n.to_string()).collect();
        let posting = posting(wire).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(posting.actions.len(), ACTIONS_MAX);
    }

    #[test]
    fn a_call_with_a_flood_of_hints_is_refused() {
        let mut wire = call();
        for n in 0..=HINTS_MAX {
            wire.hints
                .insert(format!("x-{n}"), owned(Value::from(1u32)));
        }
        assert_eq!(posting(wire).err(), Some(WireRefusal::TooManyHints));
        let mut ok = call();
        for n in 0..HINTS_MAX {
            ok.hints.insert(format!("x-{n}"), owned(Value::from(1u32)));
        }
        assert!(posting(ok).is_ok(), "the ceiling itself passes");
    }

    #[test]
    fn an_image_past_the_ceiling_is_no_image() {
        let side = i32::try_from(IMAGE_MAX_SIDE).unwrap_or(0);
        let cases: Vec<(&str, OwnedValue, bool)> = vec![
            (
                "at the ceiling",
                rgba(side, (side * side * 4) as usize),
                true,
            ),
            (
                "one pixel over",
                rgba(side + 1, ((side + 1) * (side + 1) * 4) as usize),
                false,
            ),
            ("a huge array", rgba(8, 4 << 20), false),
        ];
        for (name, image, kept) in cases {
            let mut wire = call();
            wire.hints.insert("image-data".to_owned(), image);
            let posting = posting(wire).unwrap_or_else(|e| panic!("{e:?}"));
            let has = matches!(posting.hints.image, NoteImage::Pixels(_));
            assert_eq!(has, kept, "{name}");
        }
    }

    #[test]
    fn an_app_cannot_wear_the_shells_desktop_entry() {
        let mut wire = call();
        wire.hints.insert(
            "desktop-entry".to_owned(),
            owned(Value::from("sill-accounts")),
        );
        let posting = posting(wire).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(posting.app.desktop_entry, None);
    }
}
