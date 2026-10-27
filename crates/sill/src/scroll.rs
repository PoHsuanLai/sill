//! `sill_settings::ShellFile`'s `scroll` and `scrollbar` sections, mapped to shell-host's
//! `ScrollConfig` (`HostCtx::set_scroll_config`). Both sides carry the same fields under the same
//! serde spelling (design/22 §3.7 `scroll` / §3.8 `scrollbar`), but as two crates' own newtypes and
//! enums, so the mapping is field by field rather than a shared type. Pure; `daemon.rs` calls it at
//! start and on every settings change.

use shell_host as host;
use sill_settings::{
    Momentum, NaturalScroll, Px, ScrollSettings, ScrollbarSettings, ScrollbarVisibility, ShellFile,
    TrackClick,
};

/// Momentum's drag coefficient `a`.
const MOMENTUM_A: host::ScrollScalar = host::ScrollScalar(30.0);
/// Momentum's drag exponent `b`.
const MOMENTUM_B: host::ScrollScalar = host::ScrollScalar(0.7);
/// The speed, in px/s, under which momentum stops.
const MOMENTUM_STOP_PX_S: host::ScrollScalar = host::ScrollScalar(1.0);
/// The speed, in px/s, a flick must reach to start momentum.
const MOMENTUM_START_PX_S: host::ScrollScalar = host::ScrollScalar(100.0);
/// The fastest momentum scroll, in px/s.
const MOMENTUM_CAP_PX_S: host::ScrollScalar = host::ScrollScalar(12_000.0);
/// The window, in ms, the release velocity is measured over.
const VELOCITY_WINDOW_MS: host::ScrollMs = host::ScrollMs(80);
/// How long, in ms, the fingers may rest before the release velocity is zero.
const VELOCITY_HELD_STILL_MS: host::ScrollMs = host::ScrollMs(50);
/// The overscroll stretch's shape: bounded, so it never leaves the view.
const RUBBER_BAND: host::ScrollRubberBand = host::ScrollRubberBand::Bounded;
/// The bounded stretch's constant `c`, in thousandths.
const RUBBER_BAND_C: host::ScrollFraction = host::ScrollFraction(550);
/// The overscroll, in px, under which no stretch is drawn.
const RUBBER_BAND_EDGE_MIN_PX: host::ScrollPx = host::ScrollPx(10);
/// How fast the stretch snaps back.
const RUBBER_BAND_SNAP_RATE: host::ScrollScalar = host::ScrollScalar(12.5);
/// How strongly the stretch snaps back.
const RUBBER_BAND_SNAP_GAIN: host::ScrollScalar = host::ScrollScalar(0.31);
/// How far one wheel notch scrolls, in px.
const WHEEL_DETENT_PX: host::ScrollPx = host::ScrollPx(60);
/// The window, in ms, in which wheel notches count as one burst.
const WHEEL_BURST_WINDOW_MS: host::ScrollMs = host::ScrollMs(300);
/// The speed, in px/s, above which momentum is boosted by the next scroll.
const MOMENTUM_BOOST_THRESHOLD_PX_S: host::ScrollScalar = host::ScrollScalar(350.0);
/// The grace, in ms, before a second scroll counts as a double scroll.
const DOUBLE_SCROLL_GRACE_MS: host::ScrollMs = host::ScrollMs(150);
/// How long, in ms, a scroll bar stays after scrolling stops.
const SCROLLBAR_HIDE_DELAY_MS: host::ScrollMs = host::ScrollMs(500);
/// How long, in ms, a scroll bar takes to fade out.
const SCROLLBAR_FADE_DURATION_MS: host::ScrollMs = host::ScrollMs(240);
/// How near, in px, the pointer must be for the scroll bar to widen.
const SCROLLBAR_EXPAND_ZONE_PX: host::ScrollPx = host::ScrollPx(16);
/// How long, in ms, the scroll bar takes to widen.
const SCROLLBAR_EXPAND_DURATION_MS: host::ScrollMs = host::ScrollMs(240);

/// `file.scroll` and `file.scrollbar`, as `HostCtx::set_scroll_config` wants them.
pub fn config(file: &ShellFile) -> host::ScrollConfig {
    host::ScrollConfig {
        scroll: scroll(&file.scroll),
        scrollbar: scrollbar(&file.scrollbar),
    }
}

