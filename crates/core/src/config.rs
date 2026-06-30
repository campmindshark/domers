//! Engine configuration model skeleton.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::color::{ColorPalette, PaletteEntry};
use crate::migration::{analyze_spectrum_xml, MigrationReport};

/// Minimal engine configuration used by the initial scheduler/output tests.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EngineConfig {
    /// Whether the dome hardware path is enabled.
    pub dome_enabled: bool,
    /// Whether the dome simulator path is enabled.
    pub dome_simulation_enabled: bool,
    /// Active dome visualizer index, matching Spectrum's `domeActiveVis`.
    pub dome_active_vis: u8,
    /// Dome diagnostic pattern.
    pub dome_test_pattern: u8,
    /// Active color palette slot, matching Spectrum's `colorPaletteIndex`.
    pub color_palette_index: u8,
    /// Beat flash blackout speed, matching Spectrum's `flashSpeed`.
    pub flash_speed: f64,
    /// Active runtime color palette.
    pub color_palette: ColorPalette,
}

/// Native Domers application config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DomersConfig {
    /// Dome fixture config.
    pub dome: DomeConfig,
    /// Bar fixture config.
    pub bar: BarConfig,
    /// Stage fixture config.
    pub stage: StageConfig,
    /// Tempo source config.
    pub tempo: TempoConfig,
    /// Live input adapter config.
    #[serde(default)]
    pub inputs: InputConfig,
    /// Spectrum-compatible runtime color palette.
    #[serde(default)]
    pub color_palette: ColorPalette,
    /// Active color palette slot, matching Spectrum's `colorPaletteIndex`.
    #[serde(default)]
    pub color_palette_index: u8,
    /// Madmom sidecar config.
    pub madmom: MadmomConfig,
}

/// Dome fixture config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DomeConfig {
    /// Whether hardware output is enabled.
    pub enabled: bool,
    /// Whether simulator output is enabled.
    pub simulation_enabled: bool,
    /// OPC host string, preserving Spectrum's `host:port[:channel]` shape.
    pub opc_address: String,
    /// Active visualizer index.
    pub active_visualizer: u8,
    /// Diagnostic pattern.
    pub test_pattern: u8,
    /// Brightness multiplier.
    pub brightness: f64,
}

/// Bar fixture config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BarConfig {
    /// Whether hardware output is enabled.
    pub enabled: bool,
    /// Whether simulator output is enabled.
    pub simulation_enabled: bool,
    /// Infinity strip width.
    pub infinity_width: u32,
    /// Infinity strip length.
    pub infinity_length: u32,
    /// Runner strip length.
    pub runner_length: u32,
    /// Brightness multiplier.
    pub brightness: f64,
    /// Diagnostic pattern.
    pub test_pattern: u8,
}

/// Stage fixture config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StageConfig {
    /// Whether hardware output is enabled.
    pub enabled: bool,
    /// Whether simulator output is enabled.
    pub simulation_enabled: bool,
    /// OPC host string, preserving Spectrum's `host:port[:channel]` shape.
    pub opc_address: String,
    /// Side lengths.
    pub side_lengths: Vec<u32>,
    /// Brightness multiplier.
    pub brightness: f64,
    /// Diagnostic pattern.
    pub test_pattern: u8,
}

/// Tempo source config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TempoConfig {
    /// Tempo source name.
    pub source: TempoSource,
    /// Beat flash blackout speed.
    pub flash_speed: f64,
}

/// Live input adapter config.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InputConfig {
    /// Optional UDP audio volume source.
    #[serde(default)]
    pub audio: UdpInputConfig,
    /// MIDI command source and binding config.
    #[serde(default)]
    pub midi: MidiInputConfig,
    /// Optional UDP orientation datagram source.
    #[serde(default)]
    pub orientation: UdpInputConfig,
}

/// Optional UDP input binding.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UdpInputConfig {
    /// Bind address. When unset, this input adapter is disabled.
    pub bind: Option<String>,
}

/// MIDI input binding config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MidiInputConfig {
    /// Bind address. When unset, this input adapter is disabled.
    pub bind: Option<String>,
    /// Runtime MIDI bindings.
    #[serde(default)]
    pub bindings: Vec<MidiBindingConfig>,
}

/// MIDI command kind used by config bindings.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MidiBindingCommandKind {
    /// Note on/off command.
    Note,
    /// Continuous controller command.
    ControlChange,
    /// Program change command.
    Program,
}

