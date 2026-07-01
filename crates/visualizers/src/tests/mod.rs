use domers_core::import_spectrum_xml;
use domers_outputs::{topology::DOME_PIXELS, DomeCommand};
use serde::Deserialize;

use crate::{
    diagnostics::stage_tracer_led_index,
    dome::{snake_triangles, SNAKES_STEP_FRAMES, SNAKE_TRIANGLE_DEFS},
    hash::{bar_frame_hash, frame_hash, stage_frame_hash},
    input::{
        BarDiagnosticVisualizer, DiagnosticInput, DomeDiagnosticVisualizer, LiveVisualizer,
        MidiNoteInput, OrientationOverride, StageVisualizer, StageVisualizerInput, VisualizerInput,
    },
    input::{MAX_FRAME_MIDI_NOTES, MAX_ORIENTATION_DEVICES},
    inventory::{Classification, INVENTORY},
    render::{
        render_bar_diagnostic, render_dome_diagnostic, render_dome_visualizer,
        render_stage_visualizer, render_stage_visualizer_with_input,
    },
    runtime::VisualizerRuntime,
};

pub(crate) fn frame_colors(commands: &[DomeCommand]) -> &[domers_core::Rgb] {
    commands
        .iter()
        .find_map(|command| match command {
            DomeCommand::Frame(colors) => Some(colors.as_slice()),
            DomeCommand::Flush | DomeCommand::Pixel { .. } => None,
        })
        .expect("visualizer should write a whole preview frame")
}

#[derive(Deserialize)]
pub(crate) struct VisualizerManifest {
    cases: Vec<VisualizerCase>,
}

#[derive(Deserialize)]
pub(crate) struct VisualizerCase {
    case: String,
    name: String,
    expected: ExpectedHash,
    input: ManifestInput,
}

#[derive(Deserialize)]
pub(crate) struct ExpectedHash {
    status: String,
    value: String,
}

#[derive(Clone, Deserialize)]
pub(crate) struct ManifestInput {
    volume: f32,
    beat_progress: f64,
    flash_active: bool,
    diagnostic_state: u8,
    diagnostic_step: usize,
    palette_slot: u8,
    #[serde(default)]
    midi: Vec<MidiNoteInput>,
}

#[test]
pub(crate) fn inventory_tracks_used_spectrum_visualizers() {
    assert_eq!(INVENTORY.len(), 17);
    assert_eq!(
        INVENTORY
            .iter()
            .filter(|visualizer| visualizer.classification == Classification::Live)
            .count(),
        11
    );
    assert_eq!(
        INVENTORY
            .iter()
            .filter(|visualizer| visualizer.classification == Classification::Support)
            .count(),
        6
    );
}

#[test]
pub(crate) fn spectrum_visualizer_fixture_manifest_covers_inventory() {
    let manifest = include_str!("../../../../fixtures/spectrum-csharp/visualizer_frame_cases.json");
    for visualizer in INVENTORY {
        assert!(
            manifest.contains(&format!("\"name\": \"{}\"", visualizer.name)),
            "{} should have a source-traceable fixture case",
            visualizer.name
        );
        assert!(
            manifest.contains(&format!(
                "spectrum/Spectrum/Visualizers/{}.cs",
                visualizer.name
            )),
            "{} should cite its Spectrum source file",
            visualizer.name
        );
    }
    assert_eq!(
        manifest.matches("\"source_sha256\"").count(),
        INVENTORY.len()
    );
    assert_eq!(
        manifest.matches("\"status\": \"captured\"").count(),
        INVENTORY.len()
    );
    assert!(!manifest.contains("\"pending_csharp_execution\""));
    assert!(!manifest.contains("\"value\": null"));
}

#[test]
pub(crate) fn spectrum_visualizer_sequence_manifest_covers_live_dome_visualizers() {
    let manifest =
        include_str!("../../../../fixtures/spectrum-csharp/visualizer_sequence_cases.json");
    for visualizer in INVENTORY
        .iter()
        .filter(|visualizer| visualizer.classification == Classification::Live)
    {
        assert!(
            manifest.contains(&format!("\"name\": \"{}\"", visualizer.name)),
            "{} should have a multi-frame fixture sequence",
            visualizer.name
        );
    }
    assert_eq!(
        manifest
            .matches("\"kind\": \"frame_sequence_hashes\"")
            .count(),
        11
    );
    assert_eq!(manifest.matches("\"input_sequence\"").count(), 11);
    // Sequence goldens are captured incrementally, so the split between
    // captured and still-pending cases shifts over time; only the total
    // must stay complete.
    let captured = manifest.matches("\"status\": \"captured\"").count();
    let pending = manifest
        .matches("\"status\": \"pending_csharp_execution\"")
        .count();
    assert_eq!(captured + pending, 11);
}