fn scroll(s: &ScrollSettings) -> host::ScrollSettings {
    host::ScrollSettings {
        natural: natural_scroll(s.natural),
        momentum: momentum(s.momentum),
        momentum_a: MOMENTUM_A,
        momentum_b: MOMENTUM_B,
        momentum_stop_px_s: MOMENTUM_STOP_PX_S,
        momentum_start_px_s: MOMENTUM_START_PX_S,
        momentum_cap_px_s: MOMENTUM_CAP_PX_S,
        velocity_window_ms: VELOCITY_WINDOW_MS,
        velocity_held_still_ms: VELOCITY_HELD_STILL_MS,
        rubber_band: RUBBER_BAND,
        rubber_band_c: RUBBER_BAND_C,
        rubber_band_edge_min_px: RUBBER_BAND_EDGE_MIN_PX,
        rubber_band_snap_rate: RUBBER_BAND_SNAP_RATE,
        rubber_band_snap_gain: RUBBER_BAND_SNAP_GAIN,
        wheel_detent_px: WHEEL_DETENT_PX,
        wheel_burst_window_ms: WHEEL_BURST_WINDOW_MS,
        momentum_boost_threshold_px_s: MOMENTUM_BOOST_THRESHOLD_PX_S,
        double_scroll_grace_ms: DOUBLE_SCROLL_GRACE_MS,
        scrollbars: scrollbar_visibility(s.scrollbars),
        ..host::ScrollSettings::default()
    }
}

fn scrollbar(s: &ScrollbarSettings) -> host::ScrollbarSettings {
    host::ScrollbarSettings {
        track_thickness_px: px(s.track_thickness_px),
        track_thickness_hover_px: px(s.track_thickness_hover_px),
        thumb_thickness_px: px(s.thumb_thickness_px),
        thumb_thickness_hover_px: px(s.thumb_thickness_hover_px),
        thumb_min_length_px: px(s.thumb_min_length_px),
        hide_delay_ms: SCROLLBAR_HIDE_DELAY_MS,
        fade_duration_ms: SCROLLBAR_FADE_DURATION_MS,
        expand_zone_px: SCROLLBAR_EXPAND_ZONE_PX,
        expand_duration_ms: SCROLLBAR_EXPAND_DURATION_MS,
        legacy_track_width_px: px(s.legacy_track_width_px),
        other_axis_offset_px: px(s.other_axis_offset_px),
        track_click_mode: track_click(s.track_click_mode),
    }
}

fn px(v: Px) -> host::ScrollPx {
    host::ScrollPx(v.0)
}

fn natural_scroll(v: NaturalScroll) -> host::NaturalScroll {
    match v {
        NaturalScroll::Natural => host::NaturalScroll::Natural,
        NaturalScroll::Traditional => host::NaturalScroll::Traditional,
    }
}

fn momentum(v: Momentum) -> host::ScrollMomentum {
    match v {
        Momentum::On => host::ScrollMomentum::On,
        Momentum::Off => host::ScrollMomentum::Off,
    }
}

fn scrollbar_visibility(v: ScrollbarVisibility) -> host::ScrollbarVisibility {
    match v {
        ScrollbarVisibility::WhenScrolling => host::ScrollbarVisibility::WhenScrolling,
        ScrollbarVisibility::Always => host::ScrollbarVisibility::Always,
    }
}

fn track_click(v: TrackClick) -> host::ScrollTrackClick {
    match v {
        TrackClick::PageTowardClick => host::ScrollTrackClick::PageTowardClick,
        TrackClick::JumpToClick => host::ScrollTrackClick::JumpToClick,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_match_field_by_field() {
        assert_eq!(config(&ShellFile::default()), host::ScrollConfig::default());
    }

    /// A mutation on a default `ShellFile` and the host field it must land on.
    type Case = (fn(&mut ShellFile), fn(&host::ScrollConfig) -> bool);

    /// One row per enum this module converts.
    #[test]
    fn every_enum_variant_survives_the_mapping() {
        let cases: &[Case] = &[
            (
                |f| f.scroll.natural = NaturalScroll::Traditional,
                |c| c.scroll.natural == host::NaturalScroll::Traditional,
            ),
            (
                |f| f.scroll.momentum = Momentum::Off,
                |c| c.scroll.momentum == host::ScrollMomentum::Off,
            ),
            (
                |f| f.scroll.scrollbars = ScrollbarVisibility::Always,
                |c| c.scroll.scrollbars == host::ScrollbarVisibility::Always,
            ),
            (
                |f| f.scrollbar.track_click_mode = TrackClick::JumpToClick,
                |c| c.scrollbar.track_click_mode == host::ScrollTrackClick::JumpToClick,
            ),
        ];
        for (mutate, holds) in cases {
            let mut file = ShellFile::default();
            mutate(&mut file);
            assert!(holds(&config(&file)));
        }
    }

    #[test]
    fn a_changed_size_carries_its_value() {
        let mut file = ShellFile::default();
        file.scrollbar.thumb_min_length_px = Px(30);
        let got = config(&file);
        assert_eq!(got.scrollbar.thumb_min_length_px, host::ScrollPx(30));
    }
}
