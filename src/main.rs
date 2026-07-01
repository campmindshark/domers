//! Domers operator server executable.

use std::{
    env,
    error::Error,
    fs,
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf},
    process::Command,
};

use domers_core::{import_spectrum_xml, DomersConfig, TempoSource};
use domers_inputs::MadmomLaunchConfig;
use domers_outputs::OpcAddress;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";
const DEFAULT_CONFIG_PATH: &str = "examples/domers.toml";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let options = Options::parse(env::args().skip(1))?;
    match options.command {
        CommandMode::Run {
            bind_addr,
            config_path,
        } => {
            let config = load_config(&config_path)?;
            run_preflight(bind_addr, &config)?;
            println!("Domers listening on http://{bind_addr}");
            println!("Loaded config from {}", config_path.display());
            domers_server::serve(bind_addr, config).await?;
        }
        CommandMode::Doctor {
            bind_addr,
            config_path,
        } => {
            let config = load_config(&config_path)?;
            run_doctor(bind_addr, &config_path, &config)?;
        }
        CommandMode::ImportSpectrumXml { input, output } => {
            import_spectrum_xml_command(&input, &output)?;
        }
    }
    Ok(())
}

fn run_doctor(
    bind_addr: SocketAddr,
    config_path: &Path,
    config: &DomersConfig,
) -> Result<(), Box<dyn Error>> {
    run_preflight(bind_addr, config)?;
    println!("doctor ok: {}", config_path.display());
    Ok(())
}

fn run_preflight(bind_addr: SocketAddr, config: &DomersConfig) -> Result<(), Box<dyn Error>> {
    assert_bind_available(bind_addr)?;
    if config.dome.enabled {
        OpcAddress::parse(&config.dome.opc_address)
            .map_err(|error| format!("invalid dome OPC address: {error}"))?;
    }
    if config.stage.enabled {
        OpcAddress::parse(&config.stage.opc_address)
            .map_err(|error| format!("invalid stage OPC address: {error}"))?;
    }
    if matches!(config.tempo.source, TempoSource::Madmom) {
        validate_madmom_command(config)?;
    }
    Ok(())
}

fn import_spectrum_xml_command(input: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    let xml = fs::read_to_string(input)?;
    let imported = import_spectrum_xml(&xml);
    let toml = imported.config.to_toml_string()?;
    fs::write(output, toml)?;

    for warning in imported.report.warnings {
        eprintln!("warning: {:?}: {}", warning.kind, warning.field);
    }

    Ok(())
}

fn assert_bind_available(bind_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    TcpListener::bind(bind_addr)
        .map(drop)
        .map_err(|error| format!("bind address {bind_addr} is unavailable: {error}").into())
}

fn validate_madmom_command(config: &DomersConfig) -> Result<(), Box<dyn Error>> {
    let launch = MadmomLaunchConfig {
        command: config.madmom.command.clone(),
        tracker: config.madmom.tracker.clone(),
        audio_input_index: config.madmom.audio_input_index,
    }
    .resolve();
    let mut command = Command::new(&launch.command);
    if let Some(working_directory) = launch.working_directory() {
        command.current_dir(working_directory);
    }
    if let Some(python_path) = launch.python_path() {
        command.env("PYTHONPATH", python_path);
    }
    if let Some(tracker) = &launch.tracker {
        command.arg(tracker);
    }
    let output = command.arg("--help").output();
    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!(
                "Madmom command '{}' exited with status {}.\nstdout: {}\nstderr: {}",
                launch.command, output.status, stdout, stderr
            )
            .into())
        }
        Err(error) => Err(format!(
            "Madmom command '{}' is not runnable with args {:?}: {error}",
            launch.command,
            launch.args()
        )
        .into()),
    }
}

fn load_config(path: &Path) -> Result<DomersConfig, Box<dyn Error>> {
    let toml = fs::read_to_string(path)?;
    DomersConfig::from_toml_str(&toml).map_err(Into::into)
}

#[derive(Debug, Eq, PartialEq)]
struct Options {
    command: CommandMode,
}