#[test]
#[ignore = "run explicitly while closing Spectrum visualizer exactness gaps"]
pub(crate) fn rust_visualizer_hashes_match_spectrum_csharp_goldens() {
    let manifest: VisualizerManifest = serde_json::from_str(include_str!(
        "../../../../fixtures/spectrum-csharp/visualizer_frame_cases.json"
    ))
    .expect("visualizer manifest parses");
    let spectrum_config = import_spectrum_xml(include_str!(
        "../../../../fixtures/config/spectrum_default_config.xml"
    ))
    .config;
    let mut mismatches = Vec::new();

    for test_case in &manifest.cases {
        assert_eq!(
            test_case.expected.status, "captured",
            "{} must have captured Spectrum hash",
            test_case.name
        );
        let expected = test_case
            .expected
            .value
            .parse::<u64>()
            .expect("expected hash is u64");
        let actual = render_manifest_case_hash(test_case, &spectrum_config);
        if actual != expected {
            mismatches.push(format!(
                "{} / {}: expected {expected}, got {actual}",
                test_case.case, test_case.name
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "Rust visualizer hashes differ from Spectrum C# goldens:\n{}",
        mismatches.join("\n")
    );
}

#[derive(Deserialize)]
pub(crate) struct SequenceManifest {
    capture_metadata: SequenceMeta,
    cases: Vec<SequenceCase>,
}

#[derive(Deserialize)]
pub(crate) struct SequenceMeta {
    frame_delta_ticks: i64,
    clock_base_ticks: i64,
    beat_measure_ms: u32,
}

#[derive(Deserialize)]
pub(crate) struct SequenceCase {
    case: String,
    name: String,
    expected: SequenceExpected,
    input_sequence: Vec<ManifestInput>,
    #[serde(default)]
    frame_delta_ticks: Option<i64>,
}

#[derive(Deserialize)]
pub(crate) struct SequenceExpected {
    status: String,
    #[serde(default)]
    frames: Vec<String>,
}

/// C# `TimeSpan.TicksPerMillisecond`, used to convert the capture clock's
/// tick counter into the wall-clock milliseconds the runtime consumes.
pub(crate) const TICKS_PER_MS: i64 = 10_000;

pub(crate) fn live_visualizer_for_manifest_name(name: &str) -> Option<LiveVisualizer> {
    Some(match name {
        "LEDDomeVolumeVisualizer" => LiveVisualizer::Volume,
        "LEDDomeRadialVisualizer" => LiveVisualizer::Radial,
        "LEDDomeRaceVisualizer" => LiveVisualizer::Race,
        "LEDDomeSnakesVisualizer" => LiveVisualizer::Snakes,
        "LEDDomeSplatVisualizer" => LiveVisualizer::Splat,
        "LEDDomeQuaternionTestVisualizer" => LiveVisualizer::QuaternionTest,
        "LEDDomeQuaternionMultiTestVisualizer" => LiveVisualizer::QuaternionMultiTest,
        "LEDDomeQuaternionPaintbrushVisualizer" => LiveVisualizer::QuaternionPaintbrush,
        "LEDDomeTVStaticVisualizer" => LiveVisualizer::TvStatic,
        "LEDDomeFlashVisualizer" => LiveVisualizer::Flash,
        _ => return None,
    })
}

/// Replay one captured multi-frame Spectrum sequence through the persistent
/// `VisualizerRuntime` and compare each frame's FNV-1a hash to the C# golden.
#[test]
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "capture clock ticks and frame counts stay well within range"
)]
pub(crate) fn rust_visualizer_sequences_match_spectrum_csharp_goldens() {
    let manifest: SequenceManifest = serde_json::from_str(include_str!(
        "../../../../fixtures/spectrum-csharp/visualizer_sequence_cases.json"
    ))
    .expect("sequence manifest parses");
    let config = import_spectrum_xml(include_str!(
        "../../../../fixtures/config/spectrum_default_config.xml"
    ))
    .config;
    let meta = &manifest.capture_metadata;
    let mut mismatches = Vec::new();
    let mut checked = 0usize;

    for test_case in &manifest.cases {
        if test_case.expected.status != "captured" {
            continue;
        }
        let Some(visualizer) = live_visualizer_for_manifest_name(&test_case.name) else {
            // Stage/bar sequences are hashed on their own outputs, not the
            // dome runtime; they are covered elsewhere.
            continue;
        };
        checked += 1;

        let mut runtime = VisualizerRuntime::default();
        let frame_delta = test_case
            .frame_delta_ticks
            .unwrap_or(meta.frame_delta_ticks);
        for (frame_index, frame_input) in test_case.input_sequence.iter().enumerate() {
            let now_ticks = meta.clock_base_ticks + (frame_index as i64) * frame_delta;
            let now_ms = (now_ticks / TICKS_PER_MS) as u64;

            let mut input = visualizer_input(frame_input, &config);
            input.now_ms = now_ms;
            input.measure_length_ms = Some(meta.beat_measure_ms);
            input.animation_frame = u64::try_from(frame_index).expect("frame index fits");
            input.midi_notes = [None; MAX_FRAME_MIDI_NOTES];
            for (slot, note) in frame_input
                .midi
                .iter()
                .enumerate()
                .take(MAX_FRAME_MIDI_NOTES)
            {
                input.midi_notes[slot] = Some(*note);
            }

            let commands = runtime.render_dome(visualizer, input);
            let actual = frame_hash(&commands);
            let expected = test_case
                .expected
                .frames
                .get(frame_index)
                .and_then(|value| value.parse::<u64>().ok());
            if expected != Some(actual) {
                mismatches.push(format!(
                    "{} / {} frame {frame_index}: expected {expected:?}, got {actual}",
                    test_case.case, test_case.name
                ));
            }
        }
    }

    assert!(checked > 0, "no captured dome sequences to verify");
    assert!(
        mismatches.is_empty(),
        "Rust sequence hashes differ from Spectrum C# goldens ({} of them):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

pub(crate) fn render_manifest_case_hash(
    test_case: &VisualizerCase,
    config: &domers_core::DomersConfig,
) -> u64 {
    let live_input = visualizer_input(&test_case.input, config);
    let diagnostic_input = DiagnosticInput {
        state: test_case.input.diagnostic_state,
        step: test_case.input.diagnostic_step,
        brightness: 1.0,
        volume: test_case.input.volume,
        beat_progress: test_case.input.beat_progress,
    };
    match test_case.name.as_str() {
        "LEDDomeStrutIterationDiagnosticVisualizer" => frame_hash(&render_dome_diagnostic(
            DomeDiagnosticVisualizer::StrutIteration,
            diagnostic_input,
        )),
        "LEDDomeFlashColorsDiagnosticVisualizer" => frame_hash(&render_dome_diagnostic(
            DomeDiagnosticVisualizer::FlashColors,
            diagnostic_input,
        )),
        "LEDDomeStrandTestDiagnosticVisualizer" => frame_hash(&render_dome_diagnostic(
            DomeDiagnosticVisualizer::StrandTest,
            diagnostic_input,
        )),
        "LEDDomeFullColorFlashDiagnosticVisualizer" => frame_hash(&render_dome_diagnostic(
            DomeDiagnosticVisualizer::FullColorFlash,
            diagnostic_input,
        )),
        "LEDDomeVolumeVisualizer" => {
            frame_hash(&render_dome_visualizer(LiveVisualizer::Volume, live_input))
        }
        "LEDDomeRadialVisualizer" => {
            frame_hash(&render_dome_visualizer(LiveVisualizer::Radial, live_input))
        }
        "LEDDomeRaceVisualizer" => {
            frame_hash(&render_dome_visualizer(LiveVisualizer::Race, live_input))
        }
        "LEDDomeSnakesVisualizer" => {
            frame_hash(&render_dome_visualizer(LiveVisualizer::Snakes, live_input))
        }
        "LEDDomeSplatVisualizer" => {
            frame_hash(&render_dome_visualizer(LiveVisualizer::Splat, live_input))
        }
        "LEDDomeQuaternionTestVisualizer" => frame_hash(&render_dome_visualizer(
            LiveVisualizer::QuaternionTest,
            live_input,
        )),
        "LEDDomeQuaternionMultiTestVisualizer" => frame_hash(&render_dome_visualizer(
            LiveVisualizer::QuaternionMultiTest,
            live_input,
        )),
        "LEDDomeQuaternionPaintbrushVisualizer" => frame_hash(&render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            live_input,
        )),
        "LEDDomeTVStaticVisualizer" => frame_hash(&render_dome_visualizer(
            LiveVisualizer::TvStatic,
            live_input,
        )),
        "LEDDomeFlashVisualizer" => {
            frame_hash(&render_dome_visualizer(LiveVisualizer::Flash, live_input))
        }
        "LEDBarFlashColorsDiagnosticVisualizer" => bar_frame_hash(&render_bar_diagnostic(
            BarDiagnosticVisualizer::FlashColors,
            diagnostic_input,
            config.bar.infinity_width as usize,
            config.bar.infinity_length as usize,
            config.bar.runner_length as usize,
        )),
        "LEDStageFlashColorsDiagnosticVisualizer" => stage_frame_hash(&render_stage_visualizer(
            StageVisualizer::FlashColorsDiagnostic,
            diagnostic_input,
            &stage_side_lengths(config),
        )),
        "LEDStageDepthLevelVisualizer" => stage_frame_hash(&render_stage_visualizer_with_input(
            StageVisualizer::DepthLevel,
            StageVisualizerInput {
                diagnostic: diagnostic_input,
                color_palette: config.color_palette.clone(),
                color_palette_index: test_case.input.palette_slot,
                stage_brightness: 1.0,
            },
            &stage_side_lengths(config),
        )),
        name => panic!("unhandled visualizer manifest case {name}"),
    }
}

pub(crate) fn visualizer_input(
    input: &ManifestInput,
    config: &domers_core::DomersConfig,
) -> VisualizerInput {
    let palette =
        std::array::from_fn(|index| config.color_palette.single_color(index, input.palette_slot));
    let palette_entries = std::array::from_fn(|index| {
        config
            .color_palette
            .entry(domers_core::ColorPalette::absolute_index(
                index,
                input.palette_slot,
            ))
    });
    VisualizerInput {
        volume: input.volume,
        beat_progress: input.beat_progress,
        animation_frame: 0,
        now_ms: 0,
        measure_length_ms: None,
        beat_progress_rotation: None,
        beat_progress_gradient: None,
        orientation_override: None,
        orientation_devices: [None; MAX_ORIENTATION_DEVICES],
        midi_notes: [None; MAX_FRAME_MIDI_NOTES],
        flash_active: input.flash_active,
        primary: palette[0],
        secondary: palette[1],
        accent: palette[2],
        palette,
        palette_entries,
        dome_brightness: 1.0,
    }
}

pub(crate) fn stage_side_lengths(config: &domers_core::DomersConfig) -> Vec<usize> {
    config
        .stage
        .side_lengths
        .iter()
        .map(|length| *length as usize)
        .collect()
}

#[test]
pub(crate) fn every_initial_live_dome_visualizer_produces_a_simulator_frame() {
    for visualizer in [
        LiveVisualizer::TvStatic,
        LiveVisualizer::Volume,
        LiveVisualizer::Radial,
        LiveVisualizer::Splat,
        LiveVisualizer::Race,
        LiveVisualizer::Snakes,
        LiveVisualizer::QuaternionTest,
        LiveVisualizer::QuaternionMultiTest,
        LiveVisualizer::QuaternionPaintbrush,
    ] {
        let commands = render_dome_visualizer(visualizer, VisualizerInput::default());
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, DomeCommand::Flush)),
            "{visualizer:?} should flush"
        );
        assert!(
            commands.len() >= 2,
            "{visualizer:?} should write before flush"
        );
    }

    assert!(
        render_dome_visualizer(LiveVisualizer::Flash, VisualizerInput::default()).is_empty(),
        "Flash is event-driven and has no first-frame output without an active animation"
    );
}

