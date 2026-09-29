//! `sill_settings::ShellFile`'s `scroll` and `scrollbar` sections, mapped to shell-host's
//! `ScrollConfig` (`HostCtx::set_scroll_config`). Both sides carry the same fields under the same
//! serde spelling (design/22 §3.7 `scroll` / §3.8 `scrollbar`), but as two crates' own newtypes and
//! enums, so the mapping is field by field rather than a shared type. Pure; `daemon.rs` calls it at
//! start and on every settings change.

use shell_host as host;
use sill_settings::{
    Fraction, Momentum, MomentumModel, Ms, NaturalScroll, Px, RubberBand, Scalar, ScrollSettings,
    ScrollbarSettings, ScrollbarVisibility, ShellFile, TrackClick,
};

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
        momentum_model: momentum_model(s.momentum_model),
        momentum_a: scalar(s.momentum_a),
        momentum_b: scalar(s.momentum_b),
        momentum_stop_px_s: scalar(s.momentum_stop_px_s),
        momentum_start_px_s: scalar(s.momentum_start_px_s),
        momentum_cap_px_s: scalar(s.momentum_cap_px_s),
        velocity_window_ms: ms(s.velocity_window_ms),
        velocity_held_still_ms: ms(s.velocity_held_still_ms),
        rubber_band: rubber_band(s.rubber_band),
        rubber_band_c: fraction(s.rubber_band_c),
        rubber_band_linear_divisor: scalar(s.rubber_band_linear_divisor),
        rubber_band_edge_min_px: px(s.rubber_band_edge_min_px),
        rubber_band_snap_rate: scalar(s.rubber_band_snap_rate),
        rubber_band_snap_gain: scalar(s.rubber_band_snap_gain),
        wheel_detent_px: px(s.wheel_detent_px),
        wheel_burst_window_ms: ms(s.wheel_burst_window_ms),
        momentum_boost_threshold_px_s: scalar(s.momentum_boost_threshold_px_s),
        double_scroll_grace_ms: ms(s.double_scroll_grace_ms),
        scrollbars: scrollbar_visibility(s.scrollbars),
    }
}

fn scrollbar(s: &ScrollbarSettings) -> host::ScrollbarSettings {
    host::ScrollbarSettings {
        track_thickness_px: px(s.track_thickness_px),
        track_thickness_hover_px: px(s.track_thickness_hover_px),
        thumb_thickness_px: px(s.thumb_thickness_px),
        thumb_thickness_hover_px: px(s.thumb_thickness_hover_px),
        thumb_min_length_px: px(s.thumb_min_length_px),
        hide_delay_ms: ms(s.hide_delay_ms),
        fade_duration_ms: ms(s.fade_duration_ms),
        expand_zone_px: px(s.expand_zone_px),
        expand_duration_ms: ms(s.expand_duration_ms),
        legacy_track_width_px: px(s.legacy_track_width_px),
        other_axis_offset_px: px(s.other_axis_offset_px),
        track_click_mode: track_click(s.track_click_mode),
    }
}

fn px(v: Px) -> host::ScrollPx {
    host::ScrollPx(v.0)
}

fn ms(v: Ms) -> host::ScrollMs {
    host::ScrollMs(v.0)
}

fn fraction(v: Fraction) -> host::ScrollFraction {
    host::ScrollFraction(v.0)
}

fn scalar(v: Scalar) -> host::ScrollScalar {
    host::ScrollScalar(v.0)
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

fn momentum_model(v: MomentumModel) -> host::ScrollMomentumModel {
    match v {
        MomentumModel::MacMouseFix => host::ScrollMomentumModel::MacMouseFix,
        MomentumModel::Ios => host::ScrollMomentumModel::Ios,
    }
}

fn rubber_band(v: RubberBand) -> host::ScrollRubberBand {
    match v {
        RubberBand::Bounded => host::ScrollRubberBand::Bounded,
        RubberBand::Linear => host::ScrollRubberBand::Linear,
        RubberBand::Off => host::ScrollRubberBand::Off,
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
                |f| f.scroll.momentum_model = MomentumModel::Ios,
                |c| c.scroll.momentum_model == host::ScrollMomentumModel::Ios,
            ),
            (
                |f| f.scroll.rubber_band = RubberBand::Linear,
                |c| c.scroll.rubber_band == host::ScrollRubberBand::Linear,
            ),
            (
                |f| f.scroll.rubber_band = RubberBand::Off,
                |c| c.scroll.rubber_band == host::ScrollRubberBand::Off,
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
    fn a_changed_number_carries_its_value() {
        let mut file = ShellFile::default();
        file.scroll.wheel_detent_px = Px(53);
        file.scrollbar.hide_delay_ms = Ms(900);
        let got = config(&file);
        assert_eq!(got.scroll.wheel_detent_px, host::ScrollPx(53));
        assert_eq!(got.scrollbar.hide_delay_ms, host::ScrollMs(900));
    }
}
