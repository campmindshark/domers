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
        let period = (self.beat_ms as f64 / factor).max(1.0) as u64;
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
    const TAP_WINDOW: usize = 4;
    const MADMOM_TIMEOUT_MS: u64 = 2_500;
    const MADMOM_WINDOW: usize = 4;
    const OUTLIER_NUMERATOR: u64 = 3;
    const OUTLIER_DENOMINATOR: u64 = 2;

    /// Add a human tap timestamp.
    pub fn add_tap(&mut self, timestamp_ms: u64) {
        if let Some(last) = self.taps.last().copied() {
            let interval = timestamp_ms.saturating_sub(last);
            if timestamp_ms <= last
                || interval > Self::TAP_TIMEOUT_MS
                || (self.taps.len() >= 3 && self.interval_is_outlier(interval))
            {
                self.taps.clear();
            }
        }
        self.taps.push(timestamp_ms);
        if self.taps.len() > Self::TAP_WINDOW {
            self.taps.remove(0);
        }
        if self.taps.len() >= 3 {
            let intervals: Vec<_> = self.taps.windows(2).map(|pair| pair[1] - pair[0]).collect();
            let average = intervals.iter().sum::<u64>() / intervals.len() as u64;
            self.clock = Some(BeatClock::new(average, timestamp_ms));
            self.madmom_beats.clear();
        }
    }

    /// Reset beat/tap/Madmom state.
    pub fn reset(&mut self) {
        self.taps.clear();
        self.madmom_beats.clear();
        self.clock = None;
    }

    /// Report a Madmom beat timestamp in audio-stream milliseconds.
    pub fn report_madmom_beat(&mut self, beat_ms: u64, realtime_anchor_ms: u64) {
        if let Some(last) = self.madmom_beats.last() {
            let since_last = i128::from(beat_ms) - i128::from(*last);
            if since_last <= 0 || since_last > i128::from(Self::MADMOM_TIMEOUT_MS) {
                self.madmom_beats.clear();
            } else if let Ok(interval) = u64::try_from(since_last) {
                if self.interval_is_outlier(interval) {
                    self.madmom_beats.clear();
                }
            } else {
                self.madmom_beats.clear();
            }
        }
        self.madmom_beats.push(beat_ms);
        if self.madmom_beats.len() > Self::MADMOM_WINDOW {
            self.madmom_beats.remove(0);
        }
        if self.madmom_beats.len() >= 2 {
            let mut intervals: Vec<_> = self
                .madmom_beats
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .collect();
            intervals.sort_unstable();
            let mid = intervals.len() / 2;
            let median = if intervals.len() % 2 == 1 {
                intervals[mid]
            } else {
                (intervals[mid - 1] + intervals[mid]) / 2
            };
            self.clock = Some(BeatClock::new(median, realtime_anchor_ms));
            self.taps.clear();
        }
    }

    fn interval_is_outlier(&self, interval_ms: u64) -> bool {
        let established_tap_window = self.taps.len() >= 3;
        let established_madmom_window = self.madmom_beats.len() >= 2;
        if !established_tap_window && !established_madmom_window {
            return false;
        }
        let Some(beat_ms) = self.beat_ms() else {
            return false;
        };
        interval_outside_ratio(interval_ms, beat_ms)
    }

    /// Report an Ableton Link tempo update.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "Link BPM values are positive musical tempos converted to millisecond periods"
    )]
    pub fn report_link_tempo(&mut self, bpm: f64, phase: Option<f64>, realtime_ms: u64) {
        if !bpm.is_finite() || bpm <= 0.0 {
            return;
        }
        let beat_ms = (60_000.0 / bpm).round().max(1.0) as u64;
        let phase_offset_ms = phase.map_or(0, |value| {
            (value.rem_euclid(1.0) * beat_ms as f64).round() as u64
        });
        let anchor_ms = realtime_ms.saturating_sub(phase_offset_ms);
        self.clock = Some(BeatClock::new(beat_ms, anchor_ms));
        self.taps.clear();
        self.madmom_beats.clear();
    }

    /// Beat length in milliseconds, if known.
    #[must_use]
    pub fn beat_ms(&self) -> Option<u64> {
        self.clock.as_ref().map(|clock| clock.beat_ms)
    }

    /// Spectrum-style BPM display string.
    #[must_use]
    pub fn bpm_string(&self) -> String {
        self.beat_ms().map_or_else(
            || "[none]".to_string(),
            |beat_ms| (60_000 / beat_ms).to_string(),
        )
    }

    /// Spectrum-style tap counter text.
    #[must_use]
    pub fn tap_counter_text(&self, now_ms: u64) -> String {
        if self.tap_tempo_concluded(now_ms) {
            "Tap".to_string()
        } else {
            self.taps.len().to_string()
        }
    }

    /// Whether the tap tempo counter should display as active.
    #[must_use]
    pub fn tap_counter_active(&self, now_ms: u64) -> bool {
        !self.tap_tempo_concluded(now_ms)
    }

    fn tap_tempo_concluded(&self, now_ms: u64) -> bool {
        self.taps.last().map_or(true, |last| {
            now_ms.saturating_sub(*last) > Self::TAP_TIMEOUT_MS
        })
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

fn interval_outside_ratio(interval_ms: u64, beat_ms: u64) -> bool {
    interval_ms.saturating_mul(BeatBroadcaster::OUTLIER_DENOMINATOR)
        > beat_ms.saturating_mul(BeatBroadcaster::OUTLIER_NUMERATOR)
        || interval_ms.saturating_mul(BeatBroadcaster::OUTLIER_NUMERATOR)
            < beat_ms.saturating_mul(BeatBroadcaster::OUTLIER_DENOMINATOR)
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
    fn progress_truncates_fractional_period_like_spectrum() {
        let clock = BeatClock::new(1_000, 0);

        assert_close(clock.progress(333, 3.0), 0.0);
        assert_close(clock.progress(334, 3.0), 1.0 / 333.0);
    }

    #[test]
    fn tap_tempo_sets_average_beat_length() {
        let mut beat = BeatBroadcaster::default();
        beat.add_tap(1_000);
        beat.add_tap(1_500);
        beat.add_tap(2_000);

        assert_eq!(beat.beat_ms(), Some(500));
        assert_eq!(beat.bpm_string(), "120");
        assert_eq!(beat.tap_counter_text(2_100), "3");
        assert!(beat.tap_counter_active(2_100));
        assert_eq!(beat.tap_counter_text(4_100), "Tap");
        assert!(beat.currently_flashed_off(2_250, 1.0));
    }

    #[test]
    fn tap_tempo_uses_short_rolling_window() {
        let mut beat = BeatBroadcaster::default();
        beat.add_tap(0);
        beat.add_tap(500);
        beat.add_tap(1_000);
        beat.add_tap(1_500);
        beat.add_tap(2_100);

        assert_eq!(beat.beat_ms(), Some(533));
    }

    #[test]
    fn tap_tempo_outlier_starts_fresh_window() {
        let mut beat = BeatBroadcaster::default();
        beat.add_tap(0);
        beat.add_tap(500);
        beat.add_tap(1_000);
        assert_eq!(beat.beat_ms(), Some(500));

        beat.add_tap(2_000);
        assert_eq!(beat.beat_ms(), Some(500));
        assert_eq!(beat.tap_counter_text(2_000), "1");

        beat.add_tap(3_000);
        assert_eq!(beat.beat_ms(), Some(500));
        beat.add_tap(4_000);
        assert_eq!(beat.beat_ms(), Some(1_000));
    }

    #[test]
    fn tap_tempo_zero_interval_resets_window() {
        let mut beat = BeatBroadcaster::default();
        beat.add_tap(1_000);
        beat.add_tap(1_000);
        beat.add_tap(1_500);
        beat.add_tap(2_000);

        assert_eq!(beat.beat_ms(), Some(500));
    }

    #[test]
    fn reset_clears_tempo_state() {
        let mut beat = BeatBroadcaster::default();
        beat.add_tap(1_000);
        beat.add_tap(1_500);
        beat.add_tap(2_000);
        assert_eq!(beat.bpm_string(), "120");

        beat.reset();

        assert_eq!(beat.beat_ms(), None);
        assert_eq!(beat.bpm_string(), "[none]");
        assert_close(beat.progress(2_500, 1.0), 0.0);
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

    #[test]
    fn madmom_uses_short_rolling_window() {
        let mut beat = BeatBroadcaster::default();
        beat.report_madmom_beat(1_000, 1_000);
        beat.report_madmom_beat(1_400, 1_400);
        beat.report_madmom_beat(1_800, 1_800);
        beat.report_madmom_beat(2_200, 2_200);
        beat.report_madmom_beat(2_650, 2_650);

        assert_eq!(beat.beat_ms(), Some(400));

        beat.report_madmom_beat(3_100, 3_100);
        assert_eq!(beat.beat_ms(), Some(450));
    }

    #[test]
    fn madmom_outlier_starts_fresh_window() {
        let mut beat = BeatBroadcaster::default();
        beat.report_madmom_beat(1_000, 1_000);
        beat.report_madmom_beat(1_400, 1_400);
        beat.report_madmom_beat(1_800, 1_800);
        assert_eq!(beat.beat_ms(), Some(400));

        beat.report_madmom_beat(2_800, 2_800);
        assert_eq!(beat.beat_ms(), Some(400));

        beat.report_madmom_beat(3_800, 3_800);
        assert_eq!(beat.beat_ms(), Some(1_000));
    }

    #[test]
    fn madmom_backwards_timestamp_starts_a_new_window() {
        let mut beat = BeatBroadcaster::default();
        beat.report_madmom_beat(10_000, 10_000);
        beat.report_madmom_beat(10_500, 10_500);
        assert_eq!(beat.beat_ms(), Some(500));

        beat.report_madmom_beat(100, 11_000);
        assert_eq!(beat.beat_ms(), Some(500));
        beat.report_madmom_beat(500, 11_400);
        assert_eq!(beat.beat_ms(), Some(400));
    }

    #[test]
    fn madmom_uses_median_interval() {
        let mut beat = BeatBroadcaster::default();
        beat.report_madmom_beat(1_000, 1_000);
        beat.report_madmom_beat(1_400, 1_400);
        beat.report_madmom_beat(1_800, 1_800);
        beat.report_madmom_beat(2_900, 2_900);
        beat.report_madmom_beat(3_300, 3_300);

        assert_eq!(beat.beat_ms(), Some(400));
    }

    #[test]
    fn link_tempo_sets_bpm_and_phase() {
        let mut beat = BeatBroadcaster::default();

        beat.report_link_tempo(120.0, Some(0.25), 10_000);

        assert_eq!(beat.beat_ms(), Some(500));
        assert_eq!(beat.bpm_string(), "120");
        assert_close(beat.progress(10_000, 1.0), 0.25);
    }
}
