mod app_activator;
mod config;
mod hotkey_manager;
#[cfg(target_os = "macos")]
mod launchd_manager;
mod usage_reporter;

use std::{path::PathBuf, process::exit};

pub use app_activator::AppActivator;
pub use config::Config;
pub use hotkey_manager::HotKeyManager;
use hotkey_manager::State;
#[cfg(target_os = "macos")]
pub use launchd_manager::LaunchdManager;
use log::{debug, error};
pub use usage_reporter::UsageReporter;
use xdg::BaseDirectories;

pub fn get_config(config: Option<PathBuf>) -> anyhow::Result<Config> {
    let path = config.unwrap_or_else(|| {
        let base_dirs = BaseDirectories::with_prefix("app-activate");
        let path = base_dirs.place_config_file("config.toml")
            .expect("Failed to place config file");
        debug!("Config file not provided. Using default at {path:?}");
        path
    });

    let path = if path.exists() {
        path.canonicalize()?
    } else {
        error!("Config file not found at {path:?}");
        exit(1);
    };

    debug!("Reading config file at {path:?}");
    let config = Config::from(path)?;
    Ok(config)
}
