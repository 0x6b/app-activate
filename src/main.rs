use app_activate::{get_config, AppActivator, LaunchdManager};
use log::error;

use crate::args::{
    Args,
    Command::{Register, Start, Unregister},
};

mod args;

fn main() -> anyhow::Result<()> {
    let Args { config, command } = Args::new();

    match command {
        None | Some(Start) => AppActivator::new(get_config(config)?)?.start()?,
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
