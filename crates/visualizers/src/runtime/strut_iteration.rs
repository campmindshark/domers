use domers_core::Rgb;
use domers_outputs::{
    dome_strut_index_for_control_box, dome_strut_length, topology::DOME_STRUTS, DomeCommand,
};

/// Wall-clock throttled strut iteration matching `LEDDomeStrutIterationDiagnosticVisualizer`.
#[derive(Clone, Debug)]
pub(crate) struct StrutIterationRuntime {
    last_index: i32,
    last_control_box: i32,
    color: u32,
    last_tick_ms: Option<u64>,
}

impl StrutIterationRuntime {
    pub(crate) fn new() -> Self {
        Self {
            last_index: 37,
            last_control_box: 4,
            color: 0xff_00_00,
            last_tick_ms: None,
        }
    }

    pub(crate) fn render(&mut self, now_ms: u64, brightness: f32) -> Vec<DomeCommand> {
        if self.last_tick_ms.is_none() {
            self.last_tick_ms = Some(now_ms);
            return Vec::new();
        }
        if let Some(last) = self.last_tick_ms {
            if now_ms.saturating_sub(last) <= 1_000 {
                return Vec::new();
            }
        }
        self.last_tick_ms = Some(now_ms);
        self.last_index += 1;

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Spectrum scales dome brightness into a clamped 0-255 byte"
        )]
        let brightness_byte = (255.0_f64 * f64::from(brightness)).clamp(0.0, 255.0) as u32;
        let white_color = (brightness_byte << 16) | (brightness_byte << 8) | brightness_byte;
        let mut commands = Vec::new();

        if self.last_index == 38 {
            self.last_index = 0;
            self.last_control_box = (self.last_control_box + 1) % 5;

            for strut_index in 0..DOME_STRUTS {
                let Some(strut_length) = dome_strut_length(strut_index) else {
                    continue;
                };
                for led_index in 0..strut_length {
                    commands.push(DomeCommand::Pixel {
                        strut_index,
                        led_index,
                        color: Rgb::from_u24(0x00_00_ff),
                    });
                }
            }

            if self.last_control_box == 0 {
                self.color = match self.color {
                    0xff_00_00 => 0x00_ff_00,
                    0x00_ff_00 => 0x00_00_ff,
                    0x00_00_ff => 0xff_ff_ff,
                    _ => 0xff_00_00,
                };
            }
        }

        let strut_index = dome_strut_index_for_control_box(
            usize::try_from(self.last_control_box).expect("control box fits in usize"),
            usize::try_from(self.last_index).expect("local index fits in usize"),
        );
        if let Some(strut_index) = strut_index {
            if let Some(strut_length) = dome_strut_length(strut_index) {
                let color = Rgb::from_u24(self.color & white_color);
                for led_index in 0..strut_length {
                    commands.push(DomeCommand::Pixel {
                        strut_index,
                        led_index,
                        color,
                    });
                }
            }
        }
        commands.push(DomeCommand::Flush);
        commands
    }
}