/// Runtime action driven by a MIDI binding.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MidiBindingAction {
    /// Toggle flash overlay from command value.
    Flash,
    /// Set normalized volume from command value.
    Volume,
    /// Trigger tap tempo when command value is positive.
    TapTempo,
    /// Select color palette. Uses `target_index` if present, otherwise maps value to 0-7.
    Palette,
    /// Select dome visualizer. Uses `target_index` if present, otherwise maps value to 0-8.
    Visualizer,
}

/// One MIDI command binding.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MidiBindingConfig {
    /// MIDI command kind.
    pub command_kind: MidiBindingCommandKind,
    /// Note/controller/program index.
    pub index: u8,
    /// Runtime action.
    pub action: MidiBindingAction,
    /// Optional fixed target index for palette/visualizer actions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_index: Option<u8>,
}

impl Default for MidiInputConfig {
    fn default() -> Self {
        Self {
            bind: None,
            bindings: vec![
                MidiBindingConfig {
                    command_kind: MidiBindingCommandKind::Note,
                    index: 64,
                    action: MidiBindingAction::Flash,
                    target_index: None,
                },
                MidiBindingConfig {
                    command_kind: MidiBindingCommandKind::ControlChange,
                    index: 1,
                    action: MidiBindingAction::Volume,
                    target_index: None,
                },
            ],
        }
    }
}

/// Tempo source.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TempoSource {
    /// Human tap tempo.
    Human,
    /// Madmom sidecar beat detector.
    Madmom,
    /// Ableton Link is intentionally not implemented yet.
    LinkUnsupported,
}

/// Madmom sidecar config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MadmomConfig {
    /// Sidecar executable or command name.
    pub command: String,
    /// Optional tracker/script argument for Python-style Spectrum launches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracker: Option<String>,
    /// Optional audio input index to pass through.
    pub audio_input_index: Option<u32>,
}

/// Result of importing a legacy Spectrum XML config.
#[derive(Clone, Debug)]
pub struct ImportedConfig {
    /// Native Domers config.
    pub config: DomersConfig,
    /// Migration warnings.
    pub report: MigrationReport,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            dome_enabled: false,
            dome_simulation_enabled: true,
            dome_active_vis: 0,
            dome_test_pattern: 0,
            color_palette_index: 0,
            flash_speed: 0.0,
            color_palette: ColorPalette::default(),
        }
    }
}

impl Default for DomersConfig {
    fn default() -> Self {
        Self {
            dome: DomeConfig {
                enabled: false,
                simulation_enabled: true,
                opc_address: "127.0.0.1:7890".to_string(),
                active_visualizer: 0,
                test_pattern: 0,
                brightness: 1.0,
            },
            bar: BarConfig {
                enabled: false,
                simulation_enabled: false,
                infinity_width: 50,
                infinity_length: 50,
                runner_length: 50,
                brightness: 1.0,
                test_pattern: 0,
            },
            stage: StageConfig {
                enabled: false,
                simulation_enabled: false,
                opc_address: "127.0.0.1:7890".to_string(),
                side_lengths: Vec::new(),
                brightness: 1.0,
                test_pattern: 0,
            },
            tempo: TempoConfig {
                source: TempoSource::Human,
                flash_speed: 0.0,
            },
            inputs: InputConfig::default(),
            color_palette: ColorPalette::default(),
            color_palette_index: 0,
            madmom: MadmomConfig {
                command: "DBNBeatTracker".to_string(),
                tracker: None,
                audio_input_index: None,
            },
        }
    }
}

impl From<&DomersConfig> for EngineConfig {
    fn from(config: &DomersConfig) -> Self {
        Self {
            dome_enabled: config.dome.enabled,
            dome_simulation_enabled: config.dome.simulation_enabled,
            dome_active_vis: config.dome.active_visualizer,
            dome_test_pattern: config.dome.test_pattern,
            color_palette_index: config.color_palette_index,
            flash_speed: config.tempo.flash_speed,
            color_palette: config.color_palette.clone(),
        }
    }
}

impl DomersConfig {
    /// Serialize config as pretty TOML.
    ///
    /// # Errors
    ///
    /// Returns an error if TOML serialization fails.
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(&DomersConfigTomlOut::from(self))
    }

    /// Parse native Domers TOML.
    ///
    /// # Errors
    ///
    /// Returns an error if TOML parsing fails.
    pub fn from_toml_str(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str::<DomersConfigToml>(input).map(Into::into)
    }
}

