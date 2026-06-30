//! Allocation-light scheduler semantics mirroring Spectrum's `Operator`.

/// Visualizer metadata needed by the scheduler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualizerSpec {
    /// Stable visualizer name.
    pub name: &'static str,
    /// Current priority. Priority 0 is never selected. Priority -1 is always-run.
    pub priority: i32,
    /// Whether all required inputs are enabled.
    pub inputs_enabled: bool,
}

/// Active visualizer names for one output frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScheduledFrame {
    /// Highest selected non-negative priority.
    pub top_priority: i32,
    /// Visualizers selected for this frame.
    pub active: Vec<&'static str>,
}

/// Select visualizers using Spectrum's priority rules.
#[must_use]
pub fn schedule_visualizers(specs: &[VisualizerSpec]) -> ScheduledFrame {
    let mut top_priority = 1;
    let mut active = Vec::new();
    let mut always = Vec::new();

    for spec in specs {
        if !spec.inputs_enabled {
            continue;
        }
        if spec.priority == -1 {
            always.push(spec.name);
        } else if spec.priority > top_priority {
            top_priority = spec.priority;
            active.clear();
            active.push(spec.name);
        } else if spec.priority == top_priority {
            active.push(spec.name);
        }
    }

    active.extend(always);
    ScheduledFrame {
        top_priority,
        active,
    }
}

#[cfg(test)]
mod tests {
    use super::{schedule_visualizers, VisualizerSpec};

    fn v(name: &'static str, priority: i32) -> VisualizerSpec {
        VisualizerSpec {
            name,
            priority,
            inputs_enabled: true,
        }
    }

    #[test]
    fn priority_zero_is_never_selected() {
        let frame = schedule_visualizers(&[v("dead-midi-test", 0)]);
        assert!(frame.active.is_empty());
    }

    #[test]
    fn priority_two_ties_run_together_for_flash_overlay() {
        let frame = schedule_visualizers(&[v("volume", 2), v("flash", 2), v("tv-static", 1)]);
        assert_eq!(frame.active, vec!["volume", "flash"]);
    }

    #[test]
    fn diagnostics_override_normal_modes() {
        let frame = schedule_visualizers(&[v("volume", 2), v("flash-colors", 1000)]);
        assert_eq!(frame.active, vec!["flash-colors"]);
    }

    #[test]
    fn always_run_priority_is_supported() {
        let frame = schedule_visualizers(&[v("volume", 2), v("future-overlay", -1)]);
        assert_eq!(frame.active, vec!["volume", "future-overlay"]);
    }

    #[test]
    fn disabled_inputs_block_visualizer() {
        let frame = schedule_visualizers(&[
            VisualizerSpec {
                name: "volume",
                priority: 2,
                inputs_enabled: false,
            },
            v("tv-static", 1),
        ]);
        assert_eq!(frame.active, vec!["tv-static"]);
    }
}