#[test]
pub(crate) fn buffer_based_modes_use_whole_frame_commands() {
    let commands = render_dome_visualizer(LiveVisualizer::Radial, VisualizerInput::default());
    assert!(commands
        .iter()
        .any(|command| matches!(command, DomeCommand::Frame(_))));
}

#[test]
pub(crate) fn default_volume_preview_is_dome_sized_and_visible() {
    let commands = render_dome_visualizer(LiveVisualizer::Volume, VisualizerInput::default());
    let pixel_count = commands
        .iter()
        .filter(|command| matches!(command, DomeCommand::Pixel { .. }))
        .count();
    let lit_count = commands
        .iter()
        .filter(|command| match command {
            DomeCommand::Pixel { color, .. } => *color != domers_core::Rgb::BLACK,
            DomeCommand::Flush | DomeCommand::Frame(_) => false,
        })
        .count();

    assert!(pixel_count >= DOME_PIXELS);
    assert!(
        lit_count > 1_000,
        "volume visualizer should light a substantial part of the dome"
    );
}

#[test]
pub(crate) fn splat_preview_renders_fading_blobs() {
    let commands = render_dome_visualizer(
        LiveVisualizer::Splat,
        VisualizerInput {
            animation_frame: 120,
            ..VisualizerInput::default()
        },
    );
    let frame = commands
        .iter()
        .find_map(|command| match command {
            DomeCommand::Frame(colors) => Some(colors),
            DomeCommand::Flush | DomeCommand::Pixel { .. } => None,
        })
        .expect("splat visualizer should write a whole preview frame");

    assert_eq!(frame.len(), DOME_PIXELS);
    assert!(
        frame
            .iter()
            .filter(|color| **color != domers_core::Rgb::BLACK)
            .count()
            > 100
    );
}