#[derive(Deserialize)]
struct DomersConfigToml {
    dome: DomeConfig,
    bar: BarConfig,
    stage: StageConfig,
    tempo: TempoConfig,
    #[serde(default)]
    inputs: InputConfig,
    #[serde(default)]
    color_palette: ColorPaletteToml,
    #[serde(default)]
    color_palette_index: u8,
    madmom: MadmomConfig,
}

impl From<DomersConfigToml> for DomersConfig {
    fn from(config: DomersConfigToml) -> Self {
        Self {
            dome: config.dome,
            bar: config.bar,
            stage: config.stage,
            tempo: config.tempo,
            inputs: config.inputs,
            color_palette: config.color_palette.into_color_palette(),
            color_palette_index: config.color_palette_index,
            madmom: config.madmom,
        }
    }
}

#[derive(Deserialize, Default)]
struct ColorPaletteToml {
    #[serde(default)]
    colors: Vec<PaletteEntry>,
    #[serde(default)]
    entries: BTreeMap<String, PaletteEntry>,
    #[serde(default)]
    banks: Vec<Vec<String>>,
    #[serde(default)]
    slots: Vec<String>,
}

impl ColorPaletteToml {
    fn into_color_palette(self) -> ColorPalette {
        if !self.colors.is_empty() {
            return ColorPalette {
                colors: normalize_palette_slots(self.colors),
            };
        }

        let slot_names: Vec<_> = if self.banks.is_empty() {
            self.slots
        } else {
            self.banks.into_iter().flatten().collect()
        };
        if slot_names.is_empty() {
            return ColorPalette::default();
        }

        let colors = slot_names
            .into_iter()
            .map(|name| self.entries.get(&name).copied().unwrap_or_default())
            .collect();
        ColorPalette {
            colors: normalize_palette_slots(colors),
        }
    }
}

fn normalize_palette_slots(mut colors: Vec<PaletteEntry>) -> Vec<PaletteEntry> {
    colors.truncate(ColorPalette::ENTRY_COUNT);
    colors.resize(ColorPalette::ENTRY_COUNT, PaletteEntry::default());
    colors
}

#[derive(Serialize)]
struct DomersConfigTomlOut {
    color_palette_index: u8,
    dome: DomeConfig,
    bar: BarConfig,
    stage: StageConfig,
    tempo: TempoConfig,
    inputs: InputConfig,
    color_palette: ColorPaletteDryToml,
    madmom: MadmomConfig,
}

impl From<&DomersConfig> for DomersConfigTomlOut {
    fn from(config: &DomersConfig) -> Self {
        Self {
            color_palette_index: config.color_palette_index,
            dome: config.dome.clone(),
            bar: config.bar.clone(),
            stage: config.stage.clone(),
            tempo: config.tempo.clone(),
            inputs: config.inputs.clone(),
            color_palette: ColorPaletteDryToml::from(&config.color_palette),
            madmom: config.madmom.clone(),
        }
    }
}

#[derive(Serialize)]
struct ColorPaletteDryToml {
    banks: Vec<Vec<String>>,
    entries: BTreeMap<String, PaletteEntry>,
}

impl From<&ColorPalette> for ColorPaletteDryToml {
    fn from(palette: &ColorPalette) -> Self {
        let mut unique: Vec<(PaletteEntry, String)> = Vec::new();
        let mut slots = Vec::new();
        for entry in normalize_palette_slots(palette.colors.clone()) {
            let name =
                if let Some((_, name)) = unique.iter().find(|(candidate, _)| *candidate == entry) {
                    name.clone()
                } else {
                    let name = format!("entry_{:02}", unique.len() + 1);
                    unique.push((entry, name.clone()));
                    name
                };
            slots.push(name);
        }

        let banks = slots
            .chunks(ColorPalette::COLORS_PER_BANK)
            .map(<[String]>::to_vec)
            .collect();
        let entries = unique
            .into_iter()
            .map(|(entry, name)| (name, entry))
            .collect();
        Self { banks, entries }
    }
}

