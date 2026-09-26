//! The low-battery nudge (quire design/26 G11): the battery item lifts once when a discharging
//! battery crosses `bar.battery_low_percent`, never loops, and is not armed again until the
//! battery is plugged in, so a charge wobbling around the threshold nudges once per discharge
//! (R6). Remote: nobody touched it (R5). The fill's red (G10) is the glyph's own still state
//! that carries the same fact (R8).

use dioxus::prelude::*;
use ds::detail::{Detailed, FirstShow, Moment, Touch, use_detail, use_nudge};
use ds::{BatteryPower, BatteryState, PulseKey};

/// Whether the battery is in its low band, as the nudge counts crossings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LowWatch {
    /// Armed: not low since it was last plugged in.
    Clear,
    /// Crossed into low on battery; stays until plugged in.
    Low,
}

impl LowWatch {
    /// The watch after the item shows `state` (`None`: no battery).
    pub(crate) fn next(self, state: Option<BatteryState>) -> LowWatch {
        let Some(state) = state else {
            return LowWatch::Clear;
        };
        let under = state.level.0 <= state.low_at.0.0;
        match (self, state.power, under) {
            (_, BatteryPower::Charging | BatteryPower::Held, _) => LowWatch::Clear,
            (LowWatch::Clear, BatteryPower::Battery, true)
            | (LowWatch::Low, BatteryPower::Battery, _) => LowWatch::Low,
            (LowWatch::Clear, BatteryPower::Battery, false) => LowWatch::Clear,
        }
    }
}

impl Detailed for LowWatch {
    fn moment(from: &Self, to: &Self) -> Moment {
        match (from, to) {
            (LowWatch::Clear, LowWatch::Low) => Moment::Attention,
            (LowWatch::Low, LowWatch::Low | LowWatch::Clear)
            | (LowWatch::Clear, LowWatch::Clear) => Moment::Rest,
        }
    }

    fn first(state: &Self) -> Moment {
        match state {
            // Bar chrome has no Appear (R1): a session that starts low has crossed nothing.
            LowWatch::Clear | LowWatch::Low => Moment::Rest,
        }
    }
}

/// The nudge's pulse for the battery item showing `state`: `nudge-up` once per crossing.
pub(crate) fn use_low_nudge(state: Option<BatteryState>) -> PulseKey {
    let mut watch = use_hook(|| CopyValue::new(LowWatch::Clear.next(state)));
    let now = watch.peek().next(state);
    if now != *watch.peek() {
        watch.set(now);
    }
    let detail = use_detail(now, FirstShow::Still, Touch::Remote);
    use_nudge(detail.cue())
}

#[cfg(test)]
mod tests {
    use super::LowWatch;
    use ds::detail::{Moment, first_table, moment_table};
    use ds::{BatteryPower, BatteryState, Fraction, LowAt};

    fn at(level: u16, power: BatteryPower) -> Option<BatteryState> {
        Some(BatteryState {
            level: Fraction(level),
            power,
            low_at: LowAt(Fraction(200)),
        })
    }

    #[test]
    fn a_crossing_is_counted_once_per_discharge() {
        use BatteryPower::{Battery, Charging, Held};
        use LowWatch::{Clear, Low};
        // (name, before, shown, after)
        let cases = [
            ("above stays clear", Clear, at(210, Battery), Clear),
            ("at the threshold is low", Clear, at(200, Battery), Low),
            ("under it is low", Clear, at(150, Battery), Low),
            ("wobbling back up stays low", Low, at(210, Battery), Low),
            ("plugging in re-arms", Low, at(150, Charging), Clear),
            ("held re-arms", Low, at(150, Held), Clear),
            (
                "charging under it is not low",
                Clear,
                at(100, Charging),
                Clear,
            ),
            ("no battery re-arms", Low, None, Clear),
        ];
        for (name, before, shown, want) in cases {
            assert_eq!(before.next(shown), want, "{name}");
        }
    }

    #[test]
    fn only_the_crossing_is_a_moment() {
        use LowWatch::{Clear, Low};
        moment_table(&[
            (Clear, Low, Moment::Attention),
            (Low, Clear, Moment::Rest),
            (Low, Low, Moment::Rest),
            (Clear, Clear, Moment::Rest),
        ]);
        first_table(&[(Clear, Moment::Rest), (Low, Moment::Rest)]);
    }
}