#[test]
pub(crate) fn tv_static_uses_deterministic_varied_noise() {
    let first = render_dome_visualizer(LiveVisualizer::TvStatic, VisualizerInput::default());
    let second = render_dome_visualizer(LiveVisualizer::TvStatic, VisualizerInput::default());

    assert_eq!(first, second);
    let pixels: Vec<_> = first
        .iter()
        .filter_map(|command| match command {
            DomeCommand::Pixel { color, .. } => Some(*color),
            DomeCommand::Flush | DomeCommand::Frame(_) => None,
        })
        .collect();
    assert_eq!(pixels.len(), DOME_PIXELS);
    assert!(pixels.windows(2).take(100).any(|pair| pair[0] != pair[1]));
    assert!(matches!(first.last(), Some(DomeCommand::Flush)));
}

#[test]
pub(crate) fn runtime_visualizers_animate_after_captured_first_frame() {
    for visualizer in [
        LiveVisualizer::TvStatic,
        LiveVisualizer::Radial,
        LiveVisualizer::Splat,
        LiveVisualizer::Race,
    ] {
        let first_runtime = render_dome_visualizer(
            visualizer,
            VisualizerInput {
                animation_frame: 1,
                ..VisualizerInput::default()
            },
        );
        let later_runtime = render_dome_visualizer(
            visualizer,
            VisualizerInput {
                animation_frame: 120,
                ..VisualizerInput::default()
            },
        );
        assert_ne!(
            frame_hash(&first_runtime),
            frame_hash(&later_runtime),
            "{visualizer:?} should animate during live preview"
        );
    }
}

