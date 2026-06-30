//! Shared fake test utilities for no-hardware CI.

/// Deterministic fake clock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FakeClock {
    now_ms: u64,
}

impl FakeClock {
    /// Current fake timestamp in milliseconds.
    #[must_use]
    pub const fn now_ms(self) -> u64 {
        self.now_ms
    }

    /// Advance the fake clock.
    pub fn advance_ms(&mut self, ms: u64) {
        self.now_ms = self.now_ms.saturating_add(ms);
    }
}

#[cfg(test)]
mod tests {
    use super::FakeClock;

    #[test]
    fn advances_deterministically() {
        let mut clock = FakeClock::default();
        clock.advance_ms(42);
        assert_eq!(clock.now_ms(), 42);
    }
}
