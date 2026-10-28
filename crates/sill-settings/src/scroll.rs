//! `scroll.*` (design/22 §3.7 and `scroll.scrollbars` from §3.8; design/11 §11.6). The physics
//! run in shell-host with constants (`sill::scroll`). Natural scrolling is on the Mouse & Gestures
//! page; the rest are Advanced (§5). `scroll.speed`, `scroll.lock_*` and the gains are
//! palmrest's (`palmrest/gestures.toml`, §3.9), not here.

use ds_core::word::Word;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};
use sill_model::on_off::OnOff;

/// `scroll.natural`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NaturalScroll {
    /// Content follows the fingers.
    #[default]
    Natural,
    Traditional,
}

/// `scroll.scrollbars`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum ScrollbarVisibility {
    /// Overlay thumbs that fade after scrolling stops.
    #[default]
    WhenScrolling,
    /// A legacy always-on track.
    Always,
}

/// Scrolling's feel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "scroll", page = Page::MouseAndGestures)]
pub struct ScrollSettings {
    #[settings(
        label = "Natural scrolling",
        help = "Content moves with your fingers.",
        section = "Scrolling"
    )]
    pub natural: NaturalScroll,
    #[settings(
        label = "Momentum",
        help = "A flick keeps scrolling after you lift.",
        section = "Momentum",
        advanced
    )]
    pub momentum: OnOff,
    /// `scroll.scrollbars`: listed in §3.8's table, but its dotted path is `scroll.`.
    #[settings(
        label = "Show scroll bars",
        help = "Only while scrolling, or always.",
        section = "Scroll bars",
        advanced
    )]
    pub scrollbars: ScrollbarVisibility,
}

impl Default for ScrollSettings {
    fn default() -> Self {
        ScrollSettings {
            natural: NaturalScroll::Natural,
            momentum: OnOff::On,
            scrollbars: ScrollbarVisibility::WhenScrolling,
        }
    }
}
