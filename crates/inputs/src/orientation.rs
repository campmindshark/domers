//! Orientation UDP datagram parsing and Spectrum-style device state.

use std::collections::BTreeMap;

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

/// Quaternion value in Spectrum's `(x, y, z, w)` layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrientationQuaternion {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
    /// Z component.
    pub z: f32,
    /// W component.
    pub w: f32,
}

impl OrientationQuaternion {
    /// Identity rotation.
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    /// Return the conjugate/inverse for the unit quaternions emitted by the devices.
    #[must_use]
    pub const fn inverse(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    /// Quaternion multiply matching `System.Numerics.Quaternion.Multiply`.
    #[must_use]
    pub fn multiply(self, rhs: Self) -> Self {
        Self {
            x: self.w.mul_add(
                rhs.x,
                self.x.mul_add(rhs.w, self.y * rhs.z - self.z * rhs.y),
            ),
            y: self.w.mul_add(
                rhs.y,
                (-self.x).mul_add(rhs.z, self.y * rhs.w + self.z * rhs.x),
            ),
            z: self.w.mul_add(
                rhs.z,
                self.x.mul_add(rhs.y, -self.y * rhs.x + self.z * rhs.w),
            ),
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        }
    }
}

/// Parsed orientation device datagram.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedOrientationDatagram {
    /// Device id from byte 0.
    pub device_id: u8,
    /// Device timestamp from bytes 1-4.
    pub timestamp: i32,
    /// Device kind.
    pub kind: DatagramKind,
    /// Raw Spectrum device type.
    pub device_type: u8,
    /// Current orientation.
    pub orientation: OrientationQuaternion,
    /// Wand/wristband action flag. Poi reports zero.
    pub action_flag: u8,
    /// Poi angular distance, when present.
    pub avg_distance_short: Option<f64>,
}

/// Runtime orientation device state.
#[derive(Clone, Debug, PartialEq)]
pub struct OrientationDevice {
    /// Device id from the datagram.
    pub device_id: u8,
    /// Last accepted device timestamp.
    pub timestamp: i32,
    /// Device kind.
    pub kind: DatagramKind,
    /// Raw Spectrum device type.
    pub device_type: u8,
    /// Calibration origin.
    pub calibration_origin: OrientationQuaternion,
    /// Current orientation.
    pub current_orientation: OrientationQuaternion,
    /// Last poi angular distance. Zero for devices without speed.
    pub avg_distance_short: f64,
    /// Whether this device has speed data.
    pub has_speed: bool,
    /// Current Spectrum action flag.
    pub action_flag: u8,
}

impl OrientationDevice {
    /// Calibrate this device to its current orientation.
    pub fn calibrate(&mut self) {
        self.calibration_origin = self.current_orientation;
    }

    /// Current calibrated rotation, matching Spectrum's inverse(current) * calibration.
    #[must_use]
    pub fn current_rotation(&self) -> OrientationQuaternion {
        self.current_orientation
            .inverse()
            .multiply(self.calibration_origin)
    }

    fn from_datagram(datagram: &ParsedOrientationDatagram) -> Self {
        Self {
            device_id: datagram.device_id,
            timestamp: datagram.timestamp,
            kind: datagram.kind,
            device_type: datagram.device_type,
            calibration_origin: OrientationQuaternion::IDENTITY,
            current_orientation: datagram.orientation,
            avg_distance_short: datagram.avg_distance_short.unwrap_or(0.0),
            has_speed: datagram.avg_distance_short.is_some(),
            action_flag: 0,
        }
    }
}

/// Spectrum-style orientation input state.
#[derive(Clone, Debug, PartialEq)]
pub struct OrientationInputState {
    devices: BTreeMap<u8, OrientationDevice>,
    last_seen_ms: BTreeMap<u8, u64>,
    last_event_ms: [u64; 256],
    last_checked_devices_ms: u64,
}

impl Default for OrientationInputState {
    fn default() -> Self {
        Self {
            devices: BTreeMap::new(),
            last_seen_ms: BTreeMap::new(),
            last_event_ms: [0; 256],
            last_checked_devices_ms: 0,
        }
    }
}

impl OrientationInputState {
    /// Spectrum removes devices after this much wall-clock silence.
    pub const DEVICE_TIMEOUT_MS: u64 = 1_000;
    /// Spectrum debounces action flags per device with this timeout.
    pub const DEVICE_EVENT_TIMEOUT_MS: u64 = 5;

