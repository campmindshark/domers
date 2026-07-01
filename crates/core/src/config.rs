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
    /// DJ Link / Carabiner-compatible sidecar config.
    #[serde(default)]
    pub carabiner: CarabinerConfig,
    /// Spectrum level-driver presets and channel assignments.
    #[serde(default)]
    pub level_drivers: LevelDriverConfig,
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
    /// Audio input source and selected device config.
    #[serde(default)]
    pub audio: AudioInputConfig,
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

/// Audio input config.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AudioInputConfig {
    /// Optional UDP volume bind. When unset, the UDP bridge is disabled.
    pub bind: Option<String>,
    /// Enable native macOS/Linux audio capture through CPAL.
    #[serde(default, skip_serializing_if = "is_false")]
    pub native_enabled: bool,
    /// Stable Spectrum audio endpoint id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    /// Optional fakeable all-endpoint list used for no-hardware parity tests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<AudioDeviceConfig>,
}

/// Configured audio endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct AudioDeviceConfig {
    /// Stable endpoint id.
    pub id: String,
    /// Friendly display name.
    pub name: String,
    /// Endpoint flow.
    pub flow: AudioDeviceFlowConfig,
}

/// Configured endpoint flow.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioDeviceFlowConfig {
    /// Capture/recording endpoint.
    Capture,
    /// Render/playback endpoint.
    Render,
}

/// MIDI input binding config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MidiInputConfig {
    /// Bind address. When unset, this input adapter is disabled.
    pub bind: Option<String>,
    /// Enable native macOS/Linux MIDI capture through midir.
    #[serde(default, skip_serializing_if = "is_false")]
    pub native_enabled: bool,
    /// Optional native MIDI port name to open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
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
    /// Trigger Spectrum ADSR MIDI level-driver channels. `index` is the first note in an 8-note range.
    AdsrLevelDriver,
}

/// One MIDI command binding.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MidiBindingConfig {
    /// Optional MIDI device index. When unset, the binding applies to every device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_index: Option<u8>,
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
            native_enabled: false,
            device_id: None,
            bindings: vec![
                MidiBindingConfig {
                    device_index: None,
                    command_kind: MidiBindingCommandKind::Note,
                    index: 64,
                    action: MidiBindingAction::Flash,
                    target_index: None,
                },
                MidiBindingConfig {
                    device_index: None,
                    command_kind: MidiBindingCommandKind::ControlChange,
                    index: 1,
                    action: MidiBindingAction::Volume,
                    target_index: None,
                },
                MidiBindingConfig {
                    device_index: None,
                    command_kind: MidiBindingCommandKind::Note,
                    index: 48,
                    action: MidiBindingAction::AdsrLevelDriver,
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
    /// DJ Link / Carabiner tempo sync.
    Link,
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

/// DJ Link / Carabiner-compatible sidecar config.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CarabinerConfig {
    /// Sidecar executable or command name.
    pub command: String,
    /// Extra command-line arguments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Whether human tap tempo should be sent to Link.
    #[serde(default)]
    pub human_link_output: bool,
    /// Whether Madmom tempo should be sent to Link.
    #[serde(default)]
    pub madmom_link_output: bool,
}

impl Default for CarabinerConfig {
    fn default() -> Self {
        Self {
            command: "carabiner".to_string(),
            args: Vec::new(),
            human_link_output: false,
            madmom_link_output: false,
        }
    }
}

/// Spectrum level-driver presets and channel assignments.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LevelDriverConfig {
    /// Named audio/MIDI level-driver presets.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub presets: BTreeMap<String, LevelDriverPresetConfig>,
    /// Mapping from channel index to an audio preset name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub audio_channels: BTreeMap<u8, String>,
    /// Mapping from channel index to a MIDI ADSR preset name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub midi_channels: BTreeMap<u8, String>,
}

