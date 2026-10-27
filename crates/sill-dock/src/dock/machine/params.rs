//! The numbers the machine runs on: constants, `dock.*` settings plus quire's motion tokens
//! (no easing follows the motion level). Built once per settings change, never hard-coded.

use crate::dock::tokens::DockTokens;
use ds::base::time::stamp::Ms;
use ds::motion::hover_intent::HoverProfile;
use ds::style::tokens::delay::TipDelay;
use ds::style::tokens::easing::{CubicBezier, EasingToken};
use sill_model::units::Cpx;
use sill_settings::{DockSettings, Magnification};
// `eased` is quire's one curve evaluator, which the dock's magnification shares with the reveal.
pub(super) use sill_ui_kit::autohide::eased;
use sill_ui_kit::autohide::{self, Timing};

/// How long icons take to grow under the pointer.
const MAGNIFY_ENTER_MS: u64 = 120;
/// How long icons take to shrink back.
const MAGNIFY_LEAVE_MS: u64 = 200;
/// How long after leaving the Dock labels show at once.
const HOVER_LABEL_WARM: Ms = Ms(400);
/// How long to press before an icon's menu opens.
const HOLD_TO_MENU: Ms = Ms(600);
/// How far a press can move and still open the menu; also the drag threshold.
const HOLD_TO_MENU_MOVE_PX: i32 = 8;
/// How far an icon is dragged out of the Dock to remove it.
pub(crate) const REMOVE_THRESHOLD_PX: i32 = 64;
/// How long a dragged file waits on an app before it opens.
const SPRING_LOAD: Ms = Ms(500);

/// The label's first delay: quire's `HoverProfile::Label` (100 ms; design/30 section 1.2), where
/// it was 350 ms (`--d-fly`) before the dock followed `DockLabel`.
fn fly_delay_ms() -> Ms {
    Ms(u64::try_from(HoverProfile::Label.open(TipDelay::default()).as_millis()).unwrap_or(100))
}

/// Every parameter [`super::step`] reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockParams {
    pub magnification: Magnification,
    pub enter_ms: u64,
    pub leave_ms: u64,
    /// `--e-out`.
    pub ease_out: CubicBezier,
    /// `--e-exit`.
    pub ease_exit: CubicBezier,
    /// The label's first delay, [`fly_delay_ms`].
    pub fly_delay_ms: Ms,
    pub warm_ms: Ms,
    pub hold_ms: Ms,
    /// The hold tolerance, which is also the drag threshold (A6 "live after 8 px Manhattan").
    pub drag_threshold: Cpx,
    pub autohide_delay_ms: Ms,
    pub slide_ms: u64,
    /// `gut + H_p + 8` (design/10 §10.5.5): how far a hidden dock sits below the edge.
    pub slide_distance: Cpx,
    pub spring_load_ms: Ms,
    /// The blur region's corner radius under the pill's squircle.
    pub pill_blur_radius: i32,
}

impl DockParams {
    /// The auto-hide numbers, for the shared reveal machine (`sill_ui_kit::autohide`).
    pub fn timing(&self) -> Timing {
        Timing {
            delay: self.autohide_delay_ms,
            slide_ms: self.slide_ms,
            ease_out: self.ease_out,
            ease_exit: self.ease_exit,
        }
    }

    /// The parameters for `dock` settings.
    pub fn new(dock: &DockSettings, tokens: &DockTokens) -> DockParams {
        let curve = |token: EasingToken| token.easing().curve();
        let pad = i32::from(tokens.pad);
        let gutter = 8;
        DockParams {
            magnification: dock.magnification,
            enter_ms: MAGNIFY_ENTER_MS,
            leave_ms: MAGNIFY_LEAVE_MS,
            ease_out: curve(EasingToken::Out),
            ease_exit: curve(EasingToken::Exit),
            fly_delay_ms: fly_delay_ms(),
            warm_ms: HOVER_LABEL_WARM,
            hold_ms: HOLD_TO_MENU,
            drag_threshold: Cpx::px(HOLD_TO_MENU_MOVE_PX),
            autohide_delay_ms: autohide::DELAY,
            slide_ms: autohide::SLIDE_MS,
            slide_distance: Cpx::px(gutter + (i32::from(dock.tile_size_px.0) + 2 * pad) + 8),
            spring_load_ms: SPRING_LOAD,
            pill_blur_radius: crate::dock::geometry::pill_blur_radius(tokens),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{DockParams, eased};
    use crate::dock::tokens::DockTokens;
    use ds::base::time::stamp::Ms;
    use ds::style::tokens::easing::CubicBezier;
    use sill_model::units::{Cpx, Permille};
    use sill_settings::DockSettings;

    /// The design defaults at the standard level: what every machine test runs on.
    pub(crate) fn params() -> DockParams {
        DockParams::new(&DockSettings::default(), &DockTokens::default())
    }

    #[test]
    fn defaults_are_design_10s_numbers() {
        let p = params();
        assert_eq!((p.enter_ms, p.leave_ms), (120, 200));
        assert_eq!(p.fly_delay_ms, Ms(100));
        assert_eq!(p.warm_ms, Ms(400));
        assert_eq!(p.hold_ms, Ms(600));
        assert_eq!(p.drag_threshold, Cpx::px(8));
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
