//! Open Pixel Control wire encoder and hardware frame helpers.
//!
//! Spectrum's controller expects a non-standard frame chunk without the usual
//! `0xff` prefix: `[channel][command=0][len_hi][len_lo][rgb...]`.

use domers_core::Rgb;
use serde::Deserialize;
use tokio::{io::AsyncWriteExt, net::TcpStream};

use std::sync::OnceLock;

use crate::{BarCommand, DomeCommand, StageCommand};

const DOME_MAPPING_JSON: &str = include_str!("../../../fixtures/spectrum-csharp/dome_mapping.json");
const DOME_MAX_STRIP_LENGTH: usize = 214;
const DOME_BAR_CONTROL_BOX: usize = 5;
static DOME_MAPPING: OnceLock<DomeMapping> = OnceLock::new();

/// Dense persistent channel buffer matching Spectrum's flush semantics.
#[derive(Clone, Debug, Default)]
pub struct PersistentChannel {
    next: Vec<Rgb>,
    current: Vec<Rgb>,
}

impl PersistentChannel {
    /// Set a pixel in the next frame.
    pub fn set_pixel(&mut self, index: usize, color: Rgb) {
        let needed = index + 1;
        if self.next.len() < needed {
            self.next.resize(needed, Rgb::BLACK);
        }
        self.next[index] = color;
    }

    /// Realize the next frame and preserve pixels for future sparse updates.
    pub fn flush(&mut self) {
        self.current.clone_from(&self.next);
        self.next.clone_from(&self.current);
    }

    /// Encode the realized current frame.
    #[must_use]
    pub fn encode_current(&self, channel: u8) -> Vec<u8> {
        encode_frame(channel, &self.current)
    }

    /// Return the realized current frame.
    #[must_use]
    pub fn current_pixels(&self) -> &[Rgb] {
        &self.current
    }

    /// Force the current and next frame buffers to black with at least `pixel_count` pixels.
    pub fn blackout(&mut self, pixel_count: usize) {
        self.current.resize(pixel_count, Rgb::default());
        self.next.resize(pixel_count, Rgb::default());
        for pixel in &mut self.current {
            *pixel = Rgb::default();
        }
        for pixel in &mut self.next {
            *pixel = Rgb::default();
        }
    }
}

/// Parsed OPC address in `host:port[:channel]` form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpcAddress {
    /// Hostname or IP address.
    pub host: String,
    /// TCP port.
    pub port: u16,
    /// OPC channel, defaulting to 0.
    pub channel: u8,
}

impl OpcAddress {
    /// Parse Spectrum's `host:port[:channel]` address form.
    ///
    /// # Errors
    ///
    /// Returns an error string if the host, port, or channel is invalid.
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut parts = input.split(':');
        let host = parts
            .next()
            .filter(|host| !host.is_empty())
            .ok_or("missing host")?;
        let port = parts
            .next()
            .ok_or("missing port")?
            .parse::<u16>()
            .map_err(|error| format!("invalid port: {error}"))?;
        let channel = parts
            .next()
            .map(str::parse::<u8>)
            .transpose()
            .map_err(|error| format!("invalid channel: {error}"))?
            .unwrap_or(0);
        if parts.next().is_some() {
            return Err("too many address parts".to_string());
        }
        Ok(Self {
            host: host.to_string(),
            port,
            channel,
        })
    }

    /// Return the socket address string used by Tokio.
    #[must_use]
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Async TCP writer for one OPC destination.
#[derive(Debug)]
pub struct OpcClient {
    address: OpcAddress,
    stream: TcpStream,
}

impl OpcClient {
    /// Connect to an OPC server.
    ///
    /// # Errors
    ///
    /// Returns an IO error if the TCP connection cannot be established.
    pub async fn connect(address: OpcAddress) -> std::io::Result<Self> {
        let stream = TcpStream::connect(address.socket_addr()).await?;
        Ok(Self { address, stream })
    }

    /// Send one realized RGB frame to the configured channel.
    ///
    /// # Errors
    ///
    /// Returns an IO error if the socket write fails.
    pub async fn send_frame(&mut self, pixels: &[Rgb]) -> std::io::Result<()> {
        self.stream
            .write_all(&encode_frame(self.address.channel, pixels))
            .await
    }
}

