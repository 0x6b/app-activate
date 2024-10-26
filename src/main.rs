mod config;
mod hotkey_manager;

use std::{fs::read_to_string, time::Instant};

use anyhow::Result;
use env_logger::Env;
use global_hotkey::GlobalHotKeyEvent;
use log::debug;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};

use crate::{
    config::Config,
    hotkey_manager::{HotKeyManager, State},
};

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let config: Config = toml::from_str(&read_to_string("config.toml")?)?;
    let mut manager = HotKeyManager::from_config(config)?;

    EventLoopBuilder::new().build()?.run(move |event, event_loop| {
        if event == Event::AboutToWait {
            // Check for hotkey events
            if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                debug!("Received hotkey event: {event:?}");
                manager.handle(event);

                // Set the appropriate control flow based on new state
                match &manager.state {
                    State::AwaitingSecondKey { .. } => {
                        debug!("Waiting until timeout");
                        event_loop.set_control_flow(ControlFlow::WaitUntil(
                            Instant::now() + manager.timeout,
                        ));
                    }
                    State::Waiting => {
                        debug!("Setting back to Wait");
                        event_loop.set_control_flow(ControlFlow::Wait);
                    }
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
