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
    AwaitingSecondKey { pressed_at: Instant, registered_keys: Vec<HotKey> },
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
                    let key = if key.len() == 1 {
                        let c = key.chars().next().unwrap();
                        if c.is_ascii_alphabetic() {
                            format!("Key{}", c.to_ascii_uppercase())
                        } else if c.is_ascii_digit() {
                            format!("Digit{}", c)
                        } else {
                            key.clone()
                        }
                    } else {
                        key.clone()
                    };
                    (HotKey::new(None, Code::from_str(&key).unwrap()), path.to_path_buf())
                })
                .collect::<Vec<(_, _)>>(),
        })
    }

    pub fn is_timed_out(&self) -> bool {
        match self.state {
            State::AwaitingSecondKey { pressed_at, .. } => pressed_at.elapsed() > self.timeout,
            _ => false,
        }
    }

    pub fn handle(&mut self, event: GlobalHotKeyEvent) {
        debug!("Handling GlobalHotKeyEvent: {event:?}");
        match &mut self.state {
            State::Waiting if event.id == self.leader_key.id() => {
                trace!("{:?}", event);
                let registered_keys = self
                    .applications
                    .iter()
                    .map(|(hotkey, _)| {
                        trace!("registering second shot hotkey: {hotkey:?}");
                        self.manager.register(*hotkey).unwrap();
                        *hotkey
                    })
                    .collect();

                self.state =
                    State::AwaitingSecondKey { pressed_at: Instant::now(), registered_keys };
            }
            State::AwaitingSecondKey { .. } => {
                if let Some((_, path)) =
                    self.applications.iter().find(|(hotkey, _)| hotkey.id() == event.id)
                {
                    debug!("found hotkey for {:?}", path);
                    match open::that(path) {
                        Ok(()) => debug!("Successfully launched {path:?}"),
                        Err(err) => error!("Failed to launch {path:?}: {err}"),
                    }
                    self.reset_state();
                }
            }
            _ => {}
        }
        trace!("Done. State: {:?}", self.state);
    }

    pub fn reset_state(&mut self) {
        if let State::AwaitingSecondKey { registered_keys, .. } = &self.state {
            for hotkey in registered_keys {
                trace!("unregistering {hotkey:?}");
                self.manager.unregister(*hotkey).unwrap();
            }
        }
        self.state = State::Waiting;
    }
}