/// Apply dome commands to a persistent OPC channel using Spectrum's physical map.
///
/// # Panics
///
/// Panics if the checked dome mapping fixture is invalid.
pub fn apply_dome_commands(channel: &mut PersistentChannel, commands: &[DomeCommand]) {
    let mapping = dome_mapping();
    for command in commands {
        match command {
            DomeCommand::Flush => channel.flush(),
            DomeCommand::Frame(colors) => {
                for (logical_index, color) in colors.iter().copied().enumerate() {
                    if let Some(device_index) = mapping.logical_to_device.get(logical_index) {
                        channel.set_pixel(*device_index, color);
                    }
                }
            }
            DomeCommand::Pixel {
                strut_index,
                led_index,
                color,
            } => {
                if let Some(device_index) = mapping.strut_led_device_index(*strut_index, *led_index)
                {
                    channel.set_pixel(device_index, *color);
                }
            }
        }
    }
}

/// Return Spectrum's logical strut index for a control box and local strut index.
#[must_use]
pub fn dome_strut_index_for_control_box(control_box: usize, local_index: usize) -> Option<usize> {
    dome_mapping()
        .fixture
        .strut_positions
        .iter()
        .position(|position| {
            position.control_box == control_box && position.control_box_strut_index == local_index
        })
}

/// Return the Spectrum strut length for a logical strut index.
#[must_use]
pub fn dome_strut_length(strut_index: usize) -> Option<usize> {
    let fixture = &dome_mapping().fixture;
    let position = fixture.strut_positions.get(strut_index)?;
    Some(fixture.strut_length(position.control_box_strut_index))
}

/// Apply bar commands to the dome OPC channel using Spectrum's bar-on-box-5 route.
pub fn apply_bar_commands(
    channel: &mut PersistentChannel,
    commands: &[BarCommand],
    infinity_width: usize,
    infinity_length: usize,
    runner_length: usize,
) {
    for command in commands {
        match command {
            BarCommand::Flush => channel.flush(),
            BarCommand::Pixel {
                is_runner,
                led_index,
                color,
            } => {
                if let Some(bar_index) = bar_device_pixel(
                    *is_runner,
                    *led_index,
                    infinity_width,
                    infinity_length,
                    runner_length,
                ) {
                    channel.set_pixel(
                        dome_control_box_device_index(DOME_BAR_CONTROL_BOX, bar_index),
                        *color,
                    );
                }
            }
        }
    }
}

/// Apply stage commands to a persistent OPC channel.
pub fn apply_stage_commands(
    channel: &mut PersistentChannel,
    commands: &[StageCommand],
    side_lengths: &[usize],
) {
    let max_triangle_length = max_stage_triangle_length(side_lengths);
    for command in commands {
        match command {
            StageCommand::Flush => channel.flush(),
            StageCommand::Pixel {
                side_index,
                led_index,
                layer_index,
                color,
            } => {
                if let Some(device_index) = stage_device_pixel(
                    *side_index,
                    *led_index,
                    *layer_index,
                    side_lengths,
                    max_triangle_length,
                ) {
                    channel.set_pixel(device_index, *color);
                }
            }
        }
    }
}

/// Encode one non-standard Spectrum OPC frame chunk.
///
/// # Panics
///
/// Panics if the encoded RGB payload exceeds `u16::MAX`, which is the maximum
/// length representable in the OPC frame header.
#[must_use]
pub fn encode_frame(channel: u8, pixels: &[Rgb]) -> Vec<u8> {
    let byte_len = pixels.len() * 3;
    let byte_len = u16::try_from(byte_len).expect("OPC chunk too large");
    let mut out = Vec::with_capacity(4 + usize::from(byte_len));
    out.push(channel);
    out.push(0);
    out.extend(byte_len.to_be_bytes());
    for pixel in pixels {
        out.push(pixel.r);
        out.push(pixel.g);
        out.push(pixel.b);
    }
    out
}

#[derive(Debug, Deserialize)]
struct DomeMappingFixture {
    control_box_strut_order: Vec<Vec<String>>,
    strut_lengths: std::collections::HashMap<String, usize>,
    strut_positions: Vec<StrutPosition>,
}

#[derive(Debug, Deserialize)]
struct StrutPosition {
    control_box: usize,
    control_box_strut_index: usize,
}

#[derive(Debug)]
struct DomeMapping {
    fixture: DomeMappingFixture,
    logical_to_device: Vec<usize>,
}

