//! Madmom sidecar protocol helpers.

use std::{
    env, io,
    path::{Path, PathBuf},
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
    /// Resolve common bundled/sibling Madmom paths while preserving explicit commands.
    ///
    /// This lets a `command = "DBNBeatTracker"` config work when the Spectrum
    /// Madmom checkout sits next to `domers`, which is the local development
    /// layout, without requiring operators to install `DBNBeatTracker` globally.
    #[must_use]
    pub fn resolve(&self) -> Self {
        env::current_dir().map_or_else(|_| self.clone(), |base| self.resolve_from(&base))
    }

    /// Resolve from a known base directory.
    #[must_use]
    pub fn resolve_from(&self, base: &Path) -> Self {
        let command = Path::new(&self.command);
        if command.components().count() > 1 || command.exists() || self.command != "DBNBeatTracker"
        {
            return self.clone();
        }

        let parent = base.parent().unwrap_or(base);
        for candidate in [
            base.join("Madmom/bin/DBNBeatTracker"),
            parent.join("Madmom/bin/DBNBeatTracker"),
            base.join("spectrum/Madmom/bin/DBNBeatTracker"),
            parent.join("spectrum/Madmom/bin/DBNBeatTracker"),
        ] {
            if candidate.exists() {
                let mut resolved = self.clone();
                resolved.command = candidate.to_string_lossy().to_string();
                return resolved;
            }
        }

        self.clone()
    }

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

    /// Return the Spectrum-compatible working directory for Python launches.
    ///
    /// Spectrum starts `Madmom/env/Scripts/python.exe` with
    /// `DBNBeatTracker ...` and sets the process working directory to the
    /// Scripts folder. Without this, Python-style launches only work when the
    /// operator happens to start the server from that directory.
    #[must_use]
    pub fn working_directory(&self) -> Option<PathBuf> {
        self.tracker.as_ref()?;
        let command = Path::new(&self.command);
        let file_name = command.file_name()?.to_string_lossy().to_ascii_lowercase();
        if file_name != "python" && file_name != "python.exe" && file_name != "python3" {
            return None;
        }
        command
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
    }

    /// Return a PYTHONPATH entry for source-tree Madmom launches.
    #[must_use]
    pub fn python_path(&self) -> Option<PathBuf> {
        let command = Path::new(&self.command);
        let file_name = command.file_name()?.to_string_lossy();
        if file_name != "DBNBeatTracker" && file_name != "TorchBeatTracker" {
            return None;
        }
        let bin_dir = command.parent()?;
        (bin_dir.file_name()?.to_string_lossy() == "bin")
            .then(|| bin_dir.parent().map(Path::to_path_buf))
            .flatten()
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
        if let Some(working_directory) = launch.working_directory() {
            command.current_dir(working_directory);
        }
        if let Some(python_path) = launch.python_path() {
            command.env("PYTHONPATH", python_path);
        }
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
    use std::fs;

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
    fn python_launch_uses_script_directory_as_working_directory() {
        let config = MadmomLaunchConfig {
            command: "/opt/spectrum/Madmom/env/Scripts/python.exe".to_string(),
            tracker: Some("DBNBeatTracker".to_string()),
            audio_input_index: Some(5),
        };

        assert_eq!(
            config.working_directory().as_deref(),
            Some(std::path::Path::new("/opt/spectrum/Madmom/env/Scripts"))
        );
    }

    #[test]
    fn pathless_python_launch_keeps_current_working_directory() {
        let config = MadmomLaunchConfig {
            command: "python3".to_string(),
            tracker: Some("fake_tracker.py".to_string()),
            audio_input_index: Some(5),
        };

        assert_eq!(config.working_directory(), None);
    }

    #[test]
    fn dbn_beat_tracker_resolves_from_sibling_spectrum_checkout() {
        let root =
            std::env::temp_dir().join(format!("domers-madmom-resolve-{}", std::process::id()));
        let tracker = root.join("spectrum/Madmom/bin/DBNBeatTracker");
        fs::create_dir_all(tracker.parent().expect("tracker has parent")).expect("mkdir");
        fs::write(&tracker, "#!/usr/bin/env python\n").expect("write tracker");

        let config = MadmomLaunchConfig {
            command: "DBNBeatTracker".to_string(),
            tracker: None,
            audio_input_index: None,
        };
        let domers_dir = root.join("domers");
        let resolved = config.resolve_from(&domers_dir);

        assert_eq!(resolved.command, tracker.to_string_lossy().as_ref());
        let madmom_root = root.join("spectrum/Madmom");
        assert_eq!(
            resolved.python_path().as_deref(),
            Some(madmom_root.as_path())
        );
        let _ = fs::remove_dir_all(root);
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
