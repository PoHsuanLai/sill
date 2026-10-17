//! An allowance: a token bucket on the wall clock, pure. A caller starts with
//! [`SENDER_BURST`] notifications and earns one more every [`SENDER_REFILL_MS`], so a chat app's
//! burst of messages passes and a loop does not; all callers together have a larger bucket of the
//! same kind ([`Rate::ALL_CALLERS`]), so many callers cannot add up to a flood either.

use super::ids::UnixMs;
use super::limits::{ALL_SENDERS_BURST, ALL_SENDERS_REFILL_MS, SENDER_BURST, SENDER_REFILL_MS};

/// How big a bucket is and how fast it refills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    burst: u32,
    refill_ms: i64,
}

impl Rate {
    /// What one caller may post.
    pub const ONE_CALLER: Rate = Rate {
        burst: SENDER_BURST,
        refill_ms: SENDER_REFILL_MS,
    };

    /// What all callers together may post.
    pub const ALL_CALLERS: Rate = Rate {
        burst: ALL_SENDERS_BURST,
        refill_ms: ALL_SENDERS_REFILL_MS,
    };
}

/// What a caller may still post, as of `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allowance {
    rate: Rate,
    tokens: u32,
    at: UnixMs,
}

impl Allowance {
    /// A caller that has not posted before: the whole burst of `rate`.
    pub fn full(rate: Rate, at: UnixMs) -> Allowance {
        Allowance {
            rate,
            tokens: rate.burst,
            at,
        }
    }

    /// The allowance at `now`, refilled for the time that has passed. A clock that went back
    /// starts counting from `now` again.
    fn refilled(self, now: UnixMs) -> Allowance {
        let Rate { burst, refill_ms } = self.rate;
        let elapsed = now.0.saturating_sub(self.at.0);
        if elapsed < 0 {
            return Allowance { at: now, ..self };
        }
        let earned = elapsed / refill_ms;
        let tokens = i64::from(self.tokens)
            .saturating_add(earned)
            .min(i64::from(burst));
        let tokens = u32::try_from(tokens).unwrap_or(burst);
        let at = if tokens == burst {
            now
        } else {
            UnixMs(self.at.0.saturating_add(earned * refill_ms))
        };
        Allowance { tokens, at, ..self }
    }

    /// One notification taken at `now`: the allowance after it, or the allowance unchanged when
    /// the caller has none left.
    pub fn take(self, now: UnixMs) -> Result<Allowance, Allowance> {
        let held = self.refilled(now);
        match held.tokens.checked_sub(1) {
            Some(tokens) => Ok(Allowance { tokens, ..held }),
            None => Err(held),
        }
    }

    /// Whether the caller has its whole burst back at `now`, so nothing need be remembered of it.
    pub fn is_idle(self, now: UnixMs) -> bool {
        self.refilled(now).tokens == self.rate.burst
    }

    /// When the allowance was last counted.
    pub fn counted_at(self) -> UnixMs {
        self.at
    }
}

#[cfg(test)]
mod tests {
    use super::{Allowance, Rate};
    use crate::notifications::UnixMs;
    use crate::notifications::limits::{ALL_SENDERS_BURST, SENDER_BURST, SENDER_REFILL_MS};

    #[test]
    fn a_burst_passes_then_the_sender_waits_for_each_one() {
        let start = UnixMs(1_000_000);
        let mut allowance = Allowance::full(Rate::ONE_CALLER, start);
        for n in 0..SENDER_BURST {
            allowance = allowance
                .take(start)
                .unwrap_or_else(|_| panic!("post {n} of the burst"));
        }
        let refused = allowance.take(start);
        assert!(refused.is_err(), "the burst is spent");
        let later = UnixMs(start.0 + SENDER_REFILL_MS);
        let one_more = allowance.take(later);
        assert!(one_more.is_ok(), "one refill earns one");
        let after = one_more.unwrap_or(allowance);
        assert!(after.take(later).is_err(), "and only one");
    }

    #[test]
    fn the_shared_bucket_outlasts_one_callers_and_still_runs_out() {
        let start = UnixMs(5_000);
        let drain = |rate, takes| {
            (0..takes).fold(Allowance::full(rate, start), |a, _| {
                a.take(start).unwrap_or(a)
            })
        };
        assert!(drain(Rate::ONE_CALLER, SENDER_BURST).take(start).is_err());
        assert!(drain(Rate::ALL_CALLERS, SENDER_BURST).take(start).is_ok());
        assert!(
            drain(Rate::ALL_CALLERS, ALL_SENDERS_BURST)
                .take(start)
                .is_err()
        );
    }

    #[test]
    fn a_long_silence_restores_the_whole_burst_and_a_backwards_clock_is_safe() {
        let start = UnixMs(5_000);
        let spent = (0..SENDER_BURST).fold(Allowance::full(Rate::ONE_CALLER, start), |a, _| {
            a.take(start).unwrap_or(a)
        });
        assert!(!spent.is_idle(start));
        let quiet = UnixMs(start.0 + SENDER_REFILL_MS * i64::from(SENDER_BURST));
        assert!(spent.is_idle(quiet));
        let back = UnixMs(start.0 - 60_000);
        assert!(
            spent.take(back).is_err(),
            "no tokens appear from a clock that went back"
        );
        assert_eq!(spent.refilled(back).counted_at(), back);
    }
}
