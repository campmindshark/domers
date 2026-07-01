use std::collections::VecDeque;
use std::sync::OnceLock;

use domers_core::Rgb;
use domers_outputs::{dome_strut_length, DomeCommand};

use crate::{color_util::scale_rgb_f64, input::VisualizerInput, rng::DotNetRandom};

pub(crate) const SNAKE_LENGTH: usize = 7;
pub(crate) const SNAKES_COLOR_PALETTE_COUNT: i32 = 8;
/// Preview frames per Spectrum 50 ms Snakes throttle step (10 ms preview cadence).
pub(crate) const SNAKES_STEP_FRAMES: u64 = 5;
/// Spectrum Snakes wall-clock throttle interval in milliseconds.
pub(crate) const SNAKES_STEP_MS: u64 = 50;
/// Upper bound on Snakes catch-up steps per render to avoid runaway after stalls.
pub(crate) const SNAKES_MAX_CATCHUP_STEPS: u32 = 8;

/// One dome triangle: three clockwise struts plus directional neighbors, ported
/// from Spectrum's `TriangleSegmentFactory`.
#[derive(Clone, Copy)]
pub(crate) struct TriangleSeg {
    pub(crate) struts: [usize; 3],
    pub(crate) points_up: bool,
    pub(crate) left: Option<usize>,
    pub(crate) above: Option<usize>,
    pub(crate) right: Option<usize>,
    pub(crate) below: Option<usize>,
}

pub(crate) static SNAKE_TRIANGLES: OnceLock<Vec<TriangleSeg>> = OnceLock::new();

pub(crate) fn snake_triangles() -> &'static [TriangleSeg] {
    SNAKE_TRIANGLES.get_or_init(build_snake_triangles)
}

/// (first, second, third, `points_up`) in Spectrum `LoadSegments` order.
pub(crate) const SNAKE_TRIANGLE_DEFS: &[(usize, usize, usize, bool)] = &[
    // Layer 1
    (72, 71, 0, true),
    (73, 21, 72, false),
    (74, 73, 1, true),
    (75, 22, 74, false),
    (76, 75, 2, true),
    (77, 23, 76, false),
    (78, 77, 3, true),
    (79, 24, 78, false),
    (80, 79, 4, true),
    (81, 25, 80, false),
    (82, 81, 5, true),
    (83, 26, 82, false),
    (84, 83, 6, true),
    (85, 27, 84, false),
    (86, 85, 7, true),
    (87, 28, 86, false),
    (88, 87, 8, true),
    (89, 29, 74, false),
    (90, 89, 9, true),
    (91, 30, 74, false),
    (92, 91, 10, true),
    (93, 31, 74, false),
    (94, 93, 11, true),
    (95, 32, 74, false),
    (96, 95, 12, true),
    (97, 33, 74, false),
    (98, 97, 13, true),
    (99, 34, 74, false),
    (100, 99, 14, true),
    (101, 35, 74, false),
    (102, 101, 15, true),
    (103, 36, 74, false),
    (104, 103, 16, true),
    (105, 37, 74, false),
    (106, 105, 17, true),
    (107, 38, 74, false),
    (108, 107, 18, true),
    (109, 39, 108, false),
    (70, 109, 19, true),
    (71, 20, 70, false),
    // Layer 2
    (111, 110, 20, true),
    (112, 40, 111, false),
    (113, 112, 21, true),
    (114, 113, 22, true),
    (115, 41, 114, false),
    (116, 115, 23, true),
    (117, 42, 116, false),
    (118, 117, 24, true),
    (119, 43, 118, false),
    (120, 119, 25, true),
    (121, 120, 26, true),
    (122, 44, 121, false),
    (123, 122, 27, true),
    (124, 45, 123, false),
    (125, 124, 28, true),
    (126, 46, 125, false),
    (127, 126, 29, true),
    (128, 127, 30, true),
    (129, 47, 128, false),
    (130, 129, 31, true),
    (131, 48, 130, false),
    (132, 131, 32, true),
    (133, 49, 132, false),
    (134, 133, 33, true),
    (135, 134, 34, true),
    (136, 50, 135, false),
    (137, 136, 35, true),
    (138, 51, 137, false),
    (139, 138, 36, true),
    (140, 52, 139, false),
    (141, 140, 37, true),
    (142, 141, 38, true),
    (143, 53, 142, false),
    (144, 143, 39, true),
    (110, 54, 144, false),
    // Layer 3
    (147, 146, 40, true),
    (148, 147, 41, true),
    (149, 56, 148, false),
    (150, 149, 42, true),
    (151, 57, 150, false),
    (152, 151, 43, true),
    (153, 152, 44, true),
    (154, 58, 153, false),
    (155, 154, 45, true),
    (156, 59, 155, false),
    (157, 156, 46, true),
    (158, 157, 47, true),
    (159, 60, 158, false),
    (160, 159, 48, true),
    (161, 61, 160, false),
    (162, 161, 49, true),
    (163, 162, 50, true),
    (164, 62, 163, false),
    (165, 164, 51, true),
    (166, 63, 165, false),
    (167, 166, 52, true),
    (168, 167, 53, true),
    (169, 64, 168, false),
    (145, 169, 54, true),
    (146, 55, 145, false),
    // Layer 4
    (171, 170, 55, true),
    (172, 171, 56, true),
    (173, 65, 172, false),
    (174, 173, 57, true),
    (175, 174, 58, true),
    (176, 66, 175, false),
    (177, 176, 59, true),
    (178, 177, 60, true),
    (179, 67, 178, false),
    (180, 179, 61, true),
    (181, 180, 62, true),
    (182, 68, 181, false),
    (183, 182, 63, true),
    (184, 183, 64, true),
    (170, 69, 184, false),
    // Layer 5
    (186, 185, 65, true),
    (187, 186, 66, true),
    (188, 187, 67, true),
    (189, 188, 68, true),
    (185, 189, 69, true),
];

