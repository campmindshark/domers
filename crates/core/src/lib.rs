//! Core shared types for the Domers Spectrum rewrite.

pub mod beat;
pub mod color;
pub mod config;

pub use beat::{BeatBroadcaster, BeatClock};
pub use color::Rgb;
pub use config::EngineConfig;