/// One Spectrum level-driver preset.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum LevelDriverPresetConfig {
    /// Audio band level-driver preset.
    Audio {
        /// Normalized filter range start.
        filter_range_start: f64,
        /// Normalized filter range end.
        filter_range_end: f64,
    },
    /// MIDI ADSR envelope level-driver preset.
    Midi {
        /// Attack duration in milliseconds.
        attack_time: u64,
        /// Peak level multiplier.
        peak_level: f64,
        /// Decay duration in milliseconds.
        decay_time: u64,
        /// Sustain level multiplier.
        sustain_level: f64,
        /// Release duration in milliseconds.
        release_time: u64,
    },
}

impl LevelDriverPresetConfig {
    /// Whether this preset is a MIDI ADSR preset.
    #[must_use]
    pub fn is_midi(&self) -> bool {
        matches!(self, Self::Midi { .. })
    }
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
            carabiner: CarabinerConfig::default(),
            level_drivers: LevelDriverConfig::default(),
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
    #[serde(default)]
    carabiner: CarabinerConfig,
    #[serde(default)]
    level_drivers: LevelDriverConfig,
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
            carabiner: config.carabiner,
            level_drivers: config.level_drivers,
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

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde skip_serializing_if requires a by-reference predicate"
)]
fn is_false(value: &bool) -> bool {
    !*value
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
    carabiner: CarabinerConfig,
    level_drivers: LevelDriverConfig,
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
            carabiner: config.carabiner.clone(),
            level_drivers: config.level_drivers.clone(),
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
    config.stage.opc_address = string_tag(xml, "stageBeagleboneOPCAddress")
        .map(|address| localhost_opc_address(&address))
        .unwrap_or(config.stage.opc_address);
    config.stage.side_lengths = stage_side_lengths(xml);
    config.stage.simulation_enabled = config.stage.enabled || !config.stage.side_lengths.is_empty();
    config.stage.brightness = f64_tag(xml, "stageBrightness").unwrap_or(config.stage.brightness);
    config.stage.test_pattern =
        u8_tag(xml, "stageTestPattern").unwrap_or(config.stage.test_pattern);

    config.tempo.source = match u8_tag(xml, "beatInput").unwrap_or(0) {
        1 => TempoSource::Madmom,
        2 => TempoSource::Link,
        _ => TempoSource::Human,
    };
    config.carabiner.human_link_output =
        bool_tag(xml, "humanLinkOutput").unwrap_or(config.carabiner.human_link_output);
    config.carabiner.madmom_link_output =
        bool_tag(xml, "madmomLinkOutput").unwrap_or(config.carabiner.madmom_link_output);
    config.tempo.flash_speed = f64_tag(xml, "flashSpeed").unwrap_or(config.tempo.flash_speed);
    config.inputs.audio.device_id = tag_value(xml, "audioDeviceID").map(str::to_string);
    config.level_drivers = level_driver_config(xml);
    if let Some(index) = adsr_binding_index(xml) {
        if !config.inputs.midi.bindings.iter().any(|binding| {
            binding.action == MidiBindingAction::AdsrLevelDriver && binding.index == index
        }) {
            config.inputs.midi.bindings.push(MidiBindingConfig {
                device_index: None,
                command_kind: MidiBindingCommandKind::Note,
                index,
                action: MidiBindingAction::AdsrLevelDriver,
                target_index: None,
            });
        }
    }
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

fn loose_tag_value<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let start = xml.find(&format!("<{tag}"))?;
    let value_start = xml[start..].find('>')? + start + 1;
    let end = xml[value_start..].find(&format!("</{tag}>"))? + value_start;
    Some(xml[value_start..end].trim())
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

fn u64_tag(xml: &str, tag: &str) -> Option<u64> {
    tag_value(xml, tag)?.parse().ok()
}

fn f64_tag(xml: &str, tag: &str) -> Option<f64> {
    tag_value(xml, tag)?.parse().ok()
}

fn adsr_binding_index(xml: &str) -> Option<u8> {
    let marker = r#"xsi:type="AdsrLevelDriverMidiBindingConfig""#;
    let block = xml.split(marker).nth(1)?;
    u8_tag(block, "indexRangeStart")
}

fn level_driver_config(xml: &str) -> LevelDriverConfig {
    let mut config = LevelDriverConfig::default();
    if let Some(block) = tag_value(xml, "levelDriverPresets") {
        for item in xml_items(block) {
            let Some(name) = tag_value(item, "Key") else {
                continue;
            };
            let Some(value) = loose_tag_value(item, "Value") else {
                continue;
            };
            if item.contains(r#"xsi:type="AudioLevelDriverPreset""#) {
                config.presets.insert(
                    name.to_string(),
                    LevelDriverPresetConfig::Audio {
                        filter_range_start: f64_tag(value, "FilterRangeStart").unwrap_or(0.0),
                        filter_range_end: f64_tag(value, "FilterRangeEnd").unwrap_or(1.0),
                    },
                );
            } else if item.contains(r#"xsi:type="MidiLevelDriverPreset""#) {
                config.presets.insert(
                    name.to_string(),
                    LevelDriverPresetConfig::Midi {
                        attack_time: u64_tag(value, "AttackTime").unwrap_or(0),
                        peak_level: f64_tag(value, "PeakLevel").unwrap_or(1.0),
                        decay_time: u64_tag(value, "DecayTime").unwrap_or(0),
                        sustain_level: f64_tag(value, "SustainLevel").unwrap_or(1.0),
                        release_time: u64_tag(value, "ReleaseTime").unwrap_or(0),
                    },
                );
            }
        }
    }
    config.audio_channels = channel_preset_map(xml, "channelToAudioLevelDriverPreset");
    config.midi_channels = channel_preset_map(xml, "channelToMidiLevelDriverPreset");
    config
}

fn channel_preset_map(xml: &str, tag: &str) -> BTreeMap<u8, String> {
    let mut map = BTreeMap::new();
    if let Some(block) = tag_value(xml, tag) {
        for item in xml_items(block) {
            let Some(channel) = u8_tag(item, "Key") else {
                continue;
            };
            if let Some(preset) = tag_value(item, "Value") {
                map.insert(channel, preset.to_string());
            }
        }
    }
    map
}

fn xml_items(block: &str) -> impl Iterator<Item = &str> {
    block
        .split("<Item>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</Item>").next())
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

    use super::{
        import_spectrum_xml, DomersConfig, LevelDriverPresetConfig, MidiBindingAction, TempoSource,
    };

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
        assert_eq!(
            imported.config.inputs.audio.device_id.as_deref(),
            Some("{0.0.1.00000000}.{1ce71263-2eb5-4744-94e7-5bc353f90945}")
        );
        assert_eq!(imported.config.color_palette_index, 7);
        assert_eq!(imported.config.color_palette.colors.len(), 64);
        assert_eq!(imported.config.level_drivers.presets.len(), 4);
        assert_eq!(
            imported
                .config
                .level_drivers
                .audio_channels
                .get(&0)
                .map(String::as_str),
            Some("full spectrum")
        );
        assert_eq!(
            imported
                .config
                .level_drivers
                .midi_channels
                .get(&0)
                .map(String::as_str),
            Some("midi test")
        );
        assert!(matches!(
            imported.config.level_drivers.presets.get("midi test"),
            Some(LevelDriverPresetConfig::Midi {
                attack_time: 10,
                peak_level,
                decay_time: 20,
                sustain_level,
                release_time: 10,
            }) if (*peak_level - 1.0).abs() < f64::EPSILON
                && (*sustain_level - 0.8).abs() < f64::EPSILON
        ));
        assert!(imported.config.inputs.midi.bindings.iter().any(|binding| {
            binding.action == MidiBindingAction::AdsrLevelDriver && binding.index == 48
        }));
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
