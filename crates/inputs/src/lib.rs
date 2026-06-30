//! Input parsers and fakeable input seams.

pub mod audio;
pub mod madmom;
pub mod midi;
pub mod orientation;

pub use audio::VolumeReplay;
pub use madmom::parse_beat_line;
pub use midi::{MidiCommand, MidiCommandKind, MidiReplay};
pub use orientation::{classify_datagram, DatagramKind};
