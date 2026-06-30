//! Output-side protocols and fixture topology.

pub mod commands;
pub mod opc;
pub mod simulator;
pub mod topology;

pub use commands::{BarCommand, DomeCommand, StageCommand};
pub use opc::{
    apply_bar_commands, apply_dome_commands, apply_stage_commands,
    dome_strut_index_for_control_box, dome_strut_length, encode_frame, OpcAddress, OpcClient,
    PersistentChannel,
};
pub use simulator::{DomeOutputSink, SimulatorColor};
