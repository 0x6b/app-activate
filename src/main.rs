mod config;
mod hotkey_manager;

use std::{fs::read_to_string, path::PathBuf, process::exit, time::Instant};

use anyhow::Result;
use clap::Parser;
use env_logger::Env;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use log::{debug, error};
use toml::from_str;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};
use xdg::BaseDirectories;

use crate::{
    config::Config,
    hotkey_manager::{HotKeyManager, State},
};

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct Args {
    /// Path to the configuration file. Defaults to `$XDG_CONFIG_HOME/app-activate/config.toml`.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Subcommand to run.
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Parser)]
pub enum Command {
    /// Start the application. Default if no subcommand is provided.
    Start,

    /// Register the application to start on login.
    Register,

    /// Unregister the application from starting on login.
    Unregister,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    debug!("{args:?}");
    let config = get_config(args.config)?;

    match args.command {
        None | Some(Command::Start) => start(config)?,
        Some(Command::Register) => {
            unimplemented!("Registering app to the launch service is not yet implemented")
        }
        Some(Command::Unregister) => {
            unimplemented!("Unregistering app from the launch service is not yet implemented")
        }
    }

    Ok(())
}

fn start(config: Config) -> Result<()> {
    let mut manager = HotKeyManager::from_config(config)?;

    EventLoopBuilder::new().build()?.run(move |event, event_loop| {
        if let Event::NewEvents(_) = event {
            if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                debug!("Received hotkey event: {event:?}");

                // Only process Pressed events
                if event.state == HotKeyState::Pressed {
                    manager.handle(event);

                    // Update control flow based on new state
                    let control_flow = match &manager.state {
                        State::AwaitingSecondKey { .. } => {
                            debug!("Waiting until timeout");
                            ControlFlow::WaitUntil(Instant::now() + manager.timeout)
                        }
                        State::Waiting => {
                            debug!("Setting back to Wait");
                            ControlFlow::Wait
                        }
                    };
                    event_loop.set_control_flow(control_flow);
                }
            }

            // Check for timeout if in LeaderPressed state
            if manager.is_timed_out() {
                debug!("Leader key timeout. Resetting state");
                manager.reset_state();
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    })?;

    Ok(())
}

fn get_config(config: Option<PathBuf>) -> Result<Config> {
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

    debug!("Reading config file at {path:?}");
    let config = match read_to_string(&path) {
        Ok(config) => config,
        Err(why) => {
            eprintln!("Failed to read config file at {path:?}: {why}");
            exit(1);
        }
    };

    Ok(from_str::<Config>(&config)?)
}
