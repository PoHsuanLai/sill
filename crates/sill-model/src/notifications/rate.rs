//! A sender's allowance: a token bucket on the wall clock, pure. A sender starts with
//! [`SENDER_BURST`] notifications and earns one more every [`SENDER_REFILL_MS`], so a chat app's
//! burst of messages passes and a loop does not.

use super::ids::UnixMs;
use super::limits::{SENDER_BURST, SENDER_REFILL_MS};

/// What a sender may still post, as of `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allowance {
    tokens: u32,
    at: UnixMs,
}

impl Allowance {
    /// A sender that has not posted before: the whole burst.
    pub fn full(at: UnixMs) -> Allowance {
        Allowance {
            tokens: SENDER_BURST,
            at,
        }
    }

    /// The allowance at `now`, refilled for the time that has passed. A clock that went back
    /// starts counting from `now` again.
    fn refilled(self, now: UnixMs) -> Allowance {
        let elapsed = now.0.saturating_sub(self.at.0);
        if elapsed < 0 {
            return Allowance { at: now, ..self };
        }
        let earned = elapsed / SENDER_REFILL_MS;
        let tokens = i64::from(self.tokens)
            .saturating_add(earned)
            .min(i64::from(SENDER_BURST));
        let tokens = u32::try_from(tokens).unwrap_or(SENDER_BURST);
        let at = if tokens == SENDER_BURST {
            now
        } else {
            UnixMs(self.at.0.saturating_add(earned * SENDER_REFILL_MS))
        };
        Allowance { tokens, at }
    }

    /// One notification taken at `now`: the allowance after it, or the allowance unchanged when
    /// the sender has none left.
    pub fn take(self, now: UnixMs) -> Result<Allowance, Allowance> {
        let held = self.refilled(now);
        match held.tokens.checked_sub(1) {
            Some(tokens) => Ok(Allowance { tokens, ..held }),
            None => Err(held),
        }
    }

    /// Whether the sender has its whole burst back at `now`, so nothing need be remembered of it.
    pub fn is_idle(self, now: UnixMs) -> bool {
        self.refilled(now).tokens == SENDER_BURST
    }

    /// When the allowance was last counted.
    pub fn counted_at(self) -> UnixMs {
        self.at
    }
}

#[cfg(test)]
mod tests {
    use super::Allowance;
    use crate::notifications::UnixMs;
    use crate::notifications::limits::{SENDER_BURST, SENDER_REFILL_MS};

    #[test]
    fn a_burst_passes_then_the_sender_waits_for_each_one() {
        let start = UnixMs(1_000_000);
        let mut allowance = Allowance::full(start);
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
    fn a_long_silence_restores_the_whole_burst_and_a_backwards_clock_is_safe() {
        let start = UnixMs(5_000);
        let spent =
            (0..SENDER_BURST).fold(Allowance::full(start), |a, _| a.take(start).unwrap_or(a));
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
