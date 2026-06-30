//! Fakeable audio volume input.

/// Deterministic volume replay for no-hardware tests.
#[derive(Clone, Debug)]
pub struct VolumeReplay {
    samples: Vec<f32>,
    cursor: usize,
}

impl VolumeReplay {
    /// Create a volume replay stream.
    #[must_use]
    pub fn new(samples: Vec<f32>) -> Self {
        Self { samples, cursor: 0 }
    }

    /// Return the next volume sample, clamped to `[0.0, 1.0]`.
    pub fn next_volume(&mut self) -> Option<f32> {
        let sample = *self.samples.get(self.cursor)?;
        self.cursor += 1;
        Some(sample.clamp(0.0, 1.0))
    }
}

/// Parse one live audio volume payload.
#[must_use]
pub fn parse_volume_payload(payload: &[u8]) -> Option<f32> {
    let text = std::str::from_utf8(payload).ok()?.trim();
    let volume = text.parse::<f32>().ok()?;
    Some(volume.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::{parse_volume_payload, VolumeReplay};

    #[test]
    fn replays_clamped_volume_samples() {
        let mut replay = VolumeReplay::new(vec![-1.0, 0.25, 2.0]);
        assert_eq!(replay.next_volume(), Some(0.0));
        assert_eq!(replay.next_volume(), Some(0.25));
        assert_eq!(replay.next_volume(), Some(1.0));
        assert_eq!(replay.next_volume(), None);
    }

    #[test]
    fn parses_live_volume_payloads() {
        assert_eq!(parse_volume_payload(b"0.25\n"), Some(0.25));
        assert_eq!(parse_volume_payload(b"2.0"), Some(1.0));
        assert_eq!(parse_volume_payload(b"noise"), None);
    }
}