#[test]
pub(crate) fn snakes_animate_across_throttle_steps() {
    let first = render_dome_visualizer(
        LiveVisualizer::Snakes,
        VisualizerInput {
            animation_frame: 0,
            ..VisualizerInput::default()
        },
    );
    // Same throttle window (< 50 ms) repeats the same first update.
    let same_window = render_dome_visualizer(
        LiveVisualizer::Snakes,
        VisualizerInput {
            animation_frame: SNAKES_STEP_FRAMES - 1,
            ..VisualizerInput::default()
        },
    );
    let next_step = render_dome_visualizer(
        LiveVisualizer::Snakes,
        VisualizerInput {
            animation_frame: SNAKES_STEP_FRAMES,
            ..VisualizerInput::default()
        },
    );
    assert_eq!(frame_hash(&first), frame_hash(&same_window));
    assert_ne!(
        frame_hash(&first),
        frame_hash(&next_step),
        "Snakes should advance to the next triangle after the throttle window"
    );
    // Each snake writes one triangle (3 struts) of pixels plus a trailing flush.
    assert!(matches!(first.last(), Some(DomeCommand::Flush)));
    assert!(first
        .iter()
        .any(|command| matches!(command, DomeCommand::Pixel { .. })));
}

#[test]
pub(crate) fn snakes_move_between_connected_triangles() {
    let triangles = snake_triangles();
    assert_eq!(triangles.len(), SNAKE_TRIANGLE_DEFS.len());
    // Triangle 0 (72,71,0) must have at least one directional neighbor so the
    // snake can leave the seed triangle.
    let seed = triangles[0];
    assert!(
        seed.left.is_some() || seed.above.is_some() || seed.right.is_some() || seed.below.is_some(),
        "seed triangle must connect to the graph"
    );
    // Every triangle indexes valid struts and most connect to neighbors.
    let connected = triangles
        .iter()
        .filter(|t| t.left.is_some() || t.above.is_some() || t.right.is_some() || t.below.is_some())
        .count();
    assert!(
        connected >= triangles.len() - 1,
        "snake graph should be almost fully connected, got {connected}/{}",
        triangles.len()
    );
}

pub(crate) fn pixel_commands(commands: &[DomeCommand]) -> Vec<(usize, usize, domers_core::Rgb)> {
    commands
        .iter()
        .filter_map(|command| match command {
            DomeCommand::Pixel {
                strut_index,
                led_index,
                color,
            } => Some((*strut_index, *led_index, *color)),
            DomeCommand::Flush | DomeCommand::Frame(_) => None,
        })
        .collect()
}

