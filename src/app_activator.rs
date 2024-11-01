use std::{path::PathBuf, rc::Rc, thread::spawn, time::Instant};

use anyhow::{anyhow, bail, Result};
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use log::{debug, error};
use rusqlite::Connection;
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    platform::macos::ActivationPolicy,
    window::WindowId,
};

use crate::{
    config::Config,
    hotkey_manager::{
        HotKeyManager,
        State::{AwaitingSecondKey, Waiting},
    },
};

pub struct AppActivator {
    config: Config,
    conn: Rc<Option<Connection>>,
}

struct ConfigChangeEvent;

impl AppActivator {
    pub fn new(config: Config) -> Result<Self> {
        let conn = if let Some(db) = config.db.clone() {
            let conn = Connection::open(&db)?;
            conn.execute(
                r#"CREATE TABLE IF NOT EXISTS log (
                datetime INTEGER NOT NULL,
                application TEXT NOT NULL
            )"#,
                (),
            )?;
            Some(conn)
        } else {
            None
        };
        Ok(Self { config, conn: Rc::new(conn) })
    }

    pub fn start(&self) -> Result<()> {
        let mut event_loop = EventLoop::with_user_event();
        #[cfg(target_os = "macos")]
        {
            event_loop.with_activation_policy(ActivationPolicy::Accessory);
            event_loop.with_default_menu(false);
        }
        let event_loop: EventLoop<ConfigChangeEvent> = event_loop.build()?;

        let config_path = self.config.path.clone();
        let hot_key_manager = HotKeyManager::from_config(&self.config)?;

        let (config_tx, config_rx) = std::sync::mpsc::channel();
        let _watcher = self.config.watch(config_tx)?;

        let event_loop_proxy = event_loop.create_proxy();
        spawn(move || {
            while let Ok(()) = config_rx.recv() {
                let _ = event_loop_proxy.send_event(ConfigChangeEvent);
            }
        });
        event_loop
            .run_app(&mut State {
                config_path,
                hot_key_manager,
                conn: self.conn.clone(),
            })
            .map_err(|e| anyhow!("{e}"))
    }
}

struct State {
    config_path: PathBuf,
    hot_key_manager: HotKeyManager,
    conn: Rc<Option<Connection>>,
}

impl ApplicationHandler<ConfigChangeEvent> for State {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, _: StartCause) {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            debug!("Received hotkey event: {event:?}");

            // Only process Pressed events
            if event.state == HotKeyState::Pressed {
                self.hot_key_manager.handle(event, self.conn.clone());

                // Update control flow based on new state
                let control_flow = match &self.hot_key_manager.state {
                    AwaitingSecondKey { .. } => {
                        debug!("Waiting until timeout");
                        ControlFlow::WaitUntil(Instant::now() + self.hot_key_manager.timeout)
                    }
                    Waiting => {
                        debug!("Setting back to Wait");
                        ControlFlow::Wait
                    }
                };
                event_loop.set_control_flow(control_flow);
            }
        }

        // Check for timeout if in LeaderPressed state
        if self.hot_key_manager.is_timed_out() {
            debug!("Leader key timeout. Resetting state");
            self.hot_key_manager.reset_state();
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn resumed(&mut self, _: &ActiveEventLoop) {
        // do nothing
    }

    fn user_event(&mut self, _: &ActiveEventLoop, _: ConfigChangeEvent) {
        debug!("Config file changed. Reloading from {}", self.config_path.display());
        let config = Config::from(&self.config_path).unwrap();
        match self.hot_key_manager.update_config(&config) {
            Ok(..) => debug!("Config updated successfully: {config:?}"),
            Err(why) => error!("Failed to update config: {why}"),
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {
        // do nothing
    }
}
