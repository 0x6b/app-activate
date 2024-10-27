use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use notify::{recommended_watcher, Event, Watcher};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub leader_key: String,
    pub applications: BTreeMap<String, PathBuf>,
    pub timeout_ms: u64,
}

pub enum CustomEvent {
    ConfigChanged,
}

pub fn watch_config<P>(path: P, tx: Sender<()>) -> notify::Result<notify::RecommendedWatcher>
where
    P: AsRef<Path>,
{
    let path = path.as_ref().to_path_buf();
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

    let watch_path = path.parent().unwrap_or(&path);
    watcher.watch(watch_path, notify::RecursiveMode::NonRecursive)?;

    Ok(watcher)
}
