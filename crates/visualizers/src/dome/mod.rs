mod flash;
mod paintbrush;
mod quaternion;
mod race;
mod radial;
mod snakes;
mod splat;
mod tv_static;
mod volume;
mod wipe;

pub(crate) use flash::{
    animate_flash_polygon, clear_flash_strut, flash_layout_struts, FlashPolygonAnimation,
    FlashShape,
};
pub(crate) use paintbrush::quaternion_paintbrush_frame;
pub(crate) use quaternion::{
    quaternion_multi_test_color_at, quaternion_multi_test_frame, quaternion_test_frame,
};
pub(crate) use race::{race_commands, race_pixel_color, RaceRacer, RACE_RACER_CONFIGS};
pub(crate) use radial::radial_frame;
#[cfg(test)]
pub(crate) use snakes::{snake_triangles, SNAKES_STEP_FRAMES, SNAKE_TRIANGLE_DEFS};
pub(crate) use snakes::{snakes_commands, SnakesState, SNAKES_MAX_CATCHUP_STEPS, SNAKES_STEP_MS};
pub(crate) use splat::splat_frame;
pub(crate) use tv_static::tv_static_commands;
pub(crate) use volume::{
    concentric_layout_from_point, volume_center_offset_for_input, volume_commands,
    volume_commands_with_wipe, volume_wipe_commands,
};
pub use volume::{VOLUME_GRADIENT_SPEED, VOLUME_ROTATION_SPEED};
pub(crate) use wipe::dome_blackout_commands;
