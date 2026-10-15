//! The two files: casement's `settings.toml`, of which only `[night_light]` is touched, and the
//! manual turn-on's `night-shift.json`. Both are parsed once here into the model's types, and
//! written through a temporary file and a rename.

use std::io::ErrorKind;
use std::path::Path;

use ds_core::word::Word;
use serde::{Deserialize, Serialize};
use sill_model::night_shift::{Manual, MinuteOfDay, NightLight, Schedule, UnixSeconds, Warmth};
use toml::{Table, Value};

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
    #[error("{0}")]
    Encode(#[from] toml::ser::Error),
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
/// key of the file and of the table as it was. A file that is not valid TOML is not overwritten.
pub(super) fn write_light(dir: &Path, light: &NightLight) -> Result<(), FileError> {
    let path = dir.join(SETTINGS);
    let mut table = match std::fs::read_to_string(&path) {
        Ok(text) => toml::from_str::<Table>(&text)?,
        Err(error) if error.kind() == ErrorKind::NotFound => Table::new(),
        Err(error) => return Err(error.into()),
    };
    let section = table
        .entry("night_light")
        .or_insert_with(|| Value::Table(Table::new()));
    if !section.is_table() {
        *section = Value::Table(Table::new());
    }
    if let Some(section) = section.as_table_mut() {
        section.insert(
            "mode".to_owned(),
            Value::String(light.schedule.slug().to_owned()),
        );
        section.insert(
            "from".to_owned(),
            Value::Integer(i64::from(light.from.minutes())),
        );
        section.insert(
            "to".to_owned(),
            Value::Integer(i64::from(light.to.minutes())),
        );
        section.insert(
            "kelvin".to_owned(),
            Value::Integer(i64::from(light.warmth.kelvin())),
        );
        section.insert(
            "fade_minutes".to_owned(),
            Value::Integer(i64::from(light.fade_minutes)),
        );
    }
    atomically(&path, &toml::to_string(&table)?)
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
    atomically(
        &path,
        &serde_json::to_string_pretty(&file).unwrap_or_default(),
    )
}

/// Write `text` to `path` (its folder created) through a temporary file and a rename, so a
/// reader never sees half a file.
fn atomically(path: &Path, text: &str) -> Result<(), FileError> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let part = path.with_extension("part");
    std::fs::write(&part, text)?;
    std::fs::rename(&part, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sill_model::night_shift::{Manual, MinuteOfDay, NightLight, Schedule, UnixSeconds, Warmth};
    use sill_testkit::TempDir;

    use super::{SETTINGS, read_light, read_manual, write_light, write_manual};

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
}
