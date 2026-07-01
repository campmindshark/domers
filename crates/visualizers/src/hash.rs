use domers_outputs::{BarCommand, DomeCommand, StageCommand};

#[cfg(test)]
pub(crate) fn frame_hash(commands: &[DomeCommand]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for command in commands {
        match command {
            DomeCommand::Flush => hash_byte(&mut hash, 0),
            DomeCommand::Frame(colors) => {
                hash_byte(&mut hash, 1);
                for color in colors {
                    hash_byte(&mut hash, color.r);
                    hash_byte(&mut hash, color.g);
                    hash_byte(&mut hash, color.b);
                }
            }
            DomeCommand::Pixel {
                strut_index,
                led_index,
                color,
            } => {
                hash_byte(&mut hash, 2);
                hash_usize(&mut hash, *strut_index);
                hash_usize(&mut hash, *led_index);
                hash_byte(&mut hash, color.r);
                hash_byte(&mut hash, color.g);
                hash_byte(&mut hash, color.b);
            }
        }
    }
    hash
}

#[cfg(test)]
pub(crate) fn bar_frame_hash(commands: &[BarCommand]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for command in commands {
        match command {
            BarCommand::Flush => hash_byte(&mut hash, 0),
            BarCommand::Pixel {
                is_runner,
                led_index,
                color,
            } => {
                hash_byte(&mut hash, 2);
                hash_byte(&mut hash, u8::from(*is_runner));
                hash_usize(&mut hash, *led_index);
                hash_byte(&mut hash, color.r);
                hash_byte(&mut hash, color.g);
                hash_byte(&mut hash, color.b);
            }
        }
    }
    hash
}

#[cfg(test)]
pub(crate) fn stage_frame_hash(commands: &[StageCommand]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for command in commands {
        match command {
            StageCommand::Flush => hash_byte(&mut hash, 0),
            StageCommand::Pixel {
                side_index,
                led_index,
                layer_index,
                color,
            } => {
                hash_byte(&mut hash, 2);
                hash_usize(&mut hash, *side_index);
                hash_usize(&mut hash, *led_index);
                hash_usize(&mut hash, *layer_index);
                hash_byte(&mut hash, color.r);
                hash_byte(&mut hash, color.g);
                hash_byte(&mut hash, color.b);
            }
        }
    }
    hash
}

#[cfg(test)]
pub(crate) fn hash_byte(hash: &mut u64, byte: u8) {
    *hash ^= u64::from(byte);
    *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
}

#[cfg(test)]
pub(crate) fn hash_usize(hash: &mut u64, value: usize) {
    for byte in value.to_le_bytes() {
        hash_byte(hash, byte);
    }
}
