//! Input parsers and fakeable input seams.

pub mod audio;
pub mod madmom;
pub mod midi;
pub mod orientation;

pub use audio::{parse_volume_payload, VolumeReplay};
pub use madmom::{parse_beat_line, MadmomLaunchConfig, MadmomSidecar};
pub use midi::{parse_midi_payload, MidiCommand, MidiCommandKind, MidiReplay};
pub use orientation::{
    classify_datagram, parse_datagram, DatagramKind, OrientationDevice, OrientationInputState,
    OrientationQuaternion, ParsedOrientationDatagram,
};
