//! `$XDG_STATE_HOME/sill/displays.json`: every display sill has seen, and the scale it last
//! set on each. What makes "only on first sight" true across restarts, and the one place the
//! scale decision ([`decide_scale`]) looks for history.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::identity::IdentityKey;
use super::policy::ScaleMode;
use super::scale::DisplayScale;
use crate::error::ServiceError;

/// One display's history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeenDisplay {
    /// The EDID product name, for a person reading the file.
    #[serde(default)]
    pub name: String,
    /// The scale sill set, if it ever set one. A display recorded with none was seen while
    /// `display.scale` was `manual`, or was already at its default.
    #[serde(default)]
    pub applied: Option<DisplayScale>,
}

/// The whole file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SeenDisplays {
    #[serde(default)]
    pub displays: BTreeMap<IdentityKey, SeenDisplay>,
}

/// What to do about one display's scale now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleDecision {
    /// Set this scale, then record it.
    Apply(DisplayScale),
    /// Change nothing, but record the display as seen (with what sill set, if anything).
    Record(Option<DisplayScale>),
    /// Change nothing and record nothing.
    Leave,
}

/// The facts one decision needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleFacts {
    pub mode: ScaleMode,
    /// The display's record, if it has one.
    pub seen: Option<Option<DisplayScale>>,
    /// A `display.scale_overrides` row for it.
    pub requested: Option<DisplayScale>,
    /// What its density calls for.
    pub computed: DisplayScale,
    /// What the compositor runs it at now.
    pub current: DisplayScale,
}

/// The rule, in order:
/// 1. An override row wins whenever it differs from what sill last set (a new or edited row),
///    even for a known display: the person asked for it in sill's own settings.
/// 2. `manual` never sets anything; a display first seen then is recorded, so turning `auto`
///    back on later does not rescale a display the person has been managing.
/// 3. A display already recorded is left alone: whatever scale it has now, someone chose it.
/// 4. A display seen for the first time gets its computed scale (recorded without a change
///    when it is already there).
pub fn decide_scale(facts: ScaleFacts) -> ScaleDecision {
    let last_set = facts.seen.flatten();
    match (facts.requested, facts.mode, facts.seen) {
        (Some(wanted), _, _) if Some(wanted) != last_set && wanted == facts.current => {
            ScaleDecision::Record(Some(wanted))
        }
        (Some(wanted), _, _) if Some(wanted) != last_set => ScaleDecision::Apply(wanted),
        (_, _, Some(_)) => ScaleDecision::Leave,
        (_, ScaleMode::Manual, None) => ScaleDecision::Record(None),
        (_, ScaleMode::Auto, None) if facts.computed == facts.current => {
            ScaleDecision::Record(None)
        }
        (_, ScaleMode::Auto, None) => ScaleDecision::Apply(facts.computed),
    }
}

/// `displays.json` in sill's folder of `dirs`' state directory.
pub fn path_in(dirs: &crate::env::Dirs) -> PathBuf {
    dirs.sill_state().join("displays.json")
}

/// The record at `path`; empty when there is none or it cannot be read (a broken record costs
/// at most one rescale per display, never a failed start).
pub fn load(path: &Path) -> SeenDisplays {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Write `seen` to `path` through a temporary file and a rename.
pub fn save(path: &Path, seen: &SeenDisplays) -> Result<(), ServiceError> {
    let io = |at: &Path| {
        let path = at.to_path_buf();
        move |source| ServiceError::Io { path, source }
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io(dir))?;
    }
    let text = serde_json::to_string_pretty(seen).map_err(|e| ServiceError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(e),
    })?;
    let tmp = path.with_extension("json.part");
    std::fs::write(&tmp, text).map_err(io(&tmp))?;
    std::fs::rename(&tmp, path).map_err(io(path))
}

#[cfg(test)]
mod tests {
    use super::{ScaleDecision, ScaleFacts, SeenDisplay, SeenDisplays, decide_scale, load, save};
    use crate::displays::identity::IdentityKey;
    use crate::displays::policy::ScaleMode;
    use crate::displays::scale::DisplayScale;

    const S100: DisplayScale = DisplayScale(100);
    const S150: DisplayScale = DisplayScale(150);
    const S175: DisplayScale = DisplayScale(175);

    #[test]
    fn scale_decisions_follow_the_rule() {
        use ScaleDecision::{Apply, Leave, Record};
        use ScaleMode::{Auto, Manual};
        let facts = |mode, seen, requested, current| ScaleFacts {
            mode,
            seen,
            requested,
            computed: S150,
            current,
        };
        let cases: Vec<(&str, ScaleFacts, ScaleDecision)> = vec![
            ("first sight", facts(Auto, None, None, S100), Apply(S150)),
            (
                "first sight, already right",
                facts(Auto, None, None, S150),
                Record(None),
            ),
            (
                "seen before: the person's scale stands",
                facts(Auto, Some(Some(S150)), None, S175),
                Leave,
            ),
            (
                "seen before with nothing set",
                facts(Auto, Some(None), None, S100),
                Leave,
            ),
            (
                "manual, first sight",
                facts(Manual, None, None, S100),
                Record(None),
            ),
            ("manual, seen", facts(Manual, Some(None), None, S100), Leave),
            (
                "a new override",
                facts(Auto, Some(Some(S150)), Some(S175), S150),
                Apply(S175),
            ),
            (
                "an override already applied",
                facts(Auto, Some(Some(S175)), Some(S175), S100),
                Leave,
            ),
            (
                "an override under manual",
                facts(Manual, Some(None), Some(S175), S100),
                Apply(S175),
            ),
            (
                "an override on first sight",
                facts(Auto, None, Some(S175), S100),
                Apply(S175),
            ),
            (
                "an override that is already the scale",
                facts(Auto, None, Some(S100), S100),
                Record(Some(S100)),
            ),
        ];
        for (label, facts, want) in cases {
            assert_eq!(decide_scale(facts), want, "{label}");
        }
    }

    #[test]
    fn the_record_round_trips_and_a_missing_one_is_empty() {
        let dir = std::env::temp_dir().join(format!("sill-seen-{}", std::process::id()));
        let path = dir.join("sill").join("displays.json");
        assert_eq!(load(&path), SeenDisplays::default());
        let mut seen = SeenDisplays::default();
        seen.displays.insert(
            IdentityKey("BNQ-8074-00".to_owned()),
            SeenDisplay {
                name: "BenQ MA270U".to_owned(),
                applied: Some(S150),
            },
        );
        save(&path, &seen).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path), seen);
        std::fs::write(&path, "not json").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path), SeenDisplays::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
