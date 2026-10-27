//! `scroll.*` (design/22 §3.7 and `scroll.scrollbars` from §3.8; design/11 §11.6). The physics
//! run in shell-host; these are its tunables. Natural scrolling is on the Mouse & Gestures
//! page; the rest are Advanced (§5). `scroll.speed`, `scroll.lock_*` and the gains are
//! palmrest's (`palmrest/gestures.toml`, §3.9), not here.

use ds_core::word::Word;
use ds_settings::schema::Page;
use ds_settings::{Fraction, Ms, Px, Scalar};
use serde::{Deserialize, Serialize};

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

/// `scroll.momentum`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum Momentum {
    #[default]
    On,
    Off,
}

/// `scroll.momentum_model` (design/11 §11.9 open decision 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum MomentumModel {
    /// Mac Mouse Fix's drag model `v' = -a v^b`.
    #[default]
    MacMouseFix,
    /// iOS's `v(t) = v0 r^t`, `r = .998`.
    Ios,
}

/// `scroll.rubber_band`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum RubberBand {
    /// iOS's bounded stretch.
    #[default]
    Bounded,
    /// `overscroll / divisor`.
    Linear,
    Off,
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
    pub momentum: Momentum,
    #[settings(
        label = "Momentum model",
        help = "Which slowdown curve a flick follows.",
        section = "Momentum",
        advanced
    )]
    pub momentum_model: MomentumModel,
    #[settings(
        label = "Drag constant",
        help = "The slowdown curve's first setting.",
        section = "Momentum",
        range = "0..=100",
        advanced
    )]
    pub momentum_a: Scalar,
    #[settings(
        label = "Drag exponent",
        help = "The slowdown curve's second setting.",
        section = "Momentum",
        range = "0..=2",
        advanced
    )]
    pub momentum_b: Scalar,
    #[settings(
        label = "Stop speed",
        help = "Momentum ends below this speed.",
        section = "Momentum",
        range = "0..=100",
        unit = "px/s",
        advanced
    )]
    pub momentum_stop_px_s: Scalar,
    #[settings(
        label = "Start speed",
        help = "A flick must be faster than this to glide.",
        section = "Momentum",
        range = "0..=2000",
        unit = "px/s",
        advanced
    )]
    pub momentum_start_px_s: Scalar,
    #[settings(
        label = "Top speed",
        help = "The fastest a flick can start.",
        section = "Momentum",
        range = "1000..=30000",
        unit = "px/s",
        advanced
    )]
    pub momentum_cap_px_s: Scalar,
    #[settings(
        label = "Velocity window",
        help = "How much of a gesture's end sets the release speed.",
        section = "Momentum",
        range = "10..=500",
        unit = "ms",
        advanced
    )]
    pub velocity_window_ms: Ms,
    #[settings(
        label = "Held-still time",
        help = "Pausing this long before lifting means no flick.",
        section = "Momentum",
        range = "0..=500",
        unit = "ms",
        advanced
    )]
    pub velocity_held_still_ms: Ms,
    #[settings(
        label = "Rubber band",
        help = "How content stretches past its end.",
        section = "Rubber band",
        advanced
    )]
    pub rubber_band: RubberBand,
    #[settings(
        label = "Stretch constant",
        help = "Stretch setting c (550 is 0.55).",
        section = "Rubber band",
        range = "0..=1000",
        advanced
    )]
    pub rubber_band_c: Fraction,
    #[settings(
        label = "Linear divisor",
        help = "The divisor for the linear style.",
        section = "Rubber band",
        range = "1..=100",
        advanced
    )]
    pub rubber_band_linear_divisor: Scalar,
    #[settings(
        label = "Stretch threshold",
        help = "Overscroll smaller than this doesn't stretch.",
        section = "Rubber band",
        range = "0..=100",
        unit = "px",
        advanced
    )]
    pub rubber_band_edge_min_px: Px,
    #[settings(
        label = "Snap-back rate",
        help = "How fast stretched content returns.",
        section = "Rubber band",
        range = "1..=100",
        advanced
    )]
    pub rubber_band_snap_rate: Scalar,
    #[settings(
        label = "Snap-back gain",
        help = "How much release speed carries into the snap-back.",
        section = "Rubber band",
        range = "0..=1",
        advanced
    )]
    pub rubber_band_snap_gain: Scalar,
    #[settings(
        label = "Wheel step",
        help = "How far one wheel notch scrolls.",
        section = "Scrolling",
        range = "20..=200",
        unit = "px",
        advanced
    )]
    pub wheel_detent_px: Px,
    #[settings(
        label = "Wheel burst window",
        help = "Notches closer than this scroll as one.",
        section = "Scrolling",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub wheel_burst_window_ms: Ms,
    #[settings(
        label = "Momentum boost",
        help = "A faster flick the same way adds to momentum.",
        section = "Momentum",
        range = "0..=5000",
        unit = "px/s",
        advanced
    )]
    pub momentum_boost_threshold_px_s: Scalar,
    #[settings(
        label = "Double-scroll grace",
        help = "How long after a gesture a second source is ignored.",
        section = "Scrolling",
        range = "0..=1000",
        unit = "ms",
        advanced
    )]
    pub double_scroll_grace_ms: Ms,
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
            momentum: Momentum::On,
            momentum_model: MomentumModel::MacMouseFix,
            momentum_a: Scalar(30.0),
            momentum_b: Scalar(0.7),
            momentum_stop_px_s: Scalar(1.0),
            momentum_start_px_s: Scalar(100.0),
            momentum_cap_px_s: Scalar(12_000.0),
            velocity_window_ms: Ms(80),
            velocity_held_still_ms: Ms(50),
            rubber_band: RubberBand::Bounded,
            rubber_band_c: Fraction(550),
            rubber_band_linear_divisor: Scalar(20.0),
            rubber_band_edge_min_px: Px(10),
            rubber_band_snap_rate: Scalar(12.5),
            rubber_band_snap_gain: Scalar(0.31),
            wheel_detent_px: Px(60),
            wheel_burst_window_ms: Ms(300),
            momentum_boost_threshold_px_s: Scalar(350.0),
            double_scroll_grace_ms: Ms(150),
            scrollbars: ScrollbarVisibility::WhenScrolling,
        }
    }
}
