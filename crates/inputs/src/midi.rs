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

/// Parse one live MIDI command payload.
///
/// Accepted shape: `note,64,1.0`, `cc,1,0.5`, or `program,3,1.0`.
#[must_use]
pub fn parse_midi_payload(payload: &[u8]) -> Option<MidiCommand> {
    let text = std::str::from_utf8(payload).ok()?.trim();
    let mut parts = text.split(',').map(str::trim);
    let kind = match parts.next()? {
        "note" => MidiCommandKind::Note,
        "cc" | "control_change" => MidiCommandKind::ControlChange,
        "program" => MidiCommandKind::Program,
        _ => return None,
    };
    let index = parts.next()?.parse::<u8>().ok()?;
    let value = parts.next()?.parse::<f32>().ok()?.clamp(0.0, 1.0);
    if parts.next().is_some() {
        return None;
    }
    Some(MidiCommand { kind, index, value })
}

#[cfg(test)]
mod tests {
    use super::{parse_midi_payload, MidiCommand, MidiCommandKind, MidiReplay};

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

    #[test]
    fn parses_live_midi_payloads() {
        assert_eq!(
            parse_midi_payload(b"note,64,1.0"),
            Some(MidiCommand {
                kind: MidiCommandKind::Note,
                index: 64,
                value: 1.0
            })
        );
        assert_eq!(
            parse_midi_payload(b"cc,1,0.5"),
            Some(MidiCommand {
                kind: MidiCommandKind::ControlChange,
                index: 1,
                value: 0.5
            })
        );
        assert_eq!(parse_midi_payload(b"bad,1,1"), None);
    }
}
