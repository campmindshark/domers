//! DJ Link / Carabiner-compatible sidecar parsing.

/// Parsed DJ Link tempo event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkTempoEvent {
    /// Tempo in beats per minute.
    pub bpm: f64,
    /// Optional beat phase in `[0.0, 1.0)`.
    pub phase: Option<f64>,
}

/// Parse one DJ Link/Carabiner sidecar stdout line.
///
/// Accepted examples:
///
/// - `LINK 120`
/// - `LINK 120 0.25`
/// - `BPM: 120`
/// - `tempo=120 phase=0.25`
#[must_use]
pub fn parse_link_tempo_line(line: &str) -> Option<LinkTempoEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("LINK ") {
        return parse_positional(rest);
    }
    if let Some(rest) = trimmed.strip_prefix("BPM:") {
        return parse_bpm(rest.trim(), None);
    }

    let mut bpm = None;
    let mut phase = None;
    for token in trimmed.split_ascii_whitespace() {
        if let Some(value) = token
            .strip_prefix("tempo=")
            .or_else(|| token.strip_prefix("bpm="))
        {
            bpm = value.parse::<f64>().ok();
        } else if let Some(value) = token.strip_prefix("phase=") {
            phase = value.parse::<f64>().ok().map(normalize_phase);
        }
    }
    parse_bpm_value(bpm?, phase)
}

fn parse_positional(rest: &str) -> Option<LinkTempoEvent> {
    let mut parts = rest.split_ascii_whitespace();
    let bpm = parts.next()?.parse::<f64>().ok()?;
    let phase = parts.next().and_then(|value| value.parse::<f64>().ok());
    parse_bpm_value(bpm, phase.map(normalize_phase))
}

fn parse_bpm(rest: &str, phase: Option<f64>) -> Option<LinkTempoEvent> {
    parse_bpm_value(rest.parse::<f64>().ok()?, phase)
}

fn parse_bpm_value(bpm: f64, phase: Option<f64>) -> Option<LinkTempoEvent> {
    if bpm.is_finite() && bpm > 0.0 {
        Some(LinkTempoEvent { bpm, phase })
    } else {
        None
    }
}

fn normalize_phase(value: f64) -> f64 {
    value.rem_euclid(1.0)
}

#[cfg(test)]
mod tests {
    use super::{parse_link_tempo_line, LinkTempoEvent};

    #[test]
    fn parses_link_stdout_shapes() {
        assert_eq!(
            parse_link_tempo_line("LINK 120 0.25"),
            Some(LinkTempoEvent {
                bpm: 120.0,
                phase: Some(0.25)
            })
        );
        assert_eq!(
            parse_link_tempo_line("BPM: 90"),
            Some(LinkTempoEvent {
                bpm: 90.0,
                phase: None
            })
        );
        assert_eq!(
            parse_link_tempo_line("tempo=128 phase=1.25"),
            Some(LinkTempoEvent {
                bpm: 128.0,
                phase: Some(0.25)
            })
        );
        assert_eq!(parse_link_tempo_line("tempo=0"), None);
    }
}
