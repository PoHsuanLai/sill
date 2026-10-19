//! The numbers the machine runs on: `dock.*` settings plus quire's motion tokens
//! (no easing follows the motion level). Built once per settings change, never hard-coded.

use ds::base::time::stamp::Ms;
use ds::motion::hover_intent::HoverProfile;
use ds::prelude::*;
use ds::style::tokens::delay::TipDelay;
use ds::style::tokens::easing::{CubicBezier, EasingToken};
use sill_model::units::{Cpx, Permille};
use sill_settings::{
    ActiveClick, Bounce as BounceSetting, DockModifierClicks, DockSettings, LaunchAnim,
    Magnification,
};

/// The label's first delay: quire's `HoverProfile::Label` (100 ms; design/30 section 1.2), where
/// it was 350 ms (`--d-fly`) before the dock followed `DockLabel`.
fn fly_delay_ms() -> Ms {
    Ms(u64::try_from(HoverProfile::Label.open(TipDelay::default()).as_millis()).unwrap_or(100))
}

/// Every parameter [`super::step`] reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockParams {
    pub magnification: Magnification,
    /// `dock.magnify_enter_ms`.
    pub enter_ms: u64,
    /// `dock.magnify_leave_ms`.
    pub leave_ms: u64,
    /// `--e-out`.
    pub ease_out: CubicBezier,
    /// `--e-exit`.
    pub ease_exit: CubicBezier,
    /// The label's first delay, [`fly_delay_ms`].
    pub fly_delay_ms: Ms,
    /// `dock.hover_label_warm_ms`.
    pub warm_ms: Ms,
    /// `dock.hold_to_menu_ms`.
    pub hold_ms: Ms,
    /// `dock.hold_to_menu_move_px`, which is also the drag threshold (A6 "live after 8 px
    /// Manhattan").
    pub drag_threshold: Cpx,
    pub bounce: BounceSetting,
    pub launch_animation: LaunchAnim,
    /// `dock.bounce_period_ms`.
    pub bounce_period_ms: u64,
    /// `dock.bounce_launch_cap_ms`.
    pub bounce_launch_cap_ms: Ms,
    /// Bounce amplitude: five eighths of a tile at rest (design/10 §10.3.5).
    pub amplitude: Cpx,
    pub click_active_app: ActiveClick,
    pub modifier_clicks: DockModifierClicks,
    /// `dock.autohide_delay_ms`.
    pub autohide_delay_ms: Ms,
    /// `dock.autohide_slide_ms`.
    pub slide_ms: u64,
    /// `gut + H_p + 8` (design/10 §10.5.5): how far a hidden dock sits below the edge.
    pub slide_distance: Cpx,
    /// `dock.spring_load_ms`.
    pub spring_load_ms: Ms,
    /// The blur region's corner radius under the pill's squircle.
    pub pill_blur_radius: i32,
}

/// `curve` at time `t` (thousandths, at most all): quire's one evaluator, [`CubicBezier::at`].
pub(super) fn eased(curve: CubicBezier, t: Permille) -> Permille {
    let at = Fraction(u16::try_from(t.0.min(1000)).unwrap_or(1000));
    Permille(u32::from(curve.at(at).0))
}

impl DockParams {
    /// The parameters for `dock` settings.
    pub fn new(dock: &DockSettings) -> DockParams {
        let curve = |token: EasingToken| token.easing().curve();
        let px = |v: u16| Cpx::px(i32::from(v));
        let pad = i32::from(dock.pill_padding_px.0);
        let gutter = 8;
        DockParams {
            magnification: dock.magnification,
            enter_ms: u64::from(dock.magnify_enter_ms.0),
            leave_ms: u64::from(dock.magnify_leave_ms.0),
            ease_out: curve(EasingToken::Out),
            ease_exit: curve(EasingToken::Exit),
            fly_delay_ms: fly_delay_ms(),
            warm_ms: Ms::from(dock.hover_label_warm_ms.0),
            hold_ms: Ms::from(dock.hold_to_menu_ms.0),
            drag_threshold: px(dock.hold_to_menu_move_px.0),
            bounce: dock.bounce,
            launch_animation: dock.launch_animation,
            bounce_period_ms: u64::from(dock.bounce_period_ms.0),
            bounce_launch_cap_ms: Ms::from(dock.bounce_launch_cap_ms.0),
            amplitude: bounce_amplitude(dock.tile_size_px.0),
            click_active_app: dock.click_active_app,
            modifier_clicks: dock.modifier_clicks,
            autohide_delay_ms: Ms::from(dock.autohide_delay_ms.0),
            slide_ms: u64::from(dock.autohide_slide_ms.0),
            slide_distance: Cpx::px(gutter + (i32::from(dock.tile_size_px.0) + 2 * pad) + 8),
            spring_load_ms: Ms::from(dock.spring_load_ms.0),
            pill_blur_radius: crate::dock::geometry::pill_blur_radius(dock),
        }
    }
}

/// How high a tile of `tile` px bounces: five eighths of it (design/10 §10.3.5, Plank's 0.625; a
/// whole tile hopped too high on the trial login of 2026-10-19).
fn bounce_amplitude(tile: u16) -> Cpx {
    Cpx::px(i32::from(tile) * 5 / 8)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{DockParams, eased};
    use ds::base::time::stamp::Ms;
    use ds::style::tokens::easing::CubicBezier;
    use sill_model::units::{Cpx, Permille};
    use sill_settings::DockSettings;

    /// The design defaults at the standard level: what every machine test runs on.
    pub(crate) fn params() -> DockParams {
        DockParams::new(&DockSettings::default())
    }

    #[test]
    fn defaults_are_design_10s_numbers() {
        let p = params();
        assert_eq!((p.enter_ms, p.leave_ms), (120, 200));
        assert_eq!(p.fly_delay_ms, Ms(100));
        assert_eq!(p.warm_ms, Ms(400));
        assert_eq!(p.hold_ms, Ms(600));
        assert_eq!(p.drag_threshold, Cpx::px(8));
        assert_eq!(p.bounce_period_ms, 600);
        assert_eq!(p.bounce_launch_cap_ms, Ms(10_000));
        assert_eq!(p.amplitude, Cpx::px(30), "5/8 of a 48 px tile");
        assert_eq!(p.autohide_delay_ms, Ms(200));
        assert_eq!(p.slide_ms, 500);
        assert_eq!(p.slide_distance, Cpx::px(76), "gut + H_p + 8 = 8 + 60 + 8");
        assert_eq!(p.ease_out, CubicBezier([220, 900, 300, 1000]));
    }

    #[test]
    fn the_curves_run_from_nothing_to_all_and_ease_out_leads_linear() {
        let out = CubicBezier([220, 900, 300, 1000]);
        let linear = CubicBezier([0, 0, 1000, 1000]);
        // (curve, t, at least, at most)
        let cases = [
            (out, 0, 0, 0),
            (out, 1000, 1000, 1000),
            (out, 5000, 1000, 1000),
            (out, 100, 379, 381),
            (out, 250, 747, 749),
            (out, 500, 948, 950),
            (out, 750, 992, 994),
            (linear, 500, 495, 505),
        ];
        for (curve, t, least, most) in cases {
            let got = eased(curve, Permille(t)).0;
            assert!((least..=most).contains(&got), "{curve:?} at {t}: {got}");
        }
    }
}
