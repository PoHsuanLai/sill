//! The words a `WidgetFrame` is described in: its footprint on the grid and its title row.

use ds::base::word::Word;
use ds::components::content::text_runs::TextLine;
use ds::style::icon::Icon;
use serde::{Deserialize, Serialize};

/// A widget's footprint on the grid unit (`--widget-cell`, `--widget-gap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum WidgetSize {
    /// One cell.
    #[default]
    Small,
    /// Two cells wide, one tall.
    Medium,
    /// Two cells wide, two tall.
    Large,
}

/// A widget's title row: a glyph and a name (a calendar's month).
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetTitle {
    /// The glyph before the name.
    pub glyph: Icon,
    /// The name.
    pub text: TextLine,
}

impl WidgetTitle {
    /// A title of `glyph` and `text`.
    pub fn new(glyph: Icon, text: impl Into<TextLine>) -> Self {
        WidgetTitle {
            glyph,
            text: text.into(),
        }
    }
}
