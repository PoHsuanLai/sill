//! The two files: casement's `settings.toml`, of which only `[night_light]` is touched, and the
//! manual turn-on's `night-shift.json`. Both are parsed once here into the model's types. The
//! person's `settings.toml` is edited in place (every comment, blank line and key order stays,
//! and a link to a dotfiles copy is written through) and written durably with the permissions it
//! had; the manual file is sill's own and private.

use std::io::ErrorKind;
use std::path::Path;

use ds_core::word::Word;
use serde::{Deserialize, Serialize};
use sill_model::ids::OutputName;
use sill_model::night_shift::{Manual, MinuteOfDay, NightLight, Schedule, UnixSeconds, Warmth};
use toml::{Table, Value};
use toml_edit::{DocumentMut, Item, Table as EditTable, TableLike, Value as EditValue};

use crate::durable::{self, Access};

/// casement's settings file, in its config directory.
pub(super) const SETTINGS: &str = "settings.toml";
/// The manual turn-on, in sill's state directory.
const MANUAL: &str = "night-shift.json";

/// Why a file could not be used.
#[derive(Debug, thiserror::Error)]
pub(super) enum FileError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("not a settings table: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("not a settings file: {0}")]
    Edit(#[from] toml_edit::TomlError),
}

/// casement's night light as `dir/settings.toml` says it; the defaults for a file that is not
/// there or a key that is missing or does not mean anything.
pub(super) fn read_light(dir: &Path) -> Result<NightLight, FileError> {
    match std::fs::read_to_string(dir.join(SETTINGS)) {
        Ok(text) => Ok(light_of(toml::from_str::<Table>(&text)?.get("night_light"))),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(NightLight::default()),
        Err(error) => Err(error.into()),
    }
}

fn light_of(section: Option<&Value>) -> NightLight {
    let defaults = NightLight::default();
    let Some(section) = section.and_then(Value::as_table) else {
        return defaults;
    };
    let minute = |key: &str, default: MinuteOfDay| {
        section
            .get(key)
            .and_then(Value::as_integer)
            .and_then(|minutes| u32::try_from(minutes).ok())
            .map_or(default, MinuteOfDay::wrapping)
    };
    NightLight {
        schedule: section
            .get("mode")
            .and_then(Value::as_str)
            .and_then(Schedule::parse)
            .unwrap_or(defaults.schedule),
        from: minute("from", defaults.from),
        to: minute("to", defaults.to),
        warmth: section
            .get("kelvin")
            .and_then(Value::as_integer)
            .and_then(|kelvin| u32::try_from(kelvin).ok())
            .map_or(defaults.warmth, Warmth::clamped),
        fade_minutes: section
            .get("fade_minutes")
            .and_then(Value::as_integer)
            .and_then(|minutes| u16::try_from(minutes).ok())
            .unwrap_or(defaults.fade_minutes),
    }
}

/// Write `light` into `[night_light]` of `dir/settings.toml`, key by key, leaving every other
/// key of the file and of the table, and the file's comments and layout, as they were. Nothing is
/// written when the file already says it. A file that is not valid TOML is not overwritten.
pub(super) fn write_light(dir: &Path, light: &NightLight) -> Result<(), FileError> {
    let path = dir.join(SETTINGS);
    let (before, mut doc) = load_doc(&path)?;
    if let Some(section) = night_light(&mut doc) {
        set(section, "mode", EditValue::from(light.schedule.slug()));
        set(
            section,
            "from",
            EditValue::from(i64::from(light.from.minutes())),
        );
        set(
            section,
            "to",
            EditValue::from(i64::from(light.to.minutes())),
        );
        set(
            section,
            "kelvin",
            EditValue::from(i64::from(light.warmth.kelvin())),
        );
        set(
            section,
            "fade_minutes",
            EditValue::from(i64::from(light.fade_minutes)),
        );
    }
    write_settings(&path, &before, &doc)
}

/// The key casement reads the outputs it leaves alone from.
const SKIP_KEY: &str = "skip_outputs";

/// Whether a write changed the file's `skip_outputs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Skipping {
    Changed,
    AsWas,
}

