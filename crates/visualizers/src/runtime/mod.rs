mod flash;
mod paintbrush;
mod quaternion_multi;
mod race;
mod radial;
mod snakes;
mod splat;
mod strut_iteration;
mod tv_static;
mod volume;

use domers_outputs::DomeCommand;

use crate::{
    dome::dome_blackout_commands,
    input::{LiveVisualizer, VisualizerInput},
    render::render_dome_visualizer,
};

use flash::FlashRuntime;
use paintbrush::PaintbrushRuntime;
use quaternion_multi::QuaternionMultiRuntime;
use race::RaceRuntime;
use radial::RadialRuntime;
use snakes::SnakesRuntime;
use splat::SplatRuntime;
use strut_iteration::StrutIterationRuntime;
use tv_static::TvStaticRuntime;
use volume::VolumeRuntime;

/// Persistent per-visualizer runtime driving the live and sandbox render loops.
///
/// Unlike [`render_dome_visualizer`], this keeps long-lived per-visualizer state
/// and advances it using wall-clock deltas from [`VisualizerInput::now_ms`].
#[derive(Clone, Debug, Default)]
pub struct VisualizerRuntime {
    active: Option<LiveVisualizer>,
    strut_iteration_active: bool,
    snakes: Option<SnakesRuntime>,
    race: Option<RaceRuntime>,
    radial: Option<RadialRuntime>,
    splat: Option<SplatRuntime>,
    tv_static: Option<TvStaticRuntime>,
    volume: Option<VolumeRuntime>,
    flash: Option<FlashRuntime>,
    paintbrush: Option<PaintbrushRuntime>,
    quaternion_multi: Option<QuaternionMultiRuntime>,
    strut_iteration: Option<StrutIterationRuntime>,
}

impl VisualizerRuntime {
    /// Render the dome commands for `visualizer`, advancing persistent state.
    #[must_use]
    pub fn render_dome(
        &mut self,
        visualizer: LiveVisualizer,
        input: VisualizerInput,
    ) -> Vec<DomeCommand> {
        let previous = self.active;
        let switched = previous != Some(visualizer);
        if switched {
            self.reset();
            self.active = Some(visualizer);
        }

        let mut commands = Vec::new();
        if switched && (previous.is_some() || visualizer == LiveVisualizer::Snakes) {
            commands.extend(dome_blackout_commands());
        }

        match visualizer {
            LiveVisualizer::Snakes => {
                let runtime = self.snakes.get_or_insert_with(SnakesRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::Race => {
                let runtime = self.race.get_or_insert_with(RaceRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::Radial => {
                let runtime = self.radial.get_or_insert_with(RadialRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::Splat => {
                let runtime = self.splat.get_or_insert_with(SplatRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::TvStatic => {
                let runtime = self.tv_static.get_or_insert_with(TvStaticRuntime::new);
                runtime.render(&mut commands);
            }
            LiveVisualizer::Volume => {
                let runtime = self.volume.get_or_insert_with(VolumeRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::Flash => {
                let runtime = self.flash.get_or_insert_with(FlashRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::QuaternionPaintbrush => {
                let runtime = self.paintbrush.get_or_insert_with(PaintbrushRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::QuaternionMultiTest => {
                let runtime = self
                    .quaternion_multi
                    .get_or_insert_with(QuaternionMultiRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::QuaternionTest => {
                commands.extend(render_dome_visualizer(
                    LiveVisualizer::QuaternionTest,
                    input,
                ));
            }
        }

        commands
    }

    /// Render strut iteration diagnostic with Spectrum enable-reset semantics.
    #[must_use]
    pub fn render_strut_iteration(&mut self, now_ms: u64, brightness: f32) -> Vec<DomeCommand> {
        let mut commands = Vec::new();
        if !self.strut_iteration_active {
            commands.extend(dome_blackout_commands());
            self.strut_iteration = Some(StrutIterationRuntime::new());
        }
        self.strut_iteration_active = true;
        if let Some(runtime) = &mut self.strut_iteration {
            commands.extend(runtime.render(now_ms, brightness));
        }
        commands
    }

    /// Clear strut-iteration state when the diagnostic pattern is disabled.
    pub fn clear_strut_iteration(&mut self) {
        self.strut_iteration_active = false;
        self.strut_iteration = None;
    }

    /// Drop all persistent per-visualizer state (invoked on visualizer switch).
    pub(crate) fn reset(&mut self) {
        self.snakes = None;
        self.race = None;
        self.radial = None;
        self.splat = None;
        self.tv_static = None;
        self.volume = None;
        self.flash = None;
        self.paintbrush = None;
        self.quaternion_multi = None;
    }
}
