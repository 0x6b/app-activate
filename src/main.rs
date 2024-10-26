use std::{
    collections::BTreeMap,
    fs::read_to_string,
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant},
};

use anyhow::Result;
use env_logger::Env;
use global_hotkey::{
    hotkey::{Code, HotKey},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};

#[derive(Debug)]
enum ShortcutState {
    Waiting,
    LeaderKeyPressed { time: Instant },
}

struct ShortcutManager {
    manager: GlobalHotKeyManager,
    leader_key: HotKey,
    applications: Vec<(HotKey, PathBuf)>,
    state: ShortcutState,
    timeout: Duration,
}

impl ShortcutManager {
    fn from_config(config: Config) -> Result<Self> {
        debug!("{config:?}");
        let manager = GlobalHotKeyManager::new()?;
        let leader_key = HotKey::new(None, Code::from_str(&config.leader_key)?);
        manager.register(leader_key)?;

        Ok(Self {
            manager,
            leader_key,
            state: ShortcutState::Waiting,
            timeout: Duration::from_millis(config.timeout_ms),
            applications: config
                .applications
                .iter()
                .map(|(key, path)| {
                    // add `Key` prefix if the specified key is in 'A' to 'Z'
                    let key = if key.len() == 1 && key.is_ascii() {
                        format!("Key{}", key.to_ascii_uppercase())
                    } else {
                        key.clone()
                    };
                    (HotKey::new(None, Code::from_str(&key).unwrap()), path.to_path_buf())
                })
                .collect::<Vec<(_, _)>>(),
        })
    }

    fn handle(&mut self, event: GlobalHotKeyEvent) {
        debug!("Handling GlobalHotKeyEvent: {event:?}");
        match &mut self.state {
            ShortcutState::Waiting if event.id == self.leader_key.id() => {
                trace!("{:?}", event);
                for (hotkey, _path) in &self.applications {
                    trace!("registering second shot hotkey: {hotkey:?}");
                    self.manager.register(*hotkey).unwrap();
                }
                self.state = ShortcutState::LeaderKeyPressed { time: Instant::now() };
            }
            ShortcutState::LeaderKeyPressed { .. } => {
                if let Some((_, path)) =
                    self.applications.iter().find(|(hotkey, _)| hotkey.id() == event.id)
                {
                    debug!("found hotkey for {:?}", path);
                    match open::that(path) {
                        Ok(()) => debug!("Successfully launched {path:?}"),
                        Err(err) => error!("Failed to launch {path:?}: {err}"),
                    }
                    self.reset_state()
                }
            }
            _ => {}
        }
        trace!("Done. State: {:?}", self.state);
    }

    fn reset_state(&mut self) {
        self.state = ShortcutState::Waiting;
        for (hotkey, _) in &self.applications {
            trace!("unregistering {hotkey:?}");
            self.manager.unregister(*hotkey).unwrap();
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    leader_key: String,
    applications: BTreeMap<String, PathBuf>,
    timeout_ms: u64,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let config: Config = toml::from_str(&read_to_string("config.toml")?)?;
    let mut manager = ShortcutManager::from_config(config)?;

    EventLoopBuilder::new().build()?.run(move |event, event_loop| {
        if event == Event::AboutToWait {
            // Check for hotkey events
            if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                debug!("Received hotkey event: {event:?}");
                manager.handle(event);

                // Set the appropriate control flow based on new state
                match &manager.state {
                    ShortcutState::LeaderKeyPressed { .. } => {
                        debug!("Waiting until timeout");
                        event_loop.set_control_flow(ControlFlow::WaitUntil(
                            Instant::now() + manager.timeout,
                        ));
                    }
                    ShortcutState::Waiting => {
                        debug!("Setting back to Wait");
                        event_loop.set_control_flow(ControlFlow::Wait);
                    }
                }
            }

            // Check for timeout if in LeaderPressed state
            if let ShortcutState::LeaderKeyPressed { time, .. } = &manager.state {
                if time.elapsed() > manager.timeout {
                    debug!("Leader key timeout. Resetting state");
                    manager.reset_state();
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            }
        }
    })?;

    Ok(())
}
