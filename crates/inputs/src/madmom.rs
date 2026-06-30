//! Madmom sidecar protocol helpers.

/// Parse a `BEAT:{seconds}` stdout line into milliseconds.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Madmom emits fractional seconds; Spectrum truncates to integer milliseconds"
)]
pub fn parse_beat_line(line: &str) -> Option<u64> {
    let value = line.strip_prefix("BEAT:")?;
    let seconds = value.trim().parse::<f64>().ok()?;
    if seconds.is_sign_negative() || !seconds.is_finite() {
        return None;
    }
    Some((seconds * 1_000.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::parse_beat_line;

    #[test]
    fn parses_valid_beat_lines() {
        assert_eq!(parse_beat_line("BEAT:12.345"), Some(12_345));
    }

    #[test]
    fn drops_malformed_beat_lines() {
        assert_eq!(parse_beat_line("noise"), None);
        assert_eq!(parse_beat_line("BEAT:not-a-number"), None);
        assert_eq!(parse_beat_line("BEAT:-1"), None);
    }
}