#[test]
pub(crate) fn race_racers_advance_with_wall_clock_time() {
    let mut runtime = VisualizerRuntime::default();
    let base = VisualizerInput {
        volume: 0.8,
        beat_progress: 0.25,
        now_ms: 0,
        ..VisualizerInput::default()
    };
    let first = runtime.render_dome(LiveVisualizer::Race, base);
    // A second later the volume-driven racers must have rotated.
    let later = runtime.render_dome(
        LiveVisualizer::Race,
        VisualizerInput {
            now_ms: 1_000,
            ..base
        },
    );
    assert_ne!(
        pixel_commands(&first),
        pixel_commands(&later),
        "Race racers should rotate as wall-clock time advances"
    );
}

#[test]
pub(crate) fn tv_static_advances_persistent_rng_across_frames() {
    let mut runtime = VisualizerRuntime::default();
    let first =
        pixel_commands(&runtime.render_dome(LiveVisualizer::TvStatic, VisualizerInput::default()));
    let second =
        pixel_commands(&runtime.render_dome(LiveVisualizer::TvStatic, VisualizerInput::default()));
    assert_eq!(first.len(), second.len());
    assert_ne!(
        first, second,
        "TV static should keep advancing one RNG, not repeat the same frame"
    );
}

#[test]
pub(crate) fn switch_wipes_previous_visualizer() {
    let mut runtime = VisualizerRuntime::default();
    // Snakes emits pixel deltas; the very first activation must not prepend a
    // clearing frame (nothing is on the dome yet).
    let first = runtime.render_dome(LiveVisualizer::Snakes, VisualizerInput::default());
    assert!(
        !matches!(first.first(), Some(DomeCommand::Frame(_))),
        "first activation should not emit a clearing frame"
    );
    // Switching visualizers clears the dome with a leading all-black frame.
    let switched = runtime.render_dome(LiveVisualizer::Radial, VisualizerInput::default());
    match switched.first() {
        Some(DomeCommand::Frame(colors)) => {
            assert_eq!(colors.len(), DOME_PIXELS);
            assert!(colors.iter().all(|color| *color == domers_core::Rgb::BLACK));
        }
        _ => panic!("visualizer switch should begin with a black clearing frame"),
    }
}

#[test]
pub(crate) fn splat_spawns_on_beat_wrap() {
    let mut runtime = VisualizerRuntime::default();
    let non_black = |commands: &[DomeCommand]| {
        frame_colors(commands)
            .iter()
            .any(|color| *color != domers_core::Rgb::BLACK)
    };
    // Rising progress: fade only, no spawn yet.
    let _ = runtime.render_dome(
        LiveVisualizer::Splat,
        VisualizerInput {
            beat_progress: 0.2,
            ..VisualizerInput::default()
        },
    );
    let before_wrap = runtime.render_dome(
        LiveVisualizer::Splat,
        VisualizerInput {
            beat_progress: 0.9,
            ..VisualizerInput::default()
        },
    );
    assert!(!non_black(&before_wrap), "no splat before the beat wraps");
    // Progress wraps downward (0.9 -> 0.1): a splat spawns.
    let after_wrap = runtime.render_dome(
        LiveVisualizer::Splat,
        VisualizerInput {
            beat_progress: 0.1,
            ..VisualizerInput::default()
        },
    );
    assert!(
        non_black(&after_wrap),
        "splat should spawn when beat progress wraps"
    );
}

#[test]
pub(crate) fn quaternion_multi_uses_orientation_devices_not_fake_idle_motion() {
    let idle = render_dome_visualizer(
        LiveVisualizer::QuaternionMultiTest,
        VisualizerInput {
            animation_frame: 120,
            ..VisualizerInput::default()
        },
    );
    let oriented = render_dome_visualizer(
        LiveVisualizer::QuaternionMultiTest,
        VisualizerInput {
            animation_frame: 120,
            orientation_override: Some(OrientationOverride {
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0,
            }),
            ..VisualizerInput::default()
        },
    );

    assert!(frame_colors(&idle)
        .iter()
        .all(|color| *color == domers_core::Rgb::BLACK));
    assert!(frame_colors(&oriented)
        .iter()
        .any(|color| *color != domers_core::Rgb::BLACK));
}