/// `night_light.skip_outputs` for the monitors `skip` (sill's), among the `existing` entries: an
/// entry that names a display by its identity (`edid:...`, `connector:...`; a connector name
/// has no colon) is the person's and stays, in front; every plain connector name is sill's to
/// replace.
pub(super) fn skip_list(existing: &[String], skip: &[OutputName]) -> Vec<String> {
    existing
        .iter()
        .filter(|entry| entry.trim().contains(':'))
        .cloned()
        .chain(skip.iter().map(|name| name.0.clone()))
        .collect()
}

/// Make `[night_light] skip_outputs` of `dir/settings.toml` list the monitors `skip`, keeping the
/// person's own entries ([`skip_list`]) and every other key, comment and the layout of the file.
/// Nothing is written (so casement's watch is not woken) when the list is already that; an empty
/// list removes the key. A file that is not valid TOML is not overwritten.
pub(super) fn write_skip(dir: &Path, skip: &[OutputName]) -> Result<Skipping, FileError> {
    let path = dir.join(SETTINGS);
    let (before, mut doc) = load_doc(&path)?;
    let Some(section) = night_light(&mut doc) else {
        return Ok(Skipping::AsWas);
    };
    match set_skip(section, skip) {
        Skipping::AsWas => Ok(Skipping::AsWas),
        Skipping::Changed => {
            write_settings(&path, &before, &doc)?;
            Ok(Skipping::Changed)
        }
    }
}

fn set_skip(section: &mut dyn TableLike, skip: &[OutputName]) -> Skipping {
    let existing: Vec<String> = section
        .get(SKIP_KEY)
        .and_then(Item::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_str())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let wanted = skip_list(&existing, skip);
    if wanted == existing {
        return Skipping::AsWas;
    }
    if wanted.is_empty() {
        section.remove(SKIP_KEY);
    } else {
        set(
            section,
            SKIP_KEY,
            EditValue::from(wanted.into_iter().collect::<toml_edit::Array>()),
        );
    }
    Skipping::Changed
}

/// `path` as an editable document, with the text it was read from; an empty one for a file that
/// is not there.
fn load_doc(path: &Path) -> Result<(String, DocumentMut), FileError> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let doc = text.parse::<DocumentMut>()?;
            Ok((text, doc))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            Ok((String::new(), DocumentMut::new()))
        }
        Err(error) => Err(error.into()),
    }
}

/// The `[night_light]` table of `doc`, made (or made a table) first.
fn night_light(doc: &mut DocumentMut) -> Option<&mut dyn TableLike> {
    let item = doc
        .as_table_mut()
        .entry("night_light")
        .or_insert(Item::Table(EditTable::new()));
    if item.as_table_like().is_none() {
        *item = Item::Table(EditTable::new());
    }
    item.as_table_like_mut()
}

/// `section[key] = value`, keeping the comment that sat beside the old value.
fn set(section: &mut dyn TableLike, key: &str, value: EditValue) {
    match section.get_mut(key).and_then(Item::as_value_mut) {
        Some(old) => {
            let decor = old.decor().clone();
            *old = value;
            *old.decor_mut() = decor;
        }
        None => {
            section.insert(key, Item::Value(value));
        }
    }
}

/// Write `doc` over `path` when it differs from `before`, durably, with the file's permissions.
fn write_settings(path: &Path, before: &str, doc: &DocumentMut) -> Result<(), FileError> {
    let after = doc.to_string();
    if after == before {
        return Ok(());
    }
    durable::write(path, after.as_bytes(), Access::AsFound)?;
    Ok(())
}

/// The manual turn-on's file.
#[derive(Debug, Serialize, Deserialize)]
struct ManualFile {
    until: i64,
    restore: LightFile,
}

#[derive(Debug, Serialize, Deserialize)]
struct LightFile {
    mode: String,
    from: u16,
    to: u16,
    kelvin: u16,
    fade_minutes: u16,
}

