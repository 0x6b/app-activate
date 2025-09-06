use std::{
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use global_hotkey::{
    hotkey::{Code, HotKey},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use log::{debug, error, trace};
use rusqlite::Connection;

use crate::Config;

#[derive(Debug)]
pub enum State {
    Waiting,
    AwaitingSecondKey { pressed_at: Instant, registered_keys: Vec<HotKey>, is_secondary: bool },
}

pub struct HotKeyManager {
    manager: GlobalHotKeyManager,
    leader_key: HotKey,
    applications: Vec<(HotKey, PathBuf)>,
    secondary_applications: Vec<(HotKey, PathBuf)>,
    pub state: State,
    pub timeout: Duration,
}

impl HotKeyManager {
    pub fn from_config(config: &Config) -> Result<Self> {
        debug!("{config:?}");
        let manager = GlobalHotKeyManager::new()?;
        let leader_key = HotKey::new(None, Code::from_str(&config.leader_key)?);
        manager.register(leader_key)?;

        Ok(Self {
            manager,
            leader_key,
            state: State::Waiting,
            timeout: Duration::from_millis(config.timeout_ms),
            applications: config.applications(),
            secondary_applications: config.secondary_applications(),
        })
    }

    pub fn update_config(&mut self, config: &Config) -> Result<()> {
        self.manager.unregister_all(&[self.leader_key])?;
        let leader_key = HotKey::new(None, Code::from_str(&config.leader_key)?);
        self.manager.register(leader_key)?;
        self.leader_key = leader_key;
        self.state = State::Waiting;
        self.timeout = Duration::from_millis(config.timeout_ms);
        self.applications.clear();
        self.applications = config.applications();
        self.secondary_applications.clear();
        self.secondary_applications = config.secondary_applications();

        Ok(())
    }

    pub fn is_timed_out(&self) -> bool {
        match self.state {
            State::AwaitingSecondKey { pressed_at, .. } => pressed_at.elapsed() > self.timeout,
            _ => false,
        }
    }

    pub fn handle(&mut self, event: GlobalHotKeyEvent, conn: Rc<Option<Connection>>) {
        debug!("Handling GlobalHotKeyEvent: {event:?}");
        match &mut self.state {
            State::Waiting if event.id == self.leader_key.id() => {
                trace!("{:?}", event);
                let registered_keys = self
                    .applications
                    .iter()
                    .map(|(hotkey, _)| {
                        trace!("Registering second shot hotkey: {hotkey:?}");
                        self.manager.register(*hotkey).unwrap();
                        *hotkey
                    })
                    .collect();

                self.state =
                    State::AwaitingSecondKey { pressed_at: Instant::now(), registered_keys, is_secondary: false };
            }
            State::AwaitingSecondKey { is_secondary, .. } if event.id == self.leader_key.id() => {
                // Leader key pressed while waiting for second key - swap app sets
                let current_is_secondary = *is_secondary;
                self.swap_app_sets(current_is_secondary);
            }
            State::AwaitingSecondKey { is_secondary, .. } => {
                // Look for the hotkey in the appropriate app set
                let app_set = if *is_secondary {
                    &self.secondary_applications
                } else {
                    &self.applications
                };
                
                if let Some((_, path)) = app_set.iter().find(|(hotkey, _)| hotkey.id() == event.id) {
                    debug!("Found hotkey for {path:?}");
                    match open::that_detached(path) {
                        Ok(()) => {
                            debug!("Successfully launched {path:?}");
                            if let Some(conn) = conn.as_ref()
                                && conn
                                    .execute(
                                        "INSERT INTO log (datetime, application) VALUES (?1, ?2)",
                                        (
                                            SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap() // should always success
                                                .as_secs(),
                                            path.to_string_lossy(),
                                        ),
                                    )
                                    .is_err()
                                {
                                    error!("Failed to insert a log to SQLite database")
                                }
                        }
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
                trace!("Unregistering {hotkey:?}");
                self.manager.unregister(*hotkey).unwrap();
            }
        }
        self.state = State::Waiting;
    }

    fn swap_app_sets(&mut self, current_is_secondary: bool) {
        // Unregister current app keys (but keep the leader key registered)
        if let State::AwaitingSecondKey { registered_keys, .. } = &self.state {
            for hotkey in registered_keys {
                trace!("Unregistering {hotkey:?} for app set swap");
                self.manager.unregister(*hotkey).unwrap();
            }
        }

        // Register the opposite app set
        let new_app_set = if current_is_secondary {
            &self.applications
        } else {
            &self.secondary_applications
        };

        let registered_keys = new_app_set
            .iter()
            .map(|(hotkey, _)| {
                trace!("Registering {hotkey:?} for app set swap");
                self.manager.register(*hotkey).unwrap();
                *hotkey
            })
            .collect();

        // Update state with the new app set and reset timeout
        self.state = State::AwaitingSecondKey {
            pressed_at: Instant::now(),
            registered_keys,
            is_secondary: !current_is_secondary,
        };

        debug!("Swapped to {} app set", if !current_is_secondary { "secondary" } else { "primary" });
    }
}