    /// Process one datagram at a deterministic wall-clock timestamp.
    pub fn process_datagram(&mut self, bytes: &[u8], now_ms: u64) -> Option<DatagramKind> {
        let datagram = parse_datagram(bytes)?;
        let device_id = datagram.device_id;
        self.last_seen_ms.insert(device_id, now_ms);

        let Some(device) = self.devices.get_mut(&device_id) else {
            self.devices
                .insert(device_id, OrientationDevice::from_datagram(&datagram));
            return Some(datagram.kind);
        };
        let action_flag = datagram.action_flag;
        if action_flag != 0 {
            if now_ms.saturating_sub(self.last_event_ms[usize::from(device_id)])
                > Self::DEVICE_EVENT_TIMEOUT_MS
            {
                self.last_event_ms[usize::from(device_id)] = now_ms;
                if action_flag == 4 {
                    device.calibrate();
                } else if (1..=3).contains(&action_flag) {
                    device.action_flag = action_flag;
                }
            }
        } else {
            device.action_flag = 0;
        }

        if datagram.timestamp > device.timestamp || datagram.timestamp < device.timestamp - 1_000 {
            device.timestamp = datagram.timestamp;
            device.current_orientation = datagram.orientation;
            device.avg_distance_short = datagram.avg_distance_short.unwrap_or(0.0);
            device.has_speed = datagram.avg_distance_short.is_some();
        }

        Some(datagram.kind)
    }

    /// Calibrate every active device.
    pub fn calibrate_all(&mut self) {
        for device in self.devices.values_mut() {
            device.calibrate();
        }
    }

    /// Remove devices that Spectrum would consider stale.
    pub fn remove_stale_devices(&mut self, now_ms: u64) {
        if now_ms.saturating_sub(self.last_checked_devices_ms) <= Self::DEVICE_TIMEOUT_MS {
            return;
        }
        let stale: Vec<_> = self
            .last_seen_ms
            .iter()
            .filter_map(|(device_id, last_seen)| {
                (now_ms.saturating_sub(*last_seen) > Self::DEVICE_TIMEOUT_MS).then_some(*device_id)
            })
            .collect();
        for device_id in stale {
            self.devices.remove(&device_id);
            self.last_seen_ms.remove(&device_id);
        }
        self.last_checked_devices_ms = now_ms;
    }

    /// Snapshot active devices in stable device-id order.
    #[must_use]
    pub fn devices(&self) -> Vec<OrientationDevice> {
        let mut devices: Vec<_> = self.devices.values().cloned().collect();
        devices.sort_by_key(|device| device.device_id);
        devices
    }

    /// Spectrum `OrientationInput.onlyPoi()`.
    #[must_use]
    pub fn only_poi(&self) -> bool {
        !self.devices.is_empty() && self.devices.values().all(|device| device.device_type == 2)
    }