/// The manual turn-on kept in `dir`; none for a file that is missing or damaged (a damaged
/// record is a forgotten one: casement's own schedule is what remains).
pub(super) fn read_manual(dir: &Path) -> Option<Manual> {
    let text = std::fs::read_to_string(dir.join(MANUAL)).ok()?;
    let file: ManualFile = serde_json::from_str(&text).ok()?;
    Some(Manual {
        until: UnixSeconds(file.until),
        restore: NightLight {
            schedule: Schedule::parse(&file.restore.mode)?,
            from: MinuteOfDay::wrapping(u32::from(file.restore.from)),
            to: MinuteOfDay::wrapping(u32::from(file.restore.to)),
            warmth: Warmth::clamped(u32::from(file.restore.kelvin)),
            fade_minutes: file.restore.fade_minutes,
        },
    })
}

/// Keep `manual` in `dir`, or forget it.
pub(super) fn write_manual(dir: &Path, manual: Option<&Manual>) -> Result<(), FileError> {
    let path = dir.join(MANUAL);
    let Some(manual) = manual else {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        };
    };
    let file = ManualFile {
        until: manual.until.0,
        restore: LightFile {
            mode: manual.restore.schedule.slug().to_owned(),
            from: manual.restore.from.minutes(),
            to: manual.restore.to.minutes(),
            kelvin: manual.restore.warmth.kelvin(),
            fade_minutes: manual.restore.fade_minutes,
        },
    };
    // Plain fields only: nothing here can fail to encode.
    durable::write(
        &path,
        serde_json::to_string_pretty(&file)
            .unwrap_or_default()
            .as_bytes(),
        Access::OwnerOnly,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sill_model::ids::OutputName;
    use sill_model::night_shift::{Manual, MinuteOfDay, NightLight, Schedule, UnixSeconds, Warmth};
    use sill_testkit::TempDir;

    use super::{
        SETTINGS, Skipping, read_light, read_manual, skip_list, write_light, write_manual,
        write_skip,
    };

    fn warm() -> NightLight {
        NightLight {
            schedule: Schedule::Scheduled,
            from: MinuteOfDay::wrapping(23 * 60),
            to: MinuteOfDay::wrapping(6 * 60 + 30),
            warmth: Warmth::clamped(2900),
            fade_minutes: 45,
        }
    }

    #[test]
    fn a_missing_file_is_the_defaults_and_a_written_light_reads_back() {
        let dir = TempDir::new("night-shift-light");
        assert_eq!(
            read_light(dir.path()).unwrap_or_else(|e| panic!("{e}")),
            NightLight::default()
        );
        write_light(dir.path(), &warm()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            read_light(dir.path()).unwrap_or_else(|e| panic!("{e}")),
            warm()
        );
    }

    #[test]
    fn writing_the_light_leaves_the_rest_of_casements_file_alone() {
        let dir = TempDir::new("night-shift-merge");
        std::fs::write(
            dir.path().join(SETTINGS),
            "[pointer]\nnatural_scroll = true\n\n[night_light]\nmode = \"off\"\nfuture_key = 7\n",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        write_light(dir.path(), &warm()).unwrap_or_else(|e| panic!("{e}"));
        let text = std::fs::read_to_string(dir.path().join(SETTINGS)).unwrap_or_default();
        let table: toml::Table = toml::from_str(&text).unwrap_or_default();
        assert_eq!(
            table["pointer"]["natural_scroll"],
            toml::Value::Boolean(true)
        );
        assert_eq!(table["night_light"]["future_key"], toml::Value::Integer(7));
        assert_eq!(
            table["night_light"]["mode"],
            toml::Value::String("scheduled".to_owned())
        );
        assert_eq!(table["night_light"]["from"], toml::Value::Integer(1380));
    }

    #[test]
    fn a_file_that_is_not_toml_is_not_overwritten() {
        let dir = TempDir::new("night-shift-damaged");
        std::fs::write(dir.path().join(SETTINGS), "this = is = not toml").unwrap_or_default();
        assert!(read_light(dir.path()).is_err());
        assert!(write_light(dir.path(), &warm()).is_err());
        let kept = std::fs::read_to_string(dir.path().join(SETTINGS)).unwrap_or_default();
        assert_eq!(kept, "this = is = not toml");
    }

    #[test]
    fn keys_that_mean_nothing_fall_back_one_by_one() {
        let dir = TempDir::new("night-shift-lenient");
        std::fs::write(
            dir.path().join(SETTINGS),
            "[night_light]\nmode = \"dusk\"\nfrom = -5\nto = 1500\nkelvin = 9000\nfade_minutes = \"long\"\n",
        )
        .unwrap_or_default();
        let light = read_light(dir.path()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(light.schedule, Schedule::Off, "an unknown mode");
        assert_eq!(light.from, NightLight::default().from, "a negative time");
        assert_eq!(
            light.to,
            MinuteOfDay::wrapping(60),
            "a time past the day wraps"
        );
        assert_eq!(
            light.warmth,
            Warmth::clamped(6500),
            "kelvin is kept in range"
        );
        assert_eq!(light.fade_minutes, 30, "a fade that is not a number");
    }

    #[test]
    fn the_manual_turn_on_is_kept_and_forgotten() {
        let dir = TempDir::new("night-shift-manual");
        assert_eq!(read_manual(dir.path()), None);
        let manual = Manual {
            until: UnixSeconds(1_792_200_000),
            restore: warm(),
        };
        write_manual(dir.path(), Some(&manual)).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(read_manual(dir.path()), Some(manual));
        write_manual(dir.path(), None).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(read_manual(dir.path()), None);
        write_manual(dir.path(), None).unwrap_or_else(|e| panic!("{e}"));
    }

    fn outputs(names: &[&str]) -> Vec<OutputName> {
        names
            .iter()
            .map(|name| OutputName((*name).to_owned()))
            .collect()
    }

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// A case: its name, the list already in the file, the connectors to skip, the list written.
    type SkipCase<'a> = (&'a str, &'a [&'a str], &'a [&'a str], &'a [&'a str]);

    #[test]
    fn the_skip_list_keeps_the_persons_identities_and_replaces_connector_names() {
        let cases: &[SkipCase<'_>] = &[
            ("nothing either way", &[], &[], &[]),
            (
                "sill's monitors are listed",
                &[],
                &["DP-1", "HDMI-A-1"],
                &["DP-1", "HDMI-A-1"],
            ),
            (
                "last time's connectors are replaced",
                &["DP-1", "DP-2"],
                &["HDMI-A-1"],
                &["HDMI-A-1"],
            ),
            (
                "the person's identities stay in front",
                &["DP-1", "edid:ACME/M1/S1", " connector:DP-9"],
                &["DP-2"],
                &["edid:ACME/M1/S1", " connector:DP-9", "DP-2"],
            ),
            (
                "no monitors of sill's, only the person's",
                &["edid:A/B/C", "DP-1"],
                &[],
                &["edid:A/B/C"],
            ),
        ];
        for (name, existing, skip, want) in cases {
            assert_eq!(
                skip_list(&strings(existing), &outputs(skip)),
                strings(want),
                "{name}"
            );
        }
    }

    fn skip_in(dir: &std::path::Path) -> Option<Vec<String>> {
        let text = std::fs::read_to_string(dir.join(SETTINGS)).unwrap_or_default();
        let table: toml::Table = toml::from_str(&text).unwrap_or_default();
        table
            .get("night_light")?
            .get("skip_outputs")?
            .as_array()
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|e| e.as_str().map(str::to_owned))
                    .collect()
            })
    }

    #[test]
    fn the_monitors_sill_warms_are_written_once_beside_the_rest_of_the_file() {
        let dir = TempDir::new("night-shift-skip");
        std::fs::write(
            dir.path().join(SETTINGS),
            "[pointer]\ntap = \"on\"\n\n[night_light]\nmode = \"scheduled\"\nskip_outputs = [\"edid:A/B/C\", \"DP-7\"]\n",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let wanted = outputs(&["DP-1", "HDMI-A-1"]);
        let first = write_skip(dir.path(), &wanted).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(first, Skipping::Changed);
        assert_eq!(
            skip_in(dir.path()),
            Some(strings(&["edid:A/B/C", "DP-1", "HDMI-A-1"]))
        );
        let text = std::fs::read_to_string(dir.path().join(SETTINGS)).unwrap_or_default();
        assert!(
            text.contains("tap = \"on\"") && text.contains("mode = \"scheduled\""),
            "{text}"
        );
        let again = write_skip(dir.path(), &wanted).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(again, Skipping::AsWas, "the same list is not written again");
        let gone = write_skip(dir.path(), &[]).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(gone, Skipping::Changed);
        assert_eq!(skip_in(dir.path()), Some(strings(&["edid:A/B/C"])));
    }

    #[test]
    fn nothing_to_skip_creates_no_file_and_removes_an_emptied_key() {
        let dir = TempDir::new("night-shift-skip-none");
        let none = write_skip(dir.path(), &[]).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(none, Skipping::AsWas);
        assert!(!dir.path().join(SETTINGS).exists(), "no file for no list");
        write_skip(dir.path(), &outputs(&["DP-1"])).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(skip_in(dir.path()), Some(strings(&["DP-1"])));
        write_skip(dir.path(), &[]).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            skip_in(dir.path()),
            None,
            "the key is removed with its last entry"
        );
    }

    #[test]
    fn a_file_that_is_not_toml_is_not_overwritten_by_the_skip_list() {
        let dir = TempDir::new("night-shift-skip-damaged");
        std::fs::write(dir.path().join(SETTINGS), "this = is = not toml").unwrap_or_default();
        assert!(write_skip(dir.path(), &outputs(&["DP-1"])).is_err());
        let kept = std::fs::read_to_string(dir.path().join(SETTINGS)).unwrap_or_default();
        assert_eq!(kept, "this = is = not toml");
    }

    #[test]
    fn the_persons_comments_and_layout_survive_an_edit() {
        let dir = TempDir::new("night-shift-comments");
        std::fs::write(
            dir.path().join(SETTINGS),
            "# my settings\n[pointer]\ntap = \"on\"  # taps\n\n[night_light]\n# when it starts\nfrom = 1200 # evening\nmode = \"off\"\n",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        write_light(dir.path(), &warm()).unwrap_or_else(|e| panic!("{e}"));
        let text = std::fs::read_to_string(dir.path().join(SETTINGS)).unwrap_or_default();
        assert!(
            text.starts_with("# my settings\n[pointer]\ntap = \"on\"  # taps\n\n[night_light]\n"),
            "{text}"
        );
        assert!(
            text.contains("# when it starts\nfrom = 1380 # evening\nmode = \"scheduled\""),
            "{text}"
        );
    }

    #[test]
    fn a_linked_settings_file_is_written_through_and_keeps_its_permissions() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = TempDir::new("night-shift-link");
        let real = dir.path().join("dotfiles.toml");
        std::fs::write(&real, "[night_light]\nmode = \"off\"\n").unwrap_or_else(|e| panic!("{e}"));
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o640))
            .unwrap_or_else(|e| panic!("{e}"));
        symlink(&real, dir.path().join(SETTINGS)).unwrap_or_else(|e| panic!("{e}"));
        write_light(dir.path(), &warm()).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            std::fs::symlink_metadata(dir.path().join(SETTINGS))
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
            "the link is still a link"
        );
        let text = std::fs::read_to_string(&real).unwrap_or_default();
        assert!(text.contains("mode = \"scheduled\""), "{text}");
        let mode = std::fs::metadata(&real)
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or_default();
        assert_eq!(mode, 0o640);
    }

    #[test]
    fn a_light_the_file_already_has_is_not_written_again() {
        let dir = TempDir::new("night-shift-same");
        write_light(dir.path(), &warm()).unwrap_or_else(|e| panic!("{e}"));
        let path = dir.path().join(SETTINGS);
        let first = std::fs::metadata(&path).and_then(|m| m.modified());
        std::thread::sleep(std::time::Duration::from_millis(20));
        write_light(dir.path(), &warm()).unwrap_or_else(|e| panic!("{e}"));
        let second = std::fs::metadata(&path).and_then(|m| m.modified());
        assert_eq!(first.ok(), second.ok(), "casement's watch is not woken");
    }
}
