mod args;
mod config;
mod hotkey_manager;
#[cfg(target_os = "macos")]
mod launchd_manager;

use std::{
    fmt::Debug,
    fs::read_to_string,
    path::{Path, PathBuf},
    process::exit,
    thread::spawn,
    time::Instant,
};

use anyhow::Result;
use clap::Parser;
use env_logger::Env;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use log::{debug, error};
use toml::from_str;
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    platform::macos::ActivationPolicy,
};
use xdg::BaseDirectories;

use crate::{
    args::{Args, Command},
    config::{Config, CustomEvent},
    hotkey_manager::{HotKeyManager, State},
    launchd_manager::LaunchdManager,
};

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    debug!("{args:?}");
    let config_path = get_config_path(args.config)?;
    debug!("Reading config file at {config_path:?}");
    let config = load_config(&config_path)?;

    let mut event_loop = EventLoopBuilder::with_user_event();
    #[cfg(target_os = "macos")]
    {
        event_loop.with_activation_policy(ActivationPolicy::Accessory);
        event_loop.with_default_menu(false);
    }
    let event_loop: EventLoop<CustomEvent> = event_loop.build()?;

    use Command::*;
    match args.command {
        None | Some(Start) => start(event_loop, config, config_path)?,
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

fn start(
    event_loop: EventLoop<CustomEvent>,
    initial_config: Config,
    config_path: PathBuf,
) -> Result<()> {
    let mut manager = HotKeyManager::from_config(initial_config)?;
    let (config_tx, config_rx) = std::sync::mpsc::channel();
    let _watcher = config::watch_config(&config_path, config_tx)?;

    let event_loop_proxy = event_loop.create_proxy();
    spawn(move || {
        while let Ok(()) = config_rx.recv() {
            let _ = event_loop_proxy.send_event(CustomEvent::ConfigChanged);
        }
    });

    event_loop.run(move |event, event_loop| {
        match event {
            Event::UserEvent(CustomEvent::ConfigChanged) => {
                debug!("Config file changed. Reloading from {config_path:?}");
                let config = load_config(&config_path).unwrap();
                if let Err(why) = manager.update_config(config) {
                    error!("Failed to update config: {why}");
                }
            }
            Event::NewEvents(_) => {
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
            _ => {}
        }
    })?;

    Ok(())
}

fn load_config<P>(path: P) -> Result<Config>
where
    P: AsRef<Path> + Debug,
{
    let config = match read_to_string(&path) {
        Ok(config) => config,
        Err(why) => {
            eprintln!("Failed to read config file at {path:?}: {why}");
            exit(1);
        }
    };

    let config = match from_str(&config) {
        Ok(config) => config,
        Err(why) => {
            eprintln!("Failed to parse config file: {why}");
            exit(1);
        }
    };
    Ok(config)
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