pub(crate) fn build_snake_triangles() -> Vec<TriangleSeg> {
    let mut tris: Vec<TriangleSeg> = Vec::with_capacity(SNAKE_TRIANGLE_DEFS.len());
    for &(first, second, third, points_up) in SNAKE_TRIANGLE_DEFS {
        snake_add_triangle(&mut tris, first, second, third, points_up);
    }
    tris
}

pub(crate) fn snake_add_triangle(
    tris: &mut Vec<TriangleSeg>,
    first: usize,
    second: usize,
    third: usize,
    points_up: bool,
) {
    let new_index = tris.len();
    tris.push(TriangleSeg {
        struts: [first, second, third],
        points_up,
        left: None,
        above: None,
        right: None,
        below: None,
    });

    let find_left = |tris: &[TriangleSeg]| {
        tris.iter().position(|t| {
            (t.struts[1] == first && t.points_up) || (t.struts[2] == first && !t.points_up)
        })
    };

    if points_up {
        if let Some(i) = tris.iter().position(|t| t.struts[1] == third) {
            tris[new_index].below = Some(i);
            tris[i].above = Some(new_index);
        }
        if let Some(i) = find_left(tris) {
            tris[new_index].left = Some(i);
            tris[i].right = Some(new_index);
        }
        if let Some(i) = tris.iter().position(|t| t.struts[0] == second) {
            tris[new_index].right = Some(i);
            tris[i].left = Some(new_index);
        }
    } else {
        if let Some(i) = tris.iter().position(|t| t.struts[2] == second) {
            tris[new_index].above = Some(i);
            tris[i].below = Some(new_index);
        }
        if let Some(i) = find_left(tris) {
            tris[new_index].left = Some(i);
            tris[i].right = Some(new_index);
        }
        if let Some(i) = tris.iter().position(|t| t.struts[0] == third) {
            tris[new_index].right = Some(i);
            tris[i].left = Some(new_index);
        }
    }
}

/// Persistent Snakes state mirroring the Spectrum visualizer instance fields.
#[derive(Clone, Debug)]
pub(crate) struct SnakesState {
    rng: DotNetRandom,
    snakes: [VecDeque<usize>; 2],
    color_palette_index: i32,
}

impl SnakesState {
    pub(crate) fn new() -> Self {
        Self {
            rng: DotNetRandom::new(0),
            snakes: [VecDeque::new(), VecDeque::new()],
            color_palette_index: 0,
        }
    }

    /// Advance one throttled Spectrum update, emitting the delta commands.
    pub(crate) fn step(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let trailing_color = scale_rgb_f64(
            input.palette[self.color_palette_index.unsigned_abs() as usize % 8],
            input.dome_brightness,
        );
        for snake in &mut self.snakes {
            progress_snake(snake, &mut self.rng, trailing_color, out);
        }
        self.color_palette_index =
            (self.color_palette_index + 1) % (SNAKES_COLOR_PALETTE_COUNT - 1);
        out.push(DomeCommand::Flush);
    }
}

pub(crate) fn progress_snake(
    snake: &mut VecDeque<usize>,
    rng: &mut DotNetRandom,
    trailing_color: Rgb,
    out: &mut Vec<DomeCommand>,
) {
    if snake.is_empty() {
        snake.push_back(0);
    }

    let mut next: Option<usize> = None;
    let mut attempt_count: i32 = 0;
    loop {
        let contains = next.is_some_and(|n| snake.iter().any(|&t| t == n));
        if !(next.is_none() || contains) {
            break;
        }
        let prev = attempt_count;
        attempt_count += 1;
        if prev > 10 {
            next = Some(0);
            break;
        }
        let last = *snake.back().expect("snake is non-empty after seeding");
        next = get_next_triangle(last, rng);
    }

    let next_index = next.expect("snake next triangle resolves to a fallback");
    if snake.len() > SNAKE_LENGTH {
        if let Some(tail) = snake.pop_front() {
            set_triangle_color(tail, trailing_color, out);
        }
    }
    snake.push_back(next_index);
    set_triangle_color(next_index, Rgb::BLACK, out);
}

pub(crate) fn get_next_triangle(current: usize, rng: &mut DotNetRandom) -> Option<usize> {
    let tris = snake_triangles();
    let starting = rng.next_int(0, 4);
    let mut direction = starting;
    loop {
        let neighbor = snake_directional(&tris[current], direction);
        direction += 1;
        if neighbor.is_some() {
            return neighbor;
        }
        if direction > 3 {
            direction = 0;
        }
        if direction == starting {
            return None;
        }
    }
}

pub(crate) fn snake_directional(triangle: &TriangleSeg, direction: i32) -> Option<usize> {
    match direction {
        0 => triangle.left,
        1 => triangle.above,
        3 => triangle.below,
        _ => triangle.right,
    }
}

pub(crate) fn set_triangle_color(triangle: usize, color: Rgb, out: &mut Vec<DomeCommand>) {
    let tris = snake_triangles();
    for &strut_index in &tris[triangle].struts {
        let Some(length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..length {
            out.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color,
            });
        }
    }
}

pub(crate) fn snakes_commands(input: VisualizerInput) -> Vec<DomeCommand> {
    let steps = input.animation_frame / SNAKES_STEP_FRAMES + 1;
    let mut state = SnakesState::new();
    let mut out = Vec::new();
    for _ in 0..steps {
        out.clear();
        state.step(&input, &mut out);
    }
    out
}
