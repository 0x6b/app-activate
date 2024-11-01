use std::{
    collections::BTreeMap,
    fmt::Debug,
    fs::read_to_string,
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use anyhow::Result;
use global_hotkey::hotkey::{Code, HotKey};
use notify::{recommended_watcher, Event, Watcher};
use serde::Deserialize;
use toml::from_str;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub leader_key: String,
    pub applications: BTreeMap<String, PathBuf>,
    pub timeout_ms: u64,
    pub db: Option<PathBuf>,
    #[serde(skip)]
    pub(crate) path: PathBuf, // For internal use. Not deserialized from config file
}

impl Config {
    pub fn from<P>(path: P) -> Result<Self>
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

        let mut config = match from_str::<Config>(&config) {
            Ok(config) => config,
            Err(why) => {
                eprintln!("Failed to parse config file: {why}");
                exit(1);
            }
        };

        config.path = path.as_ref().to_path_buf();
        Ok(config)
    }

    pub fn applications(&self) -> Vec<(HotKey, PathBuf)> {
        self.applications
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
            .collect::<Vec<(_, _)>>()
    }

    pub fn watch(&self, tx: Sender<()>) -> notify::Result<notify::RecommendedWatcher> {
        let mut last_event = None;
        let debounce_duration = Duration::from_millis(100);

        let mut watcher = recommended_watcher(move |result: Result<Event, _>| {
            if let Ok(event) = result {
                if event.kind.is_modify() {
                    let now = Instant::now();
                    if let Some(last) = last_event {
                        if now.duration_since(last) < debounce_duration {
                            return;
                        }
                    }
                    last_event = Some(now);
                    let _ = tx.send(());
                }
            }
        })?;

        let watch_path = self.path.parent().unwrap_or(&self.path);
        watcher.watch(watch_path, notify::RecursiveMode::NonRecursive)?;

        Ok(watcher)
    }
}
