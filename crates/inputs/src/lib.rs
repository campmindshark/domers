//! Input parsers and fakeable input seams.

pub mod madmom;
pub mod orientation;

pub use madmom::parse_beat_line;
pub use orientation::{classify_datagram, DatagramKind};