    /// Number of active devices.
    #[must_use]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}

/// Minimum bytes Spectrum reads before type-specific unpacking.
pub const MIN_DATAGRAM_LENGTH: usize = 6;

/// Return Spectrum's required datagram length for one device type.
#[must_use]
pub const fn required_datagram_length(device_type: u8) -> usize {
    match device_type {
        1 | 3 | 4 => 15,
        2 => 17,
        _ => MIN_DATAGRAM_LENGTH,
    }
}

/// Classify a datagram by Spectrum's device-type byte and expected length.
///
/// Spectrum stores device id at byte 0, timestamp at bytes 1-4, and device type
/// at byte 5. The earlier bytes are not the type.
#[must_use]
pub fn classify_datagram(bytes: &[u8]) -> Option<DatagramKind> {
    parse_datagram(bytes).map(|datagram| datagram.kind)
}

/// Parse one Spectrum orientation datagram.
#[must_use]
pub fn parse_datagram(bytes: &[u8]) -> Option<ParsedOrientationDatagram> {
    let device_id = *bytes.first()?;
    let device_type = *bytes.get(5)?;
    if bytes.len() < required_datagram_length(device_type) {
        return None;
    }
    let kind = match device_type {
        1 => Some(DatagramKind::WandV1),
        2 => Some(DatagramKind::Poi),
        3 => Some(DatagramKind::WandV2),
        4 => Some(DatagramKind::Wristband),
        _ => None,
    }?;
    let timestamp = i32::from_le_bytes(bytes[1..5].try_into().ok()?);
    let w = f32::from(i16::from_le_bytes(bytes[6..8].try_into().ok()?)) / 16_384.0;
    let x = f32::from(i16::from_le_bytes(bytes[8..10].try_into().ok()?)) / 16_384.0;
    let y = f32::from(i16::from_le_bytes(bytes[10..12].try_into().ok()?)) / 16_384.0;
    let z = f32::from(i16::from_le_bytes(bytes[12..14].try_into().ok()?)) / 16_384.0;
    let avg_distance_short = (kind == DatagramKind::Poi)
        .then(|| f64::from(u16::from_le_bytes([bytes[15], bytes[16]])) / 65_536.0);
    Some(ParsedOrientationDatagram {
        device_id,
        timestamp,
        kind,
        device_type,
        orientation: OrientationQuaternion { x, y, z, w },
        action_flag: if kind == DatagramKind::Poi {
            0
        } else {
            bytes[14]
        },
        avg_distance_short,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        classify_datagram, parse_datagram, DatagramKind, OrientationInputState,
        OrientationQuaternion,
    };

    #[test]
    fn classifies_known_lengths() {
        assert_eq!(
            classify_datagram(&datagram(42, 1, 15)),
            Some(DatagramKind::WandV1)
        );
        assert_eq!(
            classify_datagram(&datagram(42, 2, 17)),
            Some(DatagramKind::Poi)
        );
        assert_eq!(
            classify_datagram(&datagram(42, 3, 15)),
            Some(DatagramKind::WandV2)
        );
        assert_eq!(
            classify_datagram(&datagram(42, 4, 15)),
            Some(DatagramKind::Wristband)
        );
    }

    #[test]
    fn rejects_short_or_unknown_packets() {
        assert_eq!(classify_datagram(&datagram(42, 1, 14)), None);
        assert_eq!(classify_datagram(&datagram(42, 9, 15)), None);
        assert_eq!(classify_datagram(&[1, 2, 3, 4, 5]), None);
    }

    #[test]
    fn parses_spectrum_quaternion_and_action_flag() {
        let bytes = wand_datagram(7, 1_234, 1, [16_384, 1_638, -3_276, 8_192], 3);
        let parsed = parse_datagram(&bytes).expect("valid datagram");

        assert_eq!(parsed.device_id, 7);
        assert_eq!(parsed.timestamp, 1_234);
        assert_eq!(parsed.kind, DatagramKind::WandV1);
        assert_eq!(parsed.action_flag, 3);
        assert_close(parsed.orientation.w, 1.0);
        assert_close(parsed.orientation.x, 0.099_975_586);
        assert_close(parsed.orientation.y, -0.199_951_17);
        assert_close(parsed.orientation.z, 0.5);
    }

    #[test]
    fn parses_poi_speed_and_ignores_action_flag() {
        let mut bytes = wand_datagram(8, 2_000, 2, [16_384, 0, 0, 0], 9);
        bytes.resize(17, 0);
        bytes[15..17].copy_from_slice(&32_768_u16.to_le_bytes());
        let parsed = parse_datagram(&bytes).expect("valid poi datagram");

        assert_eq!(parsed.kind, DatagramKind::Poi);
        assert_eq!(parsed.action_flag, 0);
        assert_eq!(parsed.avg_distance_short, Some(0.5));
    }

    #[test]
    fn tracks_devices_actions_calibration_and_timeout() {
        let mut state = OrientationInputState::default();
        let first = wand_datagram(7, 1_000, 1, [16_384, 0, 0, 0], 1);
        assert_eq!(
            state.process_datagram(&first, 10),
            Some(DatagramKind::WandV1)
        );
        assert_eq!(state.device_count(), 1);
        assert_eq!(state.devices()[0].action_flag, 0);

        let action = wand_datagram(7, 1_001, 1, [16_384, 0, 0, 0], 1);
        assert_eq!(
            state.process_datagram(&action, 20),
            Some(DatagramKind::WandV1)
        );
        assert_eq!(state.devices()[0].action_flag, 1);

        let calibration = wand_datagram(7, 1_002, 1, [0, 16_384, 0, 0], 4);
        assert_eq!(
            state.process_datagram(&calibration, 30),
            Some(DatagramKind::WandV1)
        );
        assert_eq!(
            state.devices()[0].calibration_origin,
            OrientationQuaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0
            }
        );

        state.remove_stale_devices(900);
        assert_eq!(state.device_count(), 1);
        state.remove_stale_devices(1_100);
        assert_eq!(state.device_count(), 0);
    }

    fn datagram(device_id: u8, device_type: u8, length: usize) -> Vec<u8> {
        let mut bytes = vec![0; length];
        bytes[0] = device_id;
        if length > 5 {
            bytes[5] = device_type;
        }
        bytes
    }

    fn wand_datagram(
        device_id: u8,
        timestamp: i32,
        device_type: u8,
        quaternion: [i16; 4],
        action_flag: u8,
    ) -> Vec<u8> {
        let mut bytes = datagram(device_id, device_type, required_len(device_type));
        bytes[1..5].copy_from_slice(&timestamp.to_le_bytes());
        bytes[6..8].copy_from_slice(&quaternion[0].to_le_bytes());
        bytes[8..10].copy_from_slice(&quaternion[1].to_le_bytes());
        bytes[10..12].copy_from_slice(&quaternion[2].to_le_bytes());
        bytes[12..14].copy_from_slice(&quaternion[3].to_le_bytes());
        bytes[14] = action_flag;
        bytes
    }

    const fn required_len(device_type: u8) -> usize {
        match device_type {
            2 => 17,
            _ => 15,
        }
    }

    fn assert_close(left: f32, right: f32) {
        assert!((left - right).abs() < 0.000_001, "{left} != {right}");
    }
}
