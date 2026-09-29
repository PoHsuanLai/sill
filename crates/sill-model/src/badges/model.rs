//! Badges' state and the LauncherEntry app URI.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

use crate::units::{Permille, Urgency, Visibility};

/// Every app with something to show, by desktop id.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Badges {
    pub apps: BTreeMap<AppId, Badge>,
}

/// One app's LauncherEntry properties. The count and progress are kept even while hidden, as
/// the protocol sends value and visibility separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Badge {
    pub count: u64,
    pub count_visibility: Visibility,
    /// `progress` 0..1 as thousandths.
    pub progress: Permille,
    pub progress_visibility: Visibility,
    /// `urgent`: starts a Critical bounce (design/10 §10.3.2).
    pub urgency: Urgency,
}

impl Badge {
    /// The count to draw: visible and above zero.
    pub fn shown_count(&self) -> Option<u64> {
        (self.count_visibility == Visibility::Shown && self.count > 0).then_some(self.count)
    }

    /// The progress to draw, when visible.
    pub fn shown_progress(&self) -> Option<Permille> {
        (self.progress_visibility == Visibility::Shown).then_some(self.progress)
    }
}

/// The desktop id an `app_uri` names: `application://org.gnome.Nautilus.desktop` is
/// `org.gnome.Nautilus`. Another scheme, or an empty id, is `None`.
pub fn parse_app_uri(uri: &str) -> Option<AppId> {
    let rest = uri.strip_prefix("application://")?;
    let id = rest.strip_suffix(".desktop").unwrap_or(rest);
    (!id.is_empty() && !id.contains('/')).then(|| AppId(id.to_owned()))
}
