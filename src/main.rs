use anyhow::Result;
use app_activate::{get_config, AppActivator, LaunchdManager};
use log::error;

use crate::args::{Args, Command};

mod args;

fn main() -> Result<()> {
    let args = Args::new();
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
