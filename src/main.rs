use std::{
    collections::BTreeMap,
    fs::read_to_string,
    path::{Path, PathBuf},
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
use winit::event_loop::{ControlFlow, EventLoopBuilder};

#[derive(Debug)]
enum ShortcutState {
    Waiting,
    LeaderPressed { time: Instant, id: u32 },
}

struct ShortcutManager {
    manager: GlobalHotKeyManager,
    leader_key: HotKey,
    applications: Vec<(HotKey, PathBuf)>,
    state: ShortcutState,
    timeout: Duration,
}

impl ShortcutManager {
    fn handle(&mut self, event: GlobalHotKeyEvent) {
        match &mut self.state {
            ShortcutState::Waiting => {
                if event.id == self.leader_key.id() {
                    trace!("{:?}", event);
                    for (hotkey, _path) in &self.applications {
                        trace!("registering {hotkey:?}");
                        self.manager.register(*hotkey).unwrap();
                    }
                    self.state =
                        ShortcutState::LeaderPressed { time: Instant::now(), id: event.id };
                }
            }
            ShortcutState::LeaderPressed { time, id: _id } => {
                if time.elapsed() > self.timeout {
                    for (hotkey, _) in &self.applications {
                        trace!("deregistering {hotkey:?}");
                        self.manager.unregister(*hotkey).unwrap();
                    }
                    self.state = ShortcutState::Waiting;
                } else if let Some((_, path)) =
                    self.applications.iter().find(|(hotkey, _)| hotkey.id() == event.id)
                {
                    debug!("found hotkey for {:?}", path);
                    match open::that(path) {
                        Ok(()) => debug!("Successfully launched {path:?}"),
                        Err(err) => error!("Failed to launch {path:?}: {err}"),
                    }
                    self.state = ShortcutState::Waiting;
                    for (hotkey, _) in &self.applications {
                        self.manager.unregister(*hotkey).unwrap();
                    }
                }
            }
        }
    }

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
                    // check if the key is in 'A'..'Z'
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

    EventLoopBuilder::new().build()?.run(move |_event, event_loop| {
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            manager.handle(event);
        }
    })?;

    Ok(())
}