/// Import legacy Spectrum XML into native Domers TOML config.
#[must_use]
pub fn import_spectrum_xml(xml: &str) -> ImportedConfig {
    let report = analyze_spectrum_xml(xml);
    let mut config = DomersConfig::default();

    config.dome.enabled = bool_tag(xml, "domeEnabled").unwrap_or(config.dome.enabled);
    config.dome.simulation_enabled = true;
    config.dome.opc_address = string_tag(xml, "domeBeagleboneOPCAddress")
        .map(|address| localhost_opc_address(&address))
        .unwrap_or(config.dome.opc_address);
    config.dome.active_visualizer =
        u8_tag(xml, "domeActiveVis").unwrap_or(config.dome.active_visualizer);
    config.dome.test_pattern = u8_tag(xml, "domeTestPattern").unwrap_or(config.dome.test_pattern);
    config.dome.brightness = f64_tag(xml, "domeBrightness").unwrap_or(config.dome.brightness);

    config.bar.enabled = bool_tag(xml, "barEnabled").unwrap_or(config.bar.enabled);
    config.bar.simulation_enabled = config.bar.enabled;
    config.bar.infinity_width =
        u32_tag(xml, "barInfinityWidth").unwrap_or(config.bar.infinity_width);
    config.bar.infinity_length =
        u32_tag(xml, "barInfinityLength").unwrap_or(config.bar.infinity_length);
    config.bar.runner_length = u32_tag(xml, "barRunnerLength").unwrap_or(config.bar.runner_length);
    config.bar.brightness = f64_tag(xml, "barBrightness").unwrap_or(config.bar.brightness);
    config.bar.test_pattern = u8_tag(xml, "barTestPattern").unwrap_or(config.bar.test_pattern);

    config.stage.enabled = bool_tag(xml, "stageEnabled").unwrap_or(config.stage.enabled);
    config.stage.simulation_enabled = config.stage.enabled;
    config.stage.opc_address = string_tag(xml, "stageBeagleboneOPCAddress")
        .map(|address| localhost_opc_address(&address))
        .unwrap_or(config.stage.opc_address);
    config.stage.side_lengths = stage_side_lengths(xml);
    config.stage.brightness = f64_tag(xml, "stageBrightness").unwrap_or(config.stage.brightness);
    config.stage.test_pattern =
        u8_tag(xml, "stageTestPattern").unwrap_or(config.stage.test_pattern);

    config.tempo.source = match u8_tag(xml, "beatInput").unwrap_or(0) {
        1 => TempoSource::Madmom,
        2 => TempoSource::LinkUnsupported,
        _ => TempoSource::Human,
    };
    config.tempo.flash_speed = f64_tag(xml, "flashSpeed").unwrap_or(config.tempo.flash_speed);
    config.color_palette = color_palette(xml);
    config.color_palette_index =
        u8_tag(xml, "colorPaletteIndex").unwrap_or(config.color_palette_index);

    ImportedConfig { config, report }
}

fn localhost_opc_address(address: &str) -> String {
    let mut parts = address.split(':');
    let _host = parts.next();
    let port = parts.next().unwrap_or("7890");
    let channel = parts.next();
    match channel {
        Some(channel) => format!("127.0.0.1:{port}:{channel}"),
        None => format!("127.0.0.1:{port}"),
    }
}

fn tag_value<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");
    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml[start..].find(&end_tag)? + start;
    Some(xml[start..end].trim())
}

fn string_tag(xml: &str, tag: &str) -> Option<String> {
    tag_value(xml, tag).map(ToString::to_string)
}

fn bool_tag(xml: &str, tag: &str) -> Option<bool> {
    tag_value(xml, tag)?.parse().ok()
}

fn u8_tag(xml: &str, tag: &str) -> Option<u8> {
    tag_value(xml, tag)?.parse().ok()
}

fn u32_tag(xml: &str, tag: &str) -> Option<u32> {
    tag_value(xml, tag)?.parse().ok()
}

fn f64_tag(xml: &str, tag: &str) -> Option<f64> {
    tag_value(xml, tag)?.parse().ok()
}

