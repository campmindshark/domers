//! Domers operator server executable.

use std::{
    env,
    error::Error,
    fs,
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf},
    process::Command,
};

use domers_core::{DomersConfig, TempoSource};
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
    let config = load_config(&options.config_path)?;

    if matches!(options.command, CommandMode::Doctor) {
        run_doctor(&options, &config)?;
        return Ok(());
    }

    run_preflight(&options, &config)?;
    println!("Domers listening on http://{}", options.bind_addr);
    println!("Loaded config from {}", options.config_path.display());

    domers_server::serve(options.bind_addr, config).await?;
    Ok(())
}

fn run_doctor(options: &Options, config: &DomersConfig) -> Result<(), Box<dyn Error>> {
    run_preflight(options, config)?;
    println!("doctor ok: {}", options.config_path.display());
    Ok(())
}

fn run_preflight(options: &Options, config: &DomersConfig) -> Result<(), Box<dyn Error>> {
    assert_bind_available(options.bind_addr)?;
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
    };
    let status = Command::new(&launch.command).arg("--help").status();
    match status {
        Ok(_) => Ok(()),
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
    bind_addr: SocketAddr,
    config_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandMode {
    Run,
    Doctor,
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut command = CommandMode::Run;
        let mut bind_addr = DEFAULT_BIND_ADDR.parse()?;
        let mut config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        let mut args: Vec<_> = args.into_iter().collect();

        if let Some(first) = args.first() {
            match first.as_str() {
                "run" => {
                    let _ = args.remove(0);
                }
                "doctor" | "--check" => {
                    command = CommandMode::Doctor;
                    let _ = args.remove(0);
                }
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

        Ok(Self {
            command,
            bind_addr,
            config_path,
        })
    }
}

fn usage() -> &'static str {
    "usage: domers [run|doctor|--check] [--config domers.toml] [--bind 127.0.0.1:3000]"
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, path::PathBuf};

    use super::{CommandMode, Options, DEFAULT_CONFIG_PATH};

    #[test]
    fn parses_defaults() {
        let options = Options::parse(Vec::<String>::new()).expect("defaults parse");

        assert_eq!(
            options.bind_addr,
            "127.0.0.1:3000".parse::<SocketAddr>().expect("addr parses")
        );
        assert_eq!(options.command, CommandMode::Run);
        assert_eq!(options.config_path, PathBuf::from(DEFAULT_CONFIG_PATH));
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
            options.bind_addr,
            "127.0.0.1:4000".parse::<SocketAddr>().expect("addr parses")
        );
        assert_eq!(options.command, CommandMode::Run);
        assert_eq!(options.config_path, PathBuf::from("examples/domers.toml"));
    }

    #[test]
    fn parses_doctor_command() {
        let options = Options::parse([
            "doctor".to_string(),
            "--config".to_string(),
            "domers.toml".to_string(),
        ])
        .expect("doctor options parse");

        assert_eq!(options.command, CommandMode::Doctor);
        assert_eq!(options.config_path, PathBuf::from("domers.toml"));
    }
}
