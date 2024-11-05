use app_activate::{get_config, AppActivator, LaunchdManager};

use crate::args::{
    Args,
    Command::{Register, Unregister},
};

mod args;

fn main() -> anyhow::Result<()> {
    let Args { config, command } = Args::new();

    match command {
        Some(Register) => LaunchdManager::new("app-activate")?.register()?,
        Some(Unregister) => LaunchdManager::new("app-activate")?.unregister()?,
        _ => AppActivator::new(get_config(config)?)?.start()?,
    }

    Ok(())
}