impl DomeMapping {
    fn load() -> Self {
        let fixture: DomeMappingFixture =
            serde_json::from_str(DOME_MAPPING_JSON).expect("dome mapping fixture is valid");
        let mut logical_to_device = Vec::new();
        for (strut_index, position) in fixture.strut_positions.iter().enumerate() {
            let length = fixture.strut_length(position.control_box_strut_index);
            for led_index in 0..length {
                if let Some(device_index) = Self::device_index_for(&fixture, strut_index, led_index)
                {
                    logical_to_device.push(device_index);
                }
            }
        }
        Self {
            fixture,
            logical_to_device,
        }
    }

    fn strut_led_device_index(&self, strut_index: usize, led_index: usize) -> Option<usize> {
        Self::device_index_for(&self.fixture, strut_index, led_index)
    }

    fn device_index_for(
        fixture: &DomeMappingFixture,
        strut_index: usize,
        led_index: usize,
    ) -> Option<usize> {
        let position = fixture.strut_positions.get(strut_index)?;
        let strand_pixel =
            fixture.strand_pixel_index(position.control_box_strut_index)? + led_index;
        Some(dome_control_box_device_index(
            position.control_box,
            strand_pixel,
        ))
    }
}

fn dome_mapping() -> &'static DomeMapping {
    DOME_MAPPING.get_or_init(DomeMapping::load)
}

impl DomeMappingFixture {
    fn strut_length(&self, control_box_strut_index: usize) -> usize {
        let mut struts_left = control_box_strut_index;
        for strand in &self.control_box_strut_order {
            if strand.len() <= struts_left {
                struts_left -= strand.len();
                continue;
            }
            return self.strut_lengths[&strand[struts_left]];
        }
        0
    }

    fn strand_pixel_index(&self, control_box_strut_index: usize) -> Option<usize> {
        let mut struts_left = control_box_strut_index;
        let mut pixel_index = 0;
        for strand in &self.control_box_strut_order {
            if strand.len() <= struts_left {
                pixel_index += strand
                    .iter()
                    .map(|strut_type| self.strut_lengths[strut_type])
                    .sum::<usize>();
                struts_left -= strand.len();
                continue;
            }
            pixel_index += strand
                .iter()
                .take(struts_left)
                .map(|strut_type| self.strut_lengths[strut_type])
                .sum::<usize>();
            return Some(pixel_index);
        }
        None
    }
}

fn dome_control_box_device_index(control_box: usize, pixel_index: usize) -> usize {
    control_box * (DOME_MAX_STRIP_LENGTH * 8) + pixel_index
}

fn bar_device_pixel(
    is_runner: bool,
    led_index: usize,
    infinity_width: usize,
    infinity_length: usize,
    runner_length: usize,
) -> Option<usize> {
    let infinity_strip_length = infinity_length + infinity_width;
    let total_infinity_length = infinity_strip_length * 2;
    if is_runner {
        return (led_index < runner_length).then_some(led_index + total_infinity_length);
    }
    if led_index >= total_infinity_length {
        return None;
    }
    if led_index >= infinity_strip_length {
        Some(total_infinity_length - led_index + infinity_strip_length - 1)
    } else {
        Some(led_index)
    }
}

fn max_stage_triangle_length(side_lengths: &[usize]) -> usize {
    side_lengths
        .chunks(3)
        .map(|triangle| triangle.iter().sum::<usize>())
        .max()
        .unwrap_or(0)
        * 3
}

fn stage_device_pixel(
    side_index: usize,
    led_index: usize,
    layer_index: usize,
    side_lengths: &[usize],
    max_triangle_length: usize,
) -> Option<usize> {
    let side_length = *side_lengths.get(side_index)?;
    if led_index >= side_length || layer_index >= 3 {
        return None;
    }
    let base_side_index = (side_index / 3) * 3;
    let triangle_length = side_lengths.get(base_side_index..base_side_index + 3)?;
    let mut pixel_index = max_triangle_length * (side_index / 3) + led_index;
    pixel_index += triangle_length.iter().sum::<usize>() * layer_index;
    pixel_index += side_lengths[base_side_index..side_index]
        .iter()
        .sum::<usize>();
    Some(pixel_index)
}

#[cfg(test)]
mod tests {
    use domers_core::Rgb;
    use tokio::{io::AsyncReadExt, net::TcpListener};

    use crate::{BarCommand, DomeCommand, StageCommand};

    use super::{
        apply_bar_commands, apply_dome_commands, apply_stage_commands, encode_frame, OpcAddress,
        OpcClient, PersistentChannel,
    };

