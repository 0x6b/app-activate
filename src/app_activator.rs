use std::{thread::spawn, time::Instant};

use anyhow::Result;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use log::{debug, error};
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    platform::macos::ActivationPolicy,
};

use crate::{
    config::{Config, CustomEvent},
    hotkey_manager::{HotKeyManager, State},
};

pub struct AppActivator {
    config: Config,
    event_loop: EventLoop<CustomEvent>,
}

impl AppActivator {
    pub fn from_config(config: Config) -> Result<Self> {
        let mut event_loop = EventLoopBuilder::with_user_event();
        #[cfg(target_os = "macos")]
        {
            event_loop.with_activation_policy(ActivationPolicy::Accessory);
            event_loop.with_default_menu(false);
        }
        let event_loop: EventLoop<CustomEvent> = event_loop.build()?;

        Ok(Self { config, event_loop })
    }

    pub fn start(self) -> Result<()> {
        let mut manager = HotKeyManager::from_config(&self.config)?;
        let (config_tx, config_rx) = std::sync::mpsc::channel();
        let _watcher = self.config.watch(config_tx)?;

        let event_loop_proxy = self.event_loop.create_proxy();
        spawn(move || {
            while let Ok(()) = config_rx.recv() {
                let _ = event_loop_proxy.send_event(CustomEvent::ConfigChanged);
            }
        });

        self.event_loop.run(move |event, event_loop| {
            match event {
                Event::UserEvent(CustomEvent::ConfigChanged) => {
                    debug!("Config file changed. Reloading from {}", self.config.path.display());
                    let config = Config::from(&self.config.path).unwrap();
                    match manager.update_config(&config) {
                        Ok(..) => debug!("Config updated successfully: {config:?}"),
                        Err(why) => error!("Failed to update config: {why}"),
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
}
