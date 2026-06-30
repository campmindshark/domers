//! Beat timing primitives.

/// Minimal deterministic beat clock used by early tests and fake inputs.
#[derive(Clone, Debug)]
pub struct BeatClock {
    beat_ms: u64,
    anchor_ms: u64,
}

impl BeatClock {
    /// Create a clock with a beat length and anchor timestamp.
    #[must_use]
    pub const fn new(beat_ms: u64, anchor_ms: u64) -> Self {
        Self { beat_ms, anchor_ms }
    }

    /// Returns progress through a beat-like period in `[0.0, 1.0)`.
    #[must_use]
    pub fn progress(&self, now_ms: u64, factor: f64) -> f64 {
        if self.beat_ms == 0 || factor == 0.0 {
            return 0.0;
        }
        let period = (self.beat_ms as f64 / factor).round().max(1.0) as u64;
        ((now_ms.saturating_sub(self.anchor_ms) % period) as f64) / period as f64
    }
}

impl Default for BeatClock {
    fn default() -> Self {
        Self::new(500, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::BeatClock;

    #[test]
    fn reports_progress_through_beat() {
        let clock = BeatClock::new(1_000, 100);
        assert_eq!(clock.progress(100, 1.0), 0.0);
        assert_eq!(clock.progress(600, 1.0), 0.5);
        assert_eq!(clock.progress(1_100, 1.0), 0.0);
    }
}
