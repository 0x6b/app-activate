use anyhow::Result;
use app_activate::{get_config, AppActivator, LaunchdManager};
use clap::Parser;
use env_logger::Env;
use log::{debug, error};

use crate::args::{Args, Command};

mod args;

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    debug!("{args:?}");

    let config = get_config(args.config)?;

    use Command::*;
    match args.command {
        None | Some(Start) => AppActivator::new(config)?.start()?,
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
