//! Orientation UDP datagram classification.

/// Known orientation datagram categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatagramKind {
    /// Wand protocol v1.
    WandV1,
    /// Poi protocol.
    Poi,
    /// Wand protocol v2.
    WandV2,
    /// Wristband protocol.
    Wristband,
}

/// Classify a datagram by type byte and expected length.
#[must_use]
pub fn classify_datagram(bytes: &[u8]) -> Option<DatagramKind> {
    let (&kind, rest) = bytes.split_first()?;
    match (kind, rest.len() + 1) {
        (1, 15) => Some(DatagramKind::WandV1),
        (2, 17) => Some(DatagramKind::Poi),
        (3, 15) => Some(DatagramKind::WandV2),
        (4, 15) => Some(DatagramKind::Wristband),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{DatagramKind, classify_datagram};

    #[test]
    fn classifies_known_lengths() {
        assert_eq!(classify_datagram(&[1; 15]), Some(DatagramKind::WandV1));
        assert_eq!(classify_datagram(&[2; 17]), Some(DatagramKind::Poi));
        assert_eq!(classify_datagram(&[3; 15]), Some(DatagramKind::WandV2));
        assert_eq!(classify_datagram(&[4; 15]), Some(DatagramKind::Wristband));
    }

    #[test]
    fn rejects_short_or_unknown_packets() {
        assert_eq!(classify_datagram(&[1; 14]), None);
        assert_eq!(classify_datagram(&[9; 15]), None);
    }
}
