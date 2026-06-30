//! Core shared types for the Domers Spectrum rewrite.

pub mod beat;
pub mod color;
pub mod config;
pub mod migration;

pub use beat::{BeatBroadcaster, BeatClock};
pub use color::{ColorPalette, PaletteEntry, Rgb};
pub use config::{
    import_spectrum_xml, DomersConfig, EngineConfig, ImportedConfig, InputConfig, MadmomConfig,
    MidiBindingAction, MidiBindingCommandKind, MidiBindingConfig, MidiInputConfig, TempoSource,
    UdpInputConfig,
};
pub use migration::{analyze_spectrum_xml, MigrationReport, WarningKind};
