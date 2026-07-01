use domers_outputs::DomeCommand;

use crate::{
    dome::{SnakesState, SNAKES_MAX_CATCHUP_STEPS, SNAKES_STEP_MS},
    input::VisualizerInput,
};

/// Wall-clock throttled Snakes runtime wrapping the stateful step machine.
#[derive(Clone, Debug)]
pub(crate) struct SnakesRuntime {
    state: SnakesState,
    last_step_ms: Option<u64>,
}

impl SnakesRuntime {
    pub(crate) fn new() -> Self {
        Self {
            state: SnakesState::new(),
            last_step_ms: None,
        }
    }

    pub(crate) fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let now = input.now_ms;
        if self.last_step_ms.is_none() {
            // Mirror C# `lastUpdate = DeterministicClock.Now` at construction: the
            // first frame at the same timestamp does not step.
            self.last_step_ms = Some(now);
            return;
        }
        let Some(last) = self.last_step_ms else {
            return;
        };
        if now.saturating_sub(last) < SNAKES_STEP_MS {
            return;
        }
        let steps = u32::try_from(
            (now.saturating_sub(last) / SNAKES_STEP_MS).min(u64::from(SNAKES_MAX_CATCHUP_STEPS)),
        )
        .unwrap_or(SNAKES_MAX_CATCHUP_STEPS);
        for _ in 0..steps {
            self.state.step(input, out);
        }
        self.last_step_ms = Some(now);
    }
}
