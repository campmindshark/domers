use domers_outputs::{
    dome_strut_length,
    topology::{DOME_PIXELS, DOME_STRUTS},
    DomeCommand,
};

use crate::{input::VisualizerInput, rng::DotNetRandom};

pub(crate) fn tv_static_commands(input: VisualizerInput) -> Vec<DomeCommand> {
    let seed = i32::try_from(input.animation_frame % i32::MAX as u64)
        .expect("TV static frame seed fits in i32");
    let mut random = DotNetRandom::new(seed);
    let mut commands = Vec::with_capacity(DOME_PIXELS + 1);
    for strut_index in 0..DOME_STRUTS {
        let Some(strut_length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..strut_length {
            commands.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color: random.next_color(255),
            });
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}
