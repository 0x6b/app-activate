use anyhow::Result;
use app_activate::{AppActivator, LaunchdManager, get_config};

use crate::args::{Args, Command};

mod args;

fn main() -> Result<()> {
    let Args { config, command } = Args::new();

    match command.unwrap_or(Command::Start) {
        Command::Register => LaunchdManager::new("app-activate")?.register()?,
        Command::Unregister => LaunchdManager::new("app-activate")?.unregister()?,
        Command::Start => AppActivator::new(get_config(config)?)?.start()?,
    }

    Ok(())
}
