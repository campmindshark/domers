//! Madmom sidecar protocol helpers.

use std::{
    io,
    process::{Child, ChildStdout, Command, Stdio},
};

/// Config needed to launch the Madmom sidecar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MadmomLaunchConfig {
    /// Sidecar executable or command name.
    pub command: String,
    /// Optional tracker/script argument, e.g. `DBNBeatTracker` when command is Python.
    pub tracker: Option<String>,
    /// Audio input index passed to `DBNBeatTracker`.
    pub audio_input_index: Option<u32>,
}

impl MadmomLaunchConfig {
    /// Return the Spectrum-compatible command arguments.
    #[must_use]
    pub fn args(&self) -> Vec<String> {
        let audio_input_index = self.audio_input_index.unwrap_or(0);
        let mut args = Vec::new();
        if let Some(tracker) = &self.tracker {
            args.push(tracker.clone());
        }
        args.extend([
            "--host_api".to_string(),
            format!("--audio_input={audio_input_index}"),
            "online".to_string(),
        ]);
        args
    }
}

/// Managed Madmom child process.
#[derive(Debug, Default)]
pub struct MadmomSidecar {
    child: Option<Child>,
    launch: Option<MadmomLaunchConfig>,
}

impl MadmomSidecar {
    /// Whether a child process is currently held.
    #[must_use]
    pub const fn active(&self) -> bool {
        self.child.is_some()
    }

    /// Last launch configuration used by this sidecar.
    #[must_use]
    pub const fn launch_config(&self) -> Option<&MadmomLaunchConfig> {
        self.launch.as_ref()
    }

    /// Restart the child process when Madmom should be active.
    ///
    /// # Errors
    ///
    /// Returns an error if the child process cannot be spawned.
    pub fn update_enabled(
        &mut self,
        active: bool,
        beat_input_is_madmom: bool,
        launch: &MadmomLaunchConfig,
    ) -> io::Result<()> {
        self.stop();
        self.launch = Some(launch.clone());

        if !active || !beat_input_is_madmom {
            return Ok(());
        }

        let mut command = Command::new(&launch.command);
        command
            .args(launch.args())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null());
        self.child = Some(command.spawn()?);
        Ok(())
    }

    /// Take the child stdout pipe for line-based beat ingestion.
    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.as_mut()?.stdout.take()
    }

    /// Stop and drop the child process.
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for MadmomSidecar {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Parse a `BEAT:{seconds}` stdout line into milliseconds.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Madmom emits fractional seconds; Spectrum truncates to integer milliseconds"
)]
pub fn parse_beat_line(line: &str) -> Option<u64> {
    let value = line.strip_prefix("BEAT:")?;
    let seconds = value.trim().parse::<f64>().ok()?;
    if seconds.is_sign_negative() || !seconds.is_finite() {
        return None;
    }
    Some((seconds * 1_000.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::{parse_beat_line, MadmomLaunchConfig, MadmomSidecar};

    #[test]
    fn parses_valid_beat_lines() {
        assert_eq!(parse_beat_line("BEAT:12.345"), Some(12_345));
    }

    #[test]
    fn drops_malformed_beat_lines() {
        assert_eq!(parse_beat_line("noise"), None);
        assert_eq!(parse_beat_line("BEAT:not-a-number"), None);
        assert_eq!(parse_beat_line("BEAT:-1"), None);
    }

    #[test]
    fn builds_spectrum_compatible_launch_args() {
        let config = MadmomLaunchConfig {
            command: "DBNBeatTracker".to_string(),
            tracker: None,
            audio_input_index: Some(5),
        };

        assert_eq!(
            config.args(),
            ["--host_api", "--audio_input=5", "online"].map(String::from)
        );
    }

    #[test]
    fn builds_spectrum_python_launch_args() {
        let config = MadmomLaunchConfig {
            command: "python.exe".to_string(),
            tracker: Some("DBNBeatTracker".to_string()),
            audio_input_index: Some(5),
        };

        assert_eq!(
            config.args(),
            ["DBNBeatTracker", "--host_api", "--audio_input=5", "online"].map(String::from)
        );
    }

    #[test]
    fn disabled_sidecar_records_launch_without_spawning() {
        let mut sidecar = MadmomSidecar::default();
        let config = MadmomLaunchConfig {
            command: "DBNBeatTracker".to_string(),
            tracker: None,
            audio_input_index: Some(2),
        };

        sidecar
            .update_enabled(false, true, &config)
            .expect("disabled update does not spawn");

        assert!(!sidecar.active());
        assert_eq!(sidecar.launch_config(), Some(&config));
    }
}
