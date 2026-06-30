//! Fakeable MIDI command replay.

/// MIDI command category.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MidiCommandKind {
    /// Note on/off command.
    Note,
    /// Continuous controller command.
    ControlChange,
    /// Program change command.
    Program,
}

/// Normalized MIDI command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MidiCommand {
    /// Command kind.
    pub kind: MidiCommandKind,
    /// Note/controller/program index.
    pub index: u8,
    /// Normalized value in `[0.0, 1.0]`.
    pub value: f32,
}

/// Deterministic MIDI replay stream.
#[derive(Clone, Debug)]
pub struct MidiReplay {
    commands: Vec<MidiCommand>,
    cursor: usize,
}

impl MidiReplay {
    /// Create a MIDI replay stream.
    #[must_use]
    pub fn new(commands: Vec<MidiCommand>) -> Self {
        Self {
            commands,
            cursor: 0,
        }
    }

    /// Drain commands since the last tick.
    pub fn commands_since_last_tick(&mut self) -> Vec<MidiCommand> {
        let remaining = self.commands[self.cursor..].to_vec();
        self.cursor = self.commands.len();
        remaining
    }
}

#[cfg(test)]
mod tests {
    use super::{MidiCommand, MidiCommandKind, MidiReplay};

    #[test]
    fn drains_midi_commands_once() {
        let command = MidiCommand {
            kind: MidiCommandKind::Note,
            index: 64,
            value: 1.0,
        };
        let mut replay = MidiReplay::new(vec![command]);

        assert_eq!(replay.commands_since_last_tick(), vec![command]);
        assert!(replay.commands_since_last_tick().is_empty());
    }
}
