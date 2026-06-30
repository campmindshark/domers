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
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "Beat progress mirrors Spectrum's floating timing controls and clamps the period before integer modulo"
    )]
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

/// Deterministic beat broadcaster state mirroring Spectrum's core semantics.
#[derive(Clone, Debug, Default)]
pub struct BeatBroadcaster {
    taps: Vec<u64>,
    madmom_beats: Vec<u64>,
    clock: Option<BeatClock>,
}

impl BeatBroadcaster {
    const TAP_TIMEOUT_MS: u64 = 2_000;
    const MADMOM_TIMEOUT_MS: u64 = 2_500;
    const MADMOM_WINDOW: usize = 8;

    /// Add a human tap timestamp.
    pub fn add_tap(&mut self, timestamp_ms: u64) {
        if self
            .taps
            .last()
            .is_some_and(|last| timestamp_ms.saturating_sub(*last) > Self::TAP_TIMEOUT_MS)
        {
            self.taps.clear();
        }
        self.taps.push(timestamp_ms);
        if self.taps.len() >= 3 {
            let intervals: Vec<_> = self.taps.windows(2).map(|pair| pair[1] - pair[0]).collect();
            let average = intervals.iter().sum::<u64>() / intervals.len() as u64;
            self.clock = Some(BeatClock::new(average, timestamp_ms));
            self.madmom_beats.clear();
        }
    }

    /// Report a Madmom beat timestamp in audio-stream milliseconds.
    pub fn report_madmom_beat(&mut self, beat_ms: u64, realtime_anchor_ms: u64) {
        if self
            .madmom_beats
            .last()
            .is_some_and(|last| beat_ms.saturating_sub(*last) > Self::MADMOM_TIMEOUT_MS)
        {
            self.madmom_beats.clear();
        }
        self.madmom_beats.push(beat_ms);
        if self.madmom_beats.len() > Self::MADMOM_WINDOW {
            self.madmom_beats.remove(0);
        }
        if self.madmom_beats.len() >= 2 {
            let intervals: Vec<_> = self
                .madmom_beats
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .collect();
            let average = intervals.iter().sum::<u64>() / intervals.len() as u64;
            self.clock = Some(BeatClock::new(average, realtime_anchor_ms));
            self.taps.clear();
        }
    }

    /// Beat length in milliseconds, if known.
    #[must_use]
    pub fn beat_ms(&self) -> Option<u64> {
        self.clock.as_ref().map(|clock| clock.beat_ms)
    }

    /// Progress through a beat-like factor.
    #[must_use]
    pub fn progress(&self, now_ms: u64, factor: f64) -> f64 {
        self.clock
            .as_ref()
            .map_or(0.0, |clock| clock.progress(now_ms, factor))
    }

    /// Whether output should currently black out for a flash-speed gate.
    #[must_use]
    pub fn currently_flashed_off(&self, now_ms: u64, flash_speed: f64) -> bool {
        flash_speed != 0.0 && self.progress(now_ms, flash_speed) >= 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::{BeatBroadcaster, BeatClock};

    fn assert_close(left: f64, right: f64) {
        assert!((left - right).abs() < f64::EPSILON);
    }

    #[test]
    fn reports_progress_through_beat() {
        let clock = BeatClock::new(1_000, 100);
        assert_close(clock.progress(100, 1.0), 0.0);
        assert_close(clock.progress(600, 1.0), 0.5);
        assert_close(clock.progress(1_100, 1.0), 0.0);
    }

    #[test]
    fn tap_tempo_sets_average_beat_length() {
        let mut beat = BeatBroadcaster::default();
        beat.add_tap(1_000);
        beat.add_tap(1_500);
        beat.add_tap(2_000);

        assert_eq!(beat.beat_ms(), Some(500));
        assert!(beat.currently_flashed_off(2_250, 1.0));
    }

    #[test]
    fn madmom_beats_replace_tap_window_with_realtime_anchor() {
        let mut beat = BeatBroadcaster::default();
        beat.add_tap(1_000);
        beat.add_tap(1_500);
        beat.add_tap(2_000);

        beat.report_madmom_beat(10_000, 5_000);
        beat.report_madmom_beat(10_400, 5_400);
        beat.report_madmom_beat(10_800, 5_800);

        assert_eq!(beat.beat_ms(), Some(400));
        assert_close(beat.progress(6_000, 1.0), 0.5);
    }

    #[test]
    fn madmom_timeout_starts_a_new_window() {
        let mut beat = BeatBroadcaster::default();
        beat.report_madmom_beat(1_000, 1_000);
        beat.report_madmom_beat(1_500, 1_500);
        assert_eq!(beat.beat_ms(), Some(500));

        beat.report_madmom_beat(5_000, 5_000);
        assert_eq!(beat.beat_ms(), Some(500));
        beat.report_madmom_beat(5_250, 5_250);
        assert_eq!(beat.beat_ms(), Some(250));
    }
}
