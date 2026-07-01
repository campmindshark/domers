mod flash;
mod paintbrush;
mod race;
mod radial;
mod snakes;
mod splat;
mod tv_static;
mod volume;

use domers_core::Rgb;
use domers_outputs::{topology::DOME_PIXELS, DomeCommand};

use crate::{
    input::{LiveVisualizer, VisualizerInput},
    render::render_dome_visualizer,
};

use flash::FlashRuntime;
use paintbrush::PaintbrushRuntime;
use race::RaceRuntime;
use radial::RadialRuntime;
use snakes::SnakesRuntime;
use splat::SplatRuntime;
use tv_static::TvStaticRuntime;
use volume::VolumeRuntime;

/// Persistent per-visualizer runtime driving the live and sandbox render loops.
///
/// Unlike [`render_dome_visualizer`], this keeps long-lived per-visualizer state
/// and advances it using wall-clock deltas from [`VisualizerInput::now_ms`].
#[derive(Clone, Debug, Default)]
pub struct VisualizerRuntime {
    active: Option<LiveVisualizer>,
    snakes: Option<SnakesRuntime>,
    race: Option<RaceRuntime>,
    radial: Option<RadialRuntime>,
    splat: Option<SplatRuntime>,
    tv_static: Option<TvStaticRuntime>,
    volume: Option<VolumeRuntime>,
    flash: Option<FlashRuntime>,
    paintbrush: Option<PaintbrushRuntime>,
}

impl VisualizerRuntime {
    /// Render the dome commands for `visualizer`, advancing persistent state.
    #[must_use]
    pub fn render_dome(
        &mut self,
        visualizer: LiveVisualizer,
        input: VisualizerInput,
    ) -> Vec<DomeCommand> {
        let switched = self.active != Some(visualizer);
        // Only wipe when replacing a *previous* visualizer; the very first
        // activation has nothing on the dome to clear and must stay bit-for-bit
        // identical to the pure first-frame path used by golden tests.
        let wipe = switched && self.active.is_some();
        if switched {
            self.reset();
            self.active = Some(visualizer);
        }

        let mut commands = Vec::new();
        if wipe {
            commands.push(DomeCommand::Frame(vec![Rgb::BLACK; DOME_PIXELS]));
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
            other => commands.extend(render_dome_visualizer(other, input)),
        }

        commands
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
    }
}