#[derive(Debug, Eq, PartialEq)]
enum CommandMode {
    Run {
        bind_addr: SocketAddr,
        config_path: PathBuf,
    },
    Doctor {
        bind_addr: SocketAddr,
        config_path: PathBuf,
    },
    ImportSpectrumXml {
        input: PathBuf,
        output: PathBuf,
    },
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut mode = "run";
        let mut bind_addr = DEFAULT_BIND_ADDR.parse()?;
        let mut config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        let mut args: Vec<_> = args.into_iter().collect();

        if let Some(first) = args.first() {
            match first.as_str() {
                "run" => {
                    let _ = args.remove(0);
                }
                "doctor" | "--check" => {
                    mode = "doctor";
                    let _ = args.remove(0);
                }
                "import-spectrum-xml" => return parse_import_spectrum_xml(args),
                "--bind" | "--config" | "--help" | "-h" => {}
                _ => {
                    return Err(format!("unknown command or argument: {first}\n{}", usage()).into())
                }
            }
        }

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bind" => {
                    let value = args.next().ok_or("missing value for --bind")?;
                    bind_addr = value.parse()?;
                }
                "--config" => {
                    config_path = args
                        .next()
                        .map(PathBuf::from)
                        .ok_or("missing value for --config")?;
                }
                "--help" | "-h" => return Err(usage().into()),
                _ => return Err(format!("unknown argument: {arg}\n{}", usage()).into()),
            }
        }

        let command = match mode {
            "doctor" => CommandMode::Doctor {
                bind_addr,
                config_path,
            },
            _ => CommandMode::Run {
                bind_addr,
                config_path,
            },
        };

        Ok(Self { command })
    }
}

fn parse_import_spectrum_xml(mut args: Vec<String>) -> Result<Options, Box<dyn Error>> {
    let _ = args.remove(0);
    let mut args = args.into_iter();
    let input = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing Spectrum XML input path")?;
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing Domers TOML output path")?;
    if args.next().is_some() {
        return Err("unexpected extra arguments".into());
    }
    Ok(Options {
        command: CommandMode::ImportSpectrumXml { input, output },
    })
}

fn usage() -> &'static str {
    "usage: domers [run|doctor|--check] [--config domers.toml] [--bind 127.0.0.1:3000]\n       domers import-spectrum-xml <spectrum.xml> <domers.toml>"
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, path::PathBuf};

    use super::{CommandMode, Options, DEFAULT_CONFIG_PATH};

    #[test]
    fn parses_defaults() {
        let options = Options::parse(Vec::<String>::new()).expect("defaults parse");

        assert_eq!(
            options.command,
            CommandMode::Run {
                bind_addr: "127.0.0.1:3000".parse::<SocketAddr>().expect("addr parses"),
                config_path: PathBuf::from(DEFAULT_CONFIG_PATH)
            }
        );
    }

    #[test]
    fn parses_config_and_bind_flags() {
        let options = Options::parse([
            "--config".to_string(),
            "examples/domers.toml".to_string(),
            "--bind".to_string(),
            "127.0.0.1:4000".to_string(),
        ])
        .expect("explicit options parse");

        assert_eq!(
            options.command,
            CommandMode::Run {
                bind_addr: "127.0.0.1:4000".parse::<SocketAddr>().expect("addr parses"),
                config_path: PathBuf::from("examples/domers.toml")
            }
        );
    }

    #[test]
    fn parses_doctor_command() {
        let options = Options::parse([
            "doctor".to_string(),
            "--config".to_string(),
            "domers.toml".to_string(),
        ])
        .expect("doctor options parse");

        assert_eq!(
            options.command,
            CommandMode::Doctor {
                bind_addr: "127.0.0.1:3000".parse::<SocketAddr>().expect("addr parses"),
                config_path: PathBuf::from("domers.toml")
            }
        );
    }

    #[test]
    fn parses_import_spectrum_xml_command() {
        let options = Options::parse([
            "import-spectrum-xml".to_string(),
            "spectrum.xml".to_string(),
            "domers.toml".to_string(),
        ])
        .expect("import options parse");

        assert_eq!(
            options.command,
            CommandMode::ImportSpectrumXml {
                input: PathBuf::from("spectrum.xml"),
                output: PathBuf::from("domers.toml")
            }
        );
    }
}