    #[test]
    fn encodes_spectrum_nonstandard_header_without_magic_prefix() {
        let encoded = encode_frame(2, &[Rgb::from_u24(0x12_34_56), Rgb::from_u24(0xaa_bb_cc)]);
        assert_eq!(
            encoded,
            vec![2, 0, 0, 6, 0x12, 0x34, 0x56, 0xaa, 0xbb, 0xcc]
        );
    }

    #[test]
    fn matches_extracted_csharp_opc_fixture() {
        let expected = include_bytes!(
            "../../../fixtures/spectrum-csharp/opc_packets/two_pixels_channel_2.bin"
        );
        let encoded = encode_frame(2, &[Rgb::from_u24(0x12_34_56), Rgb::from_u24(0xaa_bb_cc)]);
        assert_eq!(encoded.as_slice(), expected);
    }

    #[test]
    fn persistent_channel_carries_forward_sparse_updates() {
        let mut channel = PersistentChannel::default();
        channel.set_pixel(0, Rgb::from_u24(0xff_00_00));
        channel.set_pixel(1, Rgb::from_u24(0x00_ff_00));
        channel.flush();

        channel.set_pixel(1, Rgb::from_u24(0x00_00_ff));
        channel.flush();

        assert_eq!(
            channel.encode_current(0),
            vec![0, 0, 0, 6, 0xff, 0, 0, 0, 0, 0xff]
        );
    }

    #[test]
    fn parses_spectrum_opc_addresses() {
        assert_eq!(
            OpcAddress::parse("127.0.0.1:7890").expect("address parses"),
            OpcAddress {
                host: "127.0.0.1".to_string(),
                port: 7890,
                channel: 0,
            }
        );
        assert_eq!(
            OpcAddress::parse("localhost:7891:3").expect("address parses"),
            OpcAddress {
                host: "localhost".to_string(),
                port: 7891,
                channel: 3,
            }
        );
    }

    #[tokio::test]
    async fn client_writes_encoded_frame_to_tcp_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let addr = listener.local_addr().expect("has local addr");
        let read = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accepts");
            let mut bytes = vec![0; 7];
            stream.read_exact(&mut bytes).await.expect("reads frame");
            bytes
        });

        let mut client = OpcClient::connect(OpcAddress {
            host: "127.0.0.1".to_string(),
            port: addr.port(),
            channel: 2,
        })
        .await
        .expect("connects");
        client
            .send_frame(&[Rgb::from_u24(0x12_34_56)])
            .await
            .expect("writes");

        assert_eq!(
            read.await.expect("read task joins"),
            vec![2, 0, 0, 3, 0x12, 0x34, 0x56]
        );
    }

    #[test]
    fn maps_dome_pixel_commands_to_device_frame() {
        let mut channel = PersistentChannel::default();

        apply_dome_commands(
            &mut channel,
            &[
                DomeCommand::Pixel {
                    strut_index: 0,
                    led_index: 0,
                    color: Rgb::from_u24(0xff_00_00),
                },
                DomeCommand::Flush,
            ],
        );

        let encoded = channel.encode_current(0);
        let device_index = 880;
        let byte_index = 4 + device_index * 3;
        assert_eq!(&encoded[byte_index..byte_index + 3], &[0xff, 0, 0]);
    }

    #[test]
    fn maps_bar_pixels_through_dome_control_box_five() {
        let mut channel = PersistentChannel::default();

        apply_bar_commands(
            &mut channel,
            &[
                BarCommand::Pixel {
                    is_runner: true,
                    led_index: 0,
                    color: Rgb::from_u24(0x00_ff_00),
                },
                BarCommand::Flush,
            ],
            50,
            50,
            50,
        );

        let encoded = channel.encode_current(0);
        let device_index = 5 * 214 * 8 + 200;
        let byte_index = 4 + device_index * 3;
        assert_eq!(&encoded[byte_index..byte_index + 3], &[0, 0xff, 0]);
    }

    #[test]
    fn maps_stage_pixels_to_flat_opc_indices() {
        let side_lengths = [18, 19, 17];
        let mut channel = PersistentChannel::default();

        apply_stage_commands(
            &mut channel,
            &[
                StageCommand::Pixel {
                    side_index: 1,
                    led_index: 0,
                    layer_index: 2,
                    color: Rgb::from_u24(0x00_00_ff),
                },
                StageCommand::Flush,
            ],
            &side_lengths,
        );

        let encoded = channel.encode_current(0);
        let device_index = (18 + 19 + 17) * 2 + 18;
        let byte_index = 4 + device_index * 3;
        assert_eq!(&encoded[byte_index..byte_index + 3], &[0, 0, 0xff]);
    }
}
