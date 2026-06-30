//! Domers operator server executable.

use std::{
    env,
    error::Error,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use domers_core::{DomersConfig, EngineConfig};

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
    let engine_config = EngineConfig::from(&config);

    println!("Domers listening on http://{}", options.bind_addr);
    println!("Loaded config from {}", options.config_path.display());

    domers_server::serve(options.bind_addr, engine_config).await?;
    Ok(())
}

fn load_config(path: &Path) -> Result<DomersConfig, Box<dyn Error>> {
    let toml = fs::read_to_string(path)?;
    DomersConfig::from_toml_str(&toml).map_err(Into::into)
}

#[derive(Debug, Eq, PartialEq)]
struct Options {
    bind_addr: SocketAddr,
    config_path: PathBuf,
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut bind_addr = DEFAULT_BIND_ADDR.parse()?;
        let mut config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
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
            bind_addr,
            config_path,
        })
    }
}

fn usage() -> &'static str {
    "usage: domers [--config domers.toml] [--bind 127.0.0.1:3000]"
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, path::PathBuf};

    use super::{Options, DEFAULT_CONFIG_PATH};

    #[test]
    fn parses_defaults() {
        let options = Options::parse(Vec::<String>::new()).expect("defaults parse");

        assert_eq!(
            options.bind_addr,
            "127.0.0.1:3000".parse::<SocketAddr>().expect("addr parses")
        );
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
        assert_eq!(options.config_path, PathBuf::from("examples/domers.toml"));
    }
}
