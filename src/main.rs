use std::{path::PathBuf, process::exit};

use anyhow::Result;
use app_activate::{AppActivator, Config, LaunchdManager};
use clap::Parser;
use env_logger::Env;
use log::{debug, error};
use xdg::BaseDirectories;

use crate::args::{Args, Command};

mod args;

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    debug!("{args:?}");
    let config_path = get_config_path(args.config)?;
    debug!("Reading config file at {config_path:?}");
    let config = Config::from(&config_path)?;

    use Command::*;
    match args.command {
        None | Some(Start) => AppActivator::from_config(config)?.start()?,
        Some(Register) => {
            if cfg!(target_os = "macos") {
                LaunchdManager::new("app-activate").register()?
            } else {
                error!("Service registration not supported on this platform");
            }
        }
        Some(Unregister) => {
            if cfg!(target_os = "macos") {
                LaunchdManager::new("app-activate").unregister()?
            } else {
                error!("Service registration not supported on this platform");
            }
        }
    }

    Ok(())
}

fn get_config_path(config: Option<PathBuf>) -> Result<PathBuf> {
    let path = config.unwrap_or_else(|| {
        let path = BaseDirectories::with_prefix("app-activate")
            .unwrap()
            .place_config_file("config.toml")
            .unwrap();
        debug!("Config file not provided. Using default at {path:?}");
        path
    });

    let path = if path.exists() {
        path.canonicalize()?
    } else {
        error!("Config file not found at {path:?}");
        exit(1);
    };

    Ok(path)
}