#[test]
pub(crate) fn volume_animation_uses_beat_progress_like_spectrum() {
    let first_runtime = render_dome_visualizer(
        LiveVisualizer::Volume,
        VisualizerInput {
            animation_frame: 1,
            beat_progress: 0.10,
            ..VisualizerInput::default()
        },
    );
    let later_runtime = render_dome_visualizer(
        LiveVisualizer::Volume,
        VisualizerInput {
            animation_frame: 120,
            beat_progress: 1.0,
            ..VisualizerInput::default()
        },
    );
    assert_ne!(
        frame_hash(&first_runtime),
        frame_hash(&later_runtime),
        "Volume should follow beat progress instead of a synthetic rotating shape"
    );
}

#[test]
pub(crate) fn quaternion_paintbrush_idle_path_uses_animation_frame() {
    let input = VisualizerInput {
        volume: 0.6,
        beat_progress: 0.25,
        animation_frame: 0,
        ..VisualizerInput::default()
    };
    let later = VisualizerInput {
        animation_frame: 360,
        ..input
    };

    assert_ne!(
        frame_hash(&render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            input
        )),
        frame_hash(&render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            later
        )),
        "idle paintbrush should not retrace a constant path when beat phase is unchanged"
    );
}

#[test]
pub(crate) fn quaternion_paintbrush_accumulates_spectrum_style_paint_layers() {
    let first = render_dome_visualizer(
        LiveVisualizer::QuaternionPaintbrush,
        VisualizerInput {
            animation_frame: 0,
            ..VisualizerInput::default()
        },
    );
    let later = render_dome_visualizer(
        LiveVisualizer::QuaternionPaintbrush,
        VisualizerInput {
            animation_frame: 360,
            ..VisualizerInput::default()
        },
    );
    let first_lit = frame_colors(&first)
        .iter()
        .filter(|color| **color != domers_core::Rgb::BLACK)
        .count();
    let later_lit = frame_colors(&later)
        .iter()
        .filter(|color| **color != domers_core::Rgb::BLACK)
        .count();

    assert!(
        later_lit > first_lit,
        "paintbrush should retain trailing paint and ripple layers after the captured first frame"
    );
}

#[test]
pub(crate) fn quaternion_paintbrush_event_layers_do_not_loop_reset() {
    let early = render_dome_visualizer(
        LiveVisualizer::QuaternionPaintbrush,
        VisualizerInput {
            animation_frame: 360,
            ..VisualizerInput::default()
        },
    );
    let later = render_dome_visualizer(
        LiveVisualizer::QuaternionPaintbrush,
        VisualizerInput {
            animation_frame: 1_460,
            ..VisualizerInput::default()
        },
    );

    assert_ne!(
        frame_hash(&early),
        frame_hash(&later),
        "paintbrush ripple/stamp event layers must not loop back into an obvious reset"
    );
}

#[test]
pub(crate) fn quaternion_paintbrush_uses_orientation_override() {
    let input = VisualizerInput {
        volume: 0.6,
        beat_progress: 0.25,
        animation_frame: 120,
        ..VisualizerInput::default()
    };
    let overridden = VisualizerInput {
        orientation_override: Some(OrientationOverride {
            yaw: std::f64::consts::FRAC_PI_2,
            pitch: -std::f64::consts::FRAC_PI_4,
            roll: 0.0,
        }),
        ..input
    };

    assert_ne!(
        frame_hash(&render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            input
        )),
        frame_hash(&render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            overridden
        )),
        "manual simulator orientation should steer orientation visualizers"
    );
}

#[test]
pub(crate) fn live_visualizer_frame_hashes_are_stable() {
    let cases = [
        (LiveVisualizer::TvStatic, 7_938_821_499_849_451_788),
        (LiveVisualizer::Volume, 3_360_946_268_713_528_047),
        (LiveVisualizer::Flash, 14_695_981_039_346_656_037),
        (LiveVisualizer::Radial, 8_095_729_372_390_775_204),
        (LiveVisualizer::Splat, 12_459_070_695_921_506_308),
        (LiveVisualizer::Race, 7_871_414_923_077_219_675),
        (LiveVisualizer::Snakes, 3_377_082_443_979_724_166),
        (LiveVisualizer::QuaternionTest, 1_564_991_241_466_880_178),
        (
            LiveVisualizer::QuaternionMultiTest,
            12_459_070_695_921_506_308,
        ),
        (
            LiveVisualizer::QuaternionPaintbrush,
            5_139_703_606_261_245_084,
        ),
    ];
    let actual: Vec<_> = cases
        .iter()
        .map(|(visualizer, _expected)| {
            let commands = render_dome_visualizer(*visualizer, VisualizerInput::default());
            (*visualizer, frame_hash(&commands))
        })
        .collect();
    let expected: Vec<_> = cases.into_iter().collect();
    assert_eq!(actual, expected);
}

