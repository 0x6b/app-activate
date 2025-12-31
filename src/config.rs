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
use notify::{Event, RecommendedWatcher, Watcher, recommended_watcher};
use serde::Deserialize;
use toml::from_str;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub leader_key: String,
    pub applications: BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub secondary_applications: BTreeMap<String, PathBuf>,
    pub timeout_ms: u64,
    pub db: Option<PathBuf>,
    #[serde(skip)]
    pub(crate) path: PathBuf,
}

fn normalize_key(key: &str) -> String {
    if let Some(c) = key.chars().next() {
        if key.len() == 1 && c.is_ascii_alphabetic() {
            return format!("Key{}", c.to_ascii_uppercase());
        }
        if key.len() == 1 && c.is_ascii_digit() {
            return format!("Digit{c}");
        }
    }
    key.to_string()
}

impl Config {
    pub fn from<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path> + Debug,
    {
        let config_str = read_to_string(&path).unwrap_or_else(|why| {
            eprintln!("Failed to read config file at {path:?}: {why}");
            exit(1);
        });

        let mut config = from_str::<Config>(&config_str).unwrap_or_else(|why| {
            eprintln!("Failed to parse config file: {why}");
            exit(1);
        });

        config.path = path.as_ref().to_path_buf();
        Ok(config)
    }

    pub fn applications(&self) -> Vec<(HotKey, PathBuf)> {
        Self::process_applications(&self.applications)
    }

    pub fn secondary_applications(&self) -> Vec<(HotKey, PathBuf)> {
        Self::process_applications(&self.secondary_applications)
    }

    pub fn watch(&self, tx: Sender<()>) -> notify::Result<RecommendedWatcher> {
        let mut last_event = None;
        let debounce_duration = Duration::from_millis(100);

        let mut watcher = recommended_watcher(move |result: Result<Event, _>| {
            let Ok(event) = result else { return };
            if !event.kind.is_modify() {
                return;
            }
            let now = Instant::now();
            if let Some(last) = last_event {
                if now.duration_since(last) < debounce_duration {
                    return;
                }
            }
            last_event = Some(now);
            let _ = tx.send(());
        })?;

        let watch_path = self.path.parent().unwrap_or(&self.path);
        watcher.watch(watch_path, notify::RecursiveMode::NonRecursive)?;

        Ok(watcher)
    }

    fn process_applications(apps: &BTreeMap<String, PathBuf>) -> Vec<(HotKey, PathBuf)> {
        apps.iter()
            .map(|(key, path)| {
                let key = normalize_key(key);
                (HotKey::new(None, Code::from_str(&key).unwrap()), path.clone())
            })
            .collect()
    }
}
