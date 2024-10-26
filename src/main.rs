mod config;
mod hotkey_manager;

use std::{fs::read_to_string, time::Instant};

use anyhow::Result;
use env_logger::Env;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
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