fn stage_side_lengths(xml: &str) -> Vec<u32> {
    tag_value(xml, "stageSideLengths")
        .map(|block| {
            block
                .split("<Int32>")
                .skip(1)
                .filter_map(|chunk| chunk.split("</Int32>").next())
                .filter_map(|value| value.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

fn color_palette(xml: &str) -> ColorPalette {
    let Some(block) = tag_value(xml, "colors") else {
        return ColorPalette::default();
    };

    let colors = block
        .split("<LEDColor>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</LEDColor>").next())
        .map(|entry| {
            let color1 = u32_tag(entry, "color1").unwrap_or(0);
            let color2 = u32_tag(entry, "color2").unwrap_or(0);
            if bool_tag(entry, "color2Enabled").unwrap_or(false) {
                PaletteEntry::gradient(color1, color2)
            } else {
                PaletteEntry::solid(color1)
            }
        })
        .collect();

    ColorPalette { colors }
}

#[cfg(test)]
mod tests {
    use crate::color::{ColorPalette, PaletteEntry};

    use super::{import_spectrum_xml, DomersConfig, TempoSource};

    #[test]
    fn default_config_fixture_contains_core_fields() {
        let xml = include_str!("../../../fixtures/config/spectrum_default_config.xml");
        assert!(xml.contains("<domeEnabled>true</domeEnabled>"));
        assert!(xml.contains("<domeActiveVis>0</domeActiveVis>"));
        assert!(xml.contains("<stageSideLengths>"));
        assert!(xml.contains("<beatInput>0</beatInput>"));
    }

    #[test]
    fn imports_spectrum_xml_to_native_toml_config() {
        let xml = include_str!("../../../fixtures/config/spectrum_default_config.xml");
        let imported = import_spectrum_xml(xml);

        assert!(imported.config.dome.enabled);
        assert!(imported.config.dome.simulation_enabled);
        assert_eq!(imported.config.dome.opc_address, "127.0.0.1:7890");
        assert_eq!(imported.config.dome.active_visualizer, 0);
        assert!(imported.config.bar.enabled);
        assert!(imported.config.bar.simulation_enabled);
        assert_eq!(imported.config.bar.infinity_length, 50);
        assert_eq!(imported.config.stage.side_lengths.len(), 48);
        assert_eq!(imported.config.tempo.source, TempoSource::Human);
        assert_eq!(imported.config.color_palette_index, 7);
        assert_eq!(imported.config.color_palette.colors.len(), 64);
        assert_eq!(
            imported.config.color_palette.colors[0],
            PaletteEntry::gradient(0xff_00_00, 0xff_00_00)
        );
        assert!(imported.report.warnings.len() >= 5);
    }

    #[test]
    fn native_config_round_trips_as_toml() {
        let config = DomersConfig::default();
        let toml = config.to_toml_string().expect("config serializes");
        assert!(toml.contains("[dome]"));
        assert!(toml.contains("[madmom]"));
        assert!(toml.contains("[color_palette]"));
        assert!(toml.contains("[color_palette.entries.entry_"));
        assert!(!toml.contains("[[color_palette.colors]]"));

        let parsed = DomersConfig::from_toml_str(&toml).expect("config parses");
        assert_eq!(parsed.dome.active_visualizer, config.dome.active_visualizer);
        assert_eq!(parsed.color_palette, config.color_palette);
    }

    #[test]
    fn example_config_parses() {
        let toml = include_str!("../../../examples/domers.toml");
        let parsed = DomersConfig::from_toml_str(toml).expect("example config parses");

        assert!(parsed.dome.simulation_enabled);
        assert_eq!(parsed.madmom.command, "DBNBeatTracker");
        assert_eq!(parsed.color_palette.colors.len(), ColorPalette::ENTRY_COUNT);
        assert!(toml.contains("[color_palette.entries.entry_"));
        assert!(!toml.contains("[[color_palette.colors]]"));
    }

    #[test]
    fn domers_config_maps_to_engine_config() {
        let mut config = DomersConfig {
            dome: super::DomeConfig {
                enabled: true,
                simulation_enabled: false,
                opc_address: "127.0.0.1:7890".to_string(),
                active_visualizer: 3,
                test_pattern: 2,
                brightness: 0.5,
            },
            ..DomersConfig::default()
        };
        config.color_palette_index = 6;

        let engine = super::EngineConfig::from(&config);

        assert!(engine.dome_enabled);
        assert!(!engine.dome_simulation_enabled);
        assert_eq!(engine.dome_active_vis, 3);
        assert_eq!(engine.dome_test_pattern, 2);
        assert_eq!(engine.color_palette_index, 6);
        assert!((engine.flash_speed - config.tempo.flash_speed).abs() < f64::EPSILON);
        assert_eq!(engine.color_palette.colors.len(), ColorPalette::ENTRY_COUNT);
    }
}
