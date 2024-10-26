use std::{
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant},
};

use global_hotkey::{
    hotkey::{Code, HotKey},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use log::{debug, error, trace};

use crate::Config;

#[derive(Debug)]
pub enum State {
    Waiting,
    LeaderKeyPressed { time: Instant },
}

pub struct HotKeyManager {
    manager: GlobalHotKeyManager,
    leader_key: HotKey,
    applications: Vec<(HotKey, PathBuf)>,
    pub state: State,
    pub timeout: Duration,
}

impl HotKeyManager {
    pub fn from_config(config: Config) -> anyhow::Result<Self> {
        debug!("{config:?}");
        let manager = GlobalHotKeyManager::new()?;
        let leader_key = HotKey::new(None, Code::from_str(&config.leader_key)?);
        manager.register(leader_key)?;

        Ok(Self {
            manager,
            leader_key,
            state: State::Waiting,
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

    pub fn handle(&mut self, event: GlobalHotKeyEvent) {
        debug!("Handling GlobalHotKeyEvent: {event:?}");
        match &mut self.state {
            State::Waiting if event.id == self.leader_key.id() => {
                trace!("{:?}", event);
                for (hotkey, _path) in &self.applications {
                    trace!("registering second shot hotkey: {hotkey:?}");
                    self.manager.register(*hotkey).unwrap();
                }
                self.state = State::LeaderKeyPressed { time: Instant::now() };
            }
            State::LeaderKeyPressed { .. } => {
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

    pub fn reset_state(&mut self) {
        self.state = State::Waiting;
        for (hotkey, _) in &self.applications {
            trace!("unregistering {hotkey:?}");
            self.manager.unregister(*hotkey).unwrap();
        }
    }
}