#[test]
pub(crate) fn live_visualizers_consume_full_palette_bank() {
    let mut custom = VisualizerInput::default();
    custom.palette[3] = domers_core::Rgb::from_u24(0x11_22_33);
    custom.palette[4] = domers_core::Rgb::from_u24(0x44_55_66);
    custom.palette[5] = domers_core::Rgb::from_u24(0x77_88_99);
    custom.palette[6] = domers_core::Rgb::from_u24(0xaa_bb_cc);
    custom.palette_entries[4] = domers_core::PaletteEntry::solid(0x44_55_66);
    custom.palette_entries[5] = domers_core::PaletteEntry::solid(0x77_88_99);
    custom.palette_entries[6] = domers_core::PaletteEntry::solid(0xaa_bb_cc);

    let visualizer = LiveVisualizer::Radial;
    assert_ne!(
        frame_hash(&render_dome_visualizer(
            visualizer,
            VisualizerInput::default()
        )),
        frame_hash(&render_dome_visualizer(visualizer, custom)),
        "{visualizer:?} should use palette entries beyond Color 1-3"
    );
}

#[test]
pub(crate) fn used_dome_diagnostics_produce_frames() {
    for visualizer in [
        DomeDiagnosticVisualizer::FlashColors,
        DomeDiagnosticVisualizer::StrutIteration,
        DomeDiagnosticVisualizer::StrandTest,
        DomeDiagnosticVisualizer::FullColorFlash,
    ] {
        let commands = render_dome_diagnostic(visualizer, DiagnosticInput::default());
        let pixels = commands
            .iter()
            .filter(|command| matches!(command, DomeCommand::Pixel { .. }))
            .count();
        assert!(pixels > 0, "diagnostic should write pixels");
        assert!(commands
            .iter()
            .any(|command| matches!(command, DomeCommand::Flush)));
    }
}

#[test]
pub(crate) fn used_bar_diagnostic_covers_runner_and_infinity() {
    let commands = render_bar_diagnostic(
        BarDiagnosticVisualizer::FlashColors,
        DiagnosticInput::default(),
        4,
        6,
        5,
    );

    assert!(commands.iter().any(|command| matches!(
        command,
        domers_outputs::BarCommand::Pixel {
            is_runner: false,
            ..
        }
    )));
    assert!(commands.iter().any(|command| matches!(
        command,
        domers_outputs::BarCommand::Pixel {
            is_runner: true,
            ..
        }
    )));
    assert!(commands
        .iter()
        .any(|command| matches!(command, domers_outputs::BarCommand::Flush)));
}

#[test]
pub(crate) fn used_stage_visualizers_produce_layered_pixels() {
    for visualizer in [
        StageVisualizer::FlashColorsDiagnostic,
        StageVisualizer::DepthLevel,
    ] {
        let commands = render_stage_visualizer(visualizer, DiagnosticInput::default(), &[3, 4, 5]);
        assert!(commands.iter().any(|command| matches!(
            command,
            domers_outputs::StageCommand::Pixel { layer_index: 2, .. }
        )));
        assert!(commands
            .iter()
            .any(|command| matches!(command, domers_outputs::StageCommand::Flush)));
    }
}

#[test]
pub(crate) fn stage_tracer_index_matches_spectrum_side_progression() {
    let side_lengths = [10, 20, 30];

    assert_eq!(stage_tracer_led_index(&side_lengths, 0, 0.0), 0);
    assert_eq!(stage_tracer_led_index(&side_lengths, 0, 0.25), 7);
    assert_eq!(stage_tracer_led_index(&side_lengths, 0, 0.5), 20);
    assert_eq!(stage_tracer_led_index(&side_lengths, 0, 0.75), 37);
}

#[test]
pub(crate) fn stage_depth_level_emits_layered_pixels() {
    let commands = render_stage_visualizer(
        StageVisualizer::DepthLevel,
        DiagnosticInput {
            beat_progress: 0.0,
            volume: 1.0,
            ..DiagnosticInput::default()
        },
        &[10, 20, 30],
    );

    assert!(commands.iter().any(|command| matches!(
        command,
        domers_outputs::StageCommand::Pixel {
            side_index: 0,
            led_index: 0,
            layer_index: 0,
            ..
        }
    )));
    assert!(commands
        .iter()
        .any(|command| matches!(command, domers_outputs::StageCommand::Flush)));
}
